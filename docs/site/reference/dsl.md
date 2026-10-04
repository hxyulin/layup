# DSL reference

Items end with a newline or semicolon; braces introduce a body. Quoted strings
contain display text, bare IDs reference semantic objects, and `key=value`
sets an attribute. See [the language walkthrough](/guide/language) for syntax
and [diagram guides](/examples) for complete rendered examples.

## Document headers

| Syntax | Meaning |
| --- | --- |
| `diagram "Title" { ... }` | Compile one diagram |
| `model "Title" { ... view NAME { ... } }` | Shared definition with named views |
| `mode=graph\|state-machine\|sequence` | Semantic/layout mode; default graph |
| `layout=manual\|auto` | Authored block flow or inferred graph layout |
| `direction=down\|up\|right\|left` | Graph flow; sequences support right/left participant order |
| `text-direction=auto\|ltr\|rtl` | Independent label direction |
| `width=N` | Positive original canvas width |
| `preset=clean\|manual` | Default automatic styling or explicit-only style policy |
| `slide=wide\|standard\|"W:H"` | Separate presentation viewport |
| `slide-padding=N`, `min-font-size=N` | Fit padding and effective text-size threshold; require slide |
| `title "..."`, `subtitle "..."`, `desc "..."` | Title, subtitle, accessible description |
| `legend [bottom] [off] [kind ...]` | Graph/machine legend; explicit legends unavailable in sequence mode |

## Structural items

| Item | Behavior |
| --- | --- |
| `row [w:w:w] [gutter=N] { ... }` | Weighted side-by-side cells; equal weights by default |
| `group id "Title" { ... }` | Hollow neutral container |
| `package id "Title" { ... }` | White frame with package typography |
| `crate id "Title" { ... }` | Hollow container with a role |
| `band "Title" { ... }` | Uppercase band label above children |
| `section "Title" { ... }` | Rule and title over a section |
| `gap [N]` | Vertical space or an empty row cell |
| `divider "Caption" [tone]` | Dashed labeled rule |
| `text "Paragraph"` | Wrapped structural prose |

## Node kinds and content

```text
KIND [id] ["Title"] [tone] [flags] [key=value ...] [{ content }]
```

| Kind | Shape |
| --- | --- |
| `node`, `card` | Filled rounded card; nested children turn generic nodes into containers |
| `api`, `trait`, `type`, `module` | Compact interface box |
| `decision`, `choice` | Diamond question or machine pseudostate |
| `process` | Compact rounded step |
| `terminal` | Capsule outcome |
| `state` | Leaf state or composite state frame |
| `initial`, `final` | Start dot or bullseye; no text body |
| `participant`, `actor` | Sequence header and lifeline; explicit ID, no body |

Flags: `hollow`, `filled`, `center`, `left`, `mono`, `sans`.
Tones: `gray`, `blue`, `green`, `yellow`, `purple`, `orange`, `red`.
Attributes: `id`, `tone`, `role`, `tag`, `href`, `align`, `text-direction`,
`gutter`; graph hints `after`/`below`, `same-layer`, `beside`.
Sequence participants have authored order; hints/gutters do not apply.

| Content item | Meaning |
| --- | --- |
| `name "..."` | Header/title |
| `code "..."` | Monospace line |
| `sub "..."`, `text "..."` | Wrapped prose |
| `role "..."`, `tag "..."` | Role or semantic tag |
| Child blocks | Containers/composite states; compact flowchart shapes need an outer group |

Custom kinds use `style NAME base=KIND tone=TONE ...`. Shapes include card,
api, package, container, decision/diamond, process, terminal, state, initial,
final, and choice. A style can set `role`, `label`, `align`, tones, and flags.

## Edges

```text
from -> to "Label" id=relation
from <- to
from <-> to
from -- to
implementation -impl-> interface labeled
```

| Option | Meaning |
| --- | --- |
| `id=NAME` | Stable authored edge ID |
| `"Label"` or `label="Label"` | Caption |
| `labeled` | Show the kind's default caption |
| `tone=TONE` or bare tone | Override color |
| `dashed`, `solid` | Override dash style |
| `from=SIDE`, `to=SIDE` | Pin endpoint sides in graph/machine modes |
| `via=SIDE` | Outside routing side |
| `bus` | Graph edges share a port/channel |
| `async`, `return` | Sequence open head or dashed return |

Sides are top, right, bottom, left. Built-in typed arrows include `impl`,
`extends`, `uses`, `exports`, `depends`, and `flow`. Inspect defaults
in the [style source](https://github.com/hxyulin/layup/blob/main/crates/layup/src/style.rs).
Custom arrows use `arrow NAME TONE [dashed] ["Default label"]`; `inherit`
uses the target node's tone.

## Sequence fragments

`note "Text" [over=ID | from=ID to=ID]` annotates an event position.
`loop "Label" { ... }` and `opt "Label" { ... }` need nonempty bodies.
`alt "Label" { branch "A" { ... }; branch "B" { ... } }` needs at least
two labeled branches. These are displayed structure, not execution rules.

## Models and views

`view NAME ["Title"] [attributes] { include ID ...; step ... }` selects shared
nodes and supplies optional per-view steps. Container includes expand their
subtree; descendants retain ancestors. Both edge endpoints must survive.
All shared node IDs are explicit. See [selection and validation rules](/guide/models).

## Presentation steps

`step NAME ["Title"] { ... }` is a direct child of a diagram or model/view.

| Directive | Meaning |
| --- | --- |
| `show ID ...` | Accumulate visible nodes |
| `show-edge ID ...` | Accumulate explicitly revealed edges |
| `highlight ID ...` | Emphasize visible nodes for this step |
| `highlight-edge ID ...` | Emphasize visible edges for this step |
| `note "Text"` | Plain-text speaker note |

For cumulative visibility and automatic edge rules, see
[progressive reveal](/guide/presentations#reveal-the-explanation-in-steps).
