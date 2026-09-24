// Checks the built site in Chromium: hydration, fonts, theming, and the
// layup/client interactions. Run `npx vitepress build` first; `just
// vitepress-test` does both.
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { chromium } from 'playwright';

const PORT = 4174;
const URL = `http://localhost:${PORT}/`;
let server;
let browser;
let page;
const errors = [];

before(async () => {
  server = spawn('npx', ['vitepress', 'preview', '--port', String(PORT)], { stdio: 'ignore' });
  for (let i = 0; ; i++) {
    try {
      if ((await fetch(URL)).ok) break;
    } catch {}
    if (i > 100) throw new Error('vitepress preview did not start');
    await new Promise((r) => setTimeout(r, 100));
  }
  browser = await chromium.launch();
  page = await browser.newPage({ viewport: { width: 1100, height: 900 } });
  page.on('console', (m) => ['error', 'warning'].includes(m.type()) && errors.push(m.text()));
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto(URL, { waitUntil: 'networkidle' });
});

after(async () => {
  await browser?.close();
  server?.kill();
});

const svg = (n = 0) => page.locator('.layup-diagram svg.layup').nth(n);
const node = (id, scope = '.layup-diagram >> nth=0') => page.locator(`${scope} >> .node[data-id="${id}"] .box`);
const state = () =>
  svg().evaluate((s) => ({ pinned: s.dataset.pinned ?? null, hl: [...s.querySelectorAll('.node.hl')].map((n) => n.dataset.id) }));

test('hydrates with styles and fonts intact', async () => {
  assert.equal(await page.locator('.layup-diagram svg style').count(), 3);
  await page.evaluate(() => document.fonts.ready);
  const faces = await page.evaluate(() => [...document.fonts].filter((f) => f.family.startsWith('Layup')).map((f) => f.status));
  assert.ok(faces.length === 9 && faces.every((s) => s === 'loaded'), String(faces));
  assert.match((await svg(1).locator("text").allTextContents()).join("\n"), /\{\{\svalue\s\}\}/);
  assert.equal(await page.locator('.layup-diagram + div[class*="language-"] code, .layup-diagram + pre code').count(), 1);
});

test('follows the VitePress dark class', async () => {
  const fill = () => node('api').evaluate((b) => getComputedStyle(b).fill);
  const light = await fill();
  await page.evaluate(() => document.documentElement.classList.add('dark'));
  assert.notEqual(await fill(), light);
  await page.evaluate(() => document.documentElement.classList.remove('dark'));
});

test('hover highlights and click pins', async () => {
  await node('store').hover();
  assert.deepEqual((await state()).hl.sort(), ['store', 'worker']);
  await page.mouse.move(2, 2);
  assert.deepEqual((await state()).hl, []);
  await node('store').click();
  await page.mouse.move(2, 2);
  assert.equal((await state()).pinned, 'store');
  await node('store').click();
  assert.equal((await state()).pinned, null);
});

test('the viewer zooms, pans, pins, responds to keys and closes', async () => {
  await page.hover('.layup-diagram >> nth=0');
  await page.click('.layup-diagram >> nth=0 >> .layup-expand');
  const viewBox = () => page.getAttribute('.layup-viewer svg', 'viewBox');
  const fitted = await viewBox();
  await page.mouse.move(550, 450);
  await page.mouse.wheel(0, -300);
  const zoomed = await viewBox();
  assert.notEqual(zoomed, fitted);
  await page.mouse.down();
  await page.mouse.move(600, 480, { steps: 4 });
  await page.mouse.up();
  assert.notEqual(await viewBox(), zoomed);
  await page.keyboard.press('0');
  assert.equal(await viewBox(), fitted);
  await page.keyboard.press('+');
  assert.notEqual(await viewBox(), fitted);
  await page.keyboard.press('0');
  await node('worker', '.layup-viewer').click();
  assert.equal(await page.locator('.layup-viewer svg').evaluate((s) => s.dataset.pinned), 'worker');
  await page.keyboard.press('Escape');
  await page.locator('dialog.layup-viewer').waitFor({ state: 'detached' });
});

test('node links navigate client-side and handlers survive navigation', async () => {
  const navigation = await page.evaluate(() => performance.getEntriesByType('navigation').length);
  await node('api').click();
  await page.waitForURL('**/other*');
  assert.equal(await page.evaluate(() => performance.getEntriesByType('navigation').length), navigation);
  await page.goBack();
  await node('worker').hover();
  assert.ok((await state()).hl.includes('worker'));
});

test('logs no errors or warnings', () => {
  assert.deepEqual(errors, []);
});
