import { xml, objectId } from './helpers.js';
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
  const tight = 'diagram main "T" type=graph width=200 {\n  node a "A" {\n    code "an_unbreakable_identifier_that_cannot_fit_in_this_card"\n  }\n}';
  assert.equal(loadSync().render(tight).warnings[0].line, 2);
  assert.throws(() => loadSync().render('diagram main "T" type=graph {\n  a -> b\n}'), (e) => e instanceof LayupError && e.line === 2);
});

test('renders html', () => {
  assert.match(loadSync().render('diagram main "T" type=graph {\n  node a\n}', { format: 'embed' }).output, /^<!doctype html>/);
});

test('user fonts match CLI measurement and embedding without leaking across renders', () => {
  const path = new URL('crates/layup/tests/fonts/Fallback.ttf', root);
  const source = 'diagram main "中" type=graph {\n  node n "中中中"\n}';
  const font = readFileSync(path);
  const cli = execFileSync(new URL('target/debug/layup', root).pathname, ['render', '-', '-o', '-', '--font', path.pathname], { input: source }).toString();
  const engine = loadSync();
  assert.equal(engine.render(source, { fonts: [font] }).output, cli);
  assert.match(cli, /font-family:'Layup User /);
  assert.doesNotMatch(engine.render(source).output, /font-family:'Layup User /);
  assert.throws(() => engine.render(source, { fonts: [new Uint8Array([0, 1, 2])] }), LayupError);
  assert.throws(() => engine.render(source, { fonts: ['bad'] }), TypeError);
});


test('decision trees match CLI output in each direction', () => {
  const engine = loadSync();
  const tree = readFileSync(new URL('examples/decision-tree.layup', root), 'utf8');
  for (const direction of ['down', 'up', 'right', 'left']) {
    const source = tree.replace('layout=auto width=1000', `layout=auto flow-direction=${direction} width=1400`);
    const cli = execFileSync(new URL('target/debug/layup', root).pathname, ['render', '-', '-o', '-'], { input: source }).toString();
    const rendered = engine.render(source);
    assert.equal(rendered.output, cli);
    assert.deepEqual(rendered.warnings, []);
    assert.match(rendered.output, /<polygon class="box/);
  }
});


test('state machines match CLI output and preserve semantic diagnostics', () => {
  const engine = loadSync();
  const machine = readFileSync(new URL('examples/state-machine.layup', root), 'utf8');
  for (const direction of ['down', 'up', 'right', 'left']) {
    const source = machine.replace('direction=right', `direction=${direction}`);
    const cli = execFileSync(new URL('target/debug/layup', root).pathname, ['render', '-', '-o', '-'], { input: source }).toString();
    const rendered = engine.render(source);
    assert.equal(rendered.output, cli);
    assert.deepEqual(rendered.warnings, []);
    assert.match(rendered.output, /<circle class="box state-final"/);
  }
  assert.throws(() => engine.render('diagram main "Bad machine" type=state-machine { initial s; final end; s -> end; end -> s }'), e => e instanceof LayupError && e.reason.includes('final marker'));
  const source = 'diagram main "Unreachable" type=state-machine { initial s; state a; state b; s -> a }';
  assert.ok(engine.render(source).warnings.some(w => w.message.includes('unreachable')));
});


test('composite states and automatic sizing match CLI in every direction', () => {
  const engine = loadSync();
  const machine = readFileSync(new URL('examples/state-composite.layup', root), 'utf8');
  for (const direction of ['down', 'up', 'right', 'left']) {
    const source = machine.replace('direction=right', `direction=${direction}`);
    const cli = execFileSync(new URL('target/debug/layup', root).pathname, ['render', '-', '-o', '-', '--strict'], { input: source }).toString();
    const rendered = engine.render(source);
    assert.equal(rendered.output, cli);
    assert.deepEqual(rendered.warnings, []);
    assert.ok(rendered.output.includes('data-id="' + xml(objectId('connected')) + '"'));
    assert.ok(rendered.output.includes('data-id="' + xml(objectId('connected', 'sending', 'waiting')) + '"'));
  }
  assert.throws(() => engine.render('diagram main "Missing scope initial" type=state-machine { initial s; state parent { state child }; s -> parent }'), e => e instanceof LayupError && e.reason.includes('composite state `parent`'));
});
