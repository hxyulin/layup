# Architecture

Layup has two published Cargo packages: `layup-cli` provides the `layup`
binary in [`main.rs`](../crates/layup-cli/src/main.rs), and `layup` provides
the layout engine and renderers in [`lib.rs`](../crates/layup/src/lib.rs).
`layup-cli` depends on `layup`; the engine does not depend on the CLI or Clap. The CLI handles files, arguments,
diagnostics, and exit codes. The library compiles text into a diagram and
scene, then exposes SVG and HTML renderers.

For the web, [`layup-wasm`](../crates/layup-wasm/src/lib.rs) wraps the engine
in a small C ABI that returns JSON, and the
[`@hxyulin/layup`](../packages/layup/README.md) npm package loads it in Node
or a browser. That package adds the markdown-it plugin and VitePress preset,
which render code blocks at build time, and `client.js`, which adds
interaction to the rendered diagrams.

![Layup's compilation and rendering pipeline](diagrams/architecture.svg)

This image is generated from [architecture.layup](diagrams/architecture.layup)
by layup itself. Run `just docs` after changing its source. `just check`
validates documentation diagrams and examples with warnings treated as errors.

## Compilation

`compile(source)` returns `Compiled { diagram, scene, warnings }`, or an
`Error` for invalid input. It does not read files or write output.

1. **Read syntax.** `lexer.rs` tokenizes the source; `parser.rs` builds generic
   items and edge statements. Syntax is separate from node-kind semantics.
   Tokens and syntax nodes carry UTF-8 byte spans and scalar columns. The
   recovering parser collects sibling errors; compilation uses strict parsing.
2. **Resolve semantics.** `model.rs` builds a typed `Diagram`, resolves styles
   from `style.rs`, assigns node IDs, validates edge targets, and builds the
   legend. `machine.rs` adds opt-in scoped-machine validation and unreachable
   state warnings, including composite initial paths. Each scope projects
   descendant transitions onto its immediate members; traversal removes back edges only from placement
   constraints, preserving all original transitions for routing.
3. **Lay out blocks.** `sizing.rs` computes font-aware preferred widths and
   grows unpinned automatic canvases. `layout.rs` measures text, wraps prose, and places the
   authored rows and containers. `text/` shapes labels against bundled IBM Plex faces or supplied
   fallback font bytes. Compact decision trees reserve subtree lanes in any
   direction, packing measured subtrees when the canvas is unpinned. Composite
   states use rounded frames with protected headers and child channels. `geometry.rs` shares diamond/rounded outlines and safe text
   areas with routing and rendering. Layout records node rectangles, outlines, and
   reserved areas that routing must avoid.
4. **Route edges.** `route.rs` chooses orthogonal paths, spreads ports, and
   places labels around nodes and reserved areas. Rectangles conservatively
   guide route searches; final endpoints extend to the actual shape boundary.
   Decision captions search all route segments and avoid endpoint fills.
5. **Check the scene.** `check.rs` adds scene-level diagnostics, including
   overlapping edge segments and labels on nodes. These join warnings already
   produced during layout and routing.

Warnings are separate from compilation errors. The CLI's `--strict` option
turns warnings into failure and prevents that diagram from being rendered.
Library callers can inspect `Compiled::warnings` and choose their own policy.

## Rendering

`diagnostic.rs` shares source excerpts and JSON diagnostics across frontends.
`format.rs` uses lossless token slices after syntax validation, without model
resolution. `lint.rs` returns recovering syntax errors, or compiles valid
syntax and adds authoring rules to existing warnings. Semantic validation
still stops at its first error. See [the language-tools guide](LANGUAGE-TOOLS.md).

`svg::render` serializes the compiled scene into an SVG with theme variables,
semantic node and edge attributes, and embedded font data. `html::render`
wraps that SVG with pan, zoom, selection, theme controls, and iframe messaging.
Neither renderer needs a network connection. CJK uses viewer fonts by
default; explicitly supplied fallback faces are embedded for offline use.

The font bytes used for measurement and embedding have one source in
`text/fonts.rs` and the optional `text::Fonts` supplied at compilation.
Bundled parsed faces are cached for reuse. Latin SVG fonts are subset to the
characters drawn; Arabic/Hebrew and user faces retain their shaping tables.
SVG and Rustybuzz both disable kerning and optional ligatures.
CJK uses system fonts and width estimates by default. Supplying a fallback
face enables measured CJK output without bundling that font in the library.
See [the font provenance](../crates/layup/fonts/README.md) for upstream
revisions, checksums, and the separate SIL OFL license.

## Where to make changes

| Change | Starting point |
| --- | --- |
| Syntax | `lexer.rs`, `parser.rs` |
| Source diagnostics, linting, formatting | `diagnostic.rs`, `lint.rs`, `format.rs` |
| Node kinds, defaults, themes | `model.rs`, `style.rs` |
| Shape boundaries, ports, safe label areas | `geometry.rs` |
| Flat machine checks and cycle placement constraints | `machine.rs` |
| Text measurement and wrapping | `text/mod.rs`, `text/fonts.rs` |
| Block geometry | `layout.rs` |
| Edge paths and labels | `route.rs` |
| Diagram diagnostics | `check.rs` and the producing layout/routing code |
| SVG serialization | `svg.rs` |
| Browser interaction and embedding | `html.rs` |
| Commands and file handling | `main.rs` |
| WebAssembly interface | `crates/layup-wasm/src/lib.rs`, `packages/layup/core.js` |
| Markdown and VitePress | `packages/layup/markdown-it.js` |
| In-page interaction | `packages/layup/client.js` |

The [design reference](DESIGN.md) covers language syntax, routing details,
and the iframe protocol. The Rust code is MIT OR Apache-2.0; bundled fonts
retain their own license, included in the package and generated diagrams.
