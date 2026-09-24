# layup

An authored-layout diagram tool and Rust library. You write a
`.layup` file that describes cards, containers, rows and typed edges;
`layup` lays it out with real font metrics, routes the edges, checks for
overflow and crossings, and writes a self-contained SVG or an
interactive HTML page.

See [the language and design reference](https://github.com/hxyulin/layup/blob/main/docs/DESIGN.md) for the rationale, the language
reference and the iframe protocol. The `examples/` directory includes architecture diagrams and a minimal
`hello.layup`.

### v0.2 development

On the development branch, `diagram "Title" layout=auto { ... }` infers
layers and rows from edges, including inside containers. Explicit rows and
section boundaries retain control. Omit the option to preserve authored flow.
Run `just preview-auto` to generate the first checkpoint's before/after gallery.
Add `below=id`, `same-layer=id`, or `beside=id` on nodes for more control
without writing rows; `just preview-hints` generates the second checkpoint.
Obstructed edges also try automatic ports and paths around nodes;
`just preview-routing` compares the old and new routers.
Disconnected graphs now occupy separate regions, and automatic-layout colors
stay tied to node IDs; `just preview-incremental` shows common edits before
and after this change.
SVGs embed only the glyphs they draw, and nodes accept `href=` links.
`packages/layup` builds the engine to WebAssembly for Node and browsers, with
a markdown-it plugin and a VitePress preset that render `layup` code blocks
at build time; see [its README](packages/layup/README.md) and
`examples/vitepress`.
These features are not in the published v0.1.0 crates.

### Architecture

Layup draws its own architecture diagram:

![Layup pipeline: parse source, resolve the model, lay out blocks with bundled fonts, route edges, check warnings, and render SVG or HTML.](https://raw.githubusercontent.com/hxyulin/layup/main/docs/diagrams/architecture.svg)

[Diagram source](https://github.com/hxyulin/layup/blob/main/docs/diagrams/architecture.layup)
· [Architecture walkthrough](https://github.com/hxyulin/layup/blob/main/docs/ARCHITECTURE.md)

Regenerate the diagram with `just docs`.

### Install

The `layup` crate is the layout engine. The `layup-cli` crate provides the
`layup` command.

```sh
cargo install layup-cli --version 0.1.0 --locked
```

Or build and install from a source checkout:

```sh
cargo build --release          # target/release/layup
cargo install --path crates/layup-cli
```

Rust stable, with no external runtime dependencies or font downloads.

### Use

```sh
layup render diagram.layup                    # diagram.svg
layup render diagram.layup --theme auto       # follows prefers-color-scheme
layup render diagram.layup --html             # interactive page
layup render diagram.layup --html --embed     # no toolbar, for an <iframe>
layup render - -o - < diagram.layup           # stdin to stdout
layup check docs/**/*.layup --strict          # CI: warnings fail
layup check docs/*.md                         # every ```layup block in Markdown
layup build docs/                             # every .layup gets an .svg beside it
```

`layup check` (and every render) reports text that overflows its box,
edges that cross a node, edges that share a line, and label chips that
sit on a node, each with the source line.

### A small example

Tones cycle automatically, so a first sketch needs no styling at all —
`examples/hello.layup` renders strict-clean as written:

```
diagram "Hello layup" {
  node api "Requests" { code "GET /items" }
  node worker "Worker" { code "poll()" }
  node store "Store" { code "items.put()" }

  api -> worker "sends"
  worker -uses-> store labeled
}
```

More examples demonstrate [service layers](examples/service-layers.layup),
a [job pipeline](examples/job-pipeline.layup), and
[storage contracts](examples/storage-contracts.layup). These are fictional
systems illustrating layout features, not claims about a real project's architecture.

### Embedding

```html
<iframe id="arch" src="storage-contracts.html?theme=auto" style="width:100%;border:0"></iframe>
<script>
  const frame = document.getElementById('arch');
  addEventListener('message', e => {
    if (e.data?.layup === 'ready') frame.style.height = Math.min(e.data.height, 800) + 'px';
    if (e.data?.layup === 'select') console.log('selected', e.data.id);
  });
  // later: frame.contentWindow.postMessage({ layup: 'focus', id: 'store' }, '*');
</script>
```

`examples/embed-host.html` is a complete host page. Open it from a local
server (`just serve`) since browsers block `postMessage` between
`file://` documents.

### Development

```sh
just check       # fmt, clippy, tests, and all diagrams with --strict
just examples    # render examples/*.layup to examples/*.svg and out/*.html
just docs        # regenerate the architecture diagram
```

### Fonts and measurement

IBM Plex Sans (regular and semibold) and IBM Plex Mono are bundled in the
library and embedded in each SVG, including SVGs inside HTML output. Layout
reads advances directly from those same font files, with no system-font
lookup, generated metrics tables, or platform safety factor. Kerning and
optional ligatures are disabled in the SVG to match advance-based measurement.

Each output embeds only the glyphs it draws, about 12 KB per face for a typical
diagram, together with the SIL Open Font License notice, so it views offline.
The OFL reserves the Plex name for unmodified fonts, so the embedded subsets
are named Layup Sans and Layup Mono. Element IDs carry a per-diagram prefix,
so several SVGs can be inlined in one page. Use a browser or SVG renderer
that supports embedded web fonts. Glyphs outside the bundled fonts use the
viewer's fallback fonts and estimated widths; complex-script shaping is not
yet supported by the layout engine.

### Rust library

```toml
[dependencies]
layup = "0.1.0"
```

```rust
let diagram = layup::compile(r#"diagram "Hello" { node api "API" }"#)?;
let svg = layup::svg::render(&diagram, layup::Theme::Light);
# Ok::<(), layup::Error>(())
```

### License

The Rust code is licensed under either [MIT](https://github.com/hxyulin/layup/blob/main/LICENSE-MIT)
or [Apache-2.0](https://github.com/hxyulin/layup/blob/main/LICENSE-APACHE), at your option.
Bundled IBM Plex fonts are separately licensed under the SIL Open Font License
1.1; see `crates/layup/fonts/OFL.txt` and `crates/layup/fonts/README.md` for
license, source revision, and checksums.

### Release checks

```sh
just check
cargo publish -p layup --dry-run
# After layup 0.1.0 is available on crates.io:
cargo publish -p layup-cli --dry-run
```

Publish `layup` (the engine, fonts, and renderers) first, then `layup-cli`
(the command-line interface). The CLI depends on the released engine version.
Publishing to crates.io is a separate release step.
