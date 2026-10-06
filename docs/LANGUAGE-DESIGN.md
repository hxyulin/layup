# Layup language design

Layup has one shared source language and a body grammar for each diagram type.
The current checkout has migrated graph, sequence and state-machine diagrams,
including styles, views, presentations and canonical examples. This is an
unreleased replacement grammar. There is no legacy source fallback.

The [DSL reference](site/reference/dsl.md) specifies implemented syntax;
[shared language rules](site/guide/language-v1.md) specify values, annotations,
scopes, inspection and unknown bodies. The
[vocabulary review](LANGUAGE-VOCABULARY.md) records why the replacement names
were selected and identifies proposed extensions.

## Shared document rules

A document contains `diagram ID ["Label"] type=TYPE { ... }` declarations.
`layup 1` is optional and asserts the supported revision. Unsupported revisions
fail. Diagram IDs are document-local; view IDs, step IDs, node/edge styles and
named connections have separate namespaces.

The shared lexer supports Unicode identifiers, quoted IDs and labels, exact
signed/unsigned 64-bit integers, finite floats, booleans, null, lists and ordered
records. Choice values retain their type separately from quoted strings.
Inspection serializes exact integers as tagged decimal strings.
Statements end at newlines or semicolons; structured values can span lines.
Inline `//` and nested `/* … */` comments are lexical content. Strings preserve
comment markers, escaped characters and literal newlines. Unknown escapes,
duplicate properties and duplicate record keys are errors.

References use dotted segments and optional root `::`. Containers and composite
states introduce scopes; layout rows and sequence fragments do not. An ID
containing a dot is a quoted single segment. Duplicate or ambiguous identities
fail. Renderer IDs encode full authored paths and are opaque: consumers use
`objectPath` and `authoredId` metadata instead of parsing those IDs.

Annotations attach to the next construct and preserve authored order, repetition,
typed values, raw text and source spans. Core `@source`, `@doc` and `@meta`
have validated contracts. Source ranges use kebab-case properties in DSL and
map explicitly to camelCase in graph input/output contracts. Arbitrary
annotations do not execute code or participate in name resolution.
Unused or dangling annotations produce errors.

Unavailable diagram types preserve their raw balanced bodies. Inspection and
formatting retain them, compilation skips them with warnings, and conservative
typo hints suggest close built-in type names. A recognized diagram with invalid
syntax cannot masquerade as an extension.

## Per-diagram grammar and semantics

| Type | Grammar and semantics |
| --- | --- |
| `graph` | Explicit nodes/containers, authored layout, automatic flow, four arrow forms |
| `sequence` | Participants/actors, named chronological messages, notes and fragments |
| `state-machine` | States/composites, initial/final/choice markers, transitions and reachability |

Declaration keywords are fixed. A custom style never becomes a keyword.
Objects require authored IDs; graph edges and machine transitions can be
anonymous, while sequence messages require names. Connection names remain
diagram-local even inside containers and fragments.

The parser validates which statements each body accepts. The document compiler
validates style inheritance, defaults, scoped references and every authored
presentation plan before diagram/view selection. It lowers typed definitions
into the existing model/layout engine. Geometry, reachability and readability
checks then apply to selected views. Consumers should validate all views in CI.

## Presentation and direct style properties

`type` selects semantics, `style` selects presentation, `shape` selects geometry,
`palette` selects a coordinated theme palette and individual color properties
override paint channels. Graph styles support rectangle, rounded rectangle,
diamond and pill geometry. State markers retain semantic geometry and
participants retain compatible header geometry.

Styles use `node-style` and `edge-style` with same-category `base` inheritance.
Precedence is built-ins, category defaults, inherited/declared style properties,
then instance properties. Unused styles and defaults still undergo validation.
One defaults declaration is allowed per category per diagram.

Paint distinguishes literals, theme tokens and `{light: ..., dark: ...}` pairs.
`fill-color`, `stroke-color`, `text-color`, `stroke-style` and `stroke-width`
are independent. `background-color` belongs to the canvas. Literal colors remain
literal in both themes; changing a palette does not erase inherited literal
channels. `auto` resets a channel, `none` suppresses fill/stroke, and container
paint does not propagate to children. Paint overrides retain measured geometry.

`flow-direction` and `participant-direction` are distinct from `text-direction`.
Text alignment can be physical or logical. Original Unicode, RTL and CJK font
policies remain intact.

Rows use `weights=[...]` and `gap=N`. Routing uses `source-side`, `target-side`
and `route-side`. `after`, `beside` and `same-rank` constrain flow. The former
combined preset is removed: width, layout, defaults and legend are independent.

Views contain typed object includes and layout/slide overrides. Steps use
`show objects=[...] connections=[...]`, `highlight` and `speaker-note`.
Grouped `slide` and `legend` statements separate their settings from diagram
attributes. `default-label` supplies connection captions on request;
`legend-label` supplies the style's legend entry.

## APIs and migration

Rust, CLI and WASM/JavaScript use the same strict source compiler, formatter,
linter and recovering inspection implementation. Typed syntax and rendered
metadata retain source locations independently of code-analysis evidence.
Scene version 1 exposes additive paint, declaration, typed attribute and
document metadata. Source IDs deliberately changed to scoped opaque IDs.
Version-1 structured graph input retains its existing ID and kind/tone contract;
its adapter goes directly to the backend and does not synthesize source text.

Canonical examples, Markdown guides, frontend fixtures and Tree-sitter sources
use the replacement grammar. Legacy low-level parser tests remain backend
regressions, rather than public source entry points. A migration must resolve
scopes and styles before rewriting references; token substitution alone is
insufficient for existing external documents.

Validation covers strict examples, native/WASM parity, recovery, exact values,
formatting equivalence, hidden invalid declarations and plans, paint inheritance,
logical text alignment, machine/sequence semantics, presentation, browser
exports and Tree-sitter regeneration.

## Remaining extensions

- ER bodies, field/key/cardinality semantics and their glyph conventions.
- Member/port identity, layout and routing. `port` is reserved and parsed;
  compilation reports that geometry is unavailable.
- New geometric primitives such as ellipses.
- A versioned full-document JSON input that uses the same typed definitions.
- Declarative property/schema introspection for editor completion and extension
  validation. Existing typed values are enums; schema descriptions are future work.
- Imports, templates, executable expressions and automatic trace playback.

These extensions can build on the shared language without retaining a second
source dialect. They need concrete semantics and regression fixtures before
being accepted as supported diagram constructs.
