// Keep published Rust tests self-contained while examples/docs remain canonical.
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const fixtures = join(root, 'crates/layup/tests/fixtures/diagrams');
const write = process.argv.includes('--write');
function diagrams(directory, prefix = '') {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const name = join(prefix, entry.name);
    if (entry.isDirectory() && ['node_modules', '.vitepress'].includes(entry.name)) return [];
    if (entry.isDirectory()) return diagrams(join(directory, entry.name), name);
    return entry.name.endsWith('.layup') ? [name] : [];
  });
}
const sources = ['examples', 'docs/diagrams', 'docs/checkpoints'].flatMap(directory =>
  diagrams(join(root, directory), directory));
if (write) {
  rmSync(fixtures, { recursive: true, force: true });
  for (const source of sources) {
    const path = join(fixtures, source);
    mkdirSync(join(path, '..'), { recursive: true });
    writeFileSync(path, readFileSync(join(root, source)));
  }
} else {
  assert.deepEqual(diagrams(fixtures).sort(), sources.sort(), 'Run pnpm fixtures:sync to refresh packaged Rust fixtures.');
  for (const source of sources) {
    assert.equal(readFileSync(join(fixtures, source), 'utf8'), readFileSync(join(root, source), 'utf8'),
      `${source}: packaged fixture differs; run pnpm fixtures:sync`);
  }
}
console.log(`Packaged Rust diagram fixtures (${sources.length}) ${write ? 'synchronized' : 'match canonical sources'}.`);
