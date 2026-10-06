// Review generated views in the standalone viewer, with preserved analysis data.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from '../examples/vitepress/node_modules/playwright/index.mjs';
import { loadSync } from '../packages/layup/node.js';

const graph = JSON.parse(readFileSync('examples/code-analysis.json', 'utf8'));
const engine = loadSync();
const directory = resolve('out/code-analysis');
mkdirSync(directory, { recursive: true });
const browser = await chromium.launch();
const cases = [];
try {
  for (const [index, view] of graph.views.entries()) {
    const scene = engine.compileModel(graph, { view: view.id });
    assert.deepEqual(scene.diagnostics, [], view.title);
    for (const theme of ['light', 'dark']) {
      const name = `${index === 0 ? 'overview' : `detail-${index}`}-${theme}`;
      const { output, warnings } = engine.renderModel(graph, { view: view.id, format: 'html', theme });
      assert.deepEqual(warnings, [], view.title);
      const path = `${directory}/${name}.html`;
      writeFileSync(path, output);
      for (const [size, viewport] of [['desktop', { width: 1440, height: 1000 }], ['narrow', { width: 390, height: 844 }]]) {
        const page = await browser.newPage({ viewport });
        const errors = [];
        page.on('pageerror', error => errors.push(error.message));
        await page.goto(pathToFileURL(path).href);
        await page.evaluate(() => document.fonts.ready);
        const evidence = await page.locator('svg.layup [data-layup-analysis]').evaluate(node => JSON.parse(node.textContent));
        assert.deepEqual(evidence.nodes.map(n => n.id).sort(), scene.nodes.map(n => n.id).sort());
        assert.deepEqual(evidence.edges.map(e => e.id).sort(), scene.edges.map(e => e.id).sort());
        assert.equal(evidence.provenance.analyzer, 'cargo-metadata');
        const links = await page.locator('svg.layup a').evaluateAll(nodes => nodes.map(node => node.getAttribute('href')));
        assert.deepEqual(links.sort(), scene.nodes.filter(n => n.href).map(n => n.href).sort());
        await page.keyboard.press('f');
        assert.deepEqual(errors, [], `${name}-${size}`);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
        await page.screenshot({ path: `${directory}/${name}-${size}.png` });
        await page.close();
        cases.push(`${name}-${size}`);
      }
    }
  }
} finally {
  await browser.close();
}
console.log(`${cases.length} analysis view/theme/viewport checks passed. Review: ${directory}`);
