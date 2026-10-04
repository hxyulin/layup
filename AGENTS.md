# Repository guide for coding assistants

Layup describes software diagrams in a DSL and renders SVG, interactive HTML
and scene JSON. Use [CONTRIBUTING.md](CONTRIBUTING.md) for setup and validation;
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) explains the engine.

## Sources and generated files

- `crates/layup/`: language, semantic model, fonts, layout, routing and rendering.
- `crates/layup-cli/`: CLI; `crates/layup-wasm/`: WASM bridge.
- `packages/layup/`: JavaScript adapters, Markdown integration, client and types.
- `docs/site/`: VitePress docs and live playground; `examples/`: canonical DSL.
- `crates/layup/src/presentation.js` is canonical. Run `pnpm wasm` to synchronize
  `packages/layup/presentation.js`; do not edit only the generated copy.
- Do not commit `target/`, `out/`, WASM binaries, caches or generated site output.
  `docs/showcase/*.svg` are intentionally tracked; regenerate with `pnpm showcase`.

## Conventions

- Follow the existing Rust and JavaScript patterns. Rust uses stable and edition
  2024; no numerical MSRV is declared. Node requires 22+, with pnpm pinned in
  `package.json`. Use the pnpm workspace and keep its lockfile synchronized.
- Treat graph direction and text direction as separate concepts. Preserve
  Unicode, RTL and CJK behavior. CJK fonts come from the system or user; do not
  add a bundled CJK font.
- Keep theme selection available for light, dark and auto exports. The VitePress
  theme follows `.dark`; README pictures select separate fixed-theme SVGs.
- Scope diagram geometry/styles to `svg.layup`. Use explicitly sized SVG icons
  for controls so browser fonts and broad SVG rules cannot distort them.
- When DSL or API behavior changes, update canonical examples, site guides,
  reference pages and TypeScript declarations as appropriate. Keep the README
  focused on a showcase, quickstart and links to the documentation.

## Verification

Run checks appropriate to the change before finishing:

- Rust: `just check` (format, clippy, tests and strict examples).
- WASM/adapters: `just js-test`.
- Markdown/client: `just vitepress-test`.
- Docs/playground: `just docs-test`, plus visual checks of affected examples
  in light/dark mode and at relevant viewport sizes.
- Workflows/templates/contributor docs: `just workflows-check` and `pnpm repo:check`.
- Renderer or showcase sources: `pnpm showcase` then `pnpm showcase:check`.

Use a focused regression test for behavior changes. Visual review tools are
listed in `justfile`; their output belongs in ignored `out/`.

Build production docs before starting `pnpm docs:preview`. Stop and restart a
preview after rebuilding, since VitePress caches the generated asset filenames.
Use `pnpm docs:dev` for hot reload; restart it after Rust/WASM changes.

Report the behavior changed, relevant validation and remaining limitations.
Keep the user's current commit/push instructions in view; do not infer package
publication or deployment authorization from a local implementation task.
