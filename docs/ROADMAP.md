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

## State machine diagrams

Build on the same graph and geometry primitives: a rounded `state` kind,
initial/final pseudostates, and a choice pseudostate backed by the diamond.
Transition labels can initially be ordinary strings with the convention
`event [guard] / action`. Rendering does not execute guards or actions.

The current engine already supports labeled transitions, self-loops, return
paths, custom edge kinds, and nested containers. Composite states can reuse
those containers. Keep transition semantics explicit if a machine-specific
mode later adds checks: unknown state references, invalid initial/final
transitions, unreachable states, and ambiguous initial states. Do not apply
these rules to ordinary architecture diagrams.

Cycle placement needs separate attention. Today a strongly connected
component shares one layer, which is predictable for dependency graphs but
can make a machine with many mutually reachable states very wide or tall.
A later machine layout policy could place the component internally while
preserving its position in the outer graph. Review examples before adding
an automatic policy or saved coordinates.

Deliver simple flat state machines first; then composite states and choice
pseudostates. History, parallel regions, entry/exit actions, fork/join, and
transition simulation can follow actual use cases.

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
