# layup

The Rust layout engine behind [Layup](https://hxyulin.github.io/layup/), a
text-based diagram tool for explaining software. It compiles the Layup DSL
into measured geometry and renders self-contained SVG, interactive HTML or
versioned scene JSON.

## Install

```sh
cargo add layup@0.3.0
```

For the command-line tool, install the `layup-cli` crate instead:

```sh
cargo install layup-cli --version 0.3.0 --locked
# Or install a supported prebuilt binary:
cargo binstall layup-cli --version 0.3.0
```

## Render a diagram

```rust
fn main() -> Result<(), layup::Error> {
    let source = r#"diagram "A request" layout=auto direction=right {
        process client "Client" blue
        process api "API" green
        client -> api "Request"
    }"#;
    let compiled = layup::compile(source)?;
    let svg = layup::svg::render(&compiled, layup::Theme::Auto);
    println!("{svg}");
    Ok(())
}
```

The engine supports authored/automatic layouts in all four graph directions,
Unicode and RTL text, decisions, state machines, sequence diagrams, slide
viewports, presentation steps and shared models with named views. Public
modules also expose the lexer/parser, formatter, linter and scene exporter.

See the [API guide](https://hxyulin.github.io/layup/reference/api.html),
[DSL reference](https://hxyulin.github.io/layup/reference/dsl.html) and
[Rust API documentation](https://docs.rs/layup/0.3.0/layup/).
Try sources in the [playground](https://hxyulin.github.io/layup/playground.html).

## Fonts and license

Latin, Arabic and Hebrew fonts are bundled. CJK uses system fallback or
user-supplied font bytes; no CJK font is bundled.

Code is licensed under MIT OR Apache-2.0. Bundled fonts use SIL OFL 1.1;
see `fonts/OFL.txt` and the
[font notes](https://github.com/hxyulin/layup/blob/v0.3.0/crates/layup/fonts/README.md).
