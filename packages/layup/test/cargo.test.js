import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import { analyzeCargo, cargoGraph } from '../cargo.js';
import { loadSync } from '../node.js';

test('real Cargo resolution handles aliases, optional features, dependency kinds, locations and deterministic views', async () => {
  const root = await mkdtemp(join(tmpdir(), 'layup-cargo-'));
  try {
    await writeFile(join(root, 'Cargo.toml'), '[workspace]\nresolver="3"\nmembers=["app", "core", "optional"]\n');
    for (const name of ['app', 'core', 'optional']) {
      await mkdir(join(root, name, 'src'), { recursive: true });
      let source = `[package]\nname="${name}"\nversion="0.1.0"\nedition="2024"\n`;
      if (name === 'app') source += `[dependencies]\nrenamed={package="core",path="../core"}\noptional={path="../optional",optional=true}\n[build-dependencies]\nrenamed={package="core",path="../core"}\n[dev-dependencies]\nrenamed={package="core",path="../core"}\n[features]\nextra=["dep:optional"]\n`;
      await writeFile(join(root, name, 'Cargo.toml'), source);
      await writeFile(join(root, name, 'src/lib.rs'), 'pub fn example() {}\n');
    }
    const manifestPath = join(root, 'Cargo.toml');
    execFileSync('cargo', ['generate-lockfile', '--offline', '--manifest-path', manifestPath], { stdio: 'pipe' });
    const lockBefore = await readFile(join(root, 'Cargo.lock'), 'utf8');
    const options = { manifestPath, sourceBaseUrl: 'https://example.test/repo/' };
    const graph = await analyzeCargo(options);
    const dependencies = graph.edges.filter(e => e.kind === 'depends');
    assert.deepEqual(dependencies.map(e => e.metadata.dependencyKind).sort(), ['build', 'normal']);
    assert.ok(dependencies.every(e => e.metadata.dependencyName === 'renamed'));
    assert.ok(dependencies.every(e => e.sourceLocations[0].uri === 'app/Cargo.toml'));
    assert.ok(dependencies.every(e => !('range' in e.sourceLocations[0])));
    assert.ok(!dependencies.some(e => e.to.includes('optional')));
    const all = await analyzeCargo({ ...options, includeDev: true, features: ['extra'] });
    assert.equal(all.edges.filter(e => e.kind === 'depends').length, 4);
    assert.ok(all.edges.some(e => e.metadata?.dependencyKind === 'dev' && e.dashed));
    assert.ok(all.edges.some(e => e.metadata?.dependencyName === 'optional'));
    const engine = loadSync();
    const overview = engine.compileModel(graph);
    assert.equal(overview.selectedView, 'overview');
    assert.equal(overview.nodes.length, 3);
    assert.ok(overview.nodes.every(n => !n.id.startsWith('cargo:target:')));
    const detail = engine.compileModel(graph, { view: 'detail:cargo:package:app/Cargo.toml' });
    assert.ok(detail.nodes.some(n => n.href === 'https://example.test/repo/app/src/lib.rs'));
    assert.ok(detail.nodes.some(n => n.parentId === 'cargo:targets:app/Cargo.toml'));
    for (const view of graph.views) assert.deepEqual(engine.compileModel(graph, { view: view.id }).diagnostics, []);
    assert.deepEqual(await analyzeCargo(options), graph);
    const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--offline', '--locked', '--format-version', '1', '--manifest-path', manifestPath], { encoding: 'utf8' }));
    const reordered = structuredClone(metadata);
    reordered.packages.reverse(); reordered.workspace_members.reverse(); reordered.resolve.nodes.reverse();
    for (const pkg of reordered.packages) pkg.targets.reverse();
    for (const node of reordered.resolve.nodes) { node.deps.reverse(); for (const dep of node.deps) dep.dep_kinds.reverse(); }
    assert.deepEqual(cargoGraph(reordered, { sourceBaseUrl: options.sourceBaseUrl }), graph);
    assert.throws(() => cargoGraph({ ...metadata, resolve: null }), /omit --no-deps/);
    assert.throws(() => cargoGraph(metadata, { sourceBaseUrl: 'https://example.test/repo' }), /end with/);
    assert.equal(await readFile(join(root, 'Cargo.lock'), 'utf8'), lockBefore);
    const cli = spawnSync(process.execPath, [new URL('../cargo-cli.js', import.meta.url).pathname, '--manifest-path', manifestPath, '--source-base-url', options.sourceBaseUrl], { encoding: 'utf8' });
    assert.equal(cli.status, 0, cli.stderr);
    assert.deepEqual(JSON.parse(cli.stdout), graph);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('external dependencies are explicit direct neighbors, and missing manifests fail', async () => {
  const manifestPath = new URL('../../../Cargo.toml', import.meta.url).pathname;
  const internal = await analyzeCargo({ manifestPath });
  const external = await analyzeCargo({ manifestPath, includeExternal: true });
  assert.ok(external.nodes.length > internal.nodes.length);
  assert.ok(external.nodes.some(n => n.metadata?.workspace === false));
  assert.ok(external.edges.filter(e => e.kind === 'depends').every(e => e.from.includes('crates/')));
  await assert.rejects(analyzeCargo({ manifestPath: join(tmpdir(), 'layup-missing-manifest/Cargo.toml') }));
});
