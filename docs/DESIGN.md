# Layup design

`layup` is a Rust layout engine, with a `layup-cli` command-line frontend, that turns a text description of an
architecture diagram into a static SVG, or into an interactive HTML page
for the web. Layout stays under the author's control, while text measurement,
edge routing, and diagnostic checks are automated.

## 1. Design choices

The examples demonstrate a consistent visual language for authored diagrams.

**Layout is authored, not solved.** Rows, bands and nesting are decided
by the writer. Mermaid's Dagre layout reflows whenever a node is added,
so a diagram never stays the shape you explained it in. Layup treats the
source as a document: blocks flow top to bottom like CSS, rows split the
width by weights, and only the *edges* are computed.

**Nodes are cards with typography, not labelled boxes.** A card has a
bold head, an optional monospace code line in the link-blue `#0550ae`,
and muted prose that wraps. Identifiers stay in the code font while the
`·` separators between them stay in the sans face. Mermaid gives one
font, one weight, and `<br/>` for line breaks.

**Tones instead of arbitrary colors.** Everything is drawn from GitHub
Primer's light palette (blue, green, yellow, purple, orange, gray, red,
each as fill / border / ink). A blue card on a page of rendered Markdown
looks native. Mermaid themes are global and its default palette is not.

**Containers are hollow, leaves are filled.** A package is a white frame
with a gray head strip and a mono name; a crate target is a hollow blue
rounded box with a small "lib crate" role label. Filled nodes inside
them are the actual API. Mermaid subgraphs are all the same tinted box.

**Typed edges with a synced legend.** Edge kind is color plus dash:
orange `impl`, purple `extends`, blue `uses`, dashed re-export edges
that take the color of the node they point at. Small label chips sit on
the long segment of an edge and rotate on verticals. The legend lists
exactly the kinds in use. Mermaid edges are one color and labels are
unstyled text with a white halo.

**Structure labels.** Uppercase, letter-spaced band labels on the left
of a group, rule-and-title sections between groups, and a dashed divider
with a centered caption ("cross target above · cargo test on the host
below"). Mermaid has subgraph titles and nothing else.

**Semantic tags.** Titles like `[re-exported] Storage error contract` or
`[used] storage adapter` render the bracketed word in bold blue, so the
diagram itself says why a node is there.

**Outside routing.** Three `uses` edges in the package diagram wrap
around the right margin instead of cutting through five packages. Layup
does this with `via=right`; Mermaid cannot be told where an edge goes.

**Render checks.** Layup checks for text that overflows its box, edges that cross nodes, and dashed
edges that share a line (two dashed lines on top of each other look
solid). Layup runs those checks on every render and `--strict` turns
them into errors.

What Mermaid does better, and layup deliberately does not attempt: fully
automatic layout for arbitrary graphs, sequence and state diagrams,
in-browser rendering from a fenced block. Layup is for diagrams you
compose on purpose.

## 2. The source language

Files end in `.layup`. The syntax is brace-structured, one item per line
(or `;`), with `//` comments. Identifiers may contain `-`, `.`, `:` and
`/` so crate names and paths need no quoting.

```
diagram "Title" width=1950 {
  note "One-line subtitle under the title."
  desc "Accessible description, emitted as <desc>."
  legend                       // auto legend from the kinds in use

  section "Upstream packages" {
    row 35:42:34:53 gutter=20 {
      package "storage-api" {
        crate storage_api {
          trait store_error "Storage error contract" { code "store::Error" }
        }
      }
      ...
    }
  }

  disk_store -impl-> store_error labeled
  application -uses-> disk_store via=right
}
```

### Diagram header

| Item | Meaning |
| --- | --- |
| `diagram "Title" [width=N] [preset=clean\|manual] { ... }` | Title is required. Width defaults to 900, or 1400 when a row has 3+ cells or nodes nest 3 deep. Wide diagrams (≥ 1400) get larger margins and type. `preset=manual` selects explicit-only behavior (fixed gray cards, 12px gutters, no auto legend/width); `clean` is the default. |
| `note "..."` | Subtitle under the title. |
| `desc "..."` | Long description for the SVG `<desc>` (screen readers, search). |
| `legend [bottom] [off] [kind ...]` | A legend built from the node and arrow kinds that actually appear. Under `clean` it is added automatically when a typed edge is used; it sits top-right beside the title when it fits, otherwise as a block at the top or, with `bottom`, at the end. `legend off` suppresses even the automatic one. Listing kinds restricts it. |
| `style NAME ...` | Declares or overrides a node kind. See below. |
| `arrow NAME ...` | Declares or overrides an edge kind. |

### Blocks

Blocks flow top to bottom, each taking the full available width.

| Block | Meaning |
| --- | --- |
| `band "Label" { ... }` | Uppercase letter-spaced label above the children. |
| `section "Label" { ... }` | Horizontal rule and a title-case label, with extra space above. |
| `row [w:w:w] [gutter=N] { ... }` | Children side by side. Weights split the width; equal by default. Default gutter is 16px under `clean` (12px under `manual`). `gap` as a child leaves an empty cell. |
| `gap [N]` | Vertical space (default 16). |
| `text "..."` | Wrapped muted paragraph (footers, captions). |
| `divider "caption" [tone]` | Dashed rule with a centered caption. |
| `NODE ...` | A node, see below. |

### Nodes

```
KIND [id] ["Title"] [tone] [flags] [key=value ...] [{ content }]
```

- The first bare word that is not a tone or flag is the id. Without one
  the id is slugged from the title (`Storage error contract` → `storage-error-contract`)
  and made unique; explicit ids are reserved first so a generated one
  never steals them.
- Tones: `gray blue green yellow purple orange red`, or omit the tone:
  `card` and `node` cycle `blue green yellow purple orange` in document
  order with manual layout. Under `layout=auto`, a fixed hash of the node ID
  selects from that palette instead. The `auto` flag opts into the applicable
  automatic tone policy even after a style sets a fixed tone.
  Typed kinds (`trait`, `type`, `module`, …) keep their semantic tones.
- `node` is a `card` under another name: the zero-config leaf. A
  `node`/`card`/`api` with nested children draws as a hollow container,
  so grouping needs no `package`/`group` vocabulary on day one.
- Flags: `hollow filled center left mono sans`.
- Attributes: `id= tone= role= tag= gutter= align=`; in automatic layout,
  also `below= same-layer= beside=`.
- A title starting with `[word]` sets a tag drawn in bold blue.

Content lines:

| Line | Meaning |
| --- | --- |
| `name "..."` | Head line (same as the title argument). |
| `code "..."` | Monospace line. If a card has no title, its first code line becomes the head. |
| `sub "..."` | Muted line, wrapped. |
| `text "..."` | Muted paragraph; several stack with spacing. |
| `role "..."` / `tag "..."` | Same as the attributes. |
| any block | Nested children (containers only). |

Built-in kinds:

| Kind | Draws |
| --- | --- |
| `card` / `node` | Filled rounded card, auto tone. Head 15px bold, code 12.5px mono, prose 12.5px muted. `node` with children becomes a hollow container. |
| `package` | White frame with a gray head strip and mono name. Holds children. |
| `crate` | Hollow blue container, mono name, role "lib crate" (override with `role="bin crate"`). |
| `group` | Hollow gray container with no role. |
| `trait` / `type` / `module` / `api` | Centered API box: 12.5px muted title over 13px mono code. Purple, green, blue, gray. Legend labels "trait", "concrete type", "module-like". |

Declaring kinds:

```
style service base=card tone=green "service"          // legend label
style port shape=api hollow purple label="port"
```

`base=` copies another kind; `shape=` is `card | api | package | container`;
words are tones and flags; `role=` and `label=` set the container role
and the legend label.

### Automatic placement (v0.2 development)

Add `layout=auto` to `diagram` to infer top-to-bottom layers from directed
edges. The default remains `layout=manual`, preserving authored block flow.
This is independent of the style `preset`.

Within each consecutive run of sibling nodes, sources precede targets and
peers share equal-width rows. Declaration order breaks ties; unconstrained rows contain at
most three nodes and wider layers wrap in that order. Disconnected graphs occupy separate regions ordered by their first
declared node. Unconnected nodes form a final region, wrapping in source order. Directed cycles share a layer; bidirectional and
undirected edges do not impose a layer order. Self-edges do not affect placement.

Containers and sections use the same rules recursively. Edges between their
descendants order the containing siblings. Explicit rows retain their cells,
weights and order; sections, dividers, text, gaps and rows are boundaries that
automatic placement cannot cross. Edges across these boundaries still route,
but do not override the authored structure. Existing width inference applies
after placement: three columns can widen an unpinned canvas to 1400px.

There is no direction option, crossing minimization, saved-position state, or
automatic routing improvement in this checkpoint. Cycles and dense graphs may
still need routing hints. Run `just preview-auto` for the review gallery.

### Predictable edits (v0.2 development)

Automatic placement is computed afresh, with no saved position state:

- Within an authored boundary, connected regions stay separate, in order of
  their earliest declaration. All edge kinds and layout hints connect regions,
  even when an edge does not impose a rank.
- Unconnected nodes form a final region. Inserting one into a connected graph's
  declarations does not cause it to share a row with the graph's sources.
- Source order still determines peer order. Extending a simple chain adds a
  layer; adding a peer reflows that layer; connecting previously independent
  regions deliberately combines them.
- Auto-layout tones derive from a fixed hash of node IDs. Inserting nodes no
  longer changes existing tones. Hash collisions can give peers the same tone;
  explicit colors still win. Renaming an ID can change its automatic tone.
  Manual layout retains the original source-order palette cycle.

This does not freeze coordinates across arbitrary edits. Longer labels can
change row heights, added peers change cell widths, and three columns or deep
nesting can trigger the existing 1400px canvas default. Pin `width=` to avoid
canvas-width changes. Earlier regions growing can move later regions down;
new edges can alter ranks and route choices. Use explicit IDs and authored
boundaries when identity and grouping matter. No position cache is required.

Run `just preview-incremental` to compare edits under checkpoints 3 and 4.

### Layout hints (v0.2 development)

With `layout=auto`, node attributes constrain placement without adding edges:

```text
node worker "Worker" below=api
node cache "Cache" same-layer=worker
node audit "Audit" beside=worker
```

- `below=id`: a strictly later layer than the target, not necessarily the
  immediately following layer. Contradictions with directed dependencies,
  cycles, or same-layer constraints are errors.
- `same-layer=id`: share a physical row. This overrides inferred edge ranks;
  if it closes a directed path into a cycle, intermediate nodes also acquire
  that rank. It does not choose left-to-right order.
- `beside=id`: immediately to the right of the target in the same row.
  Chains are supported; branching neighbors or circular chains are errors.

Targets must be immediate peers in the same consecutive automatic-layout
region. Forward references work. Cross-container or cross-section references,
explicit row-cell hints, self-references, and hints in manual layout report
source-line errors. Descendants of explicit row cells can still use hints
within their own automatic-layout region.

Same-layer groups and beside chains stay together even when they exceed the
usual three-column wrapping limit. They can require a wider canvas or shorter
labels; normal overflow checks still apply. Otherwise declaration order
remains the tie-breaker. Run `just preview-hints` for before/after comparisons.

### Edges

```
FROM -> TO ["label"] [words] [key=value ...]
FROM -KIND-> TO ...
```

Arrow forms: `->`, `<-`, `<->`, `--` (no head) and `-kind->` (any of
the four with a kind name inside).

| Word / attribute | Meaning |
| --- | --- |
| `"label"` or `label=` | Chip text on the edge. |
| `labeled` | Use the kind's default chip (`impl`, `extends`, ...). Edges are bare by default so a dense diagram is not covered in chips. |
| `dashed` / `solid` | Override the kind's stroke. |
| tone word or `tone=` | Override the kind's color. |
| `via=right` / `via=left` | Route around the outside of the content in a reserved lane. Several such edges nest: first declared is outermost. |
| `from=` / `to=` | Pin the exit or entry side: `top bottom left right`. |
| `bus` | Marks edges that intentionally share a trunk, silencing the overlap check for them. |

Built-in arrow kinds:

| Kind | Color | Dash | Chip | Legend |
| --- | --- | --- | --- | --- |
| `default` | source node's tone (gray if that is gray) | solid | | |
| `impl` | orange | solid | `impl` | implements |
| `extends` | purple | solid | `extends` | extends |
| `uses` | blue | solid | `uses` | uses |
| `exports` | target node's tone | dashed | `exports` | exports *trait* / *concrete type* / *module-like* |
| `depends` | gray | solid | | depends on |
| `flow` | gray | solid | | |

Declaring kinds:

```
arrow reads blue dashed "reads from" chip=reads
arrow reexport inherit dashed "re-exports"
```

## 3. Pipeline

```
lexer → parser → model → layout → route → check → svg | html
```

- **lexer / parser** (`lexer.rs`, `parser.rs`) produce generic items
  (`head args { body }`) and edge statements. The parser knows nothing
  about kinds, so new kinds need no grammar change.
- **model** (`model.rs`) turns items into a typed `Diagram`: blocks,
  nodes with resolved styles, edges with resolved kinds, legend. It
  assigns ids and reports unknown words with line numbers.
- **layout** (`layout.rs`) measures every block bottom-up with real
  font metrics (advances read directly from bundled IBM Plex Sans and Mono
  files, also embedded in SVG output; kerning and optional ligatures disabled), then draws top-down: block flow, rows by
  weight, containers padding their children, bands and sections adding
  their labels. It records node rectangles and "keep-out" boxes (section
  titles, divider captions) for the router and warns when a line does
  not fit even with the card's padding slack.
- **route** (`route.rs`) draws every edge orthogonally. Sides are chosen
  from the relative position of the two nodes and tried in order until
  a candidate does not cut through another node. Aligned nodes get a
  straight line; others a three-segment route whose middle sits in the
  first clear channel nearest the midpoint, or a five-point route that
  leaves sideways into a gutter when the direct one is blocked. Edges
  that share a port are spread 20px apart; edges that would run along
  an earlier edge nudge their ports and try again. Chips slide along
  their segment until they cover no leaf node, title or earlier chip.
  `via=` edges swing out into lanes the layout reserved beyond the
  content edge.
- **check** (`check.rs`) reports the scene checks that need
  the whole scene: collinear overlapping segments and chips sitting on
  a node.
- **svg** (`svg.rs`) emits one self-contained file. Colors are CSS
  custom properties on the root `.layup` class so `--theme dark` and
  `--theme auto` (a `prefers-color-scheme` media query) are the same
  geometry with different variables, and a host page can override any
  tone. Every node is `<g class="node k-KIND" data-id>` and every edge
  `<g class="edge k-KIND" data-from data-to>`.
- **html** (`html.rs`) wraps the SVG in a page that adds interaction.

### Automatic routing fallback (v0.2 development)

Clear routes keep their existing geometry. For an obstructed, overlapping, or
invalid-port route without `via`, the router now tries permitted node sides
and searches an orthogonal grid around obstacles. It prefers shorter paths
with fewer bends, keeps paths inside the existing canvas, and separates them
from earlier edge segments. Explicit `from` / `to` sides remain constraints;
explicit `via` paths retain their previous behavior.

The fallback also handles self-loops and paths beside the content. It does
not optimize all edges together: declaration order remains a tie-breaker,
perpendicular crossings are allowed, and ancestor/descendant endpoints retain
the original router. The search is capped at 40,000 grid intersections per
candidate port pair. If it cannot find a route, the original route and its
normal diagnostics remain. See `just preview-routing` for a comparison against
the checkpoint-2 implementation using identical inputs.

## 4. Interactive output and iframes

`layup render x.layup --html` produces a single HTML file with no
dependencies:

- drag to pan, wheel or pinch to zoom, double-click to fit;
- hover a node to dim everything but it and its edges, click to pin;
- toolbar: Fit, 1:1, Theme (light → dark → auto, remembered in
  `localStorage`), Clear; keys `F`, `1`, `T`, `Esc`;
- `?focus=ID` opens zoomed to a node with it pinned, `?theme=dark`
  forces a theme;
- `--embed` hides the toolbar and hint for use inside an iframe.

The page speaks `postMessage` so a host can drive it:

| Direction | Message |
| --- | --- |
| host → frame | `{layup: "focus", id, zoom?}` pin and zoom to a node |
| host → frame | `{layup: "clear"}` unpin and fit |
| host → frame | `{layup: "theme", theme: "light" \| "dark" \| "auto"}` |
| host → frame | `{layup: "fit"}` |
| frame → host | `{layup: "ready", width, height}` after load, for sizing the iframe |
| frame → host | `{layup: "select", id}` when the user pins or clears (`id: null`) |

`examples/embed-host.html` is a working host: it sizes the iframe from
`ready`, has buttons that focus nodes, and prints selections.

Static docs (mdBook, Docusaurus, GitHub README) can use the plain SVG
inline or as an image; the SVG keeps the `data-` attributes, so a page
that inlines it can add its own hover script with a few lines.

## 5. Layout rules worth knowing

- Margins are 40px (50px for wide diagrams). Blocks are separated by
  16px, rows use a 16px gutter by default, sections add 48px above.
- Cards pad 20px horizontally. Text is measured against the inner width
  and a 16px slack; a code line that overhangs by more than that warns.
- Containers pad their children 20px and add their name lines; packages
  get a 56px head strip.
- Row weights are relative: `row 3:7` gives the second cell 70% of the
  width after the gutter.
- The router does not move nodes. If an edge cannot be routed cleanly,
  the warning names the node it crosses, and the fix is a `via=`, a
  pinned side, or rearranging the row. That is deliberate: the diagram
  is a document, and the writer decides its shape.
