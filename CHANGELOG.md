# Changelog

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
