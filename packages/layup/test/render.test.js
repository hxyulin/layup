import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { loadSync, LayupError } from '../node.js';

const root = new URL('../../../', import.meta.url);
const example = new URL('examples/hello.layup', root);

test('matches the CLI byte for byte', () => {
  const source = readFileSync(example, 'utf8');
  const cli = execFileSync(new URL('target/debug/layup', root).pathname, ['render', '-', '-o', '-', '--theme', 'auto'], { input: source }).toString();
  const { output, warnings } = loadSync().render(source, { theme: 'auto' });
  assert.equal(output, cli);
  assert.deepEqual(warnings, []);
});

test('reports warnings and errors with lines', () => {
  const tight = 'diagram "T" width=200 {\n  node a "A" { code "an_unbreakable_identifier_that_cannot_fit_in_this_card" }\n}';
  assert.equal(loadSync().render(tight).warnings[0].line, 2);
  assert.throws(() => loadSync().render('diagram "T" {\n  a -> b\n}'), (e) => e instanceof LayupError && e.line === 2);
});

test('renders html', () => {
  assert.match(loadSync().render('diagram "T" { node a }', { format: 'embed' }).output, /^<!doctype html>/);
});

test('user fonts match CLI measurement and embedding without leaking across renders', () => {
  const path = new URL('crates/layup/tests/fonts/Fallback.ttf', root);
  const source = 'diagram "中" { node n "中中中" }';
  const font = readFileSync(path);
  const cli = execFileSync(new URL('target/debug/layup', root).pathname, ['render', '-', '-o', '-', '--font', path.pathname], { input: source }).toString();
  const engine = loadSync();
  assert.equal(engine.render(source, { fonts: [font] }).output, cli);
  assert.match(cli, /font-family:'Layup User /);
  assert.doesNotMatch(engine.render(source).output, /font-family:'Layup User /);
  assert.throws(() => engine.render(source, { fonts: [new Uint8Array([0, 1, 2])] }), LayupError);
  assert.throws(() => engine.render(source, { fonts: ['bad'] }), TypeError);
});
