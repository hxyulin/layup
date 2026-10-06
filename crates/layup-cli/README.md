# layup-cli

The `layup` command-line tool, powered by the [layup layout engine](https://crates.io/crates/layup).

```sh
cargo install layup-cli --version 0.4.0 --locked
layup render diagram.layup
layup render diagram.layup --html
layup check diagram.layup --strict
layup check guide.md --strict      # each ```layup block, at its Markdown line
layup lint diagram.layup --json    # structured source diagnostics
layup fmt diagram.layup --check   # formatting check without writing
layup fmt diagram.layup --write   # format in place after syntax validation
layup build docs/
layup compile diagram.layup -o scene.json
layup render model.layup --view detail --html
```

For a prebuilt binary, use [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```sh
cargo binstall layup-cli --version 0.4.0
```

Release binaries cover Linux x86_64/ARM64 (glibc 2.35+), macOS Intel/Apple Silicon,
and Windows x86_64. Other targets can build from source with `cargo install`.
See the [release archives and checksums](https://github.com/hxyulin/layup/releases/tag/v0.4.0).

The installed executable is named `layup`. For use in Rust code, depend on
`layup` instead of `layup-cli`.

Version 0.4 uses named diagrams and explicit style and paint properties.
Source files written for 0.3 need migration to the
[shared document language](https://hxyulin.github.io/layup/guide/language-v1.html).

Render/compile/check also accept structured graph
`.json` files. Use `--input-format graph` for render/compile on stdin.
This input format preserves original-code locations and analysis
evidence through scene and SVG/HTML exports. See
[generation from analysis](https://hxyulin.github.io/layup/guide/code-analysis.html)
for a Cargo adapter and generated overview/detail views.

Version 0.3 adds the `lint` and `fmt` commands.
See the [language-tools guide](https://github.com/hxyulin/layup/blob/main/docs/LANGUAGE-TOOLS.md)
for recovery, lint rules, source positions, stdin and batch formatting.

Version 0.3 also adds `compile` for versioned scene JSON, and global
`--view NAME` selection for shared models. Slide fitting and authored reveal
plans use the same compiler as SVG/HTML rendering. See the
[presentation and scene guide](https://github.com/hxyulin/layup/blob/main/docs/PRESENTATION.md)
and [sequence diagrams](https://github.com/hxyulin/layup/blob/main/docs/SEQUENCE.md).

See the [project README](https://github.com/hxyulin/layup#readme) for examples,
font details, and documentation. Licensed under MIT OR Apache-2.0.
