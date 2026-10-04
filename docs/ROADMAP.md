# Next diagram capabilities

Direction and international text are the first foundation: automatic graph
flow supports down, up, right and left, independently of the base direction
of labels. Unicode line breaking handles CJK prose without spaces. CJK fonts
come from the viewer or from explicitly supplied font bytes; no CJK font is
bundled. Arabic and Hebrew use small bundled shaping faces.

## Decision nodes and flowcharts: implemented

`decision` diamonds, rounded `process` steps, and `terminal` outcomes now
work with the existing language and labeled edges. `shape=diamond` aliases
`shape=decision` for custom kinds. Decision trees use subtree lanes in all
four directions; graphs with merges, cycles, hints, or mixed kinds retain
the general graph layout. Authored rows and sections stay available.

Shared outline geometry supplies boundary ports, point containment,
segment/rectangle intersection, and the safe text area. Diamond labels fit
the central inner rectangle; arrow endpoints meet sloping sides. Bounding
rectangles remain conservative routing obstacles. Branch labels stay
upright and avoid fills, including the endpoint nodes.

Examples and regression/browser checks cover nested branches in all four
directions, multiline CJK and RTL labels, pinned ports, hints, containers,
merges, self-loops, return paths, and long competing captions. Run
`just decision-test` for the review gallery.

Potential follow-ups are per-region tree detection in mixed diagrams,
additional compact shapes, variable node widths, and more economical
packing for trees with many leaves. Current tree lanes rely on an authored
canvas width; wide trees can require `width=` or explicit grouping.

## State machine diagrams: flat machines implemented

`mode=state-machine` enables automatic machine placement and opt-in
validation. `state`, `initial`, `final`, and `choice` shapes render rounded
states, dots, bullseyes, and small or labeled diamonds. Transition labels
use ordinary strings with the convention `event [guard] / action`; state
entry/exit annotations are optional prose lines. Nothing executes guards
or actions.

Flat machines validate the initial marker, transition direction and
endpoints, final markers, and choice branch counts. Unreachable states
warn with source lines. Final markers are optional for continuously
running machines. Ordinary architecture diagrams keep their semantics.

Cycle placement uses a deterministic traversal starting at the initial
marker. Placement omits back edges and self-loops while rendering retains
all transitions, with space for return paths and self-loop captions.
This avoids collapsing a mutually reachable set into one rank. Source
node order controls traversal; edge statement reordering preserves
positions. Explicit rows, placement hints, ports and manual layout remain
available. Regression/browser checks cover cycles and self-loops in all
four directions, choices, validation, RTL/CJK, and integration behavior.
Run `just state-test` for the review gallery.

Next consider composite-state scopes and nested initial/final markers.
Structural groups can already organize a flat machine, but `state` blocks
cannot yet contain child states. History, parallel regions, fork/join,
transition simulation, structured events/guards/actions, and active-state
traces should follow concrete use cases.

## Output and interactive views

Retain SVG as the shared rendering format. SVG and the HTML viewer will gain
new node kinds together, as will the WebAssembly/VitePress integration.

A serializable scene/geometry export is the next useful format: it would let
other tools inspect layout, build alternate renderers, and implement state
highlighting without parsing SVG. Define its coordinate, outline, text-run,
and font contracts before calling it a stable API.

PNG/PDF export can consume the SVG in an optional rendering frontend.
Interactive active-state highlighting and a transition trace overlay should
use the same semantic node/edge IDs. A simulator should be a separate,
explicitly configured layer rather than implied by a transition label.
