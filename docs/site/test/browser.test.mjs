import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { encodeShare } from '../.vitepress/theme/share.js';

const port = 4176;
const base = process.env.LAYUP_DOCS_BASE || '/layup/';
const origin = `http://127.0.0.1:${port}`;
const site = `${origin}${base}`;
let server;
let browser;
let context;
let page;
const errors = [];
const badAssets = [];

before(async () => {
  server = spawn('pnpm', ['exec', 'vitepress', 'preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { stdio: 'ignore', detached: process.platform !== 'win32' });
  for (let i = 0; ; i++) {
    try { if ((await fetch(site)).ok) break; } catch {}
    if (i > 100) throw new Error('Documentation preview did not start');
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  browser = await chromium.launch();
  context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, permissions: ['clipboard-read', 'clipboard-write'] });
  page = await context.newPage();
  page.setDefaultTimeout(10000);
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  page.on('response', response => { if (response.status() >= 400 && response.url().startsWith(origin)) badAssets.push(response.url()); });
});

after(async () => {
  await browser?.close();
  if (server?.pid) {
    try { if (process.platform === 'win32') server.kill(); else process.kill(-server.pid, 'SIGTERM'); } catch {}
  }
});

const editor = () => page.locator('.live-editor');
const source = () => editor().locator('textarea');
const preview = () => editor().locator('.rendered-svg svg.layup');
async function ready() {
  await page.waitForFunction(() => document.querySelector('.live-editor .status-ready'), null, { timeout: 30000 });
}
async function edit(text) { await source().fill(text); await ready(); }
async function openEditor(example = 'hello') {
  await page.goto(`${site}playground.html?example=${example}`);
  await ready();
}
async function download(label) {
  const waiting = page.waitForEvent('download');
  await editor().getByRole('button', { name: label, exact: true }).click();
  const result = await waiting;
  return { filename: result.suggestedFilename(), content: await readFile(await result.path(), 'utf8') };
}

test('all documentation routes and in-site links exist', async () => {
  const icon = await fetch(`${site}mark.svg`);
  assert.equal(icon.status, 200);
  assert.match(icon.headers.get('content-type'), /image\/svg\+xml/);
  const paths = ['', 'examples', 'playground', 'contributing', 'guide/getting-started', 'guide/language', 'guide/layout', 'guide/styling', 'guide/presentations', 'guide/models', 'guide/formats', 'guide/markdown', 'guide/tooling', 'reference/api', 'reference/dsl', 'diagrams/architecture', 'diagrams/decisions', 'diagrams/states', 'diagrams/sequences'];
  const links = new Set();
  for (const path of paths) {
    const response = await fetch(`${site}${path ? `${path}.html` : ''}`);
    assert.equal(response.status, 200, path);
    const html = await response.text();
    assert.match(html, /class="[^\"]*(VPHome|VPDoc)/, path);
    assert.ok(html.includes(`rel="icon" type="image/svg+xml" href="${base}mark.svg"`), path);
    for (const [, href] of html.matchAll(/href="([^"#]+)"/g)) {
      if (href.startsWith(base) && !href.includes('/assets/') && !href.includes('mark.svg')) links.add(new URL(href.replace(/&amp;/g, '&'), site).href.split('#')[0]);
    }
  }
  for (const link of links) assert.equal((await fetch(link)).status, 200, link);
});

test('server-rendered diagrams and client navigation retain fonts and styles', async () => {
  await page.goto(`${site}guide/getting-started.html`);
  await ready();
  await page.evaluate(() => document.fonts.ready);
  assert.equal(await page.locator('.vp-doc .layup-diagram svg.layup').count(), 2);
  assert.equal(await page.locator('.vp-doc .layup-diagram svg.layup').first().locator('style').count(), 1);
  assert.ok(await page.locator('.layup-diagram + div[class*="language-"]').count());
  await page.locator('.VPSidebar').getByRole('link', { name: 'Live playground', exact: true }).click();
  await ready();
  assert.ok(page.url().includes('/playground'));
  assert.match(await preview().textContent(), /Hello layup/);
});

test('the worker renders current source and ignores superseded edits', async () => {
  await openEditor();
  await edit('diagram "Changed live" layout=auto direction=right { node api "API"; node worker "Worker"; api -> worker "dispatch" }');
  assert.match(await preview().textContent(), /Changed live/);
  await source().fill('diagram "Obsolete" { node old }');
  await source().fill('diagram "Most recent" { node newest }');
  await ready();
  assert.equal(await preview().locator('.node[data-id="newest"]').count(), 1);
  assert.equal(await preview().locator('.node[data-id="old"]').count(), 0);
});

test('diagnostics preserve the last render and select exact Unicode source', async () => {
  await openEditor();
  const before = await preview().getAttribute('viewBox');
  await source().fill('diagram "错误" {\n node 客户端 "Client"\n 客户端 -> missng\n}');
  await page.waitForSelector('.live-editor .status-error');
  assert.equal(await preview().getAttribute('viewBox'), before);
  assert.equal(await editor().locator('.last-valid').count(), 1);
  assert.equal(await editor().getByRole('button', { name: 'Download SVG', exact: true }).isEnabled(), false);
  await editor().locator('.editor-diagnostics button').first().click();
  const selected = await source().evaluate(input => input.value.slice(input.selectionStart, input.selectionEnd));
  assert.equal(selected, 'missng');
  await source().fill('diagram "Recover" { node a tone=wrong; node b tone=wrong }');
  await page.waitForSelector('.live-editor .status-error');
  await edit('diagram "Recovered" { node a "Good" }');
  assert.equal(await editor().locator('.editor-diagnostics').count(), 0);
});

test('formatting preserves strings and comments, and Tab leaves the editor', async () => {
  await openEditor();
  await edit('diagram "Format"{node a "A" // keep this\nnode b "B";a->b "call"}');
  await editor().getByRole('button', { name: 'Format', exact: true }).click();
  await ready();
  const formatted = await source().inputValue();
  assert.match(formatted, /\n  node a "A" +\/\/ keep this/);
  assert.match(formatted, /a -> b "call"/);
  await source().focus();
  await page.keyboard.press('Tab');
  assert.equal(await source().evaluate(input => input === document.activeElement), false);
});

test('all canonical examples render; named views and presentation steps work', async () => {
  await openEditor();
  for (const id of ['architecture', 'decisions', 'states', 'composite', 'sequence', 'international', 'rtl', 'slides', 'models', 'presentation']) {
    await editor().getByLabel('Example', { exact: true }).selectOption(id);
    await ready();
    assert.ok(await preview().locator('.node').count(), id);
    assert.equal(await editor().locator('.editor-diagnostics').count(), 0, id);
  }
  assert.equal(await preview().getAttribute('viewBox'), '0 0 1920 1080');
  await editor().getByRole('button', { name: 'Present', exact: true }).click();
  assert.match(await editor().locator('.layup-step-status').textContent(), /1 \/ 4/);
  await editor().getByRole('button', { name: 'Next presentation step', exact: true }).click();
  assert.match(await editor().locator('.layup-step-status').textContent(), /2 \/ 4/);
  await editor().getByLabel('View', { exact: true }).selectOption('overview');
  await ready();
  assert.equal(await editor().locator('.layup-presentation').count(), 0);
  await editor().getByLabel('Example', { exact: true }).selectOption('models');
  await ready();
  await editor().getByLabel('View', { exact: true }).selectOption('detail');
  await ready();
  assert.equal(await preview().locator('.node[data-id="client"]').count(), 0);
  assert.equal(await preview().locator('.node[data-id="worker"]').count(), 1);
});

test('downloads use current source, portable themes, and selected scene metadata', async () => {
  await openEditor('models');
  await editor().getByLabel('View', { exact: true }).selectOption('detail');
  await ready();
  const original = await source().inputValue();
  assert.equal((await download('Download source')).content, original);
  const svg = await download('Download SVG');
  assert.match(svg.filename, /\.svg$/);
  assert.match(svg.content, /prefers-color-scheme/);
  await ready();
  const json = JSON.parse((await download('Download scene JSON')).content);
  assert.equal(json.version, 1);
  assert.equal(json.selectedView, 'detail');
  assert.ok(!json.nodes.some(node => node.id === 'client'));
  const html = await download('Download HTML');
  assert.match(html.content, /<!doctype html>/i);
  assert.match(html.content, /function createPresentation/);
  await ready();
  const embed = await download('Download embed');
  assert.match(embed.content, /postMessage/);
  await ready();
});

test('UTF-8 share links restore editable source without sending it to the server', async () => {
  await openEditor();
  const text = 'diagram "中文 / العربية" { node a "你好" }';
  await edit(text);
  await editor().getByRole('button', { name: 'Copy share link', exact: true }).click();
  const link = await page.evaluate(() => navigator.clipboard.readText());
  assert.ok(link.startsWith(`${site}playground.html#diagram=`), link);
  assert.equal(new URL(link).search, '');
  await page.goto(link);
  await ready();
  assert.equal(await source().inputValue(), text);
  assert.match(await preview().textContent(), /你好/);
});

test('user-supplied font bytes reach the worker and executable links are omitted', async () => {
  await openEditor();
  await edit('diagram "Fallback" { node a "中文中文" }');
  await editor().locator('.editor-fonts summary').click();
  await editor().getByLabel('Fallback font files').setInputFiles(fileURLToPath(new URL('../../../crates/layup/tests/fonts/Fallback.ttf', import.meta.url)));
  await ready();
  assert.match(await preview().locator('style').textContent(), /Layup User/);
  await editor().getByRole('button', { name: 'Clear fonts', exact: true }).click();
  await ready();
  const unsafe = 'diagram "Shared link" { node a "A" href="javascript:window.untrustedLink=true" }';
  await edit(unsafe);
  assert.equal(await preview().locator('a').getAttribute('href'), null);
  const svg = await download('Download SVG');
  assert.ok(!svg.content.includes('href="javascript:'));
  await ready();
});

test('theme, fullscreen, mobile layout, and local search work under the Pages base', async () => {
  await openEditor('presentation');
  const fill = () => preview().locator('.node .box').first().evaluate(box => getComputedStyle(box).fill);
  const light = await fill();
  await page.locator('.VPSwitchAppearance').first().click();
  assert.notEqual(await fill(), light);
  const fittedWidth = await preview().evaluate(svg => svg.getBoundingClientRect().width);
  await editor().getByRole('button', { name: 'Zoom preview in', exact: true }).click();
  const zoomedWidth = await preview().evaluate(svg => svg.getBoundingClientRect().width);
  assert.ok(zoomedWidth > fittedWidth * 1.4);
  await editor().getByRole('button', { name: 'Fit preview', exact: true }).click();
  await editor().getByRole('button', { name: 'Expand diagram', exact: true }).click();
  await page.waitForSelector('.layup-viewer[open]');
  await page.keyboard.press('Escape');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => document.querySelector('.VPSidebar').getBoundingClientRect().right <= 1);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1), false);
  const positions = await editor().locator('.editor-panels').evaluate(panel => {
    const a = panel.children[0].getBoundingClientRect();
    const b = panel.children[1].getBoundingClientRect();
    return { sourceBottom: a.bottom, previewTop: b.top };
  });
  assert.ok(positions.previewTop >= positions.sourceBottom);
  await mkdir(fileURLToPath(new URL('../../../out/docs', import.meta.url)), { recursive: true });
  await page.screenshot({ path: fileURLToPath(new URL('../../../out/docs/mobile.png', import.meta.url)), fullPage: true, animations: 'disabled' });
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.locator('.DocSearch-Button').click();
  await page.locator('.VPLocalSearchBox input').fill('lifelines');
  await page.waitForSelector('.VPLocalSearchBox .result');
  assert.match(await page.locator('.VPLocalSearchBox').textContent(), /Sequence/i);
  await page.keyboard.press('Escape');
  await page.screenshot({ path: fileURLToPath(new URL('../../../out/docs/playground.png', import.meta.url)), fullPage: true });
  await page.goto(site);
  await page.screenshot({ path: fileURLToPath(new URL('../../../out/docs/home.png', import.meta.url)), fullPage: true });
  assert.deepEqual(errors, []);
  assert.deepEqual(badAssets, []);
});
