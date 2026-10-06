import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import MarkdownIt from 'markdown-it';
import plugin from '../markdown-it.js';
import { loadSync, LayupError } from '../node.js';

const root = new URL('../../../', import.meta.url);
const cli = (args, source) => spawnSync(new URL('target/debug/layup', root).pathname, args, { input: source, encoding: 'utf8' });
const engine = loadSync();
const example = readFileSync(new URL('examples/language-v1.layup', root), 'utf8');
const source = 'layup 1\ndiagram first "First" type=graph { node a }\ndiagram second "Second" type=graph { node b }';

test('revision-one scoped graphs retain source evidence with native/WASM parity', () => {
  const native = cli(['compile', '-'], example);
  assert.equal(native.status, 0, native.stderr);
  const scene = engine.compile(example);
  assert.deepEqual(scene, JSON.parse(native.stdout));
  const api = scene.nodes.find(n => n.objectPath.at(-1) === 'API::run(&self)');
  assert.equal(api.sourceLocations[0].uri, 'src/api.rs');
  assert.equal(api.sourceLocations[0].range.startLine, 8);
  assert.equal(api.metadata.analysis.public, true);
  assert.equal(scene.document.diagramId, 'services');
  assert.deepEqual(engine.lint(example), []);
  const formatted = engine.format(example);
  assert.equal(engine.format(formatted), formatted);
  assert.equal(cli(['fmt', '-'], example).stdout, formatted);
  for (const theme of ['light', 'dark', 'auto']) {
    assert.match(engine.render(example, { theme }).output, /data-layup-document="1"/);
  }
  assert.match(engine.render(example, { format: 'html' }).output, /data-layup-document="1"/);
});

test('diagram selection is consistent in CLI, WASM and embedded Markdown', () => {
  assert.equal(engine.compile(source).title, 'First');
  const scene = engine.compile(source, { diagram: 'second' });
  const native = cli(['compile', '-', '--diagram', 'second'], source);
  assert.equal(native.status, 0, native.stderr);
  assert.deepEqual(scene, JSON.parse(native.stdout));
  assert.equal(scene.title, 'Second');
  assert.deepEqual(scene.nodes[0].objectPath, ['b']);
  assert.deepEqual(engine.lint(source, { diagram: 'second' }), []);
  const md = new MarkdownIt().use(plugin, { diagram: 'first', strict: true });
  const html = md.render('```layup diagram=second\n' + source + '\n```');
  assert.match(html, /Second/);
  assert.match(html, /data-layup-document/);
  assert.throws(() => engine.compile(source, { diagram: 'missing' }), e => e instanceof LayupError && e.code === 'document/diagram');
  assert.throws(() => engine.render(source, { diagram: 'second\noperation=format' }), TypeError);
  assert.throws(() => engine.lint(source, { diagram: 1 }), TypeError);
  assert.throws(() => engine.compile('diagram main "Legacy" type=graph {}', { diagram: 'second' }), LayupError);
});

test('versioned invalid input does not fall back or hide errors during selection', () => {
  for (const invalid of [
    source.replace('layup 1', 'layup 2'),
    source.replace('type=graph', 'type=sequence'),
    source.replace('node b', 'node b\nb -> missing'),
  ]) {
    assert.throws(() => engine.compile(invalid, { diagram: 'first' }), e => e instanceof LayupError && e.span !== null);
    const diagnostics = engine.lint(invalid, { diagram: 'first' });
    assert.equal(diagnostics[0].severity, 'error');
    const native = cli(['lint', '-', '--diagram', 'first', '--json'], invalid);
    assert.deepEqual(diagnostics, JSON.parse(native.stdout).map(r => r.diagnostic));
  }
});

test('inline and nested multiline comments preserve native/WASM formatting and spans', () => {
  const input = '/* 注释 /* nested */ */\nlayup /* revision */ 1\ndiagram main type=graph {\nnode a /* label\ncontinues */ "A" palette=/* color */blue // inline\nnode b "/* literal */"\na/* from */->b\n}';
  const native = cli(['compile', '-'], input);
  assert.equal(native.status, 0, native.stderr);
  assert.deepEqual(engine.compile(input), JSON.parse(native.stdout));
  const formatted = engine.format(input);
  assert.equal(formatted, cli(['fmt', '-'], input).stdout);
  assert.equal(engine.format(formatted), formatted);
  assert.match(formatted, /\/\* label\ncontinues \*\//);
  assert.deepEqual(engine.lint(input), []);
  const invalid = 'layup 1\ndiagram main type=graph { /* unfinished';
  assert.throws(() => engine.compile(invalid), e => e instanceof LayupError && e.code === 'document/comment');
  assert.equal(engine.lint(invalid)[0].code, 'document/comment');
});

test('optional revision, opaque diagrams and inspection preserve exact values', () => {
  const input = 'diagram future type=vendor.timeline { launch => 2027-01-01 ? custom }\ndiagram main type=graph {\n@vendor.data(choice=blue, integer=9007199254740993, max=18446744073709551615, float=1.0) node api\n}';
  const scene = engine.compile(input);
  assert.equal(scene.document.diagramId, 'main');
  assert.deepEqual(scene.document.manifest.map(entry => entry.status), ['skipped', 'selected']);
  const annotations = scene.document.objects['object:["api"]'].annotations;
  assert.deepEqual(annotations[0].arguments.integer.value, { type: 'integer', value: '9007199254740993' });
  assert.equal(BigInt(annotations[0].arguments.max.value.value), 18446744073709551615n);
  assert.deepEqual(annotations[0].arguments.choice.value, { type: 'choice', value: 'blue' });
  assert.deepEqual(annotations[0].arguments.float.value, { type: 'float', value: 1 });
  const inspected = engine.inspect(input);
  const native = cli(['inspect', '-'], input);
  assert.equal(native.status, 0, native.stderr);
  assert.deepEqual(inspected, JSON.parse(native.stdout));
  assert.equal(inspected.document.diagrams[0].body.type, 'opaque');
  assert.equal(inspected.document.diagrams[0].body.value.raw, '{ launch => 2027-01-01 ? custom }');
  assert.equal(inspected.diagnostics[0].code, 'diagram/unrecognized-type');
  assert.match(engine.format(input), /\{ launch => 2027-01-01 \? custom \}/);
  assert.throws(() => engine.compile(input, { diagram: 'future' }), error => error.code === 'diagram/unavailable-selection');
  assert.throws(() => engine.compile('layup 1.0\ndiagram main type=graph {}'), error => error.code === 'document/version');
  const invalid = 'diagram main type=graph {\nnode broken width=\nnode valid\nunknown directive\nnode later\n}';
  const recovery = engine.inspect(invalid);
  assert.equal(recovery.diagnostics.filter(d => d.severity === 'error').length, 2);
  assert.deepEqual(recovery.document.diagrams[0].body.value.map(item => item.value.id), ['valid', 'later']);
  assert.equal(cli(['inspect', '-'], invalid).status, 1);
  assert.deepEqual(engine.lint(invalid), JSON.parse(cli(['lint', '-', '--json'], invalid).stdout).map(row => row.diagnostic));
});
