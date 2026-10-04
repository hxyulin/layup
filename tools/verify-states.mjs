// Build a review gallery and check rendered glyphs against the actual outlines.
import { chromium } from '../examples/vitepress/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';

mkdirSync('out/states', { recursive: true });
const machine = readFileSync('examples/state-machine.layup', 'utf8');
const cases = ['down', 'up', 'right', 'left'].map(direction => [
  `State machine: ${direction}`,
  machine.replace('direction=right', `direction=${direction}`),
  'light',
]);
for (const file of ['state-choice', 'state-international']) {
  cases.push([file, readFileSync(`examples/${file}.layup`, 'utf8'), 'light']);
}
cases.push(['Dark theme', machine, 'dark']);
cases.push(['Labeled choice', `diagram "Check access" mode=state-machine direction=right width=1600 {
  initial start; state request "Request received"; choice permitted "Allowed?"
  state grant "Grant access"; state deny "Deny access"; final end
  start -> request; request -> permitted "check"
  permitted -> grant "[allowed]"; permitted -> deny "[else]"
  grant -> end "close"; deny -> end "close"
}`, 'light']);
const composite = readFileSync('examples/state-composite.layup', 'utf8');
for (const direction of ['down', 'up', 'right', 'left']) {
  cases.push([`Composite states: ${direction}`, composite.replace('direction=right', `direction=${direction}`), 'light']);
}
cases.push(['Composite states: dark', composite, 'dark']);
cases.push(['Composite states: CJK and RTL', composite.replace('"Connected"', '"متصل" text-direction=rtl').replace('entry / open(); exit / close()', 'دخول / فتح(); خروج / إغلاق()').replace('"Awaiting reply"', '"等待回复"').replace('"Retrying"', '"重试"'), 'light']);
for (const direction of ['down', 'up', 'right', 'left']) {
  cases.push([`Local composite transitions: ${direction}`, `diagram "Local transitions" mode=state-machine direction=${direction} {
    initial root; state parent "Parent" { initial enter; state a "A"; state b "B"; enter -> a; a -> b }
    root -> parent; b -> parent "reset"; parent -> a "resume"
  }`, 'light']);
}
const wide = `diagram "Automatic tree sizing" layout=auto { decision root "Choose outcome"; ${Array.from({length: 16}, (_, i) => `terminal q${i} "Outcome ${i}"; root -> q${i} "${i}";`).join(' ')} }`;
cases.push(['Automatic sizing: sixteen outcomes', wide, 'light']);
cases.push(['Automatic sizing: compact states', machine.replace('width=1400', ''), 'light']);
let html = '<!doctype html><meta charset=utf-8><title>State machine review</title><style>body{background:#eee;margin:24px;font:16px sans-serif}svg{display:block;max-width:100%;height:auto;background:white;margin-bottom:32px}</style>';
for (const [label, source, theme] of cases) {
  const svg = execFileSync(resolve('target/debug/layup'), ['render', '-', '-o', '-', '--theme', theme, '--strict'], { input: source }).toString();
  html += `<h2>${label}</h2>${svg}`;
}
writeFileSync('out/states/index.html', html);
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1500, height: 1000 }, deviceScaleFactor: 1 });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(pathToFileURL(resolve('out/states/index.html')).href);
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
          // Composite frames legitimately contain transitions and captions.
          if ([...svg.querySelectorAll('.node .box')].some(other => {
            if (other === outline) return false;
            const r = other.getBBox();
            return r.x > box.x && r.y > box.y && r.x + r.width < box.x + box.width && r.y + r.height < box.y + box.height;
          })) continue;
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
  assert.ok(result.count >= 25);
  const markers = await page.locator('circle.box').count();
  assert.equal(markers, 62);
  await page.screenshot({ path: 'out/states/preview.png', fullPage: true });
  await page.locator('svg').nth(10).screenshot({ path: 'out/states/composite.png' });
  await page.locator('svg').nth(13).screenshot({ path: 'out/states/composite-international.png' });
  console.log(`Verified ${result.diagrams} state-machine diagrams and ${result.count} node text elements against their actual outlines.`);
} finally {
  await browser.close();
}
