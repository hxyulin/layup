// Review opaque scoped IDs and annotations through the standalone viewer.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from '../examples/vitepress/node_modules/playwright/index.mjs';
import { loadSync } from '../packages/layup/node.js';

const source = readFileSync('examples/language-v1.layup', 'utf8');
const engine = loadSync();
const scene = engine.compile(source);
const directory = resolve('out/language-v1');
mkdirSync(directory, { recursive: true });
const browser = await chromium.launch();
let cases = 0;
try {
  for (const theme of ['light', 'dark']) {
    const { output, warnings } = engine.render(source, { theme, format: 'html' });
    assert.deepEqual(warnings, []);
    const path = `${directory}/${theme}.html`;
    writeFileSync(path, output);
    for (const [size, viewport] of [['desktop', { width: 1440, height: 1000 }], ['narrow', { width: 390, height: 844 }]]) {
      const page = await browser.newPage({ viewport });
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.goto(pathToFileURL(path).href);
      await page.evaluate(() => document.fonts.ready);
      const evidence = await page.locator('svg.layup [data-layup-document]').evaluate(node => JSON.parse(node.textContent));
      assert.equal(evidence.diagramId, 'services');
      assert.deepEqual(Object.keys(evidence.objects).sort(), scene.nodes.map(n => n.id).sort());
      const id = scene.nodes.find(n => n.objectPath.at(-1) === 'API::run(&self)').id;
      // Exercise CSS escaping in the viewer's actual host-selection handler.
      await page.evaluate(id => window.postMessage({ layup: 'focus', id, zoom: true }, '*'), id);
      await page.waitForFunction(() => document.querySelector('svg.layup').classList.contains('focus'));
      assert.equal(await page.locator('.node.hl').count() > 0, true);
      await page.keyboard.press('Escape');
      await page.keyboard.press('f');
      assert.deepEqual(errors, []);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      await page.screenshot({ path: `${directory}/${theme}-${size}.png` });
      await page.close();
      cases += 1;
    }
  }
} finally {
  await browser.close();
}
console.log(`${cases} language/theme/viewport checks passed. Review: ${directory}`);
