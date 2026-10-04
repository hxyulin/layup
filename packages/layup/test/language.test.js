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
  const source = '// 注释\r\ndiagram "国际"{node a "مرحبا"{sub "path C:\\\\tmp"};node b;a->b "完成"}// tail\r\n';
  const formatted = engine.format(source);
  assert.equal(cli(['fmt', '-'], source).stdout, formatted);
  assert.equal(engine.format(formatted), formatted);
  assert.match(formatted, /^\/\/ 注释\ndiagram/);
  assert.ok(formatted.includes('sub "path C:\\\\tmp"'));
  const normalize = svg => svg.replace(/layup-[0-9a-f]{8}/g, 'layup-namespace');
  assert.equal(normalize(engine.render(source).output), normalize(engine.render(formatted).output));
  assert.throws(() => engine.format('node a href='), e => e instanceof LayupError && e.code === 'parse/attribute-value');
});

test('Unicode diagnostics expose byte ranges, scalar columns, help and related locations', () => {
  const source = 'diagram "😀" { node 中文 aling=left }';
  assert.throws(() => engine.render(source), e => {
    assert.ok(e instanceof LayupError);
    assert.equal(e.line, 1);
    assert.equal(e.column, [...source.slice(0, source.indexOf('aling'))].length + 1);
    assert.equal(slice(source, e.span), 'aling');
    assert.match(e.help, /align/);
    return true;
  });
  const duplicate = 'diagram "T" width=900 width=1200 {}';
  const [error] = engine.lint(duplicate);
  assert.equal(error.code, 'parse/duplicate-attribute');
  assert.equal(slice(duplicate, error.span), 'width');
  assert.equal(slice(duplicate, error.related[0].span), 'width');
  assert.ok(error.related[0].span.start < error.span.start);
});

test('lint collects syntax errors and matches CLI JSON diagnostics', () => {
  for (const source of [
    'diagram "T" {\n node a href=\n a ->\n row 1:1e999 { node b }\n}',
    'diagram "T" { style unused blue; arrow event-kind dashed; node a blue green; node b; a -> b "go"; a -> b "go" }',
    'diagram "T" { node worker; worker -> wokrer }',
  ]) {
    const diagnostics = engine.lint(source);
    const result = cli(['lint', '-', '--json'], source);
    assert.deepEqual(JSON.parse(result.stdout).map(r => r.diagnostic), diagnostics);
    assert.equal(result.status, Number(diagnostics.some(d => d.severity === 'error')));
  }
  assert.equal(engine.lint('diagram "T" {\n node a href=\n a ->\n row 1:1e999 { node b }\n}').length, 3);
  assert.throws(() => engine.lint('diagram "T" {}', { fonts: ['invalid'] }), TypeError);
  const bytes = readFileSync(new URL('crates/layup/tests/fonts/Fallback.ttf', root));
  assert.deepEqual(engine.lint('diagram "中" { node a "中中" }', { fonts: [bytes] }), []);
});

test('Markdown CLI JSON ranges refer to original bytes, including related spans', () => {
  const directory = mkdtempSync(join(tmpdir(), 'layup-language-js-'));
  try {
    const source = '# 标题\r\n\r\n```layup\r\ndiagram "T" width=900 width=1200 {}\r\n```\r\n';
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
  assert.throws(() => md.render('# T\n\n```layup\ndiagram "T" {\n node 中文 aling=left\n}\n```', { path: 'guide.md' }), /guide\.md:5:10: layup error:[\s\S]*did you mean `align`/);
});
