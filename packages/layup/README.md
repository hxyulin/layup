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
