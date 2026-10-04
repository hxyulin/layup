# The source language

The DSL has three main ingredients: items, nested blocks, and relationships.
Items end at a newline or semicolon. Braces group content, and `//` begins a
comment outside a quoted string.

## Items and blocks

```layup source
diagram "A service and its worker" layout=auto {
  node api "Public API" blue {
    code "POST /jobs"
    sub "Accepts a request and returns a job identifier."
  }
  group backend "Backend" {
    node worker "Worker" green
    node store "Storage" purple
    worker -> store "save"
  }
  api -> worker "dispatch"
}
```

The kind (`node`, `group`, `decision`, and so on) determines the visual shape.
Bare words provide IDs, tones, or flags. Quoted strings provide titles and
labels. Attributes use `key=value`, and a body contains nested items.

```text
KIND [id] ["Title"] [tone] [flags] [key=value ...] [{ content }]
```

Edges may appear before the nodes they reference: declarations resolve before
layout. IDs are unique throughout the document, including inside containers.

## Keep IDs separate from labels

Use `node api "Public API"` when another item, edge, view, or step references
the node. Its title can change without changing the ID. `id=api` is an
equivalent explicit attribute.

Ordinary graph nodes can omit an ID: Layup generates a unique slug from their
title. Shared models and sequence participants require explicit IDs. These
make view filtering and message references predictable.

Edges also accept IDs:

```text
api -> worker "dispatch" id=dispatch
```

Omitted edge IDs become `edge-N` in source order, skipping explicit
reservations. Use authored IDs for reveal steps and external integrations.

## Strings, numbers, and Unicode

Strings support `\"`, `\\`, `\n`, `\t`, and `\r`. You can also write literal
newlines inside a quoted string. Unknown escapes and duplicate attributes are
errors. Numbers can be decimal or exponential, such as `.5` and `1e3`; numeric
literals must be finite. Geometry adds constraints such as positive widths
and row weights.

Identifiers can contain Unicode letters, combining marks, digits, and common
path punctuation such as `-`, `.`, `:`, and `/`. Refer to the exact ID spelling.
Strings containing `//` or braces retain those characters as text.

Backticks inside a title or prose string make an inline code run:

```text
sub "The worker calls `process(job)` before saving the result."
```

## Content versus structure

`code`, `sub`, `text`, `role`, and `tag` describe a node's content. `row`,
`group`, `section`, and `band` organize nodes. Compact decision/process/terminal
nodes accept text content but need a surrounding container for child blocks.
Nested `state` nodes introduce composite state scopes.

Graph `note` is a subtitle. In a sequence, `note` is a chronological annotation;
use `subtitle` for text under the sequence title. Inside a presentation step,
`note` supplies speaker notes. The containing block determines its meaning.

## Diagram modes

| Mode | Layout and semantics |
| --- | --- |
| `mode=graph` (default) | Authored flow or `layout=auto`; arbitrary directed relationships |
| `mode=state-machine` | Automatic machine layout by default, scoped initial/final checks and reachability |
| `mode=sequence` | Authored participant columns and chronological messages; time moves down |

For every supported item and attribute, see [the DSL reference](/reference/dsl).
