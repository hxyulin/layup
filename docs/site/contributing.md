# Develop and publish these docs

The site uses VitePress and the repository's pnpm workspace. Markdown diagrams
compile at build time through the local Layup package. The live editor uses
that same package in a browser worker, with the WASM asset emitted under the
site's base path.

## Run locally

Requirements: Rust stable, the `wasm32-unknown-unknown` target, Node 22 or
newer, and the pnpm version in the root `package.json`.

```sh
rustup target add wasm32-unknown-unknown
pnpm install --frozen-lockfile
pnpm docs:dev
```

Open `http://localhost:5173/layup/`. `docs:dev` rebuilds the WASM engine first;
restart it after Rust changes. Markdown, theme, and editor changes hot reload.

```sh
pnpm docs:build
pnpm docs:preview
pnpm --filter @layup/docs exec playwright install chromium
pnpm docs:test
```

Start the preview after the build completes. Stop and restart a running
preview after rebuilding: VitePress indexes production asset filenames when
the preview starts, so a rebuild can leave new hashed bundles returning 404
until it restarts. Use `docs:dev` for editing with hot reload.

The build outputs `docs/site/.vitepress/dist/`. Build-time `layup` fences use
strict checks, so invalid diagrams and warnings fail the site build. Browser
tests check the production build under `/layup/`, including worker/WASM asset
URLs, live edits, diagnostics, view selection, presentation controls, sharing,
downloads, theme changes, mobile layout, and client-side navigation.

## Add a page or example

Write pages under `docs/site/`, then add them to the sidebar in
`.vitepress/config.mts`. Complete compilable diagrams use `layup` fences;
partial syntax uses `text`. Add `source` to show the diagram and its code.

`<Playground preset="decisions" />` embeds an editor. Presets import canonical
`.layup` files from `examples/`, keeping runtime demos aligned with fixtures.
New presets go in `.vitepress/theme/samples.js`.

The older Markdown guides remain available in `docs/` for repository readers.
When changing language semantics, update the relevant site guide, reference,
examples, and repository reference together.

## GitHub Pages

`.github/workflows/docs.yml` builds the docs on pull requests, then deploys on
pushes to `main` or manual dispatch. Deployment uses the GitHub Pages artifact
workflow; select **GitHub Actions** as the publishing source in the repository's
**Settings → Pages**.

The default base is `/layup/`, for `https://hxyulin.github.io/layup/`.
For another repository name or a custom domain, set `LAYUP_DOCS_BASE` at build
time and update the deployment configuration. Example for a root domain:

```sh
LAYUP_DOCS_BASE=/ pnpm docs:build
```

The workflow builds from the checked-out Rust sources. It does not depend on
a separately published npm engine, a server-side rendering service, or font
downloads. The WASM binary and generated site remain ignored build artifacts.

The configuration follows the official
[VitePress deployment guide](https://vitepress.dev/guide/deploy),
[SSR guidance](https://vitepress.dev/guide/ssr-compat), and
[pnpm workspace documentation](https://pnpm.io/workspaces).
