<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/site/public/mark-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="docs/site/public/mark.svg">
    <img src="docs/site/public/mark.svg" alt="Layup logo" width="160" height="160">
  </picture>
</p>
<h1 align="center">Layup</h1>
<p align="center">Diagrams for explaining software, written as code.</p>
<p align="center">
  <a href="rust-toolchain.toml"><img alt="Rust stable" src="https://img.shields.io/badge/Rust-stable-dea584?logo=rust"></a>
  <a href="Cargo.toml"><img alt="Rust edition 2024" src="https://img.shields.io/badge/edition-2024-dea584"></a>
  <a href="package.json"><img alt="Node.js 22 or newer" src="https://img.shields.io/badge/Node.js-%E2%89%A522-5fa04e?logo=nodedotjs&logoColor=white"></a>
  <a href="package.json"><img alt="pnpm 12.9.1" src="https://img.shields.io/badge/pnpm-12.9.1-f69220?logo=pnpm&logoColor=white"></a>
  <a href="https://github.com/hxyulin/layup/actions/workflows/ci.yml"><img alt="CI status" src="https://github.com/hxyulin/layup/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  <a href="https://github.com/hxyulin/layup/actions/workflows/docs.yml"><img alt="Documentation status" src="https://github.com/hxyulin/layup/actions/workflows/docs.yml/badge.svg?branch=main"></a>
  <a href="#license"><img alt="MIT or Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue"></a>
</p>
<p align="center">
  <a href="https://hxyulin.github.io/layup/">Documentation</a> ·
  <a href="https://hxyulin.github.io/layup/playground.html">Live playground</a> ·
  <a href="https://hxyulin.github.io/layup/examples.html">Examples</a> ·
  <a href="CHANGELOG.md">Changelog</a>
</p>

Write a `.layup` file to describe your system, then render a self-contained
SVG, an interactive HTML page, or a JSON scene. Layup measures text with real
font metrics, arranges nodes, routes connections, and reports layout problems
with source locations. Use automatic placement or author the layout yourself.
The Rust engine also runs in JavaScript through WebAssembly.

<p align="center">
  <a href="https://hxyulin.github.io/layup/playground.html?example=slides">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="docs/showcase/pipeline-dark.svg">
      <source media="(prefers-color-scheme: light)" srcset="docs/showcase/pipeline-light.svg">
      <img src="docs/showcase/pipeline-light.svg" alt="A slide-sized pipeline: Client sends a Request to API, which sends a Query to Storage." width="800">
    </picture>
  </a>
</p>

[Edit this diagram live](https://hxyulin.github.io/layup/playground.html?example=slides)
· [View its source](examples/slides.layup)

| Explain | Layup supports |
| --- | --- |
| Software structure | Architecture cards, containers, typed edges, shared models and named views |
| Control flow | Decision trees, process nodes, labeled branches and merges |
| Runtime behavior | State machines, composite states, cycles and sequence diagrams |
| Presentations | Slide sizing, progressive reveal, pan/zoom and interactive selection |
| International text | Four graph directions, LTR/RTL text, Unicode and system/user CJK fonts |
| Your workflow | CLI, Rust and JavaScript APIs, Markdown/VitePress, linting and formatting |

<details>
<summary><strong>More examples: decisions and sequences</strong></summary>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/showcase/decisions-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="docs/showcase/decisions-light.svg">
  <img src="docs/showcase/decisions-light.svg" alt="A support-routing decision tree with labeled branches." width="800" loading="lazy">
</picture>

[Decision source](examples/decision-tree.layup)
· [Edit decisions](https://hxyulin.github.io/layup/playground.html?example=decisions)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/showcase/sequence-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="docs/showcase/sequence-light.svg">
  <img src="docs/showcase/sequence-light.svg" alt="A sequence diagram showing a request, retry loop and alternative responses." width="800" loading="lazy">
</picture>

[Sequence source](examples/sequence.layup)
· [Edit sequences](https://hxyulin.github.io/layup/playground.html?example=sequence)

</details>

## Quickstart

Layup 0.4 uses the named-diagram grammar shown below. Source files written
for 0.3 need migration; see the [language guide](https://hxyulin.github.io/layup/guide/language-v1.html).

Try the [playground](https://hxyulin.github.io/layup/playground.html) to edit
examples and download diagrams without installing anything. See the [changelog](CHANGELOG.md) for release history.

<details>
<summary><strong>Install the CLI and render your first diagram</strong></summary>

Install [Rust stable](https://rustup.rs/) and the CLI:

```sh
cargo install layup-cli --version 0.4.0 --locked
# Or install a supported prebuilt binary:
cargo binstall layup-cli --version 0.4.0
```

The installed command is named `layup`. See the
[release binaries and checksums](https://github.com/hxyulin/layup/releases/tag/v0.4.0)
for Linux, macOS and Windows.

Save this as `hello.layup`:

```layup
diagram main "A request" type=graph layout=auto flow-direction=right {
  node client "Client" style=process palette=blue
  node api "API" style=process palette=green
  node store "Storage" style=process palette=purple
  ::client -> ::api "Request"
  ::api -> ::store "Query"
}
```

```sh
layup render hello.layup --theme auto          # hello.svg
layup render hello.layup --html --theme auto   # hello.html, with pan/zoom
layup check hello.layup --strict               # fail on layout warnings
layup lint hello.layup                         # source and authoring diagnostics
layup fmt hello.layup --check                  # check source formatting
```

`--theme auto` follows the viewer's color scheme. Use `--theme light` or
`--theme dark` for a fixed palette. GitHub previews above use separate light
and dark SVGs so they follow the reader's theme too.

Continue with the [DSL walkthrough](https://hxyulin.github.io/layup/guide/language.html)
and [output formats](https://hxyulin.github.io/layup/guide/formats.html).

</details>

<details>
<summary><strong>Use the Rust/JavaScript APIs or run the docs locally</strong></summary>

Install the JavaScript/WASM package, including its compiled engine:

```sh
pnpm add @hxyulin/layup@0.4.0
# Or: npm install @hxyulin/layup@0.4.0
```

The [API guide](https://hxyulin.github.io/layup/reference/api.html) covers Rust,
Node.js and browser WASM. The
[Markdown guide](https://hxyulin.github.io/layup/guide/markdown.html) covers
build-time diagrams and interactive VitePress integration.

For the local playground, use Rust stable, Node 22+, and the pnpm version
pinned in [package.json](package.json):

```sh
git clone https://github.com/hxyulin/layup.git
cd layup
rustup target add wasm32-unknown-unknown
pnpm install --frozen-lockfile
pnpm docs:dev  # http://localhost:5173/layup/
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for repository setup and checks, and
[docs development](docs/site/contributing.md) for production previews and
GitHub Pages deployment.

</details>

## Contributing

Bug reports, examples, documentation and implementation changes are welcome.

- [CONTRIBUTING.md](CONTRIBUTING.md) covers setup, validation, visual review and pull requests.
- For layout issues, include the `.layup` source, expected result and a screenshot.
- The [architecture guide](docs/ARCHITECTURE.md) introduces the engine.
- [AGENTS.md](AGENTS.md) records repository conventions for coding assistants.

## License

Layup is available under either the [MIT license](LICENSE-MIT) or the
[Apache License 2.0](LICENSE-APACHE), at your option.

- Bundled fonts use [SIL OFL 1.1](crates/layup/fonts/OFL.txt); see the [font notes](crates/layup/fonts/README.md).
- CJK fonts come from the system or from the user.
