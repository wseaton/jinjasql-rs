# jinjasql-rs

WIP Rust port of the [JinjaSQL Python extension](https://github.com/sripathikrishnan/jinjasql) for Jinja2, powered by `minijinja`.

## Usage

The `inclause` and `bind` filters are the main attraction here, as they allow us to keep track of the variables injected into the SQL statement and stash them for passing into the client at query time via params.

```rust
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
```

returns:

```sql
select
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
    and stock_date = $7

-- ["EE-001", "EA-001", "BA-001", "fresh", "sweet", "moldy", "2022-01-01"]
```

## TODO

- [x] alternate param styles
- [ ] automatic `bind` filtering (depends on the ability to modify the parser stream, a-la `jinja2.Extension`)
- [ ] benchmarks
- [x] better state encapsulation
  - solved for now until a solution for [mutable filter state](https://github.com/mitsuhiko/minijinja/issues/42) stabilizes
- [ ] better tests
- [ ] publish to crates.io
  - [ ] docs
- [ ] python bindings?
