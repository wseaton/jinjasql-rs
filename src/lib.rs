#[macro_use]
extern crate ref_thread_local;
use ref_thread_local::RefThreadLocal;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use minijinja::value::Value;
use minijinja::Error;
use minijinja::{Environment, State, Source};

ref_thread_local! {
    static managed CONTEXT: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static managed PARAM_COUNT: AtomicUsize = AtomicUsize::new(0);
    static managed PARAM_STYLE: ParamStyle = ParamStyle::default();
}

#[derive(Debug, PartialEq, Clone)]
enum ParamStyle {
    Numeric,
    QMark,
    // Named,
    // Format,
    // PyFormat,
    // AsyncPg,
}

impl Default for ParamStyle {
    fn default() -> Self {
        ParamStyle::Numeric
    }
}

#[derive(Debug, PartialEq, Clone)]
enum IDQuoteChar {
    Backtick,
    DoubleQuote,
}

impl Default for IDQuoteChar {
    fn default() -> Self {
        IDQuoteChar::DoubleQuote
    }
}

#[derive(Debug, Clone)]
pub struct JinjaSql<'a> {
    param_style: ParamStyle,
    identifier_quote_character: IDQuoteChar,
    // currently the top two are not used, but placeholders for settings.
    env: Environment<'a>
}

impl<'a> JinjaSql<'a> {
    pub fn builder() -> JinjaSqlBuilder {
        JinjaSqlBuilder::default()
    }

    fn hash_query(query: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(query);
        let hash = hasher.finalize();
        format!("hash:{:x}", hash)
    }

    // make our template from a query string (or template path)
    // render it and return back the render query and param vec
    pub fn render_query(
        mut self,
        query: Option<&'a str>,
        template_name: Option<&str>,
        context: Value,
    ) -> Result<(String, Vec<String>), Error> {
        
        let tmpl =  
            if let Some(q) = query {
                let name = JinjaSql::hash_query(q);
                // TODO: don't leak (if possible)
                // we are leaking memory here due to lifetimes, the name of the 
                // string has to live for the duration of the environment, which we "solve"
                // by just making it live for the lifetime 'a, which is effectively the length
                // of the entire progrm. 
                let long_lived_name: &'a str = Box::leak(name.into_boxed_str());
                self.env.add_template(long_lived_name, q)?;
                self.env.get_template(long_lived_name)?
            } else if let Some(b) = template_name {
                self.env.get_template(b)?
            } else {
                panic!("Unreconcilable issue with template args.")
        };
        
        let res = tmpl.render(context)?;
        let ctx = CONTEXT.borrow().lock().unwrap().to_vec();

        // clear the context Vec to make room for the next query
        CONTEXT.borrow().lock().unwrap().clear();
        // set the param index back to zero
        PARAM_COUNT.borrow().store(0, Ordering::SeqCst);

        Ok((res, ctx))
    }
}

// filter used for binding a single "naked" variable, outside of an in-clause or identity expression
// eg. WHERE date = {{ date }} => WHERE date = "2020-10-01"
pub fn bind(_state: &State, value: String) -> Result<String, Error> {    
    let current_count = PARAM_COUNT.borrow().fetch_add(1, Ordering::SeqCst) + 1;
    CONTEXT.borrow().lock().unwrap().push(value.clone());
    
    match *PARAM_STYLE.borrow() {
        ParamStyle::Numeric => {
            Ok(format!("${}", current_count))
        },
        ParamStyle::QMark => Ok("?".to_string())
    }
}

// filter used for generating in-clauses
// eg. WHERE thing IN ('cat', 'hat', 'bat')
pub fn bind_in_clause(_state: &State, value: Vec<String>) -> Result<String, Error> {
    let mut outputs: Vec<String> = Vec::new();

    for val in value {
        let current_count = PARAM_COUNT.borrow().fetch_add(1, Ordering::SeqCst) + 1;
        CONTEXT.borrow().lock().unwrap().push(val);
        
        let pushed = match *PARAM_STYLE.borrow() {
            ParamStyle::Numeric => format!("${}", current_count),
            ParamStyle::QMark => "?".to_string()
        };
        outputs.push(pushed)
    }

    let final_output = outputs.join(", ");
    let res = format!("({})", &final_output);

    Ok(res)
}

#[derive(Default)]
pub struct JinjaSqlBuilder {
    // Probably lots of optional fields.
    param_style: ParamStyle,
    identifier_quote_character: IDQuoteChar,
    source: Source
}

impl JinjaSqlBuilder {
    pub fn new() -> JinjaSqlBuilder {
        JinjaSqlBuilder {
            param_style: ParamStyle::default(),
            identifier_quote_character: IDQuoteChar::default(),
            source: Source::default()
        }
    }

    pub fn set_source(mut self, source: Source) -> JinjaSqlBuilder {
        self.source = source;
        self
    }

    // currently doesn't do anything, as only 'Numeric' is supported in pure rust,
    // eg. the rust-postgresql crate
    pub fn set_param_style(mut self, s: &str) -> JinjaSqlBuilder {
        let param_style = match s {
            // "pyformat" => ParamStyle::PyFormat,
            // "format" => ParamStyle::Format,
            // "asyncpg" => ParamStyle::AsyncPg,
            "qmark" => ParamStyle::QMark,
            "numeric" => ParamStyle::Numeric,
            // "named" => ParamStyle::Named,
            _ => {
                println!("Paramstyle {} not found. Falling back to default.", s);
                ParamStyle::default()
            }
        };

        self.param_style = param_style;
        self
    }

    pub fn set_identifier_quote_character(mut self, c: &str) -> JinjaSqlBuilder {
        let iqc = match c {
            "`" => IDQuoteChar::Backtick,
            r#"""# => IDQuoteChar::DoubleQuote,
            _ => {
                println!("Quote char {} not found. Falling back to default!", c);
                IDQuoteChar::default()
            }
        };

        self.identifier_quote_character = iqc;
        self
    }

    pub fn build(self) -> JinjaSql<'static> {
        let env = Environment::new();
        let mut j = JinjaSql {
            param_style: self.param_style,
            identifier_quote_character: self.identifier_quote_character,
            env
        };

        // set the param style
        let mut x = PARAM_STYLE.try_borrow_mut().unwrap();
        *x = j.param_style.clone();

        j.env.add_filter("inclause", bind_in_clause);
        j.env.add_filter("bind", bind);
        j.env.set_source(self.source);
        j
    }
}

#[cfg(test)]
mod tests {
    use crate::JinjaSqlBuilder;
    use indoc::indoc;
    use minijinja::{context, Source};

    #[test]
    fn test_basic_render_source() {
        let mut s = Source::new();
        s.load_from_path("./templates", &["j2"]).unwrap();
        
        let j = JinjaSqlBuilder::new().set_param_style("qmark").set_source(s).build();        
        let (res, params) = j
            .render_query(
                None,
                Some("basic.sql.j2"),
                context!(
        table_name => "mytable",
        items => vec!["a", "b", "c"],
        other_items => vec!["d", "e", "f"]),
            )
            .unwrap();

        println!("{}", res);
        println!("{:?}", params);
    }


    #[test]
    fn test_basic_render() {
        let j = JinjaSqlBuilder::new().build();

        let query_string = indoc! {"
            select * from {{ table_name | upper }}
            where x in {{ other_items | reverse | inclause }}
    "};

        let (res, params) = j
            .render_query(
                Some(query_string),
                None,
                context!(
        table_name => "mytable",
        items => vec!["a", "b", "c"],
        other_items => vec!["d", "e", "f"]),
            )
            .unwrap();

        println!("{}", res);
        println!("{:?}", params);
    }

    // test combining the inclause and naked bind
    #[test]
    fn test_complex_render() {
        let j = JinjaSqlBuilder::new().build();
        let query_string = indoc! {"
        select{% for col in columns %}
            {{ col }}{%- if not loop.last %},{% endif %}{% endfor %}
        from
            (
            select {% for col in columns %}
            {{ col }}{%- if not loop.last %},{% endif %}{% endfor %} from {{ table_name | upper }}
            where sku in {{ skus | inclause }}
        )a
        where 
            tag in {{ tags | reverse | inclause }}
            and stock_date = {{ baz | bind }}
        "};

        let (res, params) = j
        .render_query(
            Some(query_string),
            None,
                context!(
                    columns => vec!["apple", "lettuce", "lemon"],
                    table_name => "orders.stock_data",
                    tags => vec!["moldy", "sweet", "fresh"],
                    skus => vec!["EE-001", "EA-001", "BA-001"],
                    baz => "2022-01-01"
                ),
            )
            .unwrap();

        println!("{}", res);
        println!("{:?}", params);

        assert_eq!(
            res,
            indoc! { 
        "select
            apple,
            lettuce,
            lemon
        from
            (
            select 
            apple,
            lettuce,
            lemon from ORDERS.STOCK_DATA
            where sku in ($1, $2, $3)
        )a
        where 
            tag in ($4, $5, $6)
            and stock_date = $7"
            }
        );

        assert_eq!(
            params,
            vec![
                "EE-001",
                "EA-001",
                "BA-001",
                "fresh",
                "sweet",
                "moldy",
                "2022-01-01"
            ]
        );
    }

    // test to ensure that our 'globals' implementation is thead-safe
    #[test]
    fn test_basic_render_threads() {
        use std::thread;

        let query_string = indoc! {"
            select * from {{ table_name | upper }}
            where x in {{ other_items | inclause }}
    "};

        thread::spawn(move || {
            println!("Running from thread!");
            let j = JinjaSqlBuilder::new().build();
            let (res, params) = j
            .render_query(
                Some(query_string),
                None,
                    context!(
                table_name => "thread",
                items => vec!["a", "b", "c"],
                other_items => vec!["d", "e", "f"]),
                )
                .unwrap();
            println!("{}", res);
            println!("{:?}", params);
        });

        println!("Running from main!");
        let j = JinjaSqlBuilder::new().build();
        let (res, params) = j
        .render_query(
            Some(query_string),
            None,
                context!(
            table_name => "main",
            items => vec!["a", "b", "c"],
            other_items => vec!["d", "e", "f"]),
            )
            .unwrap();
        println!("{}", res);
        println!("{:?}", params);
    }
}
