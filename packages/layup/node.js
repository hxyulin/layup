import { readFileSync } from 'node:fs';
import { wrap } from './core.js';

export { LayupError } from './core.js';

let layup;

/** Loads the engine synchronously; Node allows sync compilation of any size. */
export function loadSync() {
  if (!layup) {
    const bytes = readFileSync(new URL('./layup.wasm', import.meta.url));
    layup = wrap(new WebAssembly.Instance(new WebAssembly.Module(bytes)));
  }
  return layup;
}

export async function load() {
  return loadSync();
}
