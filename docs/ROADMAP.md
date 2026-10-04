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

Font-aware sizing and measured subtree packing are implemented. Unpinned
`clean` automatic canvases grow beyond the previous 1400px heuristic when
needed, and compact nodes use variable content widths. Explicit widths and
manual presets retain authored sizing. Per-region tree detection in mixed
diagrams and additional compact shapes remain possible follow-ups.

## State machine diagrams: composite states implemented

`mode=state-machine` enables automatic machine placement and opt-in
validation. `state`, `initial`, `final`, and `choice` shapes render rounded
states, dots, bullseyes, and small or labeled diamonds. Transition labels
use ordinary strings with the convention `event [guard] / action`; state
entry/exit annotations are optional prose lines. Nothing executes guards
or actions.

Machines validate scoped initial markers, transition direction and
endpoints, final markers, and choice branch counts. Unreachable states
warn with source lines. Final markers are optional for continuously
running machines. Ordinary architecture diagrams keep their semantics.

Cycle placement uses a deterministic traversal per scope starting at its
initial marker. Placement omits back edges and self-loops while rendering retains
all transitions, with space for return paths and self-loop captions.
This avoids collapsing a mutually reachable set into one rank. Source
node order controls traversal; edge statement reordering preserves
positions. Explicit rows, placement hints, ports and manual layout remain
available. Regression/browser checks cover cycles and self-loops in all
four directions, choices, validation, RTL/CJK, and integration behavior.
Run `just state-test` for the review gallery.

Nested `state` blocks now introduce composite scopes with their own initial
marker, optional final markers, a separate header, and padded child channels.
Regular transitions can enter composites or descendants and leave nested
states. Initial transitions stay in their scope. IDs remain globally unique.
Reachability includes default composite initial paths and direct descendant
entry. History, parallel regions, fork/join, and structured event syntax
should follow concrete use cases.

## Presentation and sequence diagrams: implemented

`mode=sequence` adds authored participant columns, lifelines, synchronous and
asynchronous messages, returns, self-calls, notes and nested loop/opt/alt
fragments. Time advances downward; left/right controls column order. Text
shares measured wrapping, RTL shaping and supplied-font behavior with other
diagrams. See [the sequence reference](SEQUENCE.md).

Slide viewports uniformly fit the completed scene and warn when scaled text
falls below an authored readability threshold. Progressive reveal supports
cumulative node/edge visibility, per-step highlights and speaker notes in
standalone, iframe, inline and fullscreen viewers. Reusable models define
nodes/edges once and select named views with inherited attributes and local
presentation steps. See [the presentation guide](PRESENTATION.md) and the
[combined request/retry model](../examples/presentation-model.layup).

## Next stages

Language tooling is implemented: source spans, recovering syntax diagnostics,
CLI/JavaScript linting, and formatting that preserves comments and strings.
See [the language-tools guide](LANGUAGE-TOOLS.md). Multiple semantic errors,
editor completion/LSP support and incremental parsing are possible follow-ups.

1. Automatic sizing/packing and composite states: implemented and covered by
   geometry, semantic, CLI/WASM parity, and browser checks.
2. JSON scene export: implemented as schema version 1 in Rust, CLI and JS/WASM.
   PNG/PDF can follow as optional SVG-based output frontends.
3. Interactive reveal and playback controls: implemented. Active-state sets,
   transition traces and animated traversal can use exported semantic IDs.
   Simulation needs an explicit execution contract and stays a separate layer.
4. Architecture presets and swimlanes can extend presentation workflows;
   ER/interface diagrams should follow concrete software-documentation needs.

## Output and interactive views

Retain SVG as the shared rendering format. SVG and the HTML viewer will gain
new node kinds together, as will the WebAssembly/VitePress integration.

The version-1 scene export lets other tools inspect layout, build alternate
renderers and use semantic IDs without parsing SVG. It includes original
coordinates, outlines, drawing operations, text runs, hierarchy, source
spans, fonts, views, slide transforms, sequence events and presentation plans.
Font identifiers remain separate from SVG's embedded font payloads. Consumers
must check the schema version; incompatible contracts need a version change.

PNG/PDF export can consume the SVG in an optional rendering frontend, with
explicit scale/page sizing and the same system/supplied-font policy as SVG.
Check CJK/RTL output and transparent versus themed backgrounds visually.
Interactive active-state highlighting and a transition trace overlay should
use the same semantic node/edge IDs. A simulator should be a separate,
explicitly configured layer rather than implied by a transition label.
