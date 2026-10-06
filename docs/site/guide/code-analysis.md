# Generate diagrams from analysis

An analyzer extracts facts; Layup turns a selected graph into an explanation.
Structured graph input keeps symbol IDs, original code locations, and evidence
separate from layout geometry. It uses the same semantic, text, layout, routing,
and rendering pipeline as the DSL.

The structured-input APIs and Cargo adapter described here are available in the
current source checkout and will ship in the next release.

## Analyze a Rust workspace

The Node-only Cargo adapter runs `cargo metadata --format-version 1 --locked
--offline`. It does not build or execute the project. The workspace needs an
up-to-date lockfile and locally available dependencies. Use `--online` when
Cargo needs network access; the adapter still keeps `--locked`.

From a Layup checkout:

```sh
pnpm wasm
cargo build -p layup-cli
mkdir -p out/code-analysis
node packages/layup/cargo-cli.js -o out/code-analysis/workspace.json
target/debug/layup render out/code-analysis/workspace.json --view overview --html --theme auto
target/debug/layup compile out/code-analysis/workspace.json --view overview --strict
target/debug/layup render out/code-analysis/workspace.json \
  --view 'detail:cargo:package:crates/layup-cli/Cargo.toml' \
  --html --theme dark -o out/code-analysis/cli.html
```

The package also exposes the `layup-cargo` executable and a Node API:

```js
import { analyzeCargo } from '@hxyulin/layup/cargo';
import { load } from '@hxyulin/layup';

const model = await analyzeCargo({
  manifestPath: './Cargo.toml',
  sourceBaseUrl: 'https://github.com/owner/repo/blob/main/',
});
const engine = await load();
const { output, warnings } = engine.renderModel(model, {
  view: 'overview', format: 'html', theme: 'auto',
});
const detailViews = model.views.filter(view => view.id !== 'overview');
const scene = engine.compileModel(model, { view: detailViews[0].id });
```

Without `sourceBaseUrl`, node links use local file URLs. For a shared HTML
report, supply a source-root URL ending in `/`, ideally pinned to a repository
revision. Source URIs are relative to that root. Browsers may restrict opening
local file links from an HTTP page.

## What Cargo analysis means

The overview shows workspace packages and their resolved package dependencies.
Each detail view shows one package, its outgoing dependency neighbors, and a
separate group of its production targets with links to their source entry files.
Tests, examples, and benchmarks are excluded; build-script targets are included.

- Normal and build dependencies are included. `includeDev: true` or
  `--include-dev` adds development dependencies.
- External packages are omitted by default. `includeExternal: true` or
  `--include-external` includes direct external neighbors of workspace packages,
  not the entire transitive dependency graph.
- Renamed dependencies resolve through Cargo's opaque package IDs. Dependency
  kind, target condition, and alias remain in edge metadata.
- Optional dependencies appear when activated by the selected Cargo features.
  Use `features`, `allFeatures`, and `noDefaultFeatures`, or their CLI flags.
- Cargo's default resolution includes target-specific relationships across
  platforms. Each edge retains its target condition; it is not a claim that
  every edge is active on the machine viewing the diagram.

Cargo supplies manifest paths and target entry-file paths, not declaration
ranges. These locations omit `range`. Dependency edges link their evidence to
the depending package's manifest. This adapter does not infer Rust module
imports, function calls, execution order, or runtime behavior.

Package and target identities use workspace-relative paths. Dependency identity
includes endpoints, alias, dependency kind, and target condition. Sorting makes
output independent of Cargo's array order. Source links and selected features
remain part of the report's provenance.

## Structured graph contract

`compileModel` returns scene JSON; `renderModel` returns SVG, HTML, or iframe
embed HTML. Rust provides `layup::input::{Graph, parse, compile, compile_json}`.
The CLI detects graph input from `.json` files; for stdin, pass
`--input-format graph`. `--input-format dsl` overrides extension detection.

```json
{
  "version": 1,
  "title": "Request handling",
  "direction": "right",
  "nodes": [
    {
      "id": "api::submit(Request)",
      "title": "submit(request)",
      "kind": "function",
      "sourceLocations": [{ "uri": "src/api.rs", "symbol": "submit" }]
    },
    { "id": "worker::run", "title": "run(job)", "kind": "function" }
  ],
  "edges": [
    {
      "id": "submit-calls-run",
      "from": "api::submit(Request)",
      "to": "worker::run",
      "kind": "calls",
      "metadata": { "evidence": "resolved-call" }
    }
  ],
  "views": [
    { "id": "overview", "title": "Entry point", "include": ["api::submit(Request)"] },
    { "id": "detail", "title": "Dispatch", "include": ["api::submit(Request)", "worker::run"] }
  ],
  "provenance": { "analyzer": "example" }
}
```

| Object | Fields |
| --- | --- |
| Graph | Required `version: 1`, `title`, `nodes`; optional `description`, `direction` (default `down`), `edges`, `views`, `provenance` |
| Node | Required `id`, `title`; optional `kind` (default `node`), `parentId`, `code` and `description` string arrays, `role`, `href`, `tone`, `sourceLocations`, `metadata` |
| Edge | Required `id`, `from`, `to`; optional `kind` (default `flow`), `label`, `tone`, `dashed`, `sourceLocations`, `metadata` |
| View | Required `id`, `title`, nonempty `include`; optional `direction` override |
| Provenance | Required `analyzer`; optional analyzer `version` and JSON-object `metadata` |
| Source location | Required `uri`; optional `symbol` and `range` |

IDs are arbitrary nonempty strings and are preserved verbatim. All node, edge,
and view IDs are unique within their own category. Parents and endpoints must
exist; parent cycles and duplicate/unknown view includes are errors. Hierarchy
is bounded to 128 levels. Unknown contract fields and unsupported versions fail
validation. Application-specific fields belong in `metadata`, a JSON object.

Graph node kinds use the existing architecture/flowchart shapes. Custom kinds
such as `function` render as cards; custom relationship kinds such as `calls`
render as blue arrows. Kind names must be DSL identifiers; structural keywords
and sequence-only participant kinds are unavailable. Use `tone` to override a
node or edge color. Layout is automatic. This first input version covers graph
diagrams; authored rows, sequence events, state-machine modes, slides and reveal
plans remain DSL features.

Views follow the [shared-model selection rules](/guide/models): including a
container expands its subtree, including a descendant retains ancestors, and
edges survive when both endpoints survive. The first view is the default.
Validate every view to check its geometry. Graph filtering does not yet
aggregate nodes or query neighborhoods.

## Original locations and exported evidence

A `range` contains `startLine`, `startColumn`, `endLine`, and `endColumn`.
Lines and Unicode scalar columns are 1-based; the end is exclusive. An adapter
must convert its own coordinate convention explicitly. Omit ranges when they
are unknown. Several locations on an edge can preserve multiple call sites.

Scene nodes and edges expose `sourceLocations` and `metadata`; the root exposes
`provenance`. DSL inputs have empty locations/metadata and null provenance.
Structured inputs have null `line`/`span` because no DSL text was parsed.
Original code locations never replace a DSL span or a layout diagnostic's
meaning. Structured-input errors use `input/graph` for contract failures;
semantic failures retain engine diagnostic codes without fabricated spans.

SVG and both HTML formats include selected evidence in
`<metadata data-layup-analysis="1">`. Its text content is JSON with `version`,
`provenance`, and selected node/edge IDs, locations, and metadata. Read
`JSON.parse(svg.querySelector('[data-layup-analysis]').textContent)` when a host
needs that evidence. Excluded nodes and edges are omitted. A node's explicit
`href` controls navigation; locations alone do not create links.

The canonical [code-analysis input](https://github.com/hxyulin/layup/blob/main/examples/code-analysis.json)
is generated from this workspace. Regenerate it with `pnpm analysis:example`.
Large or dense graphs still need focused views; crossing minimization and
saved positions are not part of this milestone.
