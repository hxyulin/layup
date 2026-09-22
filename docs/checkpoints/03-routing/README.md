# Checkpoint 3: automatic routing

Run `just preview-routing`, then open `out/checkpoint-03/index.html`.
The script builds checkpoint 2 (`d04da27`) in a temporary directory to provide
an actual before baseline. It leaves your checkout untouched and keeps build
outputs under ignored `out/`. Both sides render identical sources.

- A skip edge avoids an intermediate node without `via` or port hints.
- A return edge finds a clear route beside the content.
- Pinned right-side ports remain right-side ports, with an outward departure
  and inward arrival instead of following the boundary of an intervening box.

The existing inexpensive router remains first choice. When its result crosses
a node, overlaps an earlier edge, or has invalid port geometry, a bounded
orthogonal grid search tries allowed ports and obstacle-adjacent channels.
Distance plus bend cost selects a path for each candidate port pair; ties are
deterministic. Existing clear routes and explicit `via` paths remain unchanged.

Fallback paths stay within the existing canvas. The search includes reserved
text rectangles and avoids shared edge segments, but perpendicular edge
crossings are allowed. It is not a global crossing minimizer. Very dense scenes
may exceed the grid budget, and ancestor/descendant endpoints retain the
original routing rules. Existing diagnostics remain when no fallback succeeds.

Next checkpoint, after review: predictable changes as diagrams grow.
