# Contributing to Layup

Layup is a Rust diagram engine and CLI, with a JavaScript/WASM package and
VitePress documentation. Start with the [live docs](https://hxyulin.github.io/layup/)
and [architecture guide](docs/ARCHITECTURE.md). Small fixes, examples and clear
bug reports are useful contributions.

## Set up

- Rust stable, with `rustfmt` and `clippy` (selected by `rust-toolchain.toml`).
- Node.js 22 or newer; CI uses Node 24.
- pnpm at the version pinned in `package.json`. Use your preferred pnpm
  installation method; `pnpm --version` should match the pin.
- [just](https://github.com/casey/just) for the repository recipes.
- [actionlint](https://github.com/rhysd/actionlint) for workflow edits; CI pins 1.7.12.

```sh
git clone https://github.com/hxyulin/layup.git
cd layup
cargo build
rustup target add wasm32-unknown-unknown
pnpm install --frozen-lockfile
pnpm docs:dev
```

The site opens at `http://localhost:5173/layup/`. Rust-only changes do not
require Node or the WASM target. `just` lists the available recipes.

## Find the right source

| Location | Purpose |
| --- | --- |
| `crates/layup/` | Lexer/parser, model resolution, text measurement, layout, routing and rendering |
| `crates/layup-cli/` | `layup` CLI |
| `crates/layup-wasm/` | WASM bridge to the same Rust engine |
| `packages/layup/` | Node/browser adapters, types, Markdown integration and client interactions |
| `docs/site/` | Public VitePress docs, playground and production browser tests |
| `examples/` | Canonical DSL fixtures and demos |
| `tools/` | WASM build, visual reviews and repository maintenance |

`crates/layup/src/presentation.js` is the presentation controller's source.
`pnpm wasm` copies it to the npm package while building the engine. Keep the
copy synchronized through that build. The WASM binary, rendered site, caches
and local review artifacts are ignored; do not add them to commits.

## Validate a change

Choose checks that exercise the behavior you changed:

```sh
just check           # Rust formatting, clippy, tests and strict diagram checks
just js-test         # rebuild WASM; compare JavaScript behavior with the CLI
just vitepress-test  # Markdown integration and interactive client in Chromium
just docs-test       # production docs, live editor and Pages asset paths
just workflows-check # actionlint for GitHub Actions
pnpm repo:check      # contributor links, image assets and issue forms
pnpm showcase:check  # README light/dark previews match canonical examples
```

`just check` runs recovering parser, layout and rendering tests as well as
strict example checks. Add a focused regression test when fixing behavior;
avoid tests that only restate the implementation.

For layout or text changes, also run the relevant visual review recipe:
`international-test`, `decision-test`, `state-test`, `sequence-test`,
`slide-test` or `presentation-test`. These generate local artifacts under
`out/`. Inspect both themes and the directions affected by the change.
International text uses bundled Latin/Arabic/Hebrew fonts and system or
user-provided CJK fonts. Do not bundle a CJK font to make a fixture pass.

For browser changes, check desktop and narrow layouts, keyboard access and
light/dark modes. Use SVG toolbar icons with explicit dimensions, rather
than font symbols whose appearance varies by platform. Check inline and
fullscreen diagrams when changing shared client styles.

## Update examples and documentation

Add complete, strict-clean examples to `examples/`. Document language changes
in the site guide and reference, and keep relevant repository references in
`docs/` aligned. Use `layup` fences for complete diagrams and `text` for partial
syntax. Playground presets import the canonical examples.

The README is a showcase and entry point. Put detailed tutorials and API
material on the docs site. Its diagram previews are generated in both themes:

```sh
pnpm showcase        # regenerate docs/showcase/*.svg
pnpm showcase:check  # verify without changing tracked files
```

Include regenerated previews when their sources or renderer change. The README
uses `<picture>` to select a fixed light/dark SVG for the reader's color scheme;
ordinary exports can use `--theme auto`. The website uses its own theme toggle.

See [docs development](docs/site/contributing.md) for adding pages, testing
production output, restarting previews after a build and publishing Pages.

## Issues and pull requests

Use the issue forms for bugs, feature requests and documentation problems.
For rendering issues, include the smallest reproducing `.layup` source,
command or playground link, expected result, actual result, and a screenshot.
Mention direction, theme and fonts when relevant.

Describe the problem and resulting behavior in your pull request. List the
checks you ran and attach before/after images for visible changes. Keep changes
focused, and explain tradeoffs that a reviewer needs to assess. The PR template
provides a starting point; remove sections that do not apply.

## Releases (maintainers)

Keep the Rust crate versions, internal dependency versions and npm package
version aligned. Update `CHANGELOG.md` and run the applicable Rust, JavaScript,
documentation and repository checks. Build the WASM package before inspecting
`pnpm --filter @hxyulin/layup pack`; inspect Cargo packages with `cargo package`
from each crate directory. Publish the engine before the CLI that depends on
it. Publishing packages is a separate maintainer action from merging a PR.

Run `cargo publish -p layup --dry-run` before publishing the engine. After it
is available in the registry, dry-run and publish `layup-cli`. Verify a fresh
`cargo install layup-cli --version VERSION --locked` using a temporary root.

Commit the release notes and documentation, then push an annotated `vVERSION`
tag. `.github/workflows/release.yml` builds and exercises native binaries on
five platforms, packages them with `tools/package-release.mjs`, and publishes
a GitHub release with SHA-256 checksums. Tag and package versions must match.
Manual dispatch builds artifacts without creating a release when run on a
branch. The CLI's `package.metadata.binstall` must match the archive URL and
binary path. Verify `cargo binstall layup-cli --version VERSION` with
`--strategies crate-meta-data --install-path out/release/binstall-test` so the
check cannot silently fall back to a source build. Linux release builds use
Ubuntu 22.04 for a glibc 2.35 baseline.

## Licensing

Contributions use the repository's dual MIT/Apache-2.0 licensing. Retain third
party notices and the bundled fonts' SIL OFL license. See
[LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE) and the
[font notes](crates/layup/fonts/README.md).
