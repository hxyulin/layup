# Checkpoint 4: predictable incremental layout

Run `just preview-incremental`, then open `out/checkpoint-04/index.html`.
The preview builds checkpoint 3 (`ee8f8ad`) in a temporary directory and
compares exactly the same edit under both layout policies. Each section has
four diagrams: old before/after, then new before/after.

1. Insert an unrelated node between existing declarations. The connected
   service keeps its geometry, routes and colors; the new node appears below.
2. Add a connected peer. Its layer reflows as expected, while existing colors
   remain stable.
3. Append an independent subsystem. It occupies its own region instead of
   changing the earlier service's layer widths.

Connected regions follow the order of their earliest node. Unconnected peers
form a final region. Edge kinds without a direction still establish region
membership; layout hints do too. Explicit boundaries remain authoritative.

Automatic-layout colors now use a fixed hash of node IDs. This changes the
initial palette relative to checkpoint 3, but prevents insertion-driven
recoloring thereafter. Colors are not guaranteed unique; explicit tones win.
Manual diagrams keep their original palette cycle.

There is no saved-position cache. Width changes, wrapping, topology changes,
and growth above a region may legitimately move nodes. This checkpoint tests
specific useful invariants rather than promising a frozen drawing.

All four planned v0.2 checkpoints are implemented. Review this final preview
before deciding on release preparation. Graph direction remains future work.
