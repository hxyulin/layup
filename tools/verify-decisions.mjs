// Build a review gallery and check rendered glyphs against the actual outlines.
import { chromium } from '../examples/vitepress/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';

mkdirSync('out/decisions', { recursive: true });
const tree = readFileSync('examples/decision-tree.layup', 'utf8');
const cases = ['down', 'up', 'right', 'left'].map(direction => [
  `Decision tree: ${direction}`,
  tree.replace('layout=auto width=1000', `layout=auto flow-direction=${direction} width=1400`),
  'light',
]);
for (const file of ['decision-flow', 'decision-international']) {
  cases.push([file, readFileSync(`examples/${file}.layup`, 'utf8'), 'light']);
}
cases.push(['Dark theme', tree, 'dark']);
cases.push(['Merges and a return path', `diagram main "Retry validation" type=graph layout=auto flow-direction=right width=1400 {
  node request "Request" style=process
  node q "Ready?" style=decision
  node retry "Prepare again" style=process
  node done "Complete" style=terminal
  ::request -> ::q
  ::q -> ::retry "No"
  ::retry -> ::q "Retry"
  ::q -> ::done "Yes"
}`, 'light']);
cases.push(['Long branch captions', `diagram main "Check access" type=graph layout=auto width=1400 {
  node q "Does the user have permission?" style=decision
  node a "Allow access" style=terminal
  node b "Ask an administrator" style=terminal
  ::q -> ::a "Permission has been granted"
  ::q -> ::b "Permission is missing"
}`, 'light']);
let html = '<!doctype html><meta charset=utf-8><title>Decision tree review</title><style>body{background:#eee;margin:24px;font:16px sans-serif}svg{display:block;max-width:100%;height:auto;background:white;margin-bottom:32px}</style>';
for (const [label, source, theme] of cases) {
  const svg = execFileSync(resolve('target/debug/layup'), ['render', '-', '-o', '-', '--theme', theme, '--strict'], { input: source }).toString();
  html += `<h2>${label}</h2>${svg}`;
}
writeFileSync('out/decisions/index.html', html);
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1500, height: 1000 }, deviceScaleFactor: 1 });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(pathToFileURL(resolve('out/decisions/index.html')).href);
  await page.evaluate(() => document.fonts.ready);
  const result = await page.evaluate(() => {
    const failures = [];
    let count = 0;
    for (const node of document.querySelectorAll('svg .node')) {
      const outline = node.querySelector('.box');
      for (const text of node.querySelectorAll('text')) {
        count++;
        const r = text.getBBox();
        for (const [x, y] of [[r.x, r.y], [r.x + r.width, r.y], [r.x, r.y + r.height], [r.x + r.width, r.y + r.height]]) {
          if (!outline.isPointInFill(new DOMPoint(x, y))) {
            failures.push({ id: node.dataset.id, text: text.textContent, x, y, reason: 'glyph bounds outside outline' });
          }
        }
      }
    }
    // Branch captions must remain visible and clear of node fills. These
    // fixtures use upright captions, so getBBox is in the SVG's coordinates.
    for (const svg of document.querySelectorAll('svg')) {
      const captions = [...svg.querySelectorAll('.edge text')];
      for (const text of captions) {
        const r = text.getBBox();
        for (const outline of svg.querySelectorAll('.node .box')) {
          const box = outline.getBBox();
          if (r.x >= box.x + box.width || r.x + r.width <= box.x || r.y >= box.y + box.height || r.y + r.height <= box.y) continue;
          // Sample the overlapped area to detect intersection with slopes.
          for (let x = Math.max(r.x, box.x); x <= Math.min(r.x + r.width, box.x + box.width); x += 2) {
            for (let y = Math.max(r.y, box.y); y <= Math.min(r.y + r.height, box.y + box.height); y += 2) {
              if (outline.isPointInFill(new DOMPoint(x, y))) { failures.push({ text: text.textContent, reason: 'branch caption covers a node' }); break; }
            }
          }
        }
      }
    }
    return { count, failures, diagrams: document.querySelectorAll('svg').length };
  });
  assert.deepEqual(errors, []);
  assert.deepEqual(result.failures, []);
  assert.equal(result.diagrams, cases.length);
  assert.ok(result.count >= 45);
  await page.screenshot({ path: 'out/decisions/preview.png', fullPage: true });
  console.log(`Verified ${result.diagrams} decision diagrams and ${result.count} node text elements against their actual outlines.`);
} finally {
  await browser.close();
}
