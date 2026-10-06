# DSL reference

Layup source uses named diagrams, shared values and annotations, and a body
grammar selected by `type`. The optional `layup 1` line asserts the source
revision. This checkout replaces the previous title-only syntax; these changes
will ship after the published 0.3.0 packages.

See the [language walkthrough](/guide/language-v1) for scopes, exact values,
extension annotations, recovery and inspection. Statements end with a newline
or semicolon. IDs are required; a following quoted string supplies a display
label. Quote names containing punctuation, or literal `true`, `false` and `null`.

## Documents and diagram options

```text
[layup 1]
diagram ID ["Display label"] type=TYPE [PROPERTY=VALUE ...] {
  ...
}
```

A document can contain several diagrams. IDs are unique across the document.
Types currently rendered are `graph`, `sequence`, and `state-machine`.
Unavailable types retain their envelope and opaque body and produce a warning.
Selecting a supported diagram validates all supported sibling definitions.

| Property | Values and meaning |
| --- | --- |
| `type` | `graph`, `sequence`, `state-machine`, or a qualified extension type |
| `layout` | `auto` or `manual`; sequences always preserve event order |
| `flow-direction` | `down`, `up`, `right`, `left`; graph/state rank direction |
| `participant-direction` | `right`, `left`; sequence columns, with time downward |
| `text-direction` | `auto`, `ltr`, `rtl`; independent of graph direction |
| `width` | Positive scene width; separate from slide fitting |
| `subtitle` | Quoted visible header text |
| `background-color` | Canvas paint; see paint values below |

Graph flow defaults to authored block layout. State machines default to automatic
layout. A directional graph layout requires `layout=auto`. Named views can
override width, layout, text direction and the appropriate flow/participant
direction. Theme selection remains a CLI/API export option.

## Graph objects and layout

```text
node ID ["Label"] [style=STYLE] [PROPERTY=VALUE ...] [{ ... }]
group ID ["Label"] [PROPERTY=VALUE ...] { ... }
package ID ["Label"] [PROPERTY=VALUE ...] { ... }
crate ID ["Label"] [PROPERTY=VALUE ...] { ... }
edge ID SOURCE -> TARGET ["Caption"] [PROPERTY=VALUE ...]
SOURCE -> TARGET ["Caption"] [PROPERTY=VALUE ...]
```

Named containers introduce object scopes. References use `container.member`,
quoted path segments, or `::container.member` for an absolute path. A quoted
literal `"container.member"` remains one ID. Forward references work; duplicate
names and ancestor shadowing fail. Edges have a separate diagram-wide ID
namespace. Anonymous edges cannot be named in presentation directives.

| Construct | Meaning |
| --- | --- |
| `row weights=[1, 2] gap=16 { ... }` | Side-by-side cells with positive relative weights |
| `section "Label" { ... }` | Section heading and rule |
| `band "Label" { ... }` | Band heading |
| `divider ["Label"] palette=gray` | Divider with optional caption |
| `gap size=24` | Nonnegative vertical spacer |
| `gap` inside a row | Empty cell; an explicit size is rejected |
| `text "Prose"` | Wrapped prose, inside objects or as structural content |
| `code "Literal code"` | Literal code text inside an object |
| `tag "Context"` | At most one explicit context tag per object |

Layout blocks do not introduce scopes. Node layout hints are `after=REFERENCE`,
`same-rank=REFERENCE`, and `beside=REFERENCE`. They constrain automatic layout;
physical side names keep their meaning in every text/flow direction.

## Styles, defaults and paint

```text
node-style NAME [base=STYLE] [PROPERTY=VALUE ...]
edge-style NAME [base=STYLE] [PROPERTY=VALUE ...]
defaults CATEGORY PROPERTY=VALUE ...
```

Styles belong directly to a diagram. Forward bases work; cycles, duplicate
definitions and invalid unused definitions fail. A custom style is selected
with `style=NAME`, rather than a declaration keyword. Defaults categories are
`node`/`edge`, `participant`/`message`, or `state`/`transition`, according to the
body grammar. There can be one defaults declaration per category. Default
values and style references are validated even when no declaration uses them.

Properties resolve through built-in defaults, diagram defaults, declared base
styles, declared overrides and instance overrides. A palette change retains
inherited explicit paint; `fill-color=auto` clears an inherited fill override.
For connections, `palette=auto` resets an inherited palette and derives it from
the target node, as does `palette=target`. Container paint does not propagate
to contained objects.

Built-in node templates include `node`, `card`, `interface`, `container`,
`package`, `crate`, `process`, `decision`, and `terminal`. Software-oriented
templates `trait`, `type` and `module` remain usable through `style=`/`base=`.
State marker templates preserve state semantics. A graph cannot acquire an
initial/final/choice state through styling.

| Property | Meaning |
| --- | --- |
| `palette` | `auto`, `gray`, `blue`, `green`, `yellow`, `purple`, `orange`, `red`; connection palettes also accept `source` and `target` |
| `fill-color` | Node interior |
| `stroke-color` | Outline/connection paint, including arrowheads |
| `text-color` | Text owned by the object or connection |
| `stroke-style` | `solid`, `dashed`, `dotted` |
| `stroke-width` | Nonnegative thickness in scene units |
| `font-family` | `mono`, `sans` |
| `text-align` | Logical `start`, `end`, or physical `left`, `right`, `center` |
| `text-direction` | `auto`, `ltr`, `rtl`; node overrides inherit through object scopes |
| `shape` | Graph style geometry: `rectangle`, `rounded-rectangle`, `diamond`, `capsule` |
| `role` | Visible role label |
| `href` | Object link |
| `legend-label` | Style's legend description |
| Edge-style `default-label` | Caption requested by `label=style` |

Colors accept a recognized named color, quoted hex literal, approved theme token
(`background`, `text`, `muted`, `code`, `frame`), or `{light: COLOR, dark: COLOR}`.
Literal colors remain literal in both themes. `auto` restores the derived
channel, and `none` removes fill/stroke paint; text does not accept `none`.
Gradients, paint-server URLs and arbitrary CSS functions are rejected.

## Connections

`->`, `<-`, `<->`, and `--` mean directed, reverse directed, bidirectional and
undirected connections. Use `style=uses` instead of a typed arrow lexeme.
Built-in connection styles include `default`, `flow`, `impl`, `extends`,
`uses` and `exports`.

| Property | Meaning |
| --- | --- |
| `style` | Selected edge style |
| `source-side`, `target-side` | `top`, `right`, `bottom`, `left` on semantic endpoints |
| `route-side` | Preferred exterior side |
| `bus=true` / `bus=false` | Shared routing channel |
| `label=style` | Request the selected style's default caption; exclusive with a positional caption or transition `event`/`guard`/`action` |

Palette inheritance changes paint independently of caption/legend labels.

## Sequence bodies

```text
participant ID ["Label"] [PROPERTY=VALUE ...]
actor ID ["Label"] [PROPERTY=VALUE ...]
message ID SOURCE -> TARGET ["Caption"] [type=call|reply] [delivery=sync|async]
note "Text" [over=PARTICIPANT | between=[PARTICIPANT, PARTICIPANT]]
loop "Label" { ... }
optional "Label" { ... }
alternatives "Label" {
  branch "Condition" { ... }
  branch "Other condition" { ... }
}
```

Participants belong directly to the diagram and have no body. Events preserve
authored order. Fragments do not introduce participant or message scopes.
Alternatives require at least two branches; other fragments require an event.
A reply has no delivery property. Notes accept either one `over` target or two
`between` targets. Graph routing hints and custom header geometry are rejected
for participants/messages.

## State-machine bodies

```text
state ID ["Label"] [PROPERTY=VALUE ...] [{ ... }]
initial ID [PROPERTY=VALUE ...]
final ID [PROPERTY=VALUE ...]
choice ID ["Question"] [PROPERTY=VALUE ...]
transition ID SOURCE -> TARGET ["Caption"] [PROPERTY=VALUE ...]
```

Composite states introduce scopes. `initial` and `final` markers take no label
or text body. State styles preserve their declared state/initial/final/choice
semantics. Each state-machine scope has one initial marker with one outgoing
transition. Choices require at least two outgoing transitions. Reachability
diagnostics remain warnings.

A transition can supply quoted `event`, `guard` and `action` display text
instead of a positional caption. These strings are descriptive, not executed.
`entry "Code"` and `exit "Code"` provide typed action text in a state body.
Authored rows and sections remain available without creating semantic scopes.

## Views and presentation steps

```text
view ID ["Label"] [PROPERTY=VALUE ...] {
  include REFERENCE ...
  step ID ["Label"] {
    show objects=[REFERENCE, ...] connections=[CONNECTION_ID, ...]
    highlight objects=[REFERENCE, ...] connections=[CONNECTION_ID, ...]
    speaker-note "Presenter text"
  }
}
```

Views belong to a diagram; the first authored view is the default. Including
a container selects its subtree. Including descendants retains their ancestors.
Connections remain when both endpoints are selected. All views validate before
output selection. View-local steps replace shared diagram steps.

Show directives accumulate. Highlights apply only to the current step and
require visible targets. Connections explicitly assigned to a reveal step wait
for that step; other connections reveal when both endpoints become visible.
Members cannot be revealed independently.

## Slide and legend configuration

```text
slide size=wide padding=32 min-font-size=14
slide size={width: 1920, height: 1080} padding=32 min-font-size=14
legend visibility=auto position=bottom nodes=[service] edges=[calls]
```

Slide size is `wide`, `standard`, or positive finite `{width, height}` dimensions.
Padding is nonnegative and must leave usable space. `min-font-size` is positive.
Fitting preserves original scene geometry. A slide declaration inside a view
overrides the shared viewport configuration.

Legend visibility is `auto`, `visible` or `hidden`; position is `top` or `bottom`.
Filters use named styles. Explicit legends are graph/state configuration.

## Extensions and availability

`@source`, `@doc` and `@meta` have strict schemas. Arbitrary annotations preserve
their order, repeated occurrences, raw spelling, arguments, attachment and
spans. Qualified extensions do not warn merely for being unknown; strong
unqualified typos receive a hint without automatic interpretation.

ER bodies, independent member ports, public extension-schema registration and
structured document JSON input remain separate work. The existing analyzer
graph JSON input keeps its version-one contract.
