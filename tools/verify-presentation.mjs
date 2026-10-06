// Build, exercise, and capture both standalone and Markdown presentation UI.
import { chromium } from '../examples/vitepress/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { resolve } from 'node:path';

const source = `diagram main "Request walkthrough" type=graph {
  slide size=wide
  row {
    node api "API / 接口" href="https://example.com/api"
    node worker "Worker"
  }
  edge dispatch ::api -> ::worker "dispatch"
  step overview "API entry" {
    show objects=[::api]
    highlight objects=[::api]
    speaker-note "Start with the API."
  }
  step request "Dispatch request" {
    show objects=[::worker]
    show connections=[dispatch]
    highlight connections=[dispatch]
    speaker-note "<b>These are plain text notes.</b> مرحبا"
  }
}`;
mkdirSync('out/presentation', { recursive: true });
const render = format => execFileSync(resolve('target/debug/layup'), ['render', '-', '-o', '-', ...(format === 'html' ? ['--html'] : [])], { input: source }).toString();
const standalone = render('html');
const svg = render('svg');
const focusHtml = execFileSync(resolve('target/debug/layup'), ['render', '-', '-o', '-', '--html'], { input: 'diagram main "Shape focus" type=graph layout=auto {\n  slide size=wide\n  node start "Start" style=terminal\n  node choice "Continue?" style=decision\n  node end "Done" style=terminal\n  ::start -> ::choice\n  ::choice -> ::end\n}' }).toString();
writeFileSync('out/presentation/index.html', standalone);
const wrap = svg => `<div class="layup-diagram">${svg}<button class="layup-expand" aria-label="Expand diagram" hidden>Expand</button></div>`;
const inline = `<!doctype html><meta charset=utf-8><style>body{margin:30px}svg{max-width:100%;height:auto}.layup-diagram{max-width:900px}</style>${wrap(svg)}<script type="module" src="/client.js"></script>`;
const server = createServer((req, res) => {
  const route = req.url.split('?')[0];
  if (['/client.js', '/presentation.js'].includes(route)) {
    res.setHeader('Content-Type', 'text/javascript');
    res.end(readFileSync(resolve(`packages/layup${route}`)));
  } else {
    res.setHeader('Content-Type', 'text/html');
    res.end(route === '/focus' ? focusHtml : route === '/inline' ? inline : route === '/host' ? '<iframe src="/standalone" style="width:100vw;height:95vh;border:0"></iframe><script>window.reports=[];addEventListener("message",e=>reports.push(e.data));</script>' : standalone);
  }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  const shown = scope => scope.locator('svg .node:not(.presentation-hidden)');
  const edges = scope => scope.locator('svg .edge:not(.presentation-hidden)');
  for (const path of ['/standalone', '/inline']) {
    await page.goto(url + path);
    await page.getByRole('button', { name: 'Present', exact: true }).click();
    assert.equal(await shown(page).count(), 1);
    assert.equal(await edges(page).count(), 0);
    assert.equal(await page.locator('.layup-step-status').textContent(), '1 / 2: API entry');
    await page.getByRole('button', { name: 'Next presentation step' }).click();
    assert.equal(await shown(page).count(), 2);
    assert.equal(await edges(page).count(), 1);
    assert.equal(await page.locator('.layup-step-note').textContent(), '<b>These are plain text notes.</b> مرحبا');
    assert.equal(await page.locator('.layup-step-note b').count(), 0);
    await page.keyboard.press('ArrowLeft');
    assert.equal(await shown(page).count(), 1);
    await page.keyboard.press('End');
    assert.equal(await shown(page).count(), 2);
    await page.getByRole('button', { name: 'Show all', exact: true }).click();
    assert.equal(await page.locator('.presentation-hidden').count(), 0);
  }
  await page.goto(url + '/inline');
  await page.getByRole('button', { name: 'Present', exact: true }).click();
  await page.locator('.layup-expand').click({ force: true });
  const dialog = page.getByRole('dialog');
  await dialog.waitFor();
  assert.equal(await shown(dialog).count(), 1);
  await dialog.getByRole('button', { name: 'Next presentation step' }).click();
  assert.equal(await shown(dialog).count(), 2);
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await page.locator('.layup-viewer').waitFor({ state: 'detached' });
  assert.equal(await shown(page).count(), 1, 'Viewer playback must not alter the inline source');
  await page.evaluate(html => document.body.insertAdjacentHTML('beforeend', html), wrap(svg));
  await page.getByRole('button', { name: 'Present', exact: true }).last().click();
  assert.equal(await page.locator('.layup-presentation').count(), 2, 'Navigation-inserted diagrams get one controller');
  await page.evaluate(svg => {
    const host = document.querySelector('.layup-diagram');
    host.querySelector('svg.layup').outerHTML = svg;
  }, svg);
  await page.locator('.layup-diagram').first().getByRole('button', { name: 'Present', exact: true }).click();
  assert.equal(await page.locator('.layup-presentation').count(), 2, 'Replacing a diagram updates its controller without duplicating controls');

  await page.goto(url + '/host');
  const frame = page.frameLocator('iframe');
  await frame.getByRole('button', { name: 'Present', exact: true }).waitFor();
  await page.evaluate(() => document.querySelector('iframe').contentWindow.postMessage({ layup: 'step', id: 'request' }, '*'));
  await frame.locator('.layup-step-status').filter({ hasText: '2 / 2' }).waitFor();
  assert.equal(await shown(frame).count(), 2);
  await page.waitForFunction(() => reports.some(r => r.layup === 'step' && r.id === 'request' && r.index === 1));
  await page.evaluate(() => document.querySelector('iframe').contentWindow.postMessage({ layup: 'step', action: 'previous' }, '*'));
  await frame.locator('.layup-step-status').filter({ hasText: '1 / 2' }).waitFor();
  for (const id of ['start', 'choice']) {
    await page.goto(url + `/focus?focus=${encodeURIComponent("object:" + JSON.stringify([id]))}`);
    const result = await page.evaluate(id => {
      const svg = document.querySelector('svg.layup');
      const box = [...svg.querySelectorAll(".node")].find(n => n.dataset.id === "object:" + JSON.stringify([id])).querySelector(".box");
      const b = box.getBBox();
      const m = svg.getCTM().inverse().multiply(box.getCTM());
      const center = new DOMPoint(b.x + b.width/2, b.y + b.height/2).matrixTransform(m);
      const v = svg.viewBox.baseVal;
      return { dx: Math.abs(v.x + v.width/2 - center.x), dy: Math.abs(v.y + v.height/2 - center.y), finite: [v.x,v.y,v.width,v.height].every(Number.isFinite) };
    }, id);
    assert.ok(result.finite && result.dx < 0.001 && result.dy < 0.001, `Slide-transformed ${id} focus must center actual shape bounds`);
  }
  await page.goto(url + '/standalone?step=request');
  assert.equal(await page.locator('.layup-step-status').textContent(), '2 / 2: Dispatch request');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  assert.equal(await page.locator('svg .node').first().evaluate(node => getComputedStyle(node).transitionDuration), '0s');
  assert.equal(await page.getByRole('button', { name: 'Next presentation step' }).isDisabled(), true);
  await page.screenshot({ path: 'out/presentation/preview.png', fullPage: true });
  assert.deepEqual(errors, []);
  console.log('Presentation verified: standalone, inline, keyboard, full-window playback, dynamic insertion, safe notes, iframe commands/reports, initial step, reduced motion, and slide-transformed pill/diamond focus.');
} finally {
  await browser.close();
  await new Promise(resolve => server.close(resolve));
}
