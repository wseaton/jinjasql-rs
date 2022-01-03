# jinjasql-rs

WIP Rust port of the [JinjaSQL Python extension](https://github.com/sripathikrishnan/jinjasql) for Jinja2, powered by `minijinja`.


## TODO

- [ ] alternate param styles
  - currently only `numeric` is supported
- [ ] automatic `bind` filtering (depends on the ability to modify the parser stream, a-la `jinja2.Extension`)
- [ ] benchmarks
- [ ] better state encapsulation
- [ ] better tests
- [ ] publish to crates.io
- [ ] python bindings?
