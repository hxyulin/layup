---
aside: false
outline: false
---

# Live playground

Choose an example, edit its source, and see the result as you type. Compilation
runs locally in a browser worker using the same Rust/WASM engine as the CLI.
Your edits and font files are not uploaded.

<Playground full />

**Format** preserves comments and strings. Invalid edits retain the last
successful diagram and report source locations. View selection is available
for shared models. Zoom the inline preview to inspect labels, or expand it
for pan/zoom and presentation controls.
Tab moves focus; Ctrl+Enter renders immediately.

Drag the divider between source and diagram to give either pane more width.
Drag the handle below them to change both panes' height together. You can
also focus a handle and use arrow keys, Home/End, or Enter to reset it.
**Reset pane sizes** restores the default layout. On a narrow screen the panes
stack, and the shared height handle remains available.

Selecting an example updates its URL, so reloading keeps that example.
Shared links and browser history update an already open playground. If an
edit removes or renames the selected view, the editor uses the default view
and tells you about the change; source errors still need correction.

Share links put source and view selection in the URL fragment. They do not
include local fonts; download source instead for very large diagrams. Links
inside edited previews/downloads allow HTTP(S), mailto, and tel URLs.

Start with [the language walkthrough](/guide/language),
[browse examples](/examples), or read about [export formats](/guide/formats).
