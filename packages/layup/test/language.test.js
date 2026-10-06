import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { loadSync, LayupError } from '../node.js';
import MarkdownIt from 'markdown-it';
import plugin from '../markdown-it.js';

const root = new URL('../../../', import.meta.url);
const cli = (args, source) => spawnSync(new URL('target/debug/layup', root).pathname, args, { input: source, encoding: 'utf8' });
const engine = loadSync();
const slice = (source, span) => new TextDecoder().decode(new TextEncoder().encode(source).slice(span.start, span.end));

test('format is lossless for strings/comments and matches the CLI', () => {
  const source = '// 注释\r\ndiagram main "国际" type=graph{node a "مرحبا"{text "path C:\\\\tmp"};node b;a->b "完成"}// tail\r\n';
  const formatted = engine.format(source);
  assert.equal(cli(['fmt', '-'], source).stdout, formatted);
  assert.equal(engine.format(formatted), formatted);
  assert.match(formatted, /^\/\/ 注释\ndiagram/);
  assert.ok(formatted.includes('text "path C:\\\\tmp"'));
  const normalize = svg => svg.replace(/<metadata\b[^>]*>[\s\S]*?<\/metadata>/, '').replace(/layup-[0-9a-f]{8}/g, 'layup-namespace');
  assert.equal(normalize(engine.render(source).output), normalize(engine.render(formatted).output));
  assert.throws(() => engine.format('node a href='), e => e instanceof LayupError && e.code === 'document/syntax');
});

test('Unicode diagnostics expose byte ranges, scalar columns, help and related locations', () => {
  const source = 'diagram main "😀" type=graph {\n  node 中文 text-aling=left\n}';
  assert.throws(() => engine.render(source), e => {
    assert.ok(e instanceof LayupError);
    assert.equal(e.line, 2);
    assert.equal(e.column, [...source.split('\n')[1].split('text-aling')[0]].length + 1);
    assert.equal(slice(source, e.span), 'text-aling');
    assert.match(e.help, /text-align/);
    return true;
  });
  const duplicate = 'diagram main "T" type=graph width=900 width=1200 {}';
  const [error] = engine.lint(duplicate);
  assert.equal(error.code, 'document/syntax');
  assert.equal(slice(duplicate, error.span), 'width');
  assert.equal(slice(duplicate, error.related[0].span), 'width');
  assert.ok(error.related[0].span.start < error.span.start);
});

test('lint collects syntax errors and matches CLI JSON diagnostics', () => {
  for (const source of [
    'diagram main "T" type=graph {\n node a href=\n a ->\n row weights=[1, 1e999] { node b }\n}',
    'diagram main "T" type=graph {\n  node-style unused palette=blue\n  edge-style event-kind stroke-style=dashed\n  node a palette=blue palette=green\n  node b\n  ::a -> ::b "go"\n  ::a -> ::b "go"\n}',
    'diagram main "T" type=graph {\n  node worker\n  ::worker -> wokrer\n}',
  ]) {
    const diagnostics = engine.lint(source);
    const result = cli(['lint', '-', '--json'], source);
    assert.deepEqual(JSON.parse(result.stdout).map(r => r.diagnostic), diagnostics);
    assert.equal(result.status, Number(diagnostics.some(d => d.severity === 'error')));
  }
  assert.equal(engine.lint('diagram main "T" type=graph {\n node a href=\n a ->\n row weights=[1, 1e999] { node b }\n}').length, 3);
  assert.throws(() => engine.lint('diagram main "T" type=graph {\n}', { fonts: ['invalid'] }), TypeError);
  const bytes = readFileSync(new URL('crates/layup/tests/fonts/Fallback.ttf', root));
  assert.deepEqual(engine.lint('diagram main "中" type=graph {\n  node a "中中"\n}', { fonts: [bytes] }), []);
});

test('Markdown CLI JSON ranges refer to original bytes, including related spans', () => {
  const directory = mkdtempSync(join(tmpdir(), 'layup-language-js-'));
  try {
    const source = '# 标题\r\n\r\n```layup\r\ndiagram main "T" type=graph width=900 width=1200 {}\r\n```\r\n';
    const file = join(directory, 'guide.md');
    writeFileSync(file, source);
    const [result] = JSON.parse(cli(['lint', file, '--json'], '').stdout);
    assert.equal(result.fenceLine, 3);
    assert.equal(result.diagnostic.line, 4);
    assert.equal(slice(source, result.diagnostic.span), 'width');
    assert.equal(slice(source, result.diagnostic.related[0].span), 'width');
    assert.equal(result.diagnostic.related[0].span.line, 4);
  } finally { rmSync(directory, { recursive: true }); }
});

test('Markdown errors include source columns and actionable help', () => {
  const md = new MarkdownIt().use(plugin, { strict: true });
  assert.throws(() => md.render('# T\n\n```layup\ndiagram main "T" type=graph {\n node 中文 text-aling=left\n}\n```', { path: 'guide.md' }), /guide\.md:5:10: layup error:[\s\S]*did you mean `text-align`/);
});
