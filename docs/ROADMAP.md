# Next diagram capabilities

Direction and international text are the first foundation: automatic graph
flow supports down, up, right and left, independently of the base direction
of labels. Unicode line breaking handles CJK prose without spaces. CJK fonts
come from the viewer or from explicitly supplied font bytes; no CJK font is
bundled. Arabic and Hebrew use small bundled shaping faces.

## Decision nodes and flowcharts

Start with a `decision` kind and a diamond shape, plus rounded process and
terminal shapes. Keep the existing diagram language and typed/labeled edges.
A proposed example (not executable syntax yet):

```text
diagram "Validate request" layout=auto direction=right {
  node request "Request"
  decision valid "Valid?"
  terminal accepted "Accepted"
  terminal rejected "Rejected"
  request -> valid
  valid -> accepted "yes"
  valid -> rejected "no"
}
```

The main work is geometry, not adding an SVG polygon. Introduce a reusable
node outline interface for boundary ports, point containment, segment
intersection, and the safe text area. Use it in placement, routing, checks,
and rendering. Diamond labels must fit the inner text area; arrow endpoints
must touch the actual sloping boundary. Existing rectangular collision
bounds can remain a conservative broad-phase optimization.

Keep `from=`, `to=`, `via=`, explicit rows, and layer hints available. First
acceptance cases should cover yes/no branches in all four directions,
multiline CJK and RTL decision labels, nested decisions, back edges, and
branches whose labels compete for space.

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
