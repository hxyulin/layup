# Build, check and render. `just` with no arguments lists recipes.

default:
    @just --list

# Debug build of layup.
build:
    cargo build

# Format, lint, test, then check every example and documentation diagram with warnings as errors.
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo run -q -p layup-cli -- check examples/*.layup docs/diagrams/*.layup docs/checkpoints/*/*.layup examples/vitepress/*.md --strict

# Render the examples to SVG (next to the sources) and interactive HTML (in out/).
examples:
    mkdir -p out
    for f in examples/*.layup; do \
        b=$(basename "$f" .layup); \
        cargo run -q -p layup-cli -- render "$f"; \
        cargo run -q -p layup-cli -- render "$f" --html --theme auto -o "out/$b.html"; \
        cargo run -q -p layup-cli -- render "$f" --html --embed --theme auto -o "out/$b.embed.html"; \
    done

# Serve out/ and examples/ for trying the interactive pages and the iframe host.
serve: examples
    cp examples/embed-host.html out/
    python3 -m http.server 8765 --directory out

# Rasterize an SVG with Quick Look for eyeballing (macOS). Usage: just png examples/job-pipeline.svg
png svg:
    qlmanage -t -s 2000 -o out "{{svg}}"

# Regenerate the architecture diagram embedded in README.md.
docs:
    cargo run -q -p layup-cli -- render docs/diagrams/architecture.layup --strict

# Before/after gallery for v0.2 automatic placement (open the printed HTML path).
preview-auto:
    cargo build -p layup-cli
    python3 tools/preview-auto-layout.py

# Before/after gallery for v0.2 layout hints.
preview-hints:
    cargo build -p layup-cli
    python3 tools/preview-layout-hints.py

# Compare the checkpoint-2 router with the current one on identical diagrams.
preview-routing:
    python3 tools/preview-routing.py

# Compare common edits under the checkpoint-3 and current layout policies.
preview-incremental:
    python3 tools/preview-incremental.py

# Build the WebAssembly engine into the npm package.
wasm:
    node tools/build-wasm.mjs

# Test the npm package against the CLI.
js-test: wasm
    cargo build -p layup-cli
    pnpm install --frozen-lockfile
    pnpm --filter @hxyulin/layup test

# Browser checks and a multilingual preview using system CJK fallback and a synthetic user-font fixture.
international-test:
    cargo build -p layup-cli
    cargo run -q -p layup --example verify-international
    node tools/verify-international.mjs

# Review decision trees in all directions and check glyph/branch-label containment.
decision-test:
    cargo build -p layup-cli
    node tools/verify-decisions.mjs

# Review state-machine cycles, choices and multilingual text in Chromium.
state-test:
    cargo build -p layup-cli
    node tools/verify-states.mjs

# Run the VitePress example with the local package (http://localhost:5173).
vitepress: wasm
    pnpm install --frozen-lockfile
    pnpm --filter @layup/vitepress-example dev

# Build the VitePress example and test it in Chromium (downloads Chromium once).
vitepress-test: wasm
    pnpm install --frozen-lockfile
    pnpm --filter @layup/vitepress-example exec playwright install chromium
    pnpm --filter @layup/vitepress-example build
    pnpm --filter @layup/vitepress-example test

# Run the public documentation and live editor at http://localhost:5173/layup/.
docs-dev:
    pnpm install --frozen-lockfile
    pnpm docs:dev

# Build the documentation with strictly checked diagrams and the current WASM engine.
docs-build:
    pnpm install --frozen-lockfile
    pnpm docs:build

# Check the production documentation in Chromium under its GitHub Pages base path.
docs-test:
    pnpm install --frozen-lockfile
    pnpm --filter @layup/docs exec playwright install chromium
    pnpm docs:test

# Exercise presentation controls in standalone, inline, fullscreen and iframe views.
presentation-test:
    cargo build -p layup-cli
    node tools/verify-presentation.mjs

# Review slide transforms and font-size checks in Chromium.
slide-test:
    cargo build -p layup-cli
    node tools/verify-slides.mjs

# Review event-ordered sequence diagrams and multilingual message labels.
sequence-test:
    cargo build -p layup-cli
    node tools/verify-sequences.mjs
