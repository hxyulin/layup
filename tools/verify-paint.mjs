// Check theme-aware paint in the browser, including media and host theme changes.
import { chromium } from '../examples/vitepress/node_modules/playwright/index.mjs';
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { loadSync } from '../packages/layup/node.js';

const source = readFileSync('examples/paint.layup', 'utf8');
const engine = loadSync();
const render = (text, theme) => execFileSync(resolve('target/debug/layup'),
  ['render', '-', '-o', '-', '--strict', '--theme', theme], { input: text, encoding: 'utf8' });
const expected = {
  light: { fill: 'rgb(239, 246, 255)', stroke: 'rgb(29, 78, 216)', text: 'rgb(30, 58, 138)', background: 'rgb(255, 255, 255)' },
  dark: { fill: 'rgb(23, 37, 84)', stroke: 'rgb(147, 197, 253)', text: 'rgb(191, 219, 254)', background: 'rgb(16, 24, 39)' },
};
const document = svg => '<!doctype html><meta charset="utf-8"><style>body{margin:24px;background:#ddd}svg.layup{display:block;max-width:100%;height:auto;margin-bottom:24px}</style>' + svg;
const values = scope => scope.evaluate(svg => {
  const node = id => [...svg.querySelectorAll('.node')].find(node => JSON.parse(node.dataset.id.slice(7)).at(-1) === id);
  const api = node('api');
  const edge = svg.querySelector('.edge path.ln');
  const marker = document.getElementById(edge.getAttribute('marker-end').slice(5, -1)).querySelector('path');
  const legend = svg.querySelector('.legend-paint .ln');
  const legendMarker = document.getElementById(getComputedStyle(legend).markerEnd.match(/#([^"')]+)/)[1]).querySelector('path');
  return {
    legendFill: getComputedStyle(svg.querySelector('.legend-paint .box')).fill,
    legendStroke: getComputedStyle(legend).stroke,
    legendDash: getComputedStyle(legend).strokeDasharray,
    legendMarkerFill: getComputedStyle(legendMarker).fill,
    legendSampleCount: svg.querySelectorAll('.legend-paint .ln').length,
    fill: getComputedStyle(api.querySelector('.box')).fill,
    stroke: getComputedStyle(api.querySelector('.box')).stroke,
    text: getComputedStyle(api.querySelector('text')).fill,
    background: getComputedStyle(svg.querySelector('.bg')).fill,
    edgeStroke: getComputedStyle(edge).stroke,
    markerFill: getComputedStyle(marker).fill,
    edgeDash: getComputedStyle(edge).strokeDasharray,
    edgeWidth: getComputedStyle(edge).strokeWidth,
    storageFill: getComputedStyle(node('store').querySelector('.box')).fill,
    storageRadius: node('store').querySelector('.box').getAttribute('rx'),
  };
});
const check = async (svg, theme) => {
  const actual = await values(svg);
  const colors = expected[theme];
  assert.deepEqual(Object.fromEntries(Object.keys(colors).map(key => [key, actual[key]])), colors);
  assert.equal(actual.legendFill, colors.fill);
  assert.equal(actual.legendStroke, colors.stroke);
  assert.equal(actual.legendMarkerFill, colors.stroke);
  assert.equal(actual.legendDash, '1px, 4px');
  assert.equal(actual.legendSampleCount, 1);
  assert.equal(actual.edgeStroke, colors.stroke);
  assert.equal(actual.markerFill, colors.stroke);
  assert.equal(actual.edgeDash, '1px, 4px');
  assert.equal(actual.edgeWidth, '2px');
  assert.equal(actual.storageFill, 'none');
  assert.equal(Number(actual.storageRadius), 0);
};

mkdirSync('out/paint', { recursive: true });
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const gallery = document(render(source, 'light') + render(source, 'dark'));
  writeFileSync('out/paint/index.html', gallery);
  await page.setContent(gallery);
  await page.evaluate(() => document.fonts.ready);
  await check(page.locator('svg.layup').nth(0), 'light');
  await check(page.locator('svg.layup').nth(1), 'dark');
  await page.locator('svg.layup').nth(0).screenshot({ path: 'out/paint/light.png' });
  await page.locator('svg.layup').nth(1).screenshot({ path: 'out/paint/dark.png' });
  await page.setViewportSize({ width: 390, height: 844 });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false);
  await page.screenshot({ path: 'out/paint/narrow.png', fullPage: true });

  await page.setContent(document(render(source, 'auto')));
  for (const theme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme: theme });
    await check(page.locator('svg.layup'), theme);
  }
  await page.setContent(document(engine.render(source, { theme: 'auto', darkSelector: '.dark' }).output));
  for (const theme of ['light', 'dark']) {
    await page.evaluate(dark => document.documentElement.classList.toggle('dark', dark), theme === 'dark');
    await check(page.locator('svg.layup'), theme);
  }
  // Two diagrams with the same paint indexes must retain independent CSS scopes.
  const other = source.replaceAll('#eff6ff', '#ff0000').replaceAll('#172554', '#00ff00');
  await page.setContent(document(render(source, 'light') + render(other, 'light')));
  await check(page.locator('svg.layup').nth(0), 'light');
  assert.equal((await values(page.locator('svg.layup').nth(1))).fill, 'rgb(255, 0, 0)');
  // Quoted style names and names resembling escaped punctuation stay distinct,
  // including the CSS variables and marker IDs of connection legend samples.
  const collisionSource = `diagram main type=graph {
    node-style "a b" fill-color=red legend-label="Red node"
    node-style a_20_b fill-color=blue legend-label="Blue node"
    edge-style "a b" stroke-color=red legend-label="Red edge"
    edge-style a_20_b stroke-color=blue legend-label="Blue edge"
    node a style="a b"
    node b style=a_20_b
    a -> b style="a b"
    b -> a style=a_20_b
    legend visibility=visible nodes=["a b", a_20_b] edges=["a b", a_20_b]
  }`;
  for (const theme of ['light', 'dark']) {
    await page.setContent(document(render(collisionSource, theme)));
    const samples = await page.locator('.legend-paint').evaluateAll(groups => groups.map(group => {
      const line = group.querySelector('.ln');
      const shape = group.querySelector('.box');
      const markerId = line && getComputedStyle(line).markerEnd.match(/#([^"')]+)/)[1];
      return {
        label: group.textContent.trim(),
        color: getComputedStyle(line ?? shape)[line ? 'stroke' : 'fill'],
        markerId,
        markerColor: markerId && getComputedStyle(document.getElementById(markerId).querySelector('path')).fill,
      };
    }));
    assert.equal(samples.length, 4);
    for (const sample of samples) {
      const color = sample.label.startsWith('Red') ? 'rgb(255, 0, 0)' : 'rgb(0, 0, 255)';
      assert.equal(sample.color, color, sample.label);
      if (sample.markerId) assert.equal(sample.markerColor, color, sample.label);
    }
    const markerIds = samples.filter(sample => sample.markerId).map(sample => sample.markerId);
    assert.equal(new Set(markerIds).size, 2);
    await page.screenshot({ path: `out/paint/style-names-${theme}.png`, fullPage: true });
  }
  assert.deepEqual(errors, []);
  console.log('Verified fixed/automatic/host paint themes, marker and legend colors, literal channels, stroke styles, CSS isolation and narrow layout.');
} finally {
  await browser.close();
}
