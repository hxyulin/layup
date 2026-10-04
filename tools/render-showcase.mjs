import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const check = process.argv.includes('--check');
if (process.argv.slice(2).some(arg => arg !== '--check')) throw new Error('Usage: node tools/render-showcase.mjs [--check]');
const destination = join(root, 'docs/showcase');
const output = check ? mkdtempSync(join(tmpdir(), 'layup-showcase-')) : destination;
const examples = { pipeline: 'slides', decisions: 'decision-tree', sequence: 'sequence' };
mkdirSync(output, { recursive: true });
try {
  for (const [name, example] of Object.entries(examples)) {
    for (const theme of ['light', 'dark']) {
      const filename = `${name}-${theme}.svg`;
      const result = spawnSync('cargo', ['run', '-q', '-p', 'layup-cli', '--', 'render', `examples/${example}.layup`, '--strict', '--theme', theme, '-o', join(output, filename)], { cwd: root, stdio: 'inherit' });
      if (result.error) throw result.error;
      if (result.status !== 0) throw new Error(`Rendering ${filename} failed`);
      if (check && !readFileSync(join(output, filename)).equals(readFileSync(join(destination, filename)))) {
        throw new Error(`${filename} is out of date. Run pnpm showcase and include the regenerated previews.`);
      }
    }
  }
  console.log(check ? 'Showcase previews match their sources.' : 'Updated light/dark showcase previews in docs/showcase/.');
} finally {
  if (check) rmSync(output, { recursive: true, force: true });
}
