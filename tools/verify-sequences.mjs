// Browser checks for event chronology, measured Unicode labels, and playback.
import { chromium } from '../examples/vitepress/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const base = readFileSync('examples/sequence.layup', 'utf8');
const international = readFileSync('examples/sequence-international.layup', 'utf8');
const model = readFileSync('examples/presentation-model.layup', 'utf8');
const playback = `diagram "Step through a request" mode=sequence slide=wide min-font-size=1 {
  participant client "Client" blue
  participant api "API" green
  client -> api "Request" id=request
  api -> api "Validate\nAuthenticate" id=validate
  api -> client "Response" return id=response
  step request "Send the request" { show client api; show-edge request; highlight-edge request }
  step validate "Validate it" { show-edge validate; highlight-edge validate }
  step response "Return the response" { show-edge response; highlight-edge response }
}`;
const cases = [
  ['Request lifecycle', base, 'light'],
  ['Participant order: left', base.replace('mode=sequence', 'mode=sequence direction=left'), 'light'],
  ['Dark theme', base, 'dark'],
  ['International messages', international, 'light'],
  ['International messages: left', international.replace('mode=sequence', 'mode=sequence direction=left'), 'light'],
  ['Multiline messages at authored width', `diagram "Multiline labels" mode=sequence width=900 {
    participant a "Browser with a long participant title"
    participant b "API"
    a -> b "A request with several parameters and additional context\nExplicit second line" id=request
    b -> b "Perform local validation\nThen persist the result" id=validate
    note "Notes can also span multiple lines.\nThis line is authored explicitly." from=a to=b
    b -> a "Return a response after all processing has completed" return id=response
  }`, 'light'],
  ['One participant and a self-call', 'diagram "Local operation" mode=sequence { participant worker "Worker"; worker -> worker "Process the batch" async id=process }', 'light'],
  ['Sequence on a slide', playback, 'light'],
  ['RTL title, subtitle and fragments', `diagram "طلب متعدد الخطوات" mode=sequence text-direction=rtl {
    subtitle "تفاصيل معالجة الطلب"
    participant client "عميل"; participant api "خادم"
    loop "إعادة المحاولة" { client -> api "طلب"; api -> client "استجابة" return }
  }`, 'light'],
  ['Participant styles, monospace and wrapped roles', `diagram "Styled participants" mode=sequence width=900 {
    style participant process mono blue
    style actor process hollow purple
    actor user "User"
    participant api "ConnectionPool<RequestContext>" role="The service owns authorization, validation, processing and persistence for every incoming request."
    user -> api "Call"
  }`, 'light'],
  ['Shared model walkthrough', model, 'light', 'walkthrough'],
];
mkdirSync('out/sequences', { recursive: true });
function render(source, theme, html = false, view) {
  const args = ['render', '-', '-o', '-', '--theme', theme, '--strict'];
  if (html) args.push('--html');
  if (view) args.push('--view', view);
  const result = spawnSync(resolve('target/debug/layup'), args, { input: source, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stderr, '');
  return result.stdout;
}
let html = '<!doctype html><meta charset="utf-8"><title>Sequence diagram review</title><style>body{font:16px sans-serif;background:#eee;margin:24px}article{max-width:1300px;margin:auto auto 40px}svg{display:block;max-width:100%;height:auto}</style>';
for (const [label, source, theme, view] of cases) html += `<article><h2>${label}</h2>${render(source, theme, false, view)}</article>`;
writeFileSync('out/sequences/index.html', html);
writeFileSync('out/sequences/playback.html', render(playback, 'light', true));
writeFileSync('out/sequences/model-playback.html', render(model, 'light', true, 'walkthrough'));
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1500, height: 1000 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(pathToFileURL(resolve('out/sequences/index.html')).href);
  await page.evaluate(() => document.fonts.ready);
  const result = await page.evaluate(() => {
    const failures = [];
    let texts = 0;
    let messages = 0;
    let fragmentHeadings = 0;
    [...document.querySelectorAll('article svg')].forEach((svg, index) => {
      const viewport = svg.getBoundingClientRect();
      const headers = [...svg.querySelectorAll('.node .box')].map(node => node.getBoundingClientRect());
      let previousY = -Infinity;
      for (const edge of svg.querySelectorAll('.edge')) {
        messages++;
        const path = edge.querySelector('path.ln');
        const points = path.getAttribute('d').match(/-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi).map(Number);
        if (!(points[1] > previousY)) failures.push(`Case ${index}: chronology did not advance`);
        previousY = points.at(-1);
        const labelBox = edge.querySelector('rect.box');
        for (const text of edge.querySelectorAll('text')) {
          texts++;
          const actual = text.getBoundingClientRect();
          if (headers.some(header => actual.left < header.right && actual.right > header.left && actual.top < header.bottom && actual.bottom > header.top)) {
            failures.push(`Case ${index}: message label covers participant header: ${text.textContent}`);
          }
          if (labelBox) {
            const glyphs = text.getBBox();
            for (const [x, y] of [[glyphs.x, glyphs.y], [glyphs.x + glyphs.width, glyphs.y], [glyphs.x, glyphs.y + glyphs.height], [glyphs.x + glyphs.width, glyphs.y + glyphs.height]]) {
              if (!labelBox.isPointInFill(new DOMPoint(x, y))) failures.push(`Case ${index}: label glyph outside measured background: ${text.textContent}`);
            }
          }
        }
      }
      for (const text of svg.querySelectorAll('.content text')) {
        texts++;
        const actual = text.getBoundingClientRect();
        if (actual.left < viewport.left - 1 || actual.right > viewport.right + 1 || actual.top < viewport.top - 1 || actual.bottom > viewport.bottom + 1) {
          failures.push(`Case ${index}: text outside viewport: ${text.textContent}`);
        }
        if (/^(?:loop|alt|branch|opt):/.test(text.textContent)) {
          fragmentHeadings++;
          const background = text.previousElementSibling;
          if (!background?.matches('rect.box.white')) {
            failures.push(`Case ${index}: fragment heading has no background: ${text.textContent}`);
          } else {
            const glyphs = text.getBBox();
            for (const [x, y] of [[glyphs.x, glyphs.y], [glyphs.x + glyphs.width, glyphs.y], [glyphs.x, glyphs.y + glyphs.height], [glyphs.x + glyphs.width, glyphs.y + glyphs.height]]) {
              if (!background.isPointInFill(new DOMPoint(x, y))) failures.push(`Case ${index}: fragment heading glyph outside background: ${text.textContent}`);
            }
          }
        }
      }
      for (const node of svg.querySelectorAll('.node')) {
        const outline = node.querySelector('.box');
        for (const text of node.querySelectorAll('text')) {
          const glyphs = text.getBBox();
          for (const [x, y] of [[glyphs.x, glyphs.y], [glyphs.x + glyphs.width, glyphs.y], [glyphs.x, glyphs.y + glyphs.height], [glyphs.x + glyphs.width, glyphs.y + glyphs.height]]) {
            if (!outline.isPointInFill(new DOMPoint(x, y))) failures.push(`Case ${index}: header glyph outside outline: ${text.textContent}`);
          }
        }
      }
    });
    const sample = document.querySelector('article svg');
    const returned = sample.querySelector('.edge[data-edge-id="accepted"] path.ln');
    if (!returned.hasAttribute('stroke-dasharray')) failures.push('Return message is not dashed');
    const asynchronous = sample.querySelector('.edge[data-edge-id="enqueue"] path.ln');
    if (!asynchronous.getAttribute('marker-end').includes('mo-')) failures.push('Async message has no open arrowhead');
    const marker = document.getElementById(asynchronous.getAttribute('marker-end').slice(5, -1)).querySelector('path');
    if (marker.getAttribute('fill') !== 'none') failures.push('Async arrowhead is filled');
    return { failures, texts, messages, fragmentHeadings, diagrams: document.querySelectorAll('article svg').length };
  });
  assert.deepEqual(result.failures, []);
  assert.equal(result.diagrams, cases.length);
  assert.ok(result.messages >= 30);
  assert.ok(result.fragmentHeadings >= 10);
  await page.screenshot({ path: 'out/sequences/preview.png', fullPage: true });
  await page.locator('article svg').first().screenshot({ path: 'out/sequences/request.png' });
  await page.locator('article svg').nth(3).screenshot({ path: 'out/sequences/international.png' });
  await page.locator('article svg').nth(8).screenshot({ path: 'out/sequences/rtl.png' });
  await page.locator('article svg').nth(9).screenshot({ path: 'out/sequences/styled.png' });
  await page.locator('article svg').nth(10).screenshot({ path: 'out/sequences/model.png' });
  await page.goto(pathToFileURL(resolve('out/sequences/playback.html')).href);
  await page.evaluate(() => document.fonts.ready);
  const paths = await page.locator('.edge path.ln').evaluateAll(edges => edges.map(e => e.getAttribute('d')));
  await page.locator('[data-step="present"]').click();
  const settledHighlight = () => page.waitForFunction(() => {
    const highlighted = document.querySelector('.edge.presentation-highlight');
    const others = [...document.querySelectorAll('.edge:not(.presentation-highlight):not(.presentation-hidden)')];
    return highlighted && getComputedStyle(highlighted).opacity === '1' && others.every(edge => getComputedStyle(edge).opacity === '0.3');
  });
  await settledHighlight();
  const visibleEdges = () => page.locator('.edge').evaluateAll(edges => edges.filter(e => getComputedStyle(e).visibility !== 'hidden').map(e => e.dataset.edgeId));
  assert.deepEqual(await visibleEdges(), ['request']);
  assert.equal(await page.locator('.edge[data-edge-id="response"] text').evaluate(text => getComputedStyle(text).visibility), 'hidden');
  await page.screenshot({ path: 'out/sequences/step-request.png' });
  await page.locator('[data-step="next"]').click();
  await settledHighlight();
  assert.deepEqual(await visibleEdges(), ['request', 'validate']);
  await page.screenshot({ path: 'out/sequences/step-validate.png' });
  await page.locator('[data-step="next"]').click();
  await settledHighlight();
  assert.deepEqual(await visibleEdges(), ['request', 'validate', 'response']);
  assert.deepEqual(await page.locator('.edge path.ln').evaluateAll(edges => edges.map(e => e.getAttribute('d'))), paths);
  await page.locator('[data-step="all"]').click();
  assert.deepEqual(await visibleEdges(), ['request', 'validate', 'response']);
  await page.goto(pathToFileURL(resolve('out/sequences/model-playback.html')).href);
  await page.evaluate(() => document.fonts.ready);
  await page.locator('[data-step="present"]').click();
  await settledHighlight();
  assert.deepEqual(await visibleEdges(), ['request']);
  await page.locator('[data-step="next"]').click();
  await settledHighlight();
  assert.deepEqual(await visibleEdges(), ['request', 'enqueue', 'accepted']);
  await page.screenshot({ path: 'out/sequences/model-dispatch.png' });
  await page.locator('[data-step="next"]').click();
  await settledHighlight();
  assert.deepEqual(await visibleEdges(), ['request', 'enqueue', 'accepted', 'poll', 'pending']);
  await page.screenshot({ path: 'out/sequences/model-retry.png' });
  await page.locator('[data-step="next"]').click();
  await settledHighlight();
  assert.deepEqual(await visibleEdges(), ['request', 'enqueue', 'accepted', 'poll', 'pending', 'result']);
  assert.deepEqual(errors, []);
  console.log(`Verified ${result.diagrams} sequence diagrams, ${result.messages} ordered messages, ${result.texts} text elements, ${result.fragmentHeadings} masked fragment headings, and 7 playback steps in Chromium.`);
} finally {
  await browser.close();
}
