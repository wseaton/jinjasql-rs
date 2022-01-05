# jinjasql-rs

WIP Rust port of the [JinjaSQL Python extension](https://github.com/sripathikrishnan/jinjasql) for Jinja2, powered by `minijinja`.

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
