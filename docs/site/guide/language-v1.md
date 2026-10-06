# Experimental language revision

The current checkout implements the first graph checkpoint of a new Layup
language. Start a file with `layup 1` to opt in. Published 0.3.0 packages and
unversioned files use the [existing DSL](/reference/dsl).

The shared document grammar owns comments, values, annotations, names, and
references. A diagram's `kind` selects its body grammar. This checkpoint
implements `kind=graph`; sequence, state-machine, and ER grammars report an
explicit unsupported-kind error until their own parsers are implemented.

## Try a graph

```layup source
layup 1

@doc(text="An API dispatches work to a worker.")
diagram services "Service calls" kind=graph layout=auto direction=right {
  node red "API" kind=service
  @source(uri="src/worker.rs", symbol="Worker::run")
  node "Worker::run(&self)" "Worker" kind=service
  edge dispatch red -> "Worker::run(&self)" "Dispatch" kind=calls

  node-kind service tone=blue base=node
  edge-kind calls stroke=dashed tone=purple
}
```

The first name after `diagram`, `node`, `group`, `package`, or `crate` is its
required ID. A second quoted string is the display label; otherwise the label
defaults to the ID. A quoted first name is still an ID. Empty IDs are errors.
Names such as `red` and `row` are ordinary IDs, independent of colors or kinds.

Style properties use attributes: `tone=blue`, `fill=hollow`, `font=mono`,
`align=center`, and `stroke=dashed`. `node-kind` and `edge-kind` declarations
belong directly to the diagram. Their `base` may refer forward to another
kind; inheritance cycles and duplicate kinds are errors. Attributes override
the base regardless of their order. Instantiate a node with `node ID kind=NAME`
and a relationship with `kind=NAME`; a kind never adds a statement keyword.

`edge dispatch ...` declares a named relationship. `red -> worker` declares an
anonymous one. Both support `->`, `<-`, `<->`, and `--`. A single optional quoted
caption follows the endpoints. `source-side` and `target-side` refer to semantic
endpoints: `a <- b` has source `b`; bidirectional and undirected edges retain
written endpoint order. `via` sets the route's preferred side.

## Scopes and names

Named nodes and containers introduce scopes. Rows arrange content without
adding a scope:

```layup source
layup 1
diagram caches "Scoped caches" kind=graph {
  row weights=[1, 1] {
    group backend "Backend" {
      node cache "Backend cache" tone=green
    }
    group frontend "Frontend" {
      node cache "Frontend cache" tone=blue
    }
  }
  edge sync frontend.cache -> ::backend.cache "Synchronize"
}
```

`backend.cache` has two path segments. `backend."worker.v2"` has a literal
second segment containing a dot. `"backend.cache"` is one literal name, not a
qualified path. `::` starts at the diagram root. Unqualified lookup searches
the current scope and then ancestors; once a prefix matches, lookup stays with
that object. Siblings may reuse names; duplicate names in one scope and
ancestor-name shadowing are rejected, including forward declarations.

Bare names use Unicode identifiers with internal single hyphens. Quote names
containing dots, slashes, `::`, spaces, or code punctuation. Quote literal
`true`, `false`, and `null` IDs. Case and Unicode spelling remain exact.

## Shared values and annotations

Statements end at a newline or semicolon. `//` comments work outside strings,
including after a declaration on the same line. `/* ... */` comments work
between tokens and across lines, and can nest up to 128 levels:

```text
node api "API" tone=/* explicit palette */blue // inline explanation
/* A longer explanation.
   /* Nested comments are allowed. */
*/
node worker "Worker"
```

Comments act as whitespace; newlines inside a block comment do not terminate
a statement. Delimiters inside strings remain literal. Formatting retains block
comment contents and internal newlines; an unclosed block comment is an error.
Comments have no annotation semantics.

Values include strings, finite numbers, bare enum words, booleans, `null`,
comma-separated lists, and records. Lists and records support newlines and
trailing commas. Duplicate attributes and record keys are errors.

Annotations attach to the next diagram, node, or relationship in the same
block. They do not inherit into children. An orphan annotation or annotation
before a row, content item, or kind declaration is an error.

```text
@source(uri="src/api.rs", symbol="API::run",
  range={startLine: 8, startColumn: 1, endLine: 12, endColumn: 2})
@source(uri="src/lib.rs", symbol="exports::API")
@doc(text="Accepts a request.")
@meta(namespace="analysis", value={public: true, tags: [entry, "HTTP"], extra: null})
node "API::run(&self)" "API"
```

`@source` is repeatable. Ranges use 1-based lines and Unicode scalar columns,
with an exclusive end. `@doc` is a singleton. `@meta` accepts opaque data and
is unique per namespace on each target. Unknown annotations or arguments and
invalid ranges are errors. These code locations remain separate from the
original DSL byte spans exported for declarations.

Node bodies support `code`, `text`, and one `tag`, each followed by a quoted
string, plus nested graph objects. Content has no body. Labels containing
`[brackets]` remain literal; use an explicit `tag` item for a tag. Compact
shapes reject child diagrams.

## Select and export

A document can contain several graph diagrams. The first is the default;
selection is explicit in the CLI or API:

```sh
layup render document.layup --diagram services --theme auto
layup compile document.layup --diagram services -o services.json
layup lint document.layup --diagram services --json
layup fmt document.layup --check
```

```js
const scene = engine.compile(source, { diagram: 'services' });
const { output } = engine.render(source, { diagram: 'services', format: 'html' });
```

Markdown fences accept `diagram=services`; plugin options may set a default
`diagram`. All diagrams are validated before selection; choosing one does
not hide an invalid declaration in another. Formatting preserves token
spellings and comments. The revision-one linter currently reports the first
error, then layout warnings after a successful compilation.

Scene JSON retains its version-1 geometry contract and adds `document`,
`objectPath`, and `authoredId`. Node/edge IDs are opaque renderer IDs: use exact
`objectPath` segments and the selected `document.diagramId` for authored object
identity. `document` contains documentation and annotations indexed by render
ID. SVG and HTML carry the same information in `data-layup-document` metadata.

Try the [complete scoped example in the playground](/playground?example=language-v1).
The playground currently renders the first diagram; use the CLI or API to
select another diagram.

## What remains

This is an executable first checkpoint, not a migration of all Layup features.
Defaults, views, steps, slides, explicit legends, additional layout containers,
ports, and other diagram grammars remain on the design roadmap. Unsupported
constructs are rejected rather than ignored. Existing examples and these
features continue to work in unversioned syntax.

The proposed new document JSON input is also pending. Existing semantic graph
JSON and `compileModel`/`renderModel` retain their separate input contract.
See the [full design](https://github.com/hxyulin/layup/blob/main/docs/LANGUAGE-DESIGN.md)
for the target syntax and implementation checkpoints.
