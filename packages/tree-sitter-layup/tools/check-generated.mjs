import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const files = ['src/parser.c', 'src/grammar.json', 'src/node-types.json',
  ...readdirSync('src/tree_sitter').map(name => `src/tree_sitter/${name}`)];
const before = files.map(path => readFileSync(path));
const generated = spawnSync('tree-sitter', ['generate', '--abi', '15'], { stdio: 'inherit' });
assert.equal(generated.status, 0, 'Tree-sitter generation failed');
for (const [index, path] of files.entries()) {
  assert.ok(before[index].equals(readFileSync(path)), `${path} is stale; run pnpm syntax:generate`);
}
console.log('Generated parser sources match grammar.js (ABI 15).');
