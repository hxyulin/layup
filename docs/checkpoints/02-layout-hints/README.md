# Checkpoint 2: progressive layout hints

Run `just preview-hints` and open `out/checkpoint-02/index.html`.
Each comparison uses automatic placement on both sides and changes only the
hint attributes. Expand the source below each diagram to inspect the change.

- **Below:** order stages without adding artificial graph edges.
- **Same layer:** bring an otherwise disconnected cache alongside a worker.
- **Beside:** order an adjacent chain without spelling out an explicit row.

Hints are strict placement constraints within a consecutive sibling region.
Invalid targets, self-references, cross-boundary references, contradictory
below constraints and branching/cyclic beside chains fail with a source line.
Hints require `layout=auto` and cannot reposition an explicit row's cells.

Same-layer constraints take priority over inferred edge ranks. They can pull
intermediate nodes in a directed path into the same rank. Explicit groups stay
in one physical row even beyond three columns, so text overflow checks remain
important. `below` means a later layer, not necessarily the next layer.

This checkpoint does not change routing or add a direction option. Step 3 is
better automatic port and outside-lane selection, after review of this preview.
