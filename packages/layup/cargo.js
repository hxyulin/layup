// Cargo is an analyzer adapter, separate from the browser/WASM engine.
import { execFile } from 'node:child_process';
import { resolve, relative, isAbsolute, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
import { promisify } from 'node:util';

const execute = promisify(execFile);
const compare = (a, b) => a < b ? -1 : a > b ? 1 : 0;
const productionTarget = target => !target.kind.some(kind => ['test', 'example', 'bench'].includes(kind));

/** Extract resolved dependencies without building or executing project code. */
export async function analyzeCargo({ manifestPath = 'Cargo.toml', offline = true, features = [], noDefaultFeatures = false, allFeatures = false, ...options } = {}) {
  const args = ['metadata', '--format-version', '1', '--locked', '--manifest-path', resolve(manifestPath)];
  if (offline) args.push('--offline');
  if (features.length) args.push('--features', features.join(','));
  if (noDefaultFeatures) args.push('--no-default-features');
  if (allFeatures) args.push('--all-features');
  const { stdout } = await execute('cargo', args, { maxBuffer: 64 * 1024 * 1024 });
  return cargoGraph(JSON.parse(stdout), options);
}

/** Adapt Cargo metadata v1, keeping opaque Cargo IDs only as lookup keys. */
export function cargoGraph(metadata, { includeExternal = false, includeDev = false, sourceBaseUrl } = {}) {
  if (metadata.version !== 1 || !metadata.resolve?.nodes || !metadata.workspace_members?.length) {
    throw new TypeError('expected Cargo metadata v1 with a resolved graph; omit --no-deps');
  }
  const packages = new Map(metadata.packages.map(pkg => [pkg.id, pkg]));
  const members = new Set(metadata.workspace_members);
  for (const id of members) if (!packages.has(id)) throw new TypeError(`missing workspace package ${id}`);
  const workspaceRoot = resolve(metadata.workspace_root);
  const rootUrl = sourceBaseUrl ? new URL(sourceBaseUrl) : pathToFileURL(workspaceRoot + sep);
  if (!rootUrl.pathname.endsWith('/')) throw new TypeError('sourceBaseUrl must end with /');
  const local = path => relative(workspaceRoot, resolve(path)).split(sep).join('/');
  const uri = path => local(path).split('/').map(encodeURIComponent).join('/');
  const inside = path => {
    const value = relative(workspaceRoot, resolve(path));
    return !isAbsolute(value) && value !== '..' && !value.startsWith('..' + sep);
  };
  const location = (path, symbol) => ({ uri: inside(path) ? uri(path) : pathToFileURL(path).href, symbol });
  const href = path => inside(path) ? new URL(uri(path), rootUrl).href : pathToFileURL(path).href;
  const idFor = pkg => members.has(pkg.id) || pkg.source == null
    ? `cargo:package:${local(pkg.manifest_path)}`
    : `cargo:package:${pkg.source}:${pkg.name}@${pkg.version}`;
  const resolved = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
  const selected = new Set(members);
  const relations = [];
  const workspacePackages = [...members].map(id => packages.get(id)).sort((a, b) => compare(idFor(a), idFor(b)));
  for (const pkg of workspacePackages) {
    const node = resolved.get(pkg.id);
    if (!node) throw new TypeError(`missing resolution for workspace package ${pkg.name}`);
    for (const dependency of node.deps) {
      const target = packages.get(dependency.pkg);
      if (!target) throw new TypeError(`missing dependency package ${dependency.pkg}`);
      if (!includeExternal && !members.has(target.id)) continue;
      for (const entry of dependency.dep_kinds) {
        const kind = entry.kind ?? 'normal';
        if (kind === 'dev' && !includeDev) continue;
        selected.add(target.id);
        relations.push({ pkg, target, alias: dependency.name, kind, platform: entry.target ?? null });
      }
    }
  }
  const nodes = [...selected].map(id => packages.get(id)).sort((a, b) => compare(idFor(a), idFor(b))).map(pkg => ({
    id: idFor(pkg), title: pkg.name, kind: 'module', code: [pkg.version], role: members.has(pkg.id) ? 'workspace package' : 'external package',
    href: members.has(pkg.id) ? href(pkg.manifest_path) : null,
    sourceLocations: members.has(pkg.id) ? [location(pkg.manifest_path, pkg.name)] : [],
    metadata: { packageName: pkg.name, packageVersion: pkg.version, workspace: members.has(pkg.id), cargoSource: pkg.source },
  }));
  const edges = relations.map(({ pkg, target, alias, kind, platform }) => ({
    id: `cargo:dependency:${JSON.stringify([idFor(pkg), idFor(target), alias, kind, platform])}`,
    from: idFor(pkg), to: idFor(target), kind: 'depends',
    label: [kind === 'normal' ? 'depends' : kind, platform].filter(Boolean).join(' · '),
    dashed: kind === 'dev',
    sourceLocations: [location(pkg.manifest_path, alias)],
    metadata: { dependencyName: alias, dependencyKind: kind, target: platform, evidence: 'cargo-resolve' },
  })).sort((a, b) => compare(a.id, b.id));
  const views = [{ id: 'overview', title: 'Workspace dependencies', include: nodes.map(node => node.id) }];
  for (const pkg of workspacePackages) {
    const packageId = idFor(pkg);
    const groupId = `cargo:targets:${local(pkg.manifest_path)}`;
    const targets = pkg.targets.filter(productionTarget).map(target => ({ ...target, kind: [...target.kind].sort(compare), crate_types: [...target.crate_types].sort(compare) })).sort((a, b) => compare(JSON.stringify([a.name, a.kind, local(a.src_path)]), JSON.stringify([b.name, b.kind, local(b.src_path)])));
    const include = new Set([packageId, ...edges.filter(edge => edge.from === packageId).map(edge => edge.to)]);
    if (targets.length) {
      nodes.push({ id: groupId, title: `${pkg.name} targets`, kind: 'group', sourceLocations: [location(pkg.manifest_path, pkg.name)] });
      include.add(groupId);
      edges.push({ id: `cargo:contains:${packageId}`, from: packageId, to: groupId, kind: 'contains', label: 'targets', tone: 'gray', sourceLocations: [location(pkg.manifest_path, pkg.name)] });
      for (const target of targets) {
        const targetId = `cargo:target:${JSON.stringify([local(pkg.manifest_path), target.name, target.kind, local(target.src_path)])}`;
        nodes.push({ id: targetId, title: target.name, kind: 'module', parentId: groupId, code: [local(target.src_path)], role: target.kind.join(', '), href: href(target.src_path), sourceLocations: [location(target.src_path, target.name)], metadata: { targetKinds: target.kind, crateTypes: target.crate_types } });
      }
    }
    views.push({ id: `detail:${packageId}`, title: `${pkg.name}: dependencies and targets`, include: [...include].sort(compare) });
  }
  return {
    version: 1, title: 'Cargo workspace', direction: 'right', nodes, edges, views,
    provenance: { analyzer: 'cargo-metadata', version: '1', metadata: { sourceRoot: rootUrl.href, includeExternal, includeDev, scope: 'workspace outgoing resolved package dependencies and production targets', targetFilter: 'all Cargo platforms', resolvedFeatures: Object.fromEntries(workspacePackages.map(pkg => [idFor(pkg), [...(resolved.get(pkg.id).features ?? [])].sort(compare)])) } },
  };
}
