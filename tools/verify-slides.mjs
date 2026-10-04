// Render a slide gallery and verify actual browser geometry after uniform fitting.
import { chromium } from '../examples/vitepress/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const base = readFileSync('examples/slides.layup', 'utf8');
const model = `model "Shared services" layout=auto direction=right slide=wide min-font-size=14 {
  process browser "Browser" blue
  process api "API" green
  process store "Storage" purple
  browser -> api "Request"
  api -> store "Query"
  view overview "System overview" { include browser api }
  view data "Data path" slide=standard { include api store }
}`;
const long = `diagram "Too much for one slide" layout=auto direction=right slide=wide min-font-size=24 {
  ${Array.from({ length: 18 }, (_, i) => `process q${i} "Processing step ${i}"; ${i ? `q${i - 1} -> q${i};` : ''}`).join('\n')}
}`;
const cases = [
  { label: 'Wide slide', source: base, width: 1920, height: 1080 },
  { label: 'Standard slide', source: base.replace('slide=wide', 'slide=standard').replace('min-font-size=18', 'min-font-size=16'), width: 1440, height: 1080 },
  { label: 'Portrait slide', source: base.replace('slide=wide', 'slide="1080:1920"').replace('min-font-size=18', 'min-font-size=12'), width: 1080, height: 1920 },
  { label: 'Dark slide', source: base, theme: 'dark', width: 1920, height: 1080 },
  { label: 'CJK and RTL', source: base.replace('"Client"', '"客户端"').replace('"API"', '"واجهة" text-direction=rtl').replace('"Storage"', '"存储"'), width: 1920, height: 1080 },
  { label: 'Shared model: overview', source: model, view: 'overview', width: 1920, height: 1080, nodes: ['browser', 'api'] },
  { label: 'Shared model: data', source: model, view: 'data', width: 1440, height: 1080, nodes: ['api', 'store'] },
  { label: 'Readability warning', source: long, warning: true, width: 1920, height: 1080 },
];
mkdirSync('out/slides', { recursive: true });
let html = '<!doctype html><meta charset="utf-8"><title>Slide sizing review</title><style>body{font:16px sans-serif;background:#e8e8e8;margin:24px}article{max-width:1200px;margin:auto auto 40px}svg{display:block;width:100%;height:auto}pre{white-space:pre-wrap;color:#763d00}</style>';
for (const c of cases) {
  const args = ['render', '-', '-o', '-', '--theme', c.theme || 'light'];
  if (c.view) args.push('--view', c.view);
  if (!c.warning) args.push('--strict');
  const result = spawnSync(resolve('target/debug/layup'), args, { input: c.source, encoding: 'utf8' });
  assert.equal(result.status, 0, `${c.label}: ${result.stderr}`);
  if (c.warning) assert.match(result.stderr, /slide readability/);
  else assert.equal(result.stderr, '', c.label);
  const svg = result.stdout;
  assert.ok(svg.includes(`viewBox="0 0 ${c.width} ${c.height}"`), c.label);
  html += `<article><h2>${c.label}</h2>${c.warning ? '<p>This example intentionally warns that text will be too small for a presentation.</p>' : ''}${svg}</article>`;
}
writeFileSync('out/slides/index.html', html);
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(pathToFileURL(resolve('out/slides/index.html')).href);
  await page.evaluate(() => document.fonts.ready);
  const result = await page.evaluate(expected => {
    const failures = [];
    let textCount = 0;
    [...document.querySelectorAll('article svg')].forEach((svg, index) => {
      const c = expected[index];
      const viewport = svg.viewBox.baseVal;
      if (viewport.width !== c.width || viewport.height !== c.height) failures.push(`${c.label}: wrong viewport`);
      const rect = svg.getBoundingClientRect();
      // getBoundingClientRect includes every nested fit/rotation transform and
      // the actual fallback font selected by the browser for CJK and RTL.
      for (const element of svg.querySelectorAll('.node .box, .node text, .edge path, .edge text, .content > text')) {
        const bounds = element.getBoundingClientRect();
        if (bounds.left < rect.left - 1 || bounds.right > rect.right + 1 || bounds.top < rect.top - 1 || bounds.bottom > rect.bottom + 1) {
          failures.push(`${c.label}: ${element.tagName} outside viewport: ${element.textContent}`);
        }
      }
      for (const node of svg.querySelectorAll('.node')) {
        const outline = node.querySelector('.box');
        if (!outline) continue;
        for (const text of node.querySelectorAll('text')) {
          textCount++;
          const b = text.getBBox();
          for (const [x, y] of [[b.x, b.y], [b.x + b.width, b.y], [b.x, b.y + b.height], [b.x + b.width, b.y + b.height]]) {
            if (!outline.isPointInFill(new DOMPoint(x, y))) failures.push(`${c.label}: node text outside outline: ${text.textContent}`);
          }
        }
      }
      if (c.nodes) {
        const actual = [...svg.querySelectorAll('.node')].map(node => node.dataset.id);
        if (JSON.stringify(actual) !== JSON.stringify(c.nodes)) failures.push(`${c.label}: wrong selected-view nodes`);
      }
    });
    return { failures, textCount, diagrams: document.querySelectorAll('article svg').length };
  }, cases.map(({ label, width, height, nodes }) => ({ label, width, height, nodes })));
  assert.deepEqual(errors, []);
  assert.deepEqual(result.failures, []);
  assert.equal(result.diagrams, cases.length);
  assert.ok(result.textCount >= 30);
  await page.screenshot({ path: 'out/slides/preview.png', fullPage: true });
  await page.locator('article svg').first().screenshot({ path: 'out/slides/wide.png' });
  await page.locator('article svg').nth(4).screenshot({ path: 'out/slides/international.png' });
  console.log(`Verified ${result.diagrams} fitted slides and ${result.textCount} node text elements in Chromium.`);
} finally {
  await browser.close();
}
