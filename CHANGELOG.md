# Changelog

## Unreleased

- Validate unused diagram defaults, reject conflicting transition captions, and
  reset inherited connection palettes with `palette=auto`.
- Preserve distinct quoted style names in SVG legend paint and marker IDs.
- Package self-contained Rust test fixtures and update the crate quickstart for
  the current language.
- Replace the source DSL with named diagrams and per-type graph, sequence and
  state-machine grammars. The optional `layup 1` assertion checks the revision;
  legacy headers, typed arrow names, implicit object IDs and bare style flags
  are rejected. Structured graph version 1 retains its existing contract.
- Add scoped identities, exact typed values, arbitrary ordered annotations,
  preserved unavailable diagram bodies and recovering document inspection.
- Use explicit node/edge styles, category defaults, direct paint channels,
  logical text alignment, grouped slide/legend settings and typed view/reveal
  selections. Scene metadata retains declarations, attributes and resolved paint.
- Migrate canonical diagrams, Markdown guides, frontend fixtures and Tree-sitter
  syntax highlighting together. ER, member/port geometry, full-document JSON
  input and public property schemas remain future extensions.
- Version-1 semantic graph input in Rust and JS/WASM, with original symbol IDs,
  hierarchy, custom kinds, named views, original-code locations and metadata.
- CLI render/compile/check detect graph `.json` files; render/compile accept
  `--input-format graph` for stdin. Structured-input exports use null DSL spans.
- Scene JSON exposes node/edge analysis evidence and root provenance. SVG/HTML
  carry selected evidence in `metadata[data-layup-analysis]`.
- Node-only Cargo metadata adapter and `layup-cargo` executable generate
  overview/per-package dependency and target views, preserving renamed,
  optional, build, development and target-conditioned dependency evidence.
- Canonical workspace analysis example, analyzer guide and CLI/WASM parity
  and real Cargo resolution regressions. The adapter does not analyze bodies.

## 0.3.0 — 2026-10-04

### Installation and documentation

- Published `@hxyulin/layup@0.3.0` on npm with the compiled WASM engine, TypeScript declarations, Markdown/VitePress integration and interaction client.
- Published CLI and Rust engine on crates.io; cargo-binstall metadata and GitHub release archives for Linux x86_64/ARM64, macOS Intel/Apple Silicon and Windows x86_64, with SHA-256 checksums and bundled-font licenses.
- Themed README showcase, light/dark logo variants, consistent SVG toolbar icons and touch controls; contributor guidance, CODEOWNERS, issue/PR templates and actionlint/repository checks.
- A compact source canvas for the slide pipeline example; resizable playground pane widths and shared height with pointer/keyboard controls. Example/share links and browser history update reused editors, and source edits recover when the selected view is removed.
- VitePress/pnpm documentation site with DSL and diagram walkthroughs, formats/API references, a browser-worker WASM playground, editable examples, diagnostics, formatting, views, supplied fonts, downloads and UTF-8 share links; GitHub Pages build/test/deploy workflow.

### Sequences, presentations and reusable models

- Sequence diagrams with authored participant order, lifelines, self-messages, asynchronous/return arrows, notes and nested loop/opt/alt fragments; measured Unicode/RTL labels and supplied-font support.
- Slide viewports with widescreen/standard/custom dimensions, uniform fit, padding and post-fit minimum-font-size diagnostics.
- Cumulative presentation steps with stable edge IDs, highlights, speaker notes, keyboard navigation and inline/fullscreen/iframe controls.
- Reusable models and named views with explicit node identity, include filtering, attribute overrides, per-view presentation plans and CLI/JS/Markdown view selection.
- Version-1 JSON scene export through Rust `scene::export`, CLI `compile` and JavaScript/WASM `compile`, including original geometry, hierarchy, source spans, slide transforms, reveal plans and sequence metadata without font payloads.
- Rust model/scene struct additions and `Mode::Sequence` / `Item::Group` affect downstream struct literals and exhaustive matches.
- Combined presentation demo, focused semantic/geometry tests, native/WASM parity and Chromium galleries.

### Language tools

- UTF-8 source spans, Unicode scalar columns, lossless comment tokens and statement-boundary parser recovery with a nesting limit.
- Source excerpts, diagnostic codes, related locations and typo suggestions in CLI errors; structured spans and help in WASM/JavaScript errors.
- `layup lint` with Markdown-aware JSON locations, existing layout checks and authoring rules for unused declarations, conflicting flags, ignored bodies and duplicate transitions.
- `layup fmt` with stdin/stdout, `--check`, preflighted batch `--write`, and comment/string/order preservation; JavaScript `format` and `lint` APIs.
- Hyphenated typed arrow kinds, compact left arrows and decimal exponents; malformed escapes, duplicate attributes, non-finite literals and invalid geometry numbers now fail explicitly.
- Rust API additions to `Error`, lexer tokens and generic syntax nodes affect downstream struct literals and exhaustive patterns.

### Layout and state machines

- Font-aware automatic canvas growth for wide trees, long horizontal chains, and nested containers; explicit widths and manual presets retain authored sizing.
- Variable widths and measured subtree packing for compact nodes on automatic canvases.
- Composite state frames, scoped initial/final validation, nested reachability, and cross-boundary transitions in all four directions.
- Internal composite transition routing, bounded fan-out port spreading, and nested/automatic-size Rust, WASM, and browser coverage.

- Flat state-machine diagrams with `state`, `initial`, `final`, and `choice` shapes.
- Opt-in `mode=state-machine` validation for initial/final markers, directed transition endpoints, choice branches, and unreachable states.
- Deterministic machine cycle placement, extra transition-label space, and self-loops with caption clearance.
- State-machine examples, semantic/geometry tests, CLI/WASM parity, and browser visual/interaction checks.

### Decisions and flowcharts

- Decision, process, and terminal shapes with compact bodies, diamond-safe text wrapping, and outline-aware arrow attachment.
- Subtree placement for decision trees in all four directions, with general graph fallback for merges, cycles, mixed kinds, and placement hints.
- Upright branch captions that avoid actual node outlines; warnings when no clear caption placement exists.
- Decision examples, geometry/routing regressions, WASM parity tests, and Chromium visual checks.

### Directions and international text

- Automatic graph flow in all four directions, separate from `text-direction=auto|ltr|rtl`.
- Unicode line breaking for CJK, hard newlines, and nonbreaking spaces; combining-mark and Arabic/Hebrew shaping.
- RTL alignment and bidi isolation for mixed prose/code labels.
- System CJK fonts by default, optional supplied fallback font bytes in Rust, CLI, and JavaScript/WASM.
- Unicode identifier continuation, full-Unicode font-subset cmaps, and multilingual regression/browser checks.
- Roadmap for state machine semantics and scene/export formats.

## 0.2.0

### Layout

- `diagram "Title" layout=auto { ... }` infers top-to-bottom layers from
  edges, including inside containers. Explicit rows and sections keep their
  structure; without the option, layout stays authored.
- Layout hints `below=`, `same-layer=` and `beside=` place nodes without
  writing rows.
- Obstructed edges try other ports and search a path around nodes; `via=`
  and pinned sides still win.
- Disconnected graphs occupy separate regions, and automatic-layout colors
  follow node IDs, so unrelated edits no longer move or recolor nodes.
- Nodes accept `href="..."` and render as links.

### Output

- SVGs embed only the glyphs they draw: about 50–60 KB instead of 780 KB.
  The subsets are named Layup Sans and Layup Mono, since the OFL reserves
  the Plex name for unmodified fonts.
- Several SVGs can be inlined in one page. Element IDs carry a per-diagram
  prefix, stylesheet rules are scoped under `.layup`, and each font face
  declares the `unicode-range` of its subset.
- Clicking a node in `--html` pages now pins it.

### CLI

- `layup check guide.md` checks every `layup` code block in Markdown and
  reports problems at Markdown lines.

### Web

- `@hxyulin/layup` on npm: the engine compiled to WebAssembly for Node and browsers,
  with a markdown-it plugin, a VitePress preset and `@hxyulin/layup/client`, which adds
  highlighting and a full-window pan-and-zoom view. See its README.

### Breaking changes in the Rust API

- `model::Block::Node` holds a `Box<Node>`.
- `model::Node` and `layout::NodeRect` have an `href` field.
- `svg::stylesheet` takes the characters to embed and an optional dark-mode
  selector. `svg::render_with` and `svg::Options` are new.
- SVG consumers that styled `.t`, `.box` and similar classes, or referenced
  `#layup-title` or `#m-*`, need the `.layup` scope and the ID prefix.

## 0.1.0

First release of `layup` and `layup-cli`.
