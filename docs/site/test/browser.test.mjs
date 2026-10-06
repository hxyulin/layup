import { objectSelector } from '../../../packages/layup/test/helpers.js';
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
async function follow(href) {
  await page.evaluate(href => {
    const link = document.createElement('a');
    link.href = href;
    document.querySelector('.vp-doc').append(link);
    link.click();
    link.remove();
  }, href);
}
async function exampleReady(id) {
  await page.waitForFunction(id => document.querySelector('select[aria-label="Example"]')?.value === id && document.querySelector('.live-editor .status-ready'), id);
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
  const paths = ['', 'examples', 'playground', 'contributing', 'guide/getting-started', 'guide/language', 'guide/language-v1', 'guide/layout', 'guide/styling', 'guide/presentations', 'guide/models', 'guide/formats', 'guide/markdown', 'guide/tooling', 'reference/api', 'reference/dsl', 'diagrams/architecture', 'diagrams/decisions', 'diagrams/states', 'diagrams/sequences'];
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
  await edit('diagram main "Changed live" type=graph layout=auto flow-direction=right {\n  node api "API"\n  node worker "Worker"\n  ::api -> ::worker "dispatch"\n}');
  assert.match(await preview().textContent(), /Changed live/);
  await source().fill('diagram main "Obsolete" type=graph {\n  node old\n}');
  await source().fill('diagram main "Most recent" type=graph {\n  node newest\n}');
  await ready();
  assert.equal(await preview().locator(objectSelector('newest')).count(), 1);
  assert.equal(await preview().locator(objectSelector('old')).count(), 0);
});

test('diagnostics preserve the last render and select exact Unicode source', async () => {
  await openEditor();
  const before = await preview().getAttribute('viewBox');
  await source().fill('diagram main "错误" type=graph {\n  node 客户端 "Client"\n  ::客户端 -> missng\n}');
  await page.waitForSelector('.live-editor .status-error');
  assert.equal(await preview().getAttribute('viewBox'), before);
  assert.equal(await editor().locator('.last-valid').count(), 1);
  assert.equal(await editor().getByRole('button', { name: 'Download SVG', exact: true }).isEnabled(), false);
  await editor().locator('.editor-diagnostics button').first().click();
  const selected = await source().evaluate(input => input.value.slice(input.selectionStart, input.selectionEnd));
  assert.equal(selected, 'missng');
  await source().fill('diagram main "Recover" type=graph {\n  node a palette=wrong\n  node b palette=wrong\n}');
  await page.waitForSelector('.live-editor .status-error');
  await edit('diagram main "Recovered" type=graph {\n  node a "Good"\n}');
  assert.equal(await editor().locator('.editor-diagnostics').count(), 0);
});

test('formatting preserves strings and comments, and Tab leaves the editor', async () => {
  await openEditor();
  await edit('diagram main "Format" type=graph {\n  node a "A" // keep this\n  node b "B"\n  a->b "call"\n}');
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
  for (const id of ['architecture', 'decisions', 'states', 'composite', 'sequence', 'international', 'rtl', 'slides', 'models', 'language-v1', 'paint', 'presentation']) {
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
  assert.equal(await preview().locator(objectSelector('client')).count(), 0);
  assert.equal(await preview().locator(objectSelector('processing', 'worker')).count(), 1);
});

test('editing source removes stale view selection without hiding source errors', async () => {
  await openEditor('models');
  const model = await source().inputValue();
  await edit(model.replace('view overview ', 'view renamed '));
  assert.equal(await editor().getByLabel('View', { exact: true }).inputValue(), '');
  assert.match(await editor().locator('.editor-notice').textContent(), /first available view/);
  assert.equal(JSON.parse((await download('Download scene JSON')).content).selectedView, 'renamed');
  await openEditor('models');
  await edit('diagram main "Replaced model" type=graph {\n  node replacement "Valid source"\n}');
  assert.equal(await preview().locator(objectSelector('replacement')).count(), 1);
  assert.equal(await editor().getByLabel('View', { exact: true }).isEnabled(), false);
  await editor().getByLabel('Example', { exact: true }).selectOption('models');
  await ready();
  await editor().getByLabel('View', { exact: true }).selectOption('detail');
  await ready();
  await source().fill(model.replace('::processing.worker -> ::processing.store "save"', '::processing.worker -> missing "save"'));
  await page.waitForSelector('.status-error');
  assert.equal(await editor().getByLabel('View', { exact: true }).inputValue(), 'detail');
  assert.match(await editor().locator('.editor-diagnostics').textContent(), /missing/);
});

test('query-only example navigation and browser history update a reused playground', async () => {
  await openEditor();
  for (const id of ['decisions', 'models', 'states']) {
    await follow(`${site}playground.html?example=${id}`);
    await exampleReady(id);
  }
  assert.equal(await preview().locator(objectSelector('running')).count(), 1);
  await page.goBack();
  await exampleReady('models');
  assert.equal(await editor().getByLabel('View', { exact: true }).inputValue(), 'overview');
  await page.goForward();
  await exampleReady('states');
});

test('same-page share links load source; choosing another example clears the share', async () => {
  await openEditor();
  const first = 'diagram main "Shared first" type=graph {\n  node one "你好"\n}';
  const second = 'diagram main "Shared second" type=graph {\n  node two "مرحبا"\n}';
  for (const text of [first, second]) {
    await follow(`${site}playground.html#${new URLSearchParams({ diagram: encodeShare(text) })}`);
    await page.waitForFunction(text => document.querySelector('.live-editor textarea')?.value === text && document.querySelector('.status-ready'), text);
  }
  await page.goBack();
  await page.waitForFunction(text => document.querySelector('.live-editor textarea')?.value === text && document.querySelector('.status-ready'), first);
  await follow(`${page.url().split('#')[0]}#heading`);
  await ready();
  // Leaving a shared document restores the example referenced by the URL.
  assert.equal(await editor().getByLabel('Example', { exact: true }).inputValue(), 'hello');
  await editor().getByLabel('Example', { exact: true }).selectOption('slides');
  await ready();
  assert.equal(new URL(page.url()).hash, '');
  assert.equal(new URL(page.url()).searchParams.get('example'), 'slides');
  await page.reload();
  await exampleReady('slides');
  await edit('diagram main "Unsaved edit" type=graph {\n  node edited\n}');
  await follow(`${page.url().split('#')[0]}#heading`);
  assert.equal(await source().inputValue(), 'diagram main "Unsaved edit" type=graph {\n  node edited\n}');
});

test('both playground panes resize with pointer and keyboard controls', async () => {
  await openEditor('slides');
  const widths = () => editor().locator('.editor-panels').evaluate(panel => ({ source: panel.querySelector('.source-panel').getBoundingClientRect().width, preview: panel.querySelector('.preview-panel').getBoundingClientRect().width }));
  const widthHandle = editor().getByRole('separator', { name: 'Resize source and diagram widths' });
  const initial = await widths();
  const box = await widthHandle.boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 - 130, box.y + box.height / 2, { steps: 8 });
  await page.mouse.up();
  const resized = await widths();
  assert.ok(resized.source < initial.source - 120);
  assert.ok(resized.preview > initial.preview + 120);
  await widthHandle.focus();
  await page.keyboard.press('Enter');
  await page.keyboard.press('ArrowRight');
  assert.equal(await widthHandle.getAttribute('aria-valuenow'), '55');
  for (const [key, value] of [['Home', '20'], ['End', '80']]) {
    await page.keyboard.press(key);
    assert.equal(await widthHandle.getAttribute('aria-valuenow'), value);
    assert.equal(await editor().locator('.source-panel').evaluate(panel => panel.scrollWidth > panel.clientWidth + 1), false);
    assert.equal(await editor().locator('.preview-panel').evaluate(panel => panel.scrollWidth > panel.clientWidth + 1), false);
  }
  await page.keyboard.press('Enter');
  const heightHandle = editor().getByRole('separator', { name: 'Resize source and diagram height' });
  const heights = () => editor().locator('.editor-panels').evaluate(panel => ({ source: panel.querySelector('.source-panel').getBoundingClientRect().height, preview: panel.querySelector('.preview-panel').getBoundingClientRect().height }));
  const initialHeight = await heights();
  const heightBox = await heightHandle.boundingBox();
  await page.mouse.move(heightBox.x + heightBox.width / 2, heightBox.y + heightBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(heightBox.x + heightBox.width / 2, heightBox.y + heightBox.height / 2 + 100, { steps: 8 });
  await page.mouse.up();
  const resizedHeight = await heights();
  assert.ok(resizedHeight.source >= initialHeight.source + 99);
  assert.equal(resizedHeight.source, resizedHeight.preview);
  await heightHandle.focus();
  await page.keyboard.press('ArrowDown');
  assert.ok((await heights()).source >= resizedHeight.source + 39);
  await editor().getByRole('button', { name: 'Reset pane sizes', exact: true }).click();
  assert.equal(await widthHandle.getAttribute('aria-valuenow'), '50');
  assert.deepEqual(await heights(), initialHeight);
  await page.setViewportSize({ width: 390, height: 844 });
  assert.equal(await widthHandle.isVisible(), false);
  await heightHandle.focus();
  await page.keyboard.press('ArrowDown');
  assert.equal((await heights()).source, (await heights()).preview);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false);
  await page.setViewportSize({ width: 1440, height: 1100 });
});

test('the slide pipeline uses the slide width and keeps its labels readable', async () => {
  await openEditor('slides');
  await page.evaluate(() => document.fonts.ready);
  const layout = await preview().evaluate(svg => {
    const canvas = svg.getBoundingClientRect();
    const boxes = [...svg.querySelectorAll('.node .box')].map(node => node.getBoundingClientRect());
    const texts = [...svg.querySelectorAll('.node text')].map(node => node.getBoundingClientRect());
    return { viewBox: svg.getAttribute('viewBox'), occupancy: (Math.max(...boxes.map(box => box.right)) - Math.min(...boxes.map(box => box.left))) / canvas.width, smallestTextHeight: Math.min(...texts.map(text => text.height)), contained: [...boxes, ...texts].every(box => box.left >= canvas.left && box.right <= canvas.right && box.top >= canvas.top && box.bottom <= canvas.bottom) };
  });
  assert.equal(layout.viewBox, '0 0 1920 1080');
  assert.ok(layout.occupancy > .7, JSON.stringify(layout));
  assert.ok(layout.smallestTextHeight >= 11, JSON.stringify(layout));
  assert.equal(layout.contained, true);
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
  assert.ok(!json.nodes.some(node => node.objectPath?.at(-1) === 'client'));
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
  const text = 'diagram main "中文 / العربية" type=graph {\n  node a "你好"\n}';
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
  await edit('diagram main "Fallback" type=graph {\n  node a "中文中文"\n}');
  await editor().locator('.editor-fonts summary').click();
  await editor().getByLabel('Fallback font files').setInputFiles(fileURLToPath(new URL('../../../crates/layup/tests/fonts/Fallback.ttf', import.meta.url)));
  await ready();
  assert.match(await preview().locator('style').textContent(), /Layup User/);
  await editor().getByRole('button', { name: 'Clear fonts', exact: true }).click();
  await ready();
  const unsafe = 'diagram main "Shared link" type=graph {\n  node a "A" href="javascript:window.untrustedLink=true"\n}';
  await edit(unsafe);
  assert.equal(await preview().locator('a').getAttribute('href'), null);
  const svg = await download('Download SVG');
  assert.ok(!svg.content.includes('href="javascript:'));
  await ready();
});

test('inline and live toolbar icons stay centered and sized in both themes', async () => {
  await page.goto(`${site}guide/getting-started.html`);
  await ready();
  const inline = page.locator('.vp-doc .layup-diagram > svg.layup').first().locator('..');
  const live = editor().locator('.layup-diagram');
  for (const dark of [false, true]) {
    if (await page.locator('html').evaluate(html => html.classList.contains('dark')) !== dark) {
      await page.locator('.VPSwitchAppearance').first().click();
    }
    const paths = [];
    for (const host of [inline, live]) {
      const button = host.getByRole('button', { name: 'Expand diagram', exact: true });
      await page.keyboard.press('Tab');
      await button.focus();
      const geometry = await button.evaluate(button => {
        const icon = button.querySelector('svg');
        const a = button.getBoundingClientRect();
        const b = icon.getBoundingClientRect();
        return { width: a.width, height: a.height, iconWidth: b.width, iconHeight: b.height,
          offsetX: b.x + b.width / 2 - a.x - a.width / 2,
          offsetY: b.y + b.height / 2 - a.y - a.height / 2,
          path: icon.querySelector('path').getAttribute('d') };
      });
      assert.equal(geometry.width, 32);
      assert.equal(geometry.height, 32);
      assert.equal(geometry.iconWidth, 16);
      assert.equal(geometry.iconHeight, 16);
      assert.ok(Math.abs(geometry.offsetX) < .5 && Math.abs(geometry.offsetY) < .5, JSON.stringify(geometry));
      // Wait for the hover/focus transition before checking keyboard activation.
      await page.waitForFunction(button => Number(getComputedStyle(button).opacity) > .99, await button.elementHandle());
      paths.push(geometry.path);
      await page.keyboard.press('Enter');
      await page.waitForSelector('.layup-viewer[open]');
      await page.keyboard.press('Escape');
    }
    assert.equal(paths[0], paths[1]);
    for (const label of ['Zoom preview in', 'Zoom preview out']) {
      const icon = editor().getByRole('button', { name: label, exact: true }).locator('svg');
      assert.deepEqual(await icon.evaluate(svg => { const r = svg.getBoundingClientRect(); return [r.width, r.height]; }), [16, 16]);
    }
    const logo = page.locator('.VPNavBarTitle img:visible');
    assert.ok((await logo.getAttribute('src')).endsWith(dark ? 'mark-dark.svg' : 'mark.svg'));
  }
});

test('touch users can see and activate expand without hovering', async () => {
  const touch = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, colorScheme: 'dark' });
  try {
    const phone = await touch.newPage();
    await phone.goto(`${site}guide/getting-started.html`);
    await phone.waitForSelector('.live-editor .status-ready');
    assert.equal(await phone.evaluate(() => matchMedia('(hover: none)').matches), true);
    for (const button of await phone.locator('.layup-expand').all()) {
      assert.equal(await button.evaluate(el => getComputedStyle(el).opacity), '1');
    }
    await phone.locator('.live-editor .layup-expand').tap();
    await phone.waitForSelector('.layup-viewer[open]');
    await phone.getByRole('button', { name: 'Close', exact: true }).tap();
    await phone.waitForSelector('.layup-viewer', { state: 'detached' });
  } finally {
    await touch.close();
  }
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
    const a = panel.querySelector('.source-panel').getBoundingClientRect();
    const b = panel.querySelector('.preview-panel').getBoundingClientRect();
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
