# Checkpoint 1: automatic placement

Run `just preview-auto`, then open `out/checkpoint-01/index.html`.
The generated files stay local; this directory and the preview script make
all three comparisons reproducible.

- **Branching:** identical nodes and edges, manual flow versus inferred rows.
- **Nested:** the same comparison inside an authored service container.
- **Add a branch:** automatic placement before and after adding an Audit peer.
  Three inferred columns trigger the existing 1400px canvas-width default.

Manual previews intentionally expose crossing warnings that automatic
placement resolves; the gallery displays the diagnostics. The automatic
examples have no warnings. Sources are expandable below each diagram.

The feature is opt-in (`layout=auto`), top-to-bottom, and deterministic for
the same source. It respects authored boundaries, groups directed cycles,
and handles disconnected nodes. It does not promise identical positions
across edits or improved routing for arbitrary dense graphs.

Review this checkpoint before continuing:

1. Automatic placement — this checkpoint.
2. Progressive hints such as below, beside, and same layer.
3. Better automatic port and outside-lane selection.
4. Predictable incremental behavior and regression examples; saved positions
   remain optional future work unless review establishes a need.

Future feature: a graph-direction enum and `direction=down|right|up|left`.
The existing edge `Side` enum only selects ports and routing sides.
