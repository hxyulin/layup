# Architecture

Layup has two Cargo packages: `layup-cli` provides the `layup` binary in
[`main.rs`](../crates/layup-cli/src/main.rs), and `layup` provides the layout
engine and renderers in [`lib.rs`](../crates/layup/src/lib.rs).
`layup-cli` depends on `layup`; the engine does not depend on the CLI or Clap. The CLI handles files, arguments,
diagnostics, and exit codes. The library compiles text into a diagram and
scene, then exposes SVG and HTML renderers.

![Layup's compilation and rendering pipeline](diagrams/architecture.svg)

This image is generated from [architecture.layup](diagrams/architecture.layup)
by layup itself. Run `just docs` after changing its source. `just check`
validates documentation diagrams and examples with warnings treated as errors.

## Compilation

`compile(source)` returns `Compiled { diagram, scene, warnings }`, or an
`Error` for invalid input. It does not read files or write output.

1. **Read syntax.** `lexer.rs` tokenizes the source; `parser.rs` builds generic
   items and edge statements. Syntax is separate from node-kind semantics.
2. **Resolve semantics.** `model.rs` builds a typed `Diagram`, resolves styles
   from `style.rs`, assigns node IDs, validates edge targets, and builds the
   legend.
3. **Lay out blocks.** `layout.rs` measures text, wraps prose, and places the
   authored rows and containers. `text/` reads glyph advances directly from
   the bundled IBM Plex font files. Layout records node rectangles and
   reserved areas that routing must avoid.
4. **Route edges.** `route.rs` chooses orthogonal paths, spreads ports, and
   places labels around nodes and reserved areas.
5. **Check the scene.** `check.rs` adds scene-level diagnostics, including
   overlapping edge segments and labels on nodes. These join warnings already
   produced during layout and routing.

Warnings are separate from compilation errors. The CLI's `--strict` option
turns warnings into failure and prevents that diagram from being rendered.
Library callers can inspect `Compiled::warnings` and choose their own policy.

## Rendering

`svg::render` serializes the compiled scene into an SVG with theme variables,
semantic node and edge attributes, and embedded font data. `html::render`
wraps that SVG with pan, zoom, selection, theme controls, and iframe messaging.
Neither renderer needs a network connection or installed fonts.

The font bytes used for measurement and embedding have one source in
`text/fonts.rs`. Parsed font faces are cached for reuse. Each SVG embeds
subsets that `text/subset.rs` cuts to the characters the scene draws. SVG
turns off kerning and optional ligatures to match advance-based measurement.
Missing glyphs still depend on viewer fallback fonts, and complex-script
shaping is not yet modeled. See [the font provenance](../crates/layup/fonts/README.md)
for upstream revision, checksums, and the separate SIL OFL license.

## Where to make changes

| Change | Starting point |
| --- | --- |
| Syntax | `lexer.rs`, `parser.rs` |
| Node kinds, defaults, themes | `model.rs`, `style.rs` |
| Text measurement and wrapping | `text/mod.rs`, `text/fonts.rs` |
| Block geometry | `layout.rs` |
| Edge paths and labels | `route.rs` |
| Diagram diagnostics | `check.rs` and the producing layout/routing code |
| SVG serialization | `svg.rs` |
| Browser interaction and embedding | `html.rs` |
| Commands and file handling | `main.rs` |

The [design reference](DESIGN.md) covers language syntax, routing details,
and the iframe protocol. The Rust code is MIT OR Apache-2.0; bundled fonts
retain their own license, included in the package and generated diagrams.
