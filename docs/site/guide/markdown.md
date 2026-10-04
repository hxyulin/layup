# Markdown and VitePress

The JavaScript package includes a markdown-it plugin that compiles `layup`
fences to inline SVG at build time. Readers get a rendered diagram without
running a compiler; the optional client adds selection, fullscreen, and
presentation controls.

## Install and configure

In a separate project that installs the package from npm:

```sh
pnpm add -D @hxyulin/layup@0.3.0 vitepress
```

The npm package includes its WASM binary and supports the examples below.
This repository uses the workspace package and rebuilds that binary from Rust
before its docs build.

```ts
// .vitepress/config.mts
import { defineConfig } from 'vitepress';
import { vitepress as layup } from '@hxyulin/layup/markdown-it';

export default defineConfig({
  markdown: { config: md => md.use(layup, { strict: true }) },
});
```

The preset follows VitePress's `.dark` theme toggle and preserves SVG styles
and literal braces through Vue compilation. The plain markdown-it export uses
the viewer's `prefers-color-scheme` for automatic theme output.

## Write a fence

````markdown
```layup source
diagram "Dispatch" layout=auto {
  node api "API"
  node worker "Worker"
  api -> worker "dispatch"
}
```
````

`source` displays the source beneath the rendered diagram. Plugin options
include `fonts`, `view`, `theme`, and `strict`; `view=NAME` on a fence overrides
the default selected view.

````markdown
```layup view=detail
model "Services" {
  node api "API"
  node worker "Worker"
  api -> worker
  view overview { include api worker }
  view detail { include worker }
}
```
````

## Add interaction

```ts
// .vitepress/theme/index.ts
import DefaultTheme from 'vitepress/theme';
import '@hxyulin/layup/client';
export default DefaultTheme;
```

Hover or click a node to highlight it and its relationships. Expand opens a
fullscreen viewer with drag/pinch/wheel navigation. Use `+`/`-` to zoom, `0`
or `F` to fit, and Escape to close. Diagrams with authored steps also receive
presentation controls and speaker notes. Client-side navigation and replaced
SVG previews are supported.

Node `href` links are written as given. Use page-relative links when your site
lives under a base path such as `/layup/`. This site's live editor omits
executable URL schemes from previews and downloads.

## Check diagrams before deployment

```sh
layup check guide.md --strict
layup lint guide.md --json
```

Both use Markdown file locations for diagnostics. `strict: true` in the
plugin fails the build when a diagram has warnings or errors. The checked
example site under `examples/vitepress` exercises hydration, fonts, links,
theming, and fullscreen behavior.
