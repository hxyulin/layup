# Layup documentation

The public documentation lives under [`site/`](site/index.md) and is built
with VitePress. It contains the DSL walkthrough, diagram guides, examples
gallery, API/format references and browser-WASM playground.

From the repository root:

```sh
pnpm install --frozen-lockfile
pnpm docs:dev
```

Open `http://localhost:5173/layup/`. The command builds the current Rust/WASM
engine first; install `wasm32-unknown-unknown` with rustup if needed. The
[site development guide](site/contributing.md) covers builds, browser checks,
GitHub Pages setup and content maintenance.

Repository-focused Markdown references remain available:

- [Language and design reference](DESIGN.md)
- [Architecture](ARCHITECTURE.md)
- [Language tools](LANGUAGE-TOOLS.md)
- [Sequences](SEQUENCE.md)
- [Presentation and scene contract](PRESENTATION.md)
- [Roadmap](ROADMAP.md)
