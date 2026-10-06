# CLI, Rust, and JavaScript

Layup 0.4 uses the named-diagram DSL documented here. Source files written for
0.3 need migration. All frontends compile the same DSL with the same text
metrics and layout.
The CLI adds filesystem operations; Rust exposes the model and scene;
JavaScript loads the actual Rust engine compiled to WASM.

## CLI

Install with `cargo install layup-cli --version 0.4.0 --locked`, or
`cargo binstall layup-cli --version 0.4.0` for a supported prebuilt binary.
See [getting started](/guide/getting-started#_3-render-a-file) for platform details.

```sh
layup render diagram.layup --theme auto
layup render diagram.layup --html --embed
layup render model.layup --view detail -o detail.svg
layup compile model.layup --view detail -o detail.json
layup check docs/*.md --strict
layup lint diagram.layup --json
layup inspect document.layup
layup fmt diagram.layup --check
layup build docs/ --html
```

`--font PATH` can repeat. `--view NAME` applies to render, compile, check, lint,
and build. `-` reads stdin or writes stdout where supported. Render defaults
to a neighboring SVG/HTML file; compile defaults to stdout. Use `--help` on
each command for all options.

Version 0.4 also accepts semantic graph `.json` files in render,
compile and check. For render/compile on stdin, use `--input-format graph`.
See [generation from analysis](/guide/code-analysis) for the input contract
and Node-only Cargo adapter.

## Rust

Add the published engine to your project:

```sh
cargo add layup@0.4.0
```

```rust
let compiled = layup::compile(source)?;
let svg = layup::svg::render(&compiled, layup::Theme::Auto);
let html = layup::html::render(&compiled, layup::Theme::Auto, false);
let json = layup::scene::export(&compiled)?;
```

For supplied fonts and a named view:

```rust
let mut fonts = layup::text::Fonts::default();
fonts.add_fallback(std::fs::read("CJK-Regular.ttf")?)?;
let options = layup::CompileOptions { view: Some("detail".into()), ..Default::default() };
let compiled = layup::compile_with_options(source, &options, &fonts)?;
```

`Compiled` exposes `diagram`, `scene`, `warnings`, `slide`, `presentation`,
`views`, `selected_view`, `sequence`, and optional original structured `input`. `viewport()` returns output dimensions
without modifying scene coordinates. Layout warnings are separate from errors;
callers choose their own warning policy.

For structured graph input, `layup::input::parse(json)` returns a typed `Graph`.
`input::compile(&graph, &options, &fonts)` and
`input::compile_json(json, &options, &fonts)` return the same `Compiled` type.
These APIs validate the versioned contract and preserve original code locations.

The typed document parser, formatter, linter and scene exporter are public
modules. The old generic lexer/parser is an internal lowering implementation.
For types and exhaustive drawing contracts, build Rust docs with
`cargo doc --workspace --no-deps` or inspect the
[crate sources](https://github.com/hxyulin/layup/tree/main/crates/layup/src).

## JavaScript and WASM

Install the published JavaScript/WASM package:

```sh
pnpm add @hxyulin/layup@0.4.0
# Or: npm install @hxyulin/layup@0.4.0
```

The package includes the compiled engine, so installation does not require
Rust. Version 0.4.0 includes `compile`, `lint`, `format`, named views and the
same diagram features as the CLI and playground.

```js
import { load } from '@hxyulin/layup';
const engine = await load();
const { output, warnings } = engine.render(source, { theme: 'auto' });
const scene = engine.compile(source);
const formatted = engine.format(source);
const diagnostics = engine.lint(source);
```

| Method | Returns |
| --- | --- |
| `render(source, options)` | `{ output, warnings }`; format `svg`, `html`, or `embed` |
| `compile(source, options)` | Version-1 scene object |
| `inspect(source)` | Partial typed document and diagnostics; no layout |
| `format(source)` | Formatted source string |
| `lint(source, options)` | Structured diagnostics |
| `compileModel(model, options)` | Version-1 scene from semantic graph input (current checkout) |
| `renderModel(model, options)` | SVG/HTML/embed and warnings from semantic graph input (current checkout) |

Render accepts `theme`, `format`, `darkSelector`, `fonts`, and `view`.
Compile/lint accept `fonts` and `view`. In the current checkout these operations
also accept `diagram` to select a diagram in a
[named document](/guide/language-v1); the CLI uses `--diagram ID` and Rust
uses `CompileOptions.diagram`. `layup::document::parse` exposes its typed syntax
tree, and `Compiled.document` exposes selected identity and annotation data.
Graph, sequence and state-machine sources share this contract. Entity metadata
retains `declaration`, typed `attributes`, authored identities, annotations and
resolved `paint`. Scene nodes and edges expose nullable paint with separate
fill, stroke, text, stroke style and width; diagram paint carries the canvas
background. Color values distinguish literals, theme tokens and light/dark
pairs. Structured graph version 1 keeps its existing kind/tone contract.
Font bytes are `Uint8Array` values.
`LayupError` includes rich source locations, diagnostic code, help, and
related ranges.

Node also provides synchronous `loadSync()` and can supply fonts through
`readFileSync`. In browsers, `load(url)` fetches the WASM asset; its default
URL is beside the module. A bundler or host must make that asset available.
This site's editor imports it as a Vite asset and runs compilation in a Web
Worker, keeping the page responsive during layout.

Rendering is synchronous after loading. For repeated or large edits, debounce
calls and use a worker. Fonts apply per operation. WASM is not a JavaScript
reimplementation: CLI and browser parity tests exercise the same engine.

The [package README](https://github.com/hxyulin/layup/blob/main/packages/layup/README.md)
and [TypeScript definitions](https://github.com/hxyulin/layup/blob/main/packages/layup/index.d.ts)
describe the module exports and complete scene types.


## Named document inspection (current checkout)

`layup::document::parse(source)` accepts named document syntax with an optional
`layup 1` revision assertion. `document::parse_recovering(source)` returns a
partial typed document and errors; `document::inspect(source)` returns the JSON
inspection contract. In JavaScript, `engine.inspect(source)` returns the same
`{document, diagnostics}` object. These operations preserve opaque bodies and
extension annotations without measuring text or requiring a renderer.

Values distinguish choices, strings, signed/unsigned 64-bit integers, finite
floats, booleans, null, lists, records and references. Tagged JSON integers use
decimal strings to retain exact values in JavaScript. Source and value spans
remain UTF-8 byte offsets and Unicode scalar positions. Compilation rejects
syntax errors even when inspection recovers valid siblings.

`Scene.document` includes a supported/selected/skipped manifest, rich document
diagnostics, ordered annotations on rendered objects and annotated non-rendered
targets. Full opaque bodies appear in inspection, rather than rendered metadata.
See [named document syntax](/guide/language-v1) for detailed behavior and the
remaining extension checkpoints.
