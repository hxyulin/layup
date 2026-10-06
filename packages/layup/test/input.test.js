import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { loadSync, LayupError } from '../node.js';

const engine = loadSync();
const cli = fileURLToPath(new URL('../../../target/debug/layup', import.meta.url));
const model = {
  version: 1, title: 'Analysis', direction: 'right',
  nodes: [
    { id: 'pkg::API<T>', title: 'API', kind: 'group', sourceLocations: [{ uri: 'src/api.rs' }] },
    { id: 'API::run(&self)', title: 'run', kind: 'function', parentId: 'pkg::API<T>', sourceLocations: [{ uri: 'src/api.rs', range: { startLine: 4, startColumn: 1, endLine: 5, endColumn: 1 }, symbol: 'run' }] },
    { id: 'save', title: 'save', metadata: { confidence: 0.8 } },
  ],
  edges: [{ id: 'call@4', from: 'API::run(&self)', to: 'save', kind: 'calls', sourceLocations: [{ uri: 'src/api.rs' }], metadata: { evidence: 'resolved' } }],
  views: [{ id: 'overview', title: 'Overview', include: ['save'] }, { id: 'detail', title: 'Detail', include: ['API::run(&self)', 'save'] }],
  provenance: { analyzer: 'test', metadata: { sourceRoot: 'https://example.test/repo/' } },
};

test('graph input has CLI/WASM parity, preserves locations and emits selected SVG/HTML metadata', () => {
  const before = JSON.stringify(model);
  for (const view of ['overview', 'detail']) {
    const result = spawnSync(cli, ['compile', '-', '--input-format', 'graph', '--view', view, '--strict'], { input: before, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    const scene = engine.compileModel(model, { view });
    assert.deepEqual(scene, JSON.parse(result.stdout));
    assert.equal(scene.nodes[0].span, null);
    assert.equal(scene.nodes[0].line, null);
    assert.equal(scene.provenance.analyzer, 'test');
    for (const format of ['svg', 'html', 'embed']) {
      const { output, warnings } = engine.renderModel(model, { view, format, theme: 'dark' });
      const args = ['render', '-', '--input-format', 'graph', '--view', view, '--theme', 'dark', '--strict', '-o', '-'];
      if (format !== 'svg') args.push('--html');
      if (format === 'embed') args.push('--embed');
      const rendered = spawnSync(cli, args, { input: before, encoding: 'utf8' });
      assert.equal(rendered.status, 0, rendered.stderr);
      assert.equal(rendered.stdout, output);
      assert.deepEqual(warnings, []);
      assert.match(output, /data-layup-analysis="1"/);
      assert.equal(output.includes('call@4'), view === 'detail');
    }
  }
  const detail = engine.compileModel(model, { view: 'detail' });
  const run = detail.nodes.find(n => n.id === 'API::run(&self)');
  assert.equal(run.parentId, 'pkg::API<T>');
  assert.deepEqual(run.sourceLocations[0].range, model.nodes[1].sourceLocations[0].range);
  assert.equal(detail.edges[0].metadata.evidence, 'resolved');
  assert.equal(JSON.stringify(model), before);
});

test('structured input rejects malformed contracts before filtering views', () => {
  for (const change of [
    g => { g.version = 2; },
    g => { g.nodes[1].parentId = 'missing'; },
    g => { g.edges[0].to = 'missing'; },
    g => { g.nodes[1].sourceLocations[0].range.endLine = 1; },
    g => { g.nodes[1].unexpected = true; },
    g => { g.nodes[1].tone = 'invalid'; },
  ]) {
    const graph = structuredClone(model);
    change(graph);
    assert.throws(() => engine.compileModel(graph), e => e instanceof LayupError && e.code === 'input/graph' && e.span === null && e.line === null);
  }
  assert.throws(() => engine.compileModel(model, { view: 'missing' }), e => e.code === 'view/unknown' && e.span === null);
  assert.throws(() => engine.compileModel('bad'), TypeError);
  assert.throws(() => engine.renderModel(model, { view: 'detail\noperation=format' }), TypeError);
});

test('canonical code-analysis input is strict-clean in every generated view and detected by CLI extension', () => {
  const path = new URL('../../../examples/code-analysis.json', import.meta.url);
  const graph = JSON.parse(readFileSync(path, 'utf8'));
  for (const view of graph.views) {
    const result = spawnSync(cli, ['compile', path.pathname, '--view', view.id, '--strict'], { encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(engine.compileModel(graph, { view: view.id }), JSON.parse(result.stdout));
  }
});
