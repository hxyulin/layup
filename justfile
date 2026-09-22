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
    cargo run -q -p layup-cli -- check examples/*.layup docs/diagrams/*.layup docs/checkpoints/01-auto-layout/*.layup --strict

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
