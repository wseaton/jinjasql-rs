use lazy_static::lazy_static;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use minijinja::value::Value;
use minijinja::Error;
use minijinja::{Environment, State};

lazy_static! {
    static ref CONTEXT: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

static PARAM_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Clone)]
enum ParamStyle {
    Numeric,
    // QMark,
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
    env: Environment<'a>,
}

impl<'a> JinjaSql<'a> {
    pub fn builder() -> JinjaSqlBuilder {
        JinjaSqlBuilder::default()
    }

    // make our template from a query string, render it and return back the render query and param vec
    pub fn render_query(
        mut self,
        query: &'a str,
        context: Value,
    ) -> Result<(String, Vec<String>), Error> {
        self.env.add_template("tmp", &query)?;
        let tmpl = self.env.get_template("tmp")?;
        let res = tmpl.render(context)?;
        let ctx = CONTEXT.lock().unwrap().to_vec();

        // clear the context vector to make room for the next query
        CONTEXT.lock().unwrap().clear();
        // set the param index back to zero
        PARAM_COUNT.store(0, Ordering::SeqCst);

        Ok((res, ctx))
    }
}

// filter used for binding a single "naked" variable, outside of an in-clause or identity expression
// eg. WHERE date = {{ date }} => WHERE date = "2020-10-01"
pub fn bind(_state: &State, value: String) -> Result<String, Error> {
    let current_count = PARAM_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    CONTEXT.lock().unwrap().push(value.clone());
    Ok(format!("${}", current_count))
}

// filter used for generating in-clauses
// eg. WHERE thing IN ('cat', 'hat', 'bat')
pub fn bind_in_clause(_state: &State, value: Vec<String>) -> Result<String, Error> {
    let mut outputs: Vec<String> = Vec::new();

    for val in value {
        let current_count = PARAM_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        CONTEXT.lock().unwrap().push(val);
        outputs.push(format!("${}", current_count))
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
}

impl JinjaSqlBuilder {
    pub fn new() -> JinjaSqlBuilder {
        JinjaSqlBuilder {
            param_style: ParamStyle::default(),
            identifier_quote_character: IDQuoteChar::default(),
        }
    }

    // currently doesn't do anything, as only 'Numeric' is supported in pure rust,
    // eg. the rust-postgresql crate
    pub fn set_param_style(mut self, s: &str) -> JinjaSqlBuilder {
        let param_style = match s {
            // "pyformat" => ParamStyle::PyFormat,
            // "format" => ParamStyle::Format,
            // "asyncpg" => ParamStyle::AsyncPg,
            // "qmark" => ParamStyle::QMark,
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
            env,
        };

        j.env.add_filter("inclause", bind_in_clause);
        j.env.add_filter("bind", bind);
        j
    }
}

#[cfg(test)]
mod tests {
    use crate::JinjaSqlBuilder;
    use indoc::indoc;
    use minijinja::context;

    #[test]
    fn test_basic_render() {
        let j = JinjaSqlBuilder::new().build();

        let query_string = indoc! {"
        select * from
            (
            select * from {{ table_name | upper }}
            where x in {{ other_items | reverse | inclause }}
        )a
        where column in {{ items | inclause }}
    "};

        let (res, context) = j
            .render_query(
                query_string,
                context!(
        table_name => "mytable",
        items => vec!["a", "b", "c"],
        other_items => vec!["d", "e", "f"]),
            )
            .unwrap();

        println!("{}", res);
        println!("{:?}", context);
    }

    // test combining the inclause and naked bind
    #[test]

    fn test_complex_render() {
        let j = JinjaSqlBuilder::new().build();
        let query_string = indoc! {"
        select * from
            (
            select * from {{ table_name | upper }}
            where x in {{ other_items | inclause }}
        )a
        where column in {{ items | reverse | inclause }} and other_column = '{{ baz | bind }}'
        "};

        let (res, context) = j
            .render_query(
                query_string,
                context!(
                    table_name => "mytable",
                    items => vec!["a", "b", "c"],
                    other_items => vec!["d", "e", "f"],
                    baz => "baz"
                ),
            )
            .unwrap();

        println!("{}", res);
        println!("{:?}", context);
    }
}
