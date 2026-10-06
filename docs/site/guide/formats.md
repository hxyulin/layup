# Output formats

Choose SVG for a portable diagram, HTML for an interactive explanation, or
scene JSON when another tool needs layout geometry and semantic identity.

| Format | Best for | What it contains |
| --- | --- | --- |
| SVG | Documents, slides, images, and inline web content | Vector drawing, semantic attributes, themes, embedded used fonts |
| Standalone HTML | Walkthroughs and offline sharing | SVG plus pan/zoom, selection, themes, presentation controls |
| Embed HTML | Iframes controlled by a host | Viewer without the toolbar, with `postMessage` controls |
| Scene JSON | Editors, alternate renderers, analysis | Versioned geometry, source spans, hierarchy, views, slide/reveal/sequence data |

The playground can download all four, plus `.layup` source. SVG export follows
the viewer's color-scheme preference; inline documentation follows the site's
theme toggle. Named views and supplied fonts affect compilation in every
format.

## SVG and HTML

```sh
layup render diagram.layup
layup render diagram.layup --theme auto
layup render diagram.layup --html --theme auto
layup render diagram.layup --html --embed -o diagram.embed.html
layup render - -o - < diagram.layup
```

SVGs embed used Latin subsets and required Arabic/Hebrew/user faces. CJK still
uses system fonts unless a supplied face covers it. A static SVG describes the
complete diagram; reveal metadata lets an interactive host opt in to steps.

HTML is a self-contained file. Serve iframe examples over HTTP when testing
host messaging: browsers restrict messages between `file://` pages. See
[iframe presentation controls](/guide/presentations#drive-an-iframe) and
[the complete host protocol](https://github.com/hxyulin/layup/blob/main/docs/DESIGN.md#4-interactive-output-and-iframes).

## Light and dark images on GitHub

Use `--theme auto` for SVGs that should follow `prefers-color-scheme` in a
browser or image viewer. Inline VitePress diagrams instead follow the site's
light/dark toggle. Use a fixed palette when converting an SVG to a raster
image or when the host cannot communicate its theme.

For a GitHub README, generate both palettes and let `<picture>` select the
image for the reader:

```sh
layup render diagram.layup --theme light -o diagram-light.svg
layup render diagram.layup --theme dark -o diagram-dark.svg
```

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="diagram-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="diagram-light.svg">
  <img src="diagram-light.svg" alt="Describe the diagram">
</picture>
```

The fallback image is light. To always show a dark diagram, use an ordinary
image pointing to `diagram-dark.svg`. GitHub supports the theme-dependent
picture pattern in its [Markdown quickstart](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/quickstart-for-writing-on-github).
The repository README uses this pattern; `pnpm showcase` regenerates its
previews from canonical examples.

## Scene JSON

```sh
layup compile diagram.layup -o scene.json
layup compile model.layup --view detail --strict
```

`compile` writes stdout by default, with warnings on stderr. Strict failure
suppresses output and preserves an existing destination file.

The root contract uses `version: 1`, `units: 'svg-user-units'`, and
`coordinateSystem: 'scene'`. Geometry retains full floating-point precision.
Check the schema version before interpreting it.

| Field | Contents |
| --- | --- |
| `width`, `height`, `margin`, `contentLeft`, `contentRight` | Original layout canvas |
| `viewport` | Rendered dimensions and optional slide transform |
| `nodes` | IDs, kinds, parents, rectangles, outlines, tones, links, source spans |
| `edges` | IDs, endpoints, path points, style, captions, source spans |
| Node/edge `sourceLocations`, `metadata`; root `provenance` | Original code locations and analysis evidence for structured graph inputs; empty/null for DSL |
| `items`, `keepout` | Ordered drawing operations and reserved rectangles |
| `views`, `selectedView` | Available views and current selection |
| `presentation` | Versioned visibility and highlight steps |
| `sequence` | Participants, messages, and note/fragment rectangles, or `null` |
| `fonts` | Font identifiers and fallback policy, without bytes or file paths |
| `diagnostics` | Compilation warnings and available source locations |

For slides, transform original coordinates exactly once:

```text
x' = offsetX + scale * x
y' = offsetY + scale * y
```

Use `viewport.slide` for those values. Sizes scale by the same factor. Without
a slide, scene coordinates are already viewport coordinates. Text positions
use SVG baselines and physical anchors; direction and styled runs remain
explicit. The [TypeScript declarations](https://github.com/hxyulin/layup/blob/main/packages/layup/index.d.ts)
cover every drawing variant.

Scene JSON contains font identifiers rather than payloads. An alternate
renderer must supply the named fonts. For ready-to-share rendered font data,
use SVG/HTML. [Source positions](/guide/tooling#source-positions) use UTF-8
bytes and Unicode scalar columns.

In the current checkout, [structured graph inputs](/guide/code-analysis) use
null node/edge `line` and `span`, and null view spans. Original-code locations
live in `sourceLocations`, separately from DSL locations. SVG/HTML preserve
selected evidence in JSON inside `metadata[data-layup-analysis]`.

## PNG and PDF

Native PNG/PDF export is not implemented. You can place an SVG in a slide or
convert it with a separate renderer. Such conversion needs the same system
fonts for estimated CJK output, or a supplied embedded face. Slide sizing
already gives a consistent export viewport.
