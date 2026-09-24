import { test } from 'node:test';
import assert from 'node:assert/strict';
import MarkdownIt from 'markdown-it';
import layup, { vitepress } from '../markdown-it.js';

const doc = 'Intro\n\n```layup\ndiagram "T" {\n  node a "A {{ b }}"\n}\n```\n\n```js\nlet x\n```\n';

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
  assert.throws(() => md.render('x\n\n```layup\ndiagram "T" {\n  a -> b\n}\n```\n', { relativePath: 'guide.md' }), /^Error: guide\.md:5: layup error: /);
  const shown = new MarkdownIt().use(layup);
  const original = console.error;
  console.error = () => {};
  try {
    assert.match(shown.render('```layup\ndiagram {\n```\n'), /<pre class="layup-error">markdown:2: layup error: /);
  } finally {
    console.error = original;
  }
});
