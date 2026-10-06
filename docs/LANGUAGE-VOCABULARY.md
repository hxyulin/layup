# Layup vocabulary review

This review covers the existing DSL, the implemented document/graph checkpoint,
and the sequence, state, ER, view, and presentation syntax in the language
design. Its spellings are the agreed design for the replacement language; they
are not yet the parser's accepted keywords. The current implementation remains
documented in the [experimental guide](site/guide/language-v1.md).

The replacement will become the only source grammar. Unversioned source will
use it; an optional `layup 1` header will assert the language revision rather
than select a compatibility dialect. Unknown revisions will fail. There is no
need for a permanent legacy parser or aliases, given the project's internal
usage. Migration must preserve all existing diagram features before removing
the old entry points. The header does not promise ongoing support for multiple
source revisions; only the supported revision is accepted.

## Naming rules

- Declaration words name the object or operation: `node`, `message`, `state`,
  `row`, `include`, `show`.
- Properties name what they control. Avoid `kind`, `mode`, or `preset` when a
  specific word such as `style`, `palette`, or `flow-direction` is available.
- Use lowercase hyphenated property names, including typed annotation records.
  Keep full words: `text-direction`, not `textdir`; `optional`, not `opt`.
- Keep semantic type, reusable style, shape geometry, colors, and layout separate.
  Applying a style must not create a new grammar or change a semantic object type.
- Use explicit booleans for independent flags and enums for mutually exclusive
  choices. Do not combine unrelated choices into one enum.
- Named declarations have one required ID and one optional positional label.
  IDs, labels, and styles cannot be guessed from colors or generated keywords.
- Every property has a target schema, inheritance rule, and observable effect.
  An unsupported property is an error, including a known property on the wrong
  construct. Extension annotations and unknown diagram bodies are preserved by
  the explicit extension rules below. Avoid compatibility synonyms and silent
  no-ops in recognized syntax.

## Types and reusable styles

`kind` currently selects both a diagram grammar and a configurable node/edge
presentation. Those are different operations. In `style.rs`, node styles bundle
shape, palette, font, alignment, and display labels; arrow styles bundle color,
dash patterns, and labels. Calling these reusable bundles styles is more direct.

| Current or draft spelling | Recommendation | Meaning |
| --- | --- | --- |
| Diagram `mode=...` / `kind=...` | `type=graph\|sequence\|state-machine\|er` | Select the diagram grammar and validator |
| Node/edge `kind=service` | `style=service` | Apply a named presentation style |
| `style NAME` / `node-kind NAME` | `node-style NAME` | Declare a reusable node/object presentation |
| `arrow NAME` / `edge-kind NAME` | `edge-style NAME` | Declare a reusable connection presentation |
| `base=NAME` | Keep `base=NAME` | Inherit a named style of the same category |
| `shape=...` | Keep `shape`; audit its values | Select geometry, not a semantic object or diagram type |
| `preset=clean\|manual` | Remove this combined switch | Express palette, spacing, legend, and sizing policies independently |
| `defaults CATEGORY ...` | Keep | Set properties for a target category in the diagram |
| Graph declaration `card`, `api`, `trait`, `type`, `module`, `process`, `decision`, `terminal` | `node ID style=...` | Built-in styles select the presentation; optional `role` retains the original displayed classification |
| Palette aliases `grey`, `neutral`, `amber`, `violet` | Canonical `gray`, `yellow`, `purple` names where they currently alias | Keep a small documented palette registry; distinct named palettes must have distinct definitions |

Built-in visual templates belong in the style registry. `card`, `interface`,
`container`, and `package` can be built-in node styles. Shape values should
describe geometry such as `rectangle`, `rounded-rectangle`, `diamond`,
`ellipse`, and `capsule`. A package tab or interface content arrangement is a
template feature, not a new semantic type inferred from a shape. State initial,
final, and choice markers retain their state-grammar rules independently of
styling; styling cannot turn a regular graph node into an initial state.

Resolve built-in defaults, diagram defaults, base styles, declared overrides,
and instance overrides in that order. Within a declaration, attributes are
order independent. Forward bases remain valid; duplicate styles, incompatible
bases, and cycles are errors. One style reference per object is sufficient for
this revision; multiple-style composition is deferred.

## Colors and paint

The current `tone` is a named light/dark palette. It changes several drawing
colors, while ordinary text and code also use independent theme ink colors.
Renaming it to `fg-color` would inaccurately describe its current behavior.

Prefer `palette` for the convenient coordinated choice, with explicit paint
properties for precise control:

| Property | Target and effect |
| --- | --- |
| `palette=blue` | A coordinated theme-aware palette for an object or connection |
| `fill-color=...` | Interior of a node shape; `none` removes its fill |
| `stroke-color=...` | Node outline or connection line and its arrowheads |
| `text-color=...` | Visible text owned by that target; on a connection, its caption |
| `background-color=...` | Diagram canvas; distinct from a node's interior |
| `stroke-style=solid\|dashed\|dotted` | Pattern of the outline or connection line |
| `stroke-width=N` | Outline/line thickness in scene units |

The explicit properties identify the painted part. `fg-color` is ambiguous
between text, outlines, and arrows. `bg-color` can mean a node interior, a
caption chip, or the diagram canvas. Do not introduce those abbreviations as
aliases. `border-color` is also less consistent across nodes and connections.

SVG distinguishes interior fill paint and outline stroke paint. These terms
provide a useful common vocabulary for node shapes and connection lines.
Layup's properties operate on semantic targets, however, rather than applying
arbitrary SVG properties to every descendant. In particular, `fill-color` on
a node does not recolor its text, even though SVG also uses fill paint for
glyphs. See [SVG 2 painting](https://www.w3.org/TR/SVG2/painting.html).

Direct colors initially accept recognized named colors, quoted hex strings,
approved theme tokens, or a `{light: COLOR, dark: COLOR}` record. A literal
color stays literal in both themes. A palette or theme token adapts to the
selected theme. Thus `palette=blue` is a coordinated blue treatment;
`fill-color=blue` is a literal blue interior. `auto` restores the property's
derived default; `none` is valid for shape/line paint, not an enum meaning
"use the default". Gradients, paint-server URLs, arbitrary CSS, and custom
color functions are outside the initial color grammar.

Resolve the final palette and inherited explicit channel overrides separately.
An explicit channel wins over the palette, regardless of attribute order.
Inherited literal overrides remain explicit when a child changes palette;
`fill-color=auto`, for example, discards an inherited fill override. Container
paint does not automatically propagate to children. Document which text and
typography defaults inherit through semantic scopes; layout blocks do not
create accidental inheritance boundaries.

Replace `fill=hollow` with `fill-color=none`. Replace `fill=filled|solid` with
`fill-color=auto`. Replace `stroke=dashed` with `stroke-style=dashed` so stroke
color, width, and pattern remain independently expressible. Unsupported
channels on a construct are errors; a line has no node interior to fill.
`dotted` and new paint overrides require renderer support before acceptance.

## Text, labels, and metadata

| Current or draft spelling | Recommendation | Meaning |
| --- | --- | --- |
| Optional positional quoted title/caption | Keep | Display label; default to a named object's ID |
| Instance `name`, `title`, `label` aliases | Remove duplicate assignment paths | One label assignment per declaration |
| Caption fallback from arrow style `label="calls"` | `default-label="calls"` | Default caption supplied by a connection style |
| Arrow `chip="impl"` | `default-label="impl"` | Caption text, not the geometry or background of a chip |
| Node/arrow style legend label | `legend-label="..."` | Legend entry text, independently of a connection caption |
| `labeled` / `caption=kind` | `label=style` | Request the selected style's default caption; cannot coexist with a positional caption |
| `font=mono\|sans` | `font-family=mono\|sans` | Logical supplied/bundled font category, not an arbitrary installed family name |
| `align=...` | `text-align=start\|end\|left\|right\|center` | Text alignment, not node positioning |
| `textdir` / `text-direction` | `text-direction=auto\|ltr\|rtl` | Text base direction, independently of diagram flow |
| `role="..."` | Keep with explicit display semantics | Small descriptive role label, not an executable classification |
| `tag "..."` / `tag=...` | Keep the `tag "..."` content item | Short displayed tag; not a hidden prefix extracted from the main label |
| `code "..."` | Keep | Literal monospace content |
| `sub "..."` / `text "..."` | `text "..."` | Prose content; no content-body nesting |
| Diagram `note` / `subtitle` | `subtitle "..."` | Visible diagram subtitle |
| `desc "..."` | `@doc(text="...")` | Documentation/accessibility metadata |
| `@source(uri, symbol, range)` | Keep the annotation and argument names | Original code location, separate from the DSL declaration span |
| Source range `startLine`, etc. | DSL `start-line`, `start-column`, `end-line`, `end-column` | Consistent source property spelling; retain documented 1-based scalar positions and exclusive end |
| `@meta(namespace, value)` | Keep | Opaque data, unique per namespace on a target |
| `true`, `false`, `null` | Keep | Typed values; quote these words when used as literal IDs |

Wire JSON may retain camelCase names such as `startLine`; map them explicitly
to the same typed model. Source conventions do not require changing a stable
wire contract. Keys inside opaque `@meta` data are user data and retain their
exact case and spelling. Do not normalize arbitrary metadata keys.

Annotations retain next-target attachment, same-block boundaries, repeatable
`@source`, singleton `@doc`, and no implicit inheritance. Arbitrary names use
the preservation and registration rules below. Comments use `//` and nested
`/* ... */`; imports,
macros, executable conditions, and interpolation are deferred.

## Extension annotations

Annotations are the extension surface. Accept arbitrary unqualified or dotted
names, with namespaces recommended for extensions:

```text
@acme.owner(team="payments", reviewers=["alice", "bob"])
@acme.analysis(confidence=0.92, generated=true)
node api "API" style=service

@review(status="draft")
view overview "Overview" { include api }

@acme.layout(priority=10)
row gap=24 { node worker; node queue }
```

The shared parser attaches annotations to the next complete construct in the
same block. This includes diagrams, objects, connections, style declarations,
layout/content items, views, steps, and diagram-specific events or fragments.
Known annotation schemas subsequently restrict targets; an unknown annotation
does not acquire a target restriction by guessing from its name. Comments and
blank lines do not break attachment. A closing brace does; orphan annotations
remain errors. Annotations never inherit implicitly.

| Annotation case | Default behavior |
| --- | --- |
| Registered annotation such as `@source` | Validate its arguments, target, and cardinality; invalid use is an error |
| Unknown namespaced annotation such as `@acme.owner` | Preserve its exact name, arguments, order, attachment, and source spans; no warning merely for being unknown |
| Unknown unqualified annotation such as `@review` | Preserve in the same way; no warning unless there is a strong typo candidate |
| Likely misspelling such as `@sorce` | Preserve; warn with a suggested registered spelling; never rename or interpret it automatically |
| Repeated unknown annotations | Preserve every occurrence in order; do not collapse them into a map |
| Malformed shared argument syntax or duplicate argument keys | Syntax error, regardless of whether the name is known |

Core names `source`, `doc`, and `meta` are reserved for their registered
semantics. An extension must not override them or make malformed core
annotations opaque. `@meta` remains a convenient metadata container, rather
than the only permitted extension name. Names and values retain exact Unicode
spelling; qualified names are registry names, not object references.
Annotation names use the shared identifier grammar, with dots separating
namespace segments; arbitrary names mean no fixed registry whitelist, not
arbitrary punctuation in a declaration name.

The ordered annotation list is the authoritative data. Core fields such as
source locations and documentation are derived from it, rather than replacing
or discarding the original annotations. Data values use the shared value
grammar. Unknown arguments do not bind object references or run expressions;
preserve a syntactic reference as an unresolved value if one is supplied.
An extension schema can explicitly opt into shared reference resolution.
Wire serialization must distinguish unresolved references from strings and
ordinary records instead of inserting ambiguous magic keys into user data.

A host may explicitly register an annotation handler/schema with a qualified
name, allowed targets, typed arguments, and cardinality. If registered, its
validation failures are errors. Registration does not add statement keywords,
modify core meanings, or trigger automatic package loading. Unregistered
annotations have no effect on layout or rendering. Consumers can inspect them
through the typed document API and exports; annotations on non-rendered syntax
need document-level target records, not just node/edge metadata maps.

Keep first-class `@...` syntax. Do not add a second semantic-comment spelling
in this revision: `//` and `/* ... */` remain ordinary lossless trivia. A separate tool may
interpret comments it owns, but Layup's annotation attachment and export
contract apply to actual annotations. Registered core annotations remain
strict while extension annotations remain open.

## Unrecognized diagram types

Documents can contain diagrams whose type has no installed parser/renderer:

```text
diagram services "Services" type=graph {
  node api "API"
}

@acme.owner(team="design")
diagram roadmap "Roadmap" type=acme.timeline {
  milestone launch => 2027-01-01
  extension-specific ? punctuation
}
```

Accept qualified diagram type names. A name such as `acme.timeline` is a
registry key, not a scoped object reference. Preserve the diagram's ID, label,
header attributes, annotations, complete raw body, and spans. Do not validate
its body as a graph, discard it, or coerce its type to a guessed spelling.
Its ID still participates in document-wide duplicate-ID checks.

The parser must read the common envelope before dispatching into a body
grammar. For an unrecognized type, scan a balanced opaque block directly from
source. It must track shared braces, double-quoted strings and backslash escapes,
line comments, and nested block comments; braces in strings/comments do not end
the block. Do not run
the core body lexer over opaque text: extension punctuation may be legal to
another grammar. Body skipping checks boundaries, not literal/value semantics.
An unterminated string or unbalanced block is a document error because the next
diagram boundary cannot be determined reliably. An extension's own quoting or
comment conventions cannot override these shared envelope boundaries.
The scanner uses the same nesting bound as supported bodies. Annotations
inside an opaque body remain raw body text; there is no per-object attachment
or validation until that diagram's grammar is available. Envelope annotations
still receive their normal registered validation.

| Situation | Default behavior |
| --- | --- |
| Unknown type among supported diagrams | Preserve and skip its compilation; emit one warning per diagram |
| Unknown type close to a registered name, such as `grap` | The warning also suggests `graph`; do not silently correct it |
| Registered type whose handler is unavailable/disabled | Skip with an unavailable-handler warning; no misleading spelling suggestion |
| Explicit `--diagram roadmap` / `{ diagram: 'roadmap' }` for an opaque diagram | Error identifying the unavailable type; never select another diagram instead |
| No explicit selector | Choose the first diagram with an available handler; keep all declared diagrams in the document manifest |
| No supported diagram for render/scene compilation | Error with the unavailable types and any typo hints; do not emit an empty diagram |
| Lint/check an otherwise valid document with only opaque diagrams | Report skip warnings; validate the common envelope; report that no bodies were semantically checked |
| Invalid common header, duplicate IDs, unclosed body | Error, even for an unknown type |
| Known diagram type with malformed body | Error; never downgrade malformed recognized syntax into an opaque extension |

Parser availability and output availability are distinct. If a registered
parser/validator exists but its layout/output handler is unavailable, still
parse and validate the typed body, then report that it cannot render. Use an
opaque body only when the body parser is unavailable. Record which stage was
skipped in the manifest; missing output support must not hide a syntax error
that the installed parser can diagnose.

The default output policy is **warn and skip**. CLI
`--unknown-diagrams=warn|error|ignore` and API
`unknownDiagrams: 'warn' | 'error' | 'ignore'` let a caller choose whether
unavailable types warn, fail, or are intentionally ignored. This setting never
turns a malformed envelope into valid input, makes an unsupported selection
renderable, or changes preservation. Existing strict warning handling makes
the default skip warning fail CI; callers handling mixed-extension documents
can explicitly choose `ignore`. `ignore` still exposes skipped entries in the
document/result manifest.

Batch builds can omit documents with no available diagram and report them in
their skipped-file summary, rather than creating empty output. Explicit single
render/compile operations still need a supported selected diagram. Diagnostics
and skip counts must remain visible even if successful siblings produce output.

Warning codes distinguish `diagram/unrecognized-type`,
`diagram/unavailable-type`, and `annotation/possible-typo`. Include the type/name
span, diagram ID where applicable, preservation/skip outcome, and actionable
help. A successful render's warnings must include skipped diagrams outside the
selected diagram; selection must not hide partial-document processing.

Typo candidates come only from registered names. Use a conservative Unicode
edit-distance check that recognizes adjacent transpositions, with a unique
strong match; omit suggestions for weak or
tied candidates. Compare qualified names only within the same namespace; an
unknown `acme.graph` is not a typo for core `graph`. Case corrections may be
suggested but never applied. Exact name resolution remains case sensitive.

Keep opacity in the typed model, for example an `Opaque` diagram-body variant
with its raw source and origin. Structured document JSON likewise preserves an
unknown discriminator's JSON payload as opaque data, while recognized bodies
retain strict schemas. Parse/inspect/format APIs can operate on a document even
when none of its diagrams can render. Formatting an opaque body preserves its
interior exactly; do not reindent, normalize comments, or decode its strings.

Rendered scene/SVG/HTML exports expose a manifest of supported, selected, and
skipped diagrams with their types and reasons. They preserve annotation data
needed by consumers, but do not embed entire unsupported bodies by default.
Lossless source/document exports retain those bodies. Preserved does not mean
semantically validated or rendered; make this distinction explicit in API
results and editor status. An editor keeps opaque diagrams selectable for
source inspection and labels their preview as unavailable.

## Layout and connections

| Current or draft spelling | Recommendation | Meaning |
| --- | --- | --- |
| `layout=auto\|manual` | Keep | Inferred graph placement or authored block placement |
| Graph/state `direction` | `flow-direction=down\|up\|right\|left` | Direction along graph ranks |
| Sequence `direction` | `participant-direction=right\|left` | Participant order; chronology still proceeds downward |
| `width=N` | Keep | Scene canvas constraint, separate from slide fitting |
| `row weights=[...]` | Keep | Relative cell widths |
| Row `gutter=N` | `gap=N` | Space between row cells |
| Node `gutter=N` | Remove until a defined spacing property has an effect | Currently accepted and stored without controlling rendered geometry |
| `below=REF` / `after=REF` | `after=REF` | Later rank along flow, not necessarily physically below |
| `same-layer=REF` | `same-rank=REF` | Same graph rank |
| `beside=REF` | Keep | Immediate peer neighbor in the same rank; preserve its directional ordering semantics |
| `from=SIDE`, `to=SIDE` | `source-side=SIDE`, `target-side=SIDE` | Anchors of semantic endpoints |
| `via=SIDE` | `route-side=SIDE` | Preferred exterior side; current value is a side, not a waypoint |
| `port ID side=SIDE` | Keep | Named member anchor on its owner |
| `bus` | `bus=true\|false` | Enable or disable shared routing channels |
| Arrow `inherit` | `palette=target` | Explicitly take the target object's palette; `palette=source` selects the source |
| `->`, `<-`, `<->`, `--` | Keep | Directed, reverse directed, bidirectional, undirected connections |
| Typed lexemes such as `-uses->` | Remove | Select a connection style with `style=uses` |
| `id=NAME` on an edge | `edge NAME ...` | Authored relationship identity; bare connection remains anonymous |

Keep graph `node`, `edge`, `group`, `package`, and `crate` declarations. Keep
`row`, `section`, `band`, `divider`, and `gap` as layout/content constructs,
without semantic scopes. Prefer `gap size=N` for an explicit spacer instead
of an unlabelled numeric argument. A row's empty cell is distinct from a sized
spacer; reject a size if the row layout cannot honor it.

Keep physical sides `top`, `right`, `bottom`, and `left`; they identify geometry
and do not reverse with text direction. Keep direction values spelled out;
remove `TB`, `TD`, `BT`, `LR`, and `RL` aliases. `start`/`end` text alignment
is logical and follows text direction; `left`/`right` alignment remains physical.

The node-gutter finding was checked against layout reads and with a native
scene comparison: changing only a parent node's gutter from 16 to 96 produced
an identical complete exported scene. Do not carry that no-op into the new
schema. Likewise, the existing style `preset` also affects automatic width and
packing, so its removal needs an explicit layout/size policy rather than a
simple token rename.

The legacy node flag helper also accepts `dashed`, `solid`, `inherit`, `top`,
`bottom`, and `caps` without changing a node style. These must not survive as
accepted no-ops. `stroke-style` can replace the stroke flags once node outline
patterns are implemented; the other words need actual typed semantics or must
be rejected. Resolve automatic palettes on the complete definition before
filtering a view so selecting a subset cannot inadvertently recolor its objects.

## Diagram-specific words

| Area | Recommendation |
| --- | --- |
| Sequence declarations | Keep `participant`, `actor`, and `message` |
| Sequence fragments | Keep `loop` and `branch`; spell `opt` as `optional`, `alt` as `alternatives` |
| Message semantics | `type=call\|reply`; `delivery=sync\|async` applies to calls |
| Existing `return` / `delivery=return` | Replace with `type=reply`; a reply is a message type, not a delivery mechanism |
| Sequence note | Keep `note`; `over=REF` for one participant, `between=[REF, REF]` for a span; reject both on one note |
| State declarations | Keep `state`, `initial`, `final`, `choice`, and `transition` |
| State transition content | Keep `event`, `guard`, and `action`; display-only strings, not evaluated expressions |
| State entry/exit content | Keep `entry` and `exit` when implemented as typed code text |
| ER declarations | Keep `entity`, `fields`, `field`, and `relationship` |
| ER field display type | `data-type="uuid"`, instead of a generic `type` property |
| ER nullability | Keep `nullable=true\|false` |
| ER `key=primary\|foreign\|unique` | Independent `primary-key`, `foreign-key`, and `unique` booleans; a field can be both a primary and foreign key |
| ER endpoint cardinality | `source-cardinality` / `target-cardinality`, consistently with side properties |
| Cardinality values | `one`, `zero-or-one`, `zero-or-more`, `one-or-more`; remove the ambiguous `many` shorthand |

Database types, key groups, and cardinalities remain descriptive diagram data;
this vocabulary does not introduce a database validator or executable simulator.
Composite keys still need a named group model; independent field flags cannot
express them. Message lifetime and activation rules need their own grammar,
not generic graph style flags.

## Views, steps, and export configuration

| Current or draft spelling | Recommendation | Meaning |
| --- | --- | --- |
| `model` wrapper | Remove | A named `diagram` owns the definition and its views |
| `view`, `include`, `step` | Keep | Select a subset and present it in ordered steps |
| `show REF ...`, `show-edge REF ...` | `show objects=[...] connections=[...]` | Typed identity lists across graph, sequence, state, and ER grammars |
| `highlight REF ...`, `highlight-edge REF ...` | `highlight objects=[...] connections=[...]` | Same target categories as `show` |
| Step `note` | `speaker-note "..."` | Plain presenter text, distinct from sequence notes or documentation |
| `legend` with bare flags | `legend visibility=auto\|visible\|hidden position=top\|bottom` | Separate display policy and position |
| Legend `kinds` / `arrows` filters | `nodes=[STYLE, ...] edges=[STYLE, ...]` | Select named style entries |
| Root `slide`, `slide-padding`, `min-font-size` attributes | `slide size=wide padding=32 min-font-size=14` | Group slide viewport properties in one configuration declaration |
| Custom slide dimensions | `size={width: 1920, height: 1080}` | Explicit dimensions; avoids confusing a ratio with the existing dimension string |
| CLI/API `theme=light\|dark\|auto` | Keep as an export option | Output color-scheme policy, independent of source grammar |
| CLI/API `diagram`, `view`, `format`, `fonts` | Keep | Select output and supply measurement fonts; distinct from source style properties |

`objects` resolves scoped node/participant/state/entity identities; `connections`
resolves diagram-wide edge/message/transition/relationship IDs. These lists
avoid new presentation verbs for each body grammar and preserve separate ID
namespaces. Members remain ineligible for independent reveal until measured
member geometry and reveal behavior are implemented.

## Example of the recommended vocabulary

This example is a target for migration, not currently accepted syntax:

```text
diagram services "Service calls" type=graph layout=auto flow-direction=right {
  defaults node palette=gray
  node-style service base=card palette=blue font-family=sans
  edge-style calls palette=purple stroke-style=dashed default-label="calls"

  @source(uri="src/api.rs", symbol="API::run",
    range={start-line: 8, start-column: 1, end-line: 12, end-column: 2})
  node api "API" style=service
  node worker "Worker" style=service fill-color=none
  edge dispatch api -> worker "Dispatch" style=calls

  view detail "Request path" {
    include api worker
    step request {
      show objects=[api, worker] connections=[dispatch]
      highlight objects=[worker]
      speaker-note "The API dispatches work."
    }
  }
}
```

An exact themed override would be
`fill-color={light: "#eef6ff", dark: "#14263d"} text-color={light: "#162033", dark: "#eef4ff"}`.
Changing a palette or one paint channel must not change object identity, shape,
diagram type, connectivity, or layout constraints.

## Replacement order

1. Adopt this vocabulary in the target design and freeze the per-construct
   property schemas, including inherited versus local properties. Implement
   ordered extension annotations and opaque diagram preservation at the document
   boundary before grammar-specific lowering can lose their data.
2. Update the graph parser and presentation model together; adding color names
   without independent paint channels would be misleading.
3. Implement sequence/state bodies, views, steps, and slide/legend configuration
   to cover the existing canonical fixtures. Add ER afterward.
4. Convert the repository's examples, guides, tests, and generated previews in
   one breaking change. An internal conversion helper may resolve old IDs and
   references; it need not become a supported product API.
5. Make the replacement parser the sole source entry point, remove dialect
   fallback and obsolete aliases, and keep source/structured-input parity tests.

Scene JSON and analyzer graph JSON have their own version contracts. Audit
their `kind`, `tone`, and style fields when independent paint channels are
implemented; do not silently assign new meanings to old fields. Internal Rust
enum names need not mirror every source spelling. Neither removing a source
compatibility layer nor changing a keyword requires discarding source-origin
data or changing renderer IDs.
