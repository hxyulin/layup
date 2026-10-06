# @hxyulin/layup

The layup layout engine compiled to WebAssembly. It produces the same SVG and
HTML as the `layup` CLI, byte for byte.

```sh
pnpm add @hxyulin/layup@0.3.0
# Or: npm install @hxyulin/layup@0.3.0
```

```js
import { load } from '@hxyulin/layup';

const layup = await load();              // Node: also `loadSync()`
const { output, warnings } = layup.render(source, { theme: 'auto' });
```

`render` accepts `theme` (`light`, `dark`, `auto`) and `format` (`svg`,
`html`, or `embed` for iframe HTML without the toolbar). Invalid source throws
`LayupError`, which has a `line`. Warnings are returned as `{ line, message }`.

Version 0.3 adds language tools with the same implementation as the CLI:

```js
const formatted = layup.format(source);
const diagnostics = layup.lint(source, { fonts: [] });
```

`format` preserves comments, quoted text and statement order and throws for
invalid syntax. `lint` returns structured errors and warnings, collecting
recoverable syntax errors and checking valid documents for semantic, layout
and authoring issues. Diagnostics and render errors include `column`, UTF-8
byte `span`, `code`, `help` and `related` locations. Columns count one-based
Unicode scalars. `LayupError` keeps its `line`, `reason` and message format.
See [the language-tools guide](https://hxyulin.github.io/layup/guide/tooling.html) for rules and
source-position conventions.

Version 0.3 also exposes a versioned scene API:

```js
const scene = layup.compile(source, { view: 'detail', fonts: [] });
const { output } = layup.render(source, { view: 'detail', format: 'html' });
const diagnostics = layup.lint(source, { view: 'detail' });
```

`compile` returns schema version 1 with nodes, edges, drawing operations,
source spans, font identifiers, warnings, selected views, optional sequence
metadata, slide transforms and presentation steps. Geometry stays in the
original scene coordinate system; `scene.viewport.slide` supplies the
separate fit transform. Font bytes are not exported. See the
[scene contract and presentation guide](https://hxyulin.github.io/layup/guide/formats.html) for the
complete schema and [sequence reference](https://hxyulin.github.io/layup/diagrams/sequences.html) for the DSL.

The npm package includes the compiled engine; users do not need Rust or a
WASM build step to install it.

In the browser, `load(url)` fetches `layup.wasm` from beside the module unless
you pass another URL.

## Revision-one graph syntax (unreleased)

The current checkout accepts an opt-in `layup 1` document through the existing
`compile`, `render`, `format`, and `lint` methods. It adds explicit IDs, scoped
references, source/doc/metadata annotations, order-independent kinds, and
multiple graph diagrams. Select one with `{ diagram: 'services' }`; Markdown
fences accept `diagram=services`. Scene JSON exports authored `objectPath`
segments separately from opaque render IDs. Other diagram grammars, views and
new document JSON input remain pending. See the
[experimental guide](https://github.com/hxyulin/layup/blob/main/docs/site/guide/language-v1.md).

## Structured input and Cargo analysis (unreleased)

The current checkout accepts version-1 semantic graph objects through
`compileModel(model, options)` and `renderModel(model, options)`. The graph
contract preserves arbitrary symbol IDs, parent hierarchy, named views,
original-code `sourceLocations`, JSON metadata and analyzer provenance.
Scene JSON and SVG/HTML exports preserve the selected analysis evidence.

The Node-only `@hxyulin/layup/cargo` entry point provides
`analyzeCargo({ manifestPath, includeExternal, includeDev, sourceBaseUrl })`
and `cargoGraph(metadata, options)`. `layup-cargo` emits a model with an
overview and per-package dependency/target views. It runs Cargo metadata
with `--locked` and offline by default, without building or executing code.
It analyzes package relationships, not function bodies.

See [generation from analysis](https://hxyulin.github.io/layup/guide/code-analysis.html)
for the contract, feature flags, source-link policy and CLI examples. These
APIs will ship in the next release.

## Markdown and VitePress

`@hxyulin/layup/markdown-it` renders ```` ```layup ```` fences to inline SVG when the
Markdown is built, so pages need no runtime to show diagrams. Diagnostics are
printed with the Markdown file and line; `strict: true` fails the build
instead. `layup check guide.md` reports the same problems from the CLI
without building the site. A ```` ```layup source ```` fence also shows its
source as a code block below the diagram.

For VitePress, use the `vitepress` preset in `.vitepress/config.mts`:

```ts
import { defineConfig } from 'vitepress';
import { vitepress as layup } from '@hxyulin/layup/markdown-it';

export default defineConfig({
  markdown: { config: (md) => md.use(layup) },
});
```

The preset follows VitePress's light/dark toggle (`darkSelector: '.dark'`)
and writes markup that survives Vue's template compiler, which would drop an
SVG's `<style>` and interpolate `{{ }}`. For other markdown-it hosts use the
default export; its `auto` theme follows `prefers-color-scheme`.

The plugin accepts a default `view` option; a fence's `view=NAME` overrides it.
Authored `step` plans are embedded as SVG metadata. Importing the client adds
Present/Previous/Next/All controls and plain-text speaker notes to diagrams
with steps, including fullscreen and keyboard navigation. Static output
displays the complete diagram before presentation begins.

## Interaction

`@hxyulin/layup/client` makes diagrams from the Markdown plugin interactive: hovering
or clicking a node highlights it and its edges, and a button in the corner
opens a full-window view with drag to pan, wheel or pinch to zoom, and
double-click to fit. In that view, `+` and `-` zoom, `0` or `F` fits, arrow
keys pan and Esc closes. Without it, diagrams are static and the button stays
hidden. In VitePress, import it from `.vitepress/theme/index.ts`:

```ts
import DefaultTheme from 'vitepress/theme';
import '@hxyulin/layup/client';

export default DefaultTheme;
```

The module does nothing during server rendering. Nodes with `href="..."` are
links; VitePress routes them without a full page load. Write them relative to
the page if the site sets a `base`. The client also works around a VitePress
prefetch bug that throws on SVG links, by giving `SVGAElement` the `pathname`
property that VitePress's prefetch code expects. The bug is fixed upstream in
vuejs/vitepress#5442, which is not yet released.

`examples/vitepress` is a working site: `just vitepress`. `just
vitepress-test` builds it and checks hydration, fonts, theming and these
interactions in headless Chromium.

## Development

In a checkout of the repository, `just wasm` builds `layup.wasm` into this
directory and `just js-test` compares the package's output with the CLI.
`npm pack` and `npm publish` rebuild the WebAssembly first.

Repository development uses pnpm: run `pnpm install --frozen-lockfile` from
the root. `pnpm docs:dev` starts the documentation and live browser editor;
`pnpm --filter @hxyulin/layup test` runs package tests against the current
WASM binary. See [the site development guide](https://hxyulin.github.io/layup/contributing.html).

## License

MIT OR Apache-2.0. `layup.wasm` embeds IBM Plex fonts, licensed under the SIL
Open Font License 1.1 (`OFL.txt`).

### International text and user fonts

Graph flow (`direction=down|up|right|left`) and label direction
(`text-direction=auto|ltr|rtl`) are independent source-language settings.
CJK uses Unicode wrapping and system fallback by default; no CJK font is
bundled. For reproducible CJK measurement, pass font bytes explicitly:

```js
const bytes = new Uint8Array(await (await fetch('/fonts/CJK-Regular.ttf')).arrayBuffer());
const { output } = layup.render(source, { fonts: [bytes], theme: 'auto' });
```

In Node, `readFileSync('/path/to/CJK-Regular.ttf')` also provides a suitable
`Uint8Array`. Fonts are fallbacks after the bundled Latin, Arabic and Hebrew
faces, and are measured and embedded intact when used. Large supplied fonts
increase output size. Font options apply only to that render. For inline SVG
using system fonts, `--layup-font-fallback` selects the CSS family; measurements
remain estimates unless font bytes are supplied.

The markdown-it and VitePress plugins accept the same `fonts` array in their
plugin options, for example `md.use(vitepress, { fonts: [fontBytes] })`.
