import { test } from 'node:test';
import assert from 'node:assert/strict';
import MarkdownIt from 'markdown-it';
import { readFileSync } from 'node:fs';
import layup, { vitepress } from '../markdown-it.js';

const doc = 'Intro\n\n```layup\ndiagram main "T" type=graph {\n  node a "A {{ b }}"\n}\n```\n\n```js\nlet x\n```\n';

test('renders layup fences and leaves others alone', () => {
  const html = new MarkdownIt().use(layup).render(doc);
  assert.match(html, /<div class="layup-diagram"><svg style="max-width:100%;height:auto" /);
  assert.match(html, /<style>/);
  assert.match(html, /@media \(prefers-color-scheme: dark\)/);
  assert.match(html, /<code class="language-js">/);
  assert.match(html, /<button type="button" class="layup-expand" hidden/);
});

test('vitepress output survives Vue compilation', () => {
  const html = new MarkdownIt().use(vitepress).render(doc);
  assert.doesNotMatch(html, /<style>/);
  assert.match(html, /<component is="style">[^<]*:is\(\.dark\) \.layup\.auto\{/);
  assert.match(html, /A\u00a0&#123;&#123;\u00a0b\u00a0&#125;&#125;/);
});

test('reports errors at the Markdown line', () => {
  const md = new MarkdownIt().use(layup, { strict: true });
  assert.throws(() => md.render('x\n\n```layup\ndiagram main "T" type=graph {\n  a -> b\n}\n```\n', { relativePath: 'guide.md' }), /^Error: guide\.md:5:3: layup error: /);
  const shown = new MarkdownIt().use(layup);
  const original = console.error;
  console.error = () => {};
  try {
    assert.match(shown.render('```layup\ndiagram {\n```\n'), /<pre class="layup-error">markdown:2:9: layup error: /);
  } finally {
    console.error = original;
  }
});

test('shows the source after the diagram when asked', () => {
  const html = new MarkdownIt().use(layup).render('```layup source\ndiagram main "T" type=graph { node a }\n```\n');
  const diagram = html.indexOf('<div class="layup-diagram">');
  const code = html.indexOf('<pre><code class="language-text">diagram main &quot;T&quot; type=graph { node a }');
  assert.ok(diagram >= 0 && code > diagram, html);
  assert.doesNotMatch(new MarkdownIt().use(layup).render('```layup\ndiagram main "T" type=graph {}\n```\n'), /<pre>/);
});


test('passes supplied fallback fonts through the Markdown plugin', () => {
  const font = readFileSync(new URL('../../../crates/layup/tests/fonts/Fallback.ttf', import.meta.url));
  const html = new MarkdownIt().use(layup, { fonts: [font], strict: true }).render('```layup\ndiagram main "中" type=graph { node n "中中中" }\n```');
  assert.match(html, /font-family:'Layup User /);
});
