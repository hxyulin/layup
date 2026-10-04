# layup

An authored-layout diagram tool and Rust library. You write a
`.layup` file that describes cards, containers, rows and typed edges;
`layup` lays it out with real font metrics, routes the edges, checks for
overflow and crossings, and writes a self-contained SVG or an
interactive HTML page.

See [the language and design reference](https://github.com/hxyulin/layup/blob/main/docs/DESIGN.md) for the rationale, the language
reference and the iframe protocol. The `examples/` directory includes architecture diagrams and a minimal
`hello.layup`.

### New in 0.3

Automatic graph flow in all four directions, Unicode/RTL text with optional
user fonts, decision trees, state machines with composite scopes, and
font-aware automatic sizing. Source tooling now includes recovering syntax
diagnostics, typo suggestions, linting, and formatting in the CLI and
JavaScript/WASM API. The [changelog](CHANGELOG.md) covers the features and
API changes, and the [language-tools guide](docs/LANGUAGE-TOOLS.md) explains
the new tooling.

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
cargo install layup-cli --version 0.3.0 --locked
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
layup lint diagram.layup --json               # source diagnostics for tools
layup fmt diagram.layup --check               # formatting check, without writing
layup build docs/                             # every .layup gets an .svg beside it
```

`layup check` (and every render) reports text that overflows its box,
edges that cross a node, edges that share a line, and label chips that
sit on a node, each with the source line.

Layup 0.3 provides `layup lint` for recovering syntax checks,
semantic/layout diagnostics and authoring rules, and `layup fmt` for source
formatting that preserves comments and quoted text. Errors include source
ranges, excerpts and typo suggestions. See [the language-tools guide](docs/LANGUAGE-TOOLS.md)
for Rust/JavaScript APIs, JSON locations and syntax compatibility.

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

More examples demonstrate [service layers](https://github.com/hxyulin/layup/blob/main/examples/service-layers.layup),
a [job pipeline](https://github.com/hxyulin/layup/blob/main/examples/job-pipeline.layup), and
[storage contracts](https://github.com/hxyulin/layup/blob/main/examples/storage-contracts.layup). These are fictional
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

### Direction and international text

Automatic graphs accept `direction=down|up|right|left` (default `down`).
Labels have a separate `text-direction=auto|ltr|rtl`, on the diagram or an
individual node. Node overrides also apply to their descendants. Automatic
text direction follows the paragraph's first strong character; code spans
are isolated LTR runs. Default node alignment follows the resolved text
direction; `align=left|right|center|start` overrides it.

```text
diagram "معالجة الطلبات" layout=auto direction=left text-direction=rtl {
  node request "طلب جديد"
  node worker "المعالجة" { sub "يستخدم العامل `GET /items` لقراءة البيانات." }
  node result "النتيجة"
  request -> worker
  worker -> result
}
```

Authored rows and section order stay fixed. Within automatic regions, ranks
follow the selected direction and peers occupy the perpendicular axis.
`after=` is a direction-neutral alias of `below=`; `same-layer=` and `beside=`
constrain peers. See `examples/directions.layup`, `examples/international.layup`,
and `examples/right-to-left.layup`. The [next-capabilities roadmap](docs/ROADMAP.md)
tracks layout, state machines, and output formats.

Under the default `clean` preset, automatic layout computes the canvas width
from font measurements, tree extents, rank widths, and container padding.
The canvas starts at 900px and grows for wide trees or long horizontal
chains. Compact nodes use content widths and trees pack uneven subtrees
around their measured extents. Set `width=N` to keep a fixed canvas; authored
rows and the `manual` preset retain their sizing policy.

### Decision trees and flowcharts

Use `decision` for a diamond question, `terminal` for a start or outcome,
and `process` for a rounded step. All three are compact, centered nodes
with the same text, styling, link, and edge options as cards.

```text
diagram "Approve a request" layout=auto direction=down {
  decision ready "Are all requirements met?"
  terminal approve "Approve" green
  terminal followup "Request more information" orange
  ready -> approve "Yes"
  ready -> followup "No"
}
```

Automatic regions consisting of these compact kinds use subtree lanes when
they form a single directed tree containing a decision: each question stays
centered over its descendants, and separate branches keep their own space.
Child declaration order determines branch order. All four graph directions
work. Merges, cycles, mixed card/flowchart regions, and explicit placement
hints use the existing graph layout. Authored rows and sections remain
available. Put related trees in separate `group` or `section` blocks.

Diamond text wraps within a safe inner area. Arrows meet the sloping or
rounded boundary, and branch captions stay upright and avoid node outlines.
If a caption cannot fit, rendering reports a warning; use a wider canvas,
shorter label, or placement hints. Compact shapes accept title/code/prose,
but child blocks belong in a surrounding container. Custom kinds can use
`shape=decision` (or `diamond`), `process`, or `terminal`.

See [the support decision tree](examples/decision-tree.layup),
[request validation](examples/decision-flow.layup), and
[multilingual decisions](examples/decision-international.layup).
Run `just decision-test` to generate a Chromium review gallery and check
text against the rendered outlines.

### State machines

`mode=state-machine` adds state-machine checks and enables automatic
layout by default. Use `state` for a rounded state, `initial` for the filled
start dot, `final` for the bullseye, and `choice` for a diamond pseudostate:

```text
diagram "Job lifecycle" mode=state-machine direction=right {
  initial start
  state idle "Idle"
  state running "Running" { sub "entry / begin()" }
  final done

  start -> idle
  idle -> running "start / begin()"
  running -> running "tick / update()"
  running -> idle "reset"
  running -> done "finish [completed]"
}
```

Transition labels follow the optional convention `event [guard] / action`.
They remain display text; rendering does not execute a machine or evaluate
guards. `choice allowed` draws a small unlabeled diamond; an optional title
turns it into a larger labeled diamond. Put guard labels on outgoing edges.
Initial and final markers take IDs and styling, without text bodies.

Machine mode requires one initial marker per scope with one outgoing transition,
rejects transitions into an initial or out of a final marker, requires two
or more outgoing transitions from a choice, and warns about states that
cannot be reached from the initial marker. A continuously running machine
can omit a final marker. Custom kinds can inherit `state` or the marker
shapes. These semantic checks apply only in machine mode.

Cycles get forward placement from the initial marker while return
transitions and self-loops remain visible. Node declaration order controls
traversal; reordering edge statements preserves state placement. All four
directions, explicit ports, placement hints, `layout=manual`, and authored
rows remain available. Structural groups organize nodes without introducing
a new machine scope.

Nest states to make a composite with its own initial marker:

```text
diagram "Connection" mode=state-machine direction=right {
  initial start
  state offline "Offline"
  state connected "Connected" {
    initial enter
    state ready "Ready"
    state sending "Sending"
    enter -> ready
    ready -> sending "send"
    sending -> ready "ack"
  }
  start -> offline
  offline -> connected "connect"
  connected -> offline "disconnect"
}
```

A composite draws a rounded frame with a separate title/action header.
Initial transitions stay within their own scope; regular transitions can
enter a composite, target a descendant directly, or leave a nested state.
IDs remain unique across the diagram. Final markers finish their containing
scope; transitions can enter a final from that scope or a descendant scope.
Reachability follows declared transitions and composite initial paths,
without evaluating guards or actions. Parallel regions, history, and
simulation remain future work.

See [the job lifecycle](examples/state-machine.layup),
[guarded choices](examples/state-choice.layup), and
[multilingual states](examples/state-international.layup), and
[nested connection states](examples/state-composite.layup). Run
`just state-test` for the Chromium review gallery.

### Fonts and measurement

IBM Plex Sans (regular and semibold), Mono, Arabic, and Hebrew are bundled.
Latin output embeds small glyph subsets. Arabic/Hebrew faces are embedded
only when used and remain intact to preserve shaping. Text measurement uses
Rustybuzz shaping with the same font bytes. Kerning and optional ligatures
are disabled in both measurement and SVG; required script shaping remains.

CJK prose wraps at Unicode line-break opportunities, including punctuation
rules, without requiring spaces. **No CJK font is bundled.** By default the
viewer supplies the glyphs and layout estimates their widths. System-font
appearance and exact wrapping can vary between viewers. An inline host can
choose a family with `.layup { --layup-font-fallback: "Noto Sans CJK SC", sans-serif; }`.
This CSS choice changes rendering, not the engine's width estimates.

For measured, reproducible fallback text, supply a standalone OpenType or
TrueType face. Fonts are tried in order after the bundled faces, measured,
and embedded unmodified when needed:

```sh
layup render examples/international.layup --font /path/to/CJK-Regular.ttf
layup check examples/international.layup --font /path/to/CJK-Regular.ttf --strict
```

Repeat `--font` for additional fallback faces. The Rust API accepts
`text::Fonts` with `add_fallback(bytes)`, then `compile_with_fonts(source, &fonts)`.
The JavaScript package accepts `render(source, { fonts: [fontBytes] })`, with
`Uint8Array` values. Supplied faces retain their shaping tables and are
embedded whole, so a large CJK font can make an output substantially larger.
Standalone static faces are recommended; weight/variation selection is not
currently configurable.

Bundled font subsets are named Layup Sans and Layup Mono under the SIL OFL.
Each SVG carries the bundled-font license notice. Element IDs and supplied
font-family names include content hashes so diagrams can share a page.
Characters outside the available fonts still use viewer fallback and
estimated widths; non-CJK missing glyphs produce diagnostics.

### Rust library

```toml
[dependencies]
layup = "0.3.0"
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
just check && just js-test && just vitepress-test
just international-test # Chromium metrics and multilingual preview
cargo publish -p layup -p layup-cli --dry-run
```

Publish `layup` (the engine, fonts, and renderers) first, then `layup-cli`
(the command-line interface). The CLI depends on the released engine version.
Publishing to crates.io is a separate release step.
