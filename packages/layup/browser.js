import { wrap } from './core.js';

export { LayupError } from './core.js';

let layup;

/** Fetches and instantiates the engine once; pass a URL to host the .wasm elsewhere. */
export function load(url = new URL('./layup.wasm', import.meta.url)) {
  layup ??= WebAssembly.instantiateStreaming(fetch(url)).then(({ instance }) => wrap(instance));
  return layup;
}
