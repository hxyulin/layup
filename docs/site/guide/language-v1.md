# Named documents and shared syntax

The current checkout implements Layup's shared document language and scoped
graph, sequence and state-machine grammars. A file can start directly with a named diagram. An optional
`layup 1` header asserts the supported source revision; it does not select a
dialect. Unknown revisions and non-integer revision numbers are errors.
These additions will ship after the published 0.3.0 packages.

The shared language owns comments, values, annotations, names, references and
diagram envelopes. `type` selects a diagram's body grammar. Graph, sequence and state-machine bodies are implemented, including views and
presentation steps. Unavailable types are preserved as opaque source with a
warning. The replacement grammar is the only source entry point; title-only
headers, implicit IDs, bare styling flags and typed arrow lexemes are rejected.

## Try a graph

```layup source
diagram services "Service calls" type=graph layout=auto flow-direction=right {
  node red "API" style=service
  @source(uri="src/worker.rs", symbol="Worker::run")
  node "Worker::run(&self)" "Worker" style=service
  edge dispatch red -> "Worker::run(&self)" "Dispatch" style=calls

  node-style service palette=blue base=node
  edge-style calls stroke-style=dashed palette=purple
}
```

The first name after `diagram`, `node`, `group`, `package`, or `crate` is its
required ID. A second quoted string is the display label; otherwise it defaults
to the ID. Quoted first names remain IDs. IDs cannot be empty or contain control
characters. Keywords and palette names such as `node`, `row` and `red` are
contextual and can be IDs. Quote literal `true`, `false` and `null` names.

Styles use attributes such as `palette=blue`, `font-family=mono`,
`text-align=center` and `stroke-style=dashed`. Style declarations belong
at diagram level. Bases may refer forward; cycles and duplicates are errors.
Attribute order never changes meaning. Use `fill-color`, `stroke-color`,
`text-color`, `background-color` (canvas), and `stroke-width` for direct paint.
Colors can be literal, theme tokens, or `{light: COLOR, dark: COLOR}` pairs.
`auto` clears a channel override; `none` removes shape/line paint. Obsolete
spellings such as `kind`, `tone`, `node-kind` and `edge-kind` are rejected.

A named `edge ID ...` can be referenced by views or steps. Anonymous
connections use the endpoints directly. Both accept `->`, `<-`, `<->` and `--`
and one optional quoted caption. `source-side` and `target-side` refer to
semantic endpoints; for `a <- b`, the source is `b`. `route-side` sets the
preferred route side. Text direction and flow direction are separate properties.

## Scopes and names

Named objects introduce semantic scopes. Rows arrange content without creating
a scope:

```layup source
diagram caches "Scoped caches" type=graph {
  row weights=[1, 1] {
    group backend "Backend" { node cache "Backend cache" palette=green }
    group frontend "Frontend" { node cache "Frontend cache" palette=blue }
  }
  edge sync frontend.cache -> ::backend.cache "Synchronize"
}
```

`backend.cache` has two path segments. `backend."worker.v2"` has a literal
second segment containing a dot. `"backend.cache"` is one literal name.
`::` starts at the diagram root. Relative lookup searches the current scope
and ancestors; once a prefix matches, lookup stays with that object. Siblings
can reuse local names. Duplicate names in a scope and ancestor-name shadowing
are rejected, including forward declarations.

Bare names use Unicode identifiers with internal single hyphens. Quote dots,
slashes, `::`, spaces and code punctuation. Case and Unicode spelling are exact.

## Comments and values

Newlines and semicolons separate statements; a closing brace ends the final
statement. `//` comments work after declarations and `/* … */` comments can
span lines and nest to 128 levels. Newlines inside block comments do not end
statements. Comments never carry annotation semantics. Strings keep comment
markers literally and support multiline content and `\"`, `\\`, `\n`, `\t`,
`\r` escapes. Unsupported escapes and unclosed comments/strings are errors.

Values distinguish choices, strings, integers, floats, booleans, null, lists,
records and syntactic references. Bare `blue` is a choice; `"blue"` is a string.
`1` is an integer; `1.0` and `1e0` are floats. Integers preserve exact signed
64-bit negative and unsigned 64-bit nonnegative values. Overflow is an error;
quote larger identifiers. Floats must be finite. Geometry validates its own
ranges and converts numeric values only when building the renderer model.

Collections allow newlines around separators and trailing commas. Record keys
are data strings, can be empty, and do not introduce object IDs. Record keys
and attributes cannot repeat. The shared nesting limit is 128, and each
annotation is limited to 1 MiB. Formatting preserves numeric spelling,
comments, string content, references and statement/event order.

## Annotations and extensions

Annotations use `@name(key=value, ...)`, with parentheses even when empty.
Names may be dotted, such as `@company.analysis(...)`. They attach to the next
construct in the same block, including styles, layout and content items.
Blank lines and comments preserve attachment; a closing brace does not.
Annotations can appear on the same line as their target and never inherit.

```text
@source(uri="src/api.rs", symbol="API::run",
  range={start-line: 8, start-column: 1, end-line: 12, end-column: 2})
@source(uri="src/lib.rs", symbol="exports::API")
@doc(text="Accepts a request.")
@company.analysis(confidence=0.92, generated=true, count=9007199254740993)
@meta(namespace="analysis", value={public: true, tags: [entry, "HTTP"], extra: null})
node "API::run(&self)" "API"
```

`@source`, `@doc` and `@meta` retain strict argument and cardinality checks.
Source evidence is repeatable and targets diagrams, objects or relationships;
ranges use 1-based lines/scalar columns and an exclusive end. `@doc` is a
singleton and `@meta` is unique per namespace. These locations describe original
code, separately from the declaration's DSL span.

Unknown annotations preserve their name, raw spelling, arguments, repeated
occurrences, attachment and spans. They have no rendering effect. A strong
unqualified typo such as `@sorce` receives a warning without being interpreted
as `@source`. Namespaced extensions do not warn merely for being unknown.
Opaque reference values in extension arguments are preserved without lookup.
A public host schema-registration API remains a separate checkpoint.

## Unavailable diagram types

```text
@company.owner(team=design)
diagram roadmap "Roadmap" type=company.timeline {
  launch => 2027-01-01 ? extension-specific punctuation
}
diagram services "Services" type=graph { node api "API" }
```

The unavailable diagram retains its envelope and balanced raw body. Braces
inside quoted strings or comments do not end the body. Shared quoting/comment
boundaries still apply, and malformed boundaries are document errors.
Formatting preserves the entire opaque block byte-for-byte. Its ID participates
in duplicate checks, and registered envelope annotations still validate.

Rendering defaults to the first supported diagram and warns about all skipped
entries. Explicitly selecting an unavailable diagram is an error. Supported
siblings are validated before selection, so selecting one cannot hide another
supported diagram's errors. `--strict` makes skip warnings fail CI. A document
with no supported diagram can be inspected and formatted but cannot render.
Typo hints only use a unique strong match in the same namespace.

## Inspect, select and export

```sh
layup inspect document.layup
layup render document.layup --diagram services --theme auto
layup compile document.layup --diagram services -o services.json
layup lint document.layup --json
layup fmt document.layup --check
```

```js
const inspection = engine.inspect(source);
const scene = engine.compile(source, { diagram: 'services' });
const { output, warnings } = engine.render(source, { diagram: 'services', format: 'html' });
```

Inspection returns `{document, diagnostics}` without layout. It exposes the
original source, typed diagrams, raw opaque bodies, annotations and value spans.
Recovery retains valid siblings and reports independent syntax/lexical errors;
the returned partial document does not make erroneous source renderable.
CLI inspection writes JSON and exits unsuccessfully if errors are present.
Formatting requires valid shared syntax but does not require an available body
renderer or successful semantic resolution.

Tagged values in inspection and annotation exports use forms such as
`{type: "choice", value: "blue"}`, `{type: "integer", value: "9007199254740993"}`
and `{type: "float", value: 1.0}`. Integer strings prevent JavaScript `Number`
rounding; use `BigInt(value.value)` when arithmetic is needed. Convenience
`@meta` JSON remains ordinary JSON, so use the authoritative tagged annotation
values for integers outside JavaScript's exact range. Reference tags distinguish
unresolved paths from strings and records.

Scene JSON retains version-1 geometry and adds document metadata: a manifest
of supported/selected/skipped diagrams, diagnostics, ordered annotations and
annotated non-rendered targets. SVG and HTML carry the same document metadata.
Full unavailable bodies are available through inspection, not embedded in
rendered exports. Renderer IDs remain opaque; use `document.diagramId`,
`objectPath` segments and `authoredId` for authored identity.

Markdown fences accept `diagram=services`. The playground renders the default
supported diagram. Try the [scoped example](/playground?example=language-v1).

## Diagram-specific syntax

The common grammar dispatches to graph, sequence or state-machine rules.
Sequence bodies use `participant`, `actor`, named `message` declarations,
`loop`, `optional`, `alternatives`/`branch`, and participant notes. Message
`type=call|reply` is separate from call `delivery=sync|async`.

State bodies use `state`, `initial`, `final`, `choice`, and named `transition`
declarations. Composite states introduce scopes; layout rows/sections do not.
Transition `event`, `guard` and `action` are display text.

All supported bodies own views, ordered presentation steps and slide
configuration. Graph/state bodies additionally support explicit legends.
Use `show objects=[...] connections=[...]`, `highlight` with the same lists,
and `speaker-note`. A view can override shared slide settings with its own
`slide size=...` declaration.

See the [complete syntax reference](/reference/dsl), [sequence guide](/diagrams/sequences),
[state guide](/diagrams/states), and [presentation guide](/guide/presentations).

## Remaining extensions

ER bodies, member-port geometry, public annotation-schema registration and
structured document JSON input remain separate work. Current analyzer graph
JSON and `compileModel`/`renderModel` retain their version-one contract.
The inspection API exposes the typed body, declaration/property/reference
spans and preserved extension data. Rendered document metadata also retains
object/connection declaration categories, attributes, source evidence and
resolved paint without embedding opaque foreign bodies.
