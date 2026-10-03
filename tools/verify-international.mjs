// Uses the Chromium dependency already installed by `just vitepress-test`.
// Checks browser shaping against Rust metrics, actual containment, and RTL anchors.
import { chromium } from '../examples/vitepress/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { execFileSync } from 'node:child_process';

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1200, height: 1200 }, deviceScaleFactor: 1 });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.goto(pathToFileURL(resolve('out/international/index.html')).href);
  await page.evaluate(() => document.fonts.ready);
  const result = await page.evaluate(() => {
    const failures = [];
    const texts = [...document.querySelectorAll('svg text')];
    for (const text of texts) {
      const expected = Number(text.dataset.measuredWidth);
      const actual = text.getComputedTextLength();
      if (text.dataset.exact === 'true' && Math.abs(actual - expected) > 0.6) failures.push({ text: text.textContent, expected, actual });
      const node = text.closest('.node');
      if (node) {
        const box = node.querySelector('.box').getBBox();
        const bounds = text.getBBox();
        if (bounds.x < box.x - 0.6 || bounds.x + bounds.width > box.x + box.width + 0.6) {
          failures.push({ text: text.textContent, reason: 'outside node', bounds: { x: bounds.x, width: bounds.width }, box: { x: box.x, width: box.width } });
        }
      }
    }
    return { count: texts.length, failures, faces: [...document.fonts].map(font => ({ family: font.family, status: font.status })) };
  });
  assert.deepEqual(errors, []);
  assert.deepEqual(result.failures, []);
  assert.ok(result.count > 20);
  assert.ok(result.faces.some(face => face.family === 'Layup Arabic' && face.status === 'loaded'));

  await page.screenshot({ path: 'out/international/preview.png', fullPage: true });
  const custom = execFileSync(resolve('target/debug/layup'), ['render', '-', '-o', '-', '--font', resolve('crates/layup/tests/fonts/Fallback.ttf')], {
    input: 'diagram "User font" { row { node cjk "中中中"; node latin "MMM" } }',
  }).toString();
  await page.setContent(custom);
  await page.evaluate(() => document.fonts.ready);
  const lengths = await page.evaluate(() => ['cjk', 'latin'].map(id => document.querySelector(`.node[data-id="${id}"] text`).getComputedTextLength()));
  assert.ok(Math.abs(lengths[0] - lengths[1]) < 0.6, `supplied glyph advances differ: ${lengths}`);
  console.log(`Verified ${result.count} text elements: exact metrics for supplied/bundled fonts and containment for system fallback.`);
} finally {
  await browser.close();
}
