# layup-cli

The `layup` command-line tool, powered by the [layup layout engine](https://crates.io/crates/layup).

```sh
cargo install layup-cli --version 0.1.0 --locked
layup render diagram.layup
layup render diagram.layup --html
layup check diagram.layup --strict
layup build docs/
```

The installed executable is named `layup`. For use in Rust code, depend on
`layup` instead of `layup-cli`.

See the [project README](https://github.com/hxyulin/layup#readme) for examples,
font details, and documentation. Licensed under MIT OR Apache-2.0.
