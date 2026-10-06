# The source language

A Layup document contains named diagrams. Shared rules cover comments, values,
annotations and scoped references; the diagram's `type` selects its body
grammar. The optional `layup 1` header asserts the supported revision.
The current checkout uses this replacement syntax throughout; older title-only
diagrams and bare presentation flags are rejected.

## Items and blocks

```layup source
diagram main "A service and its worker" type=graph layout=auto {
  node api "Public API" palette=blue {
    code "POST /jobs"
    text "Accepts a request and returns a job identifier."
  }
  group backend "Backend" {
    node worker "Worker" palette=green
    node store "Storage" palette=purple
    edge save worker -> store "save"
  }
  edge dispatch api -> backend.worker "dispatch"
}
```

Statements end at a newline or semicolon. Braces contain nested statements.
`//` comments continue to the end of the line; `/* … */` comments can appear
inline, span lines and nest. Comment markers inside strings remain text.

Graph object declarations have an explicit ID, optional display label and named
properties. A quoted ID can contain punctuation without changing reference
rules. Every graph object uses a fixed declaration such as `node` or `group`;
`style=service` selects a presentation style without introducing a keyword.

```text
node api "Public API" style=service palette=blue
edge dispatch api -> worker "dispatch" stroke-style=dashed
```

Edges can precede their endpoints. All supported definitions are validated
before diagram or view selection.

## Scoped identities and stable names

IDs are unique within their container. A group, package, crate or composite
state introduces a scope. A reference such as `backend.worker` resolves from
the current scope; `::backend.worker` resolves from the diagram root.
Sibling scopes may each declare `worker`. Rows and sequence fragments organize
content without creating identity scopes.

Labels can change without changing IDs. Objects require explicit IDs.
Named edges use `edge ID FROM -> TO`; sequence messages require `message ID`
and states use `transition ID`. Graph edges and state transitions can also
be anonymous. Name relationships that views, steps or external tools use.
Renderer IDs are opaque; use document metadata's `objectPath` and `authoredId`
to recover authored identities.

## Typed values, strings and Unicode

Properties and annotation arguments distinguish choices, strings, integers,
floats, booleans, null, lists, records and references. `blue` and `"blue"` retain
different types in inspection even when a style property accepts both.
Integers are exact within signed/unsigned 64-bit ranges; floats must be finite.
Geometry imposes further constraints, such as positive widths and row weights.

```text
row weights=[1, 2] gap=24 { … }
slide size={width: 1200, height: 1800}
@company.analysis(data={public: true, score: 1.5, tags: ["entry", "HTTP"]})
```

Strings support `\"`, `\\`, `\n`, `\t` and `\r`, as well as literal
newlines. Unknown escapes, repeated properties and duplicate record keys are
errors. Identifiers support Unicode letters and combining marks, digits,
underscores and internal hyphens. Dots separate reference segments; quote a
segment whose ID contains punctuation.

Backticks inside a prose string create an inline code run:

```text
text "The worker calls `process(job)` before saving the result."
```

## Content and diagram grammars

Graph nodes accept `text`, `code` and `tag` content. Containers hold child
objects; `row`, `section`, `band`, `divider` and `gap` organize layout.
Sequence diagrams use participants, named messages, chronological notes and
fragments. State diagrams use semantic states, markers and transitions, plus
`entry` and `exit` display actions.

| Type | Layout and semantics |
| --- | --- |
| `type=graph` | Authored blocks or `layout=auto` with directed relationships |
| `type=state-machine` | Scoped initial/final validation, reachability and machine placement |
| `type=sequence` | Authored participant columns; event order advances downward |

Annotations such as `@source`, `@doc` and `@meta` attach code locations and
evidence. Arbitrary annotations are preserved in order. Unavailable diagram
types are preserved and skipped with warnings. See
[shared language rules](/guide/language-v1) and
[the DSL reference](/reference/dsl) for the complete contract.
