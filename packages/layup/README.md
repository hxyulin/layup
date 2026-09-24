# layup (npm)

The layup layout engine compiled to WebAssembly. It produces the same SVG and
HTML as the `layup` CLI, byte for byte.

Build the engine into this directory first:

```sh
just wasm       # packages/layup/layup.wasm
just js-test    # also compares output with the CLI
```

```js
import { load } from 'layup';

const layup = await load();              // Node: also `loadSync()`
const { output, warnings } = layup.render(source, { theme: 'auto' });
```

`render` accepts `theme` (`light`, `dark`, `auto`) and `format` (`svg`,
`html`, or `embed` for iframe HTML without the toolbar). Invalid source throws
`LayupError`, which has a `line`. Warnings are returned as `{ line, message }`.

In the browser, `load(url)` fetches `layup.wasm` from beside the module unless
you pass another URL.

## Markdown and VitePress

`layup/markdown-it` renders ```` ```layup ```` fences to inline SVG when the
Markdown is built, so pages need no runtime to show diagrams. Diagnostics are
printed with the Markdown file and line; `strict: true` fails the build
instead.

For VitePress, use the `vitepress` preset in `.vitepress/config.mts`:

```ts
import { defineConfig } from 'vitepress';
import { vitepress as layup } from 'layup/markdown-it';

export default defineConfig({
  markdown: { config: (md) => md.use(layup) },
});
```

The preset follows VitePress's light/dark toggle (`darkSelector: '.dark'`)
and writes markup that survives Vue's template compiler, which would drop an
SVG's `<style>` and interpolate `{{ }}`. For other markdown-it hosts use the
default export; its `auto` theme follows `prefers-color-scheme`.

`examples/vitepress` is a working site: `just vitepress`.
