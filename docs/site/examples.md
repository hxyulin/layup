# What can you explain with Layup?

The examples below are starting points. Each opens with editable source in
the playground; downloads include the source so you can keep a diagram in
version control.

| Example | What it demonstrates | Edit |
| --- | --- | --- |
| First diagram | Cards, code, automatic tones, typed relationships | [Open](/playground?example=hello) |
| Service architecture | Boundaries, groups, rows, and component relationships | [Open](/playground?example=architecture) |
| Support decision tree | Diamond questions, outcomes, and subtree lanes | [Open](/playground?example=decisions) |
| Job lifecycle | Initial/final states, cycles, and self-transitions | [Open](/playground?example=states) |
| Composite connection states | Nested scopes and cross-boundary transitions | [Open](/playground?example=composite) |
| Request sequence | Async work, returns, retries, notes, and alternatives | [Open](/playground?example=sequence) |
| CJK and Arabic sequence | Multiline messages and script shaping | [Open](/playground?example=international) |
| Right-to-left graph | Independent text and graph directions | [Open](/playground?example=rtl) |
| Slide-sized pipeline | Uniform fitting and readability checks | [Open](/playground?example=slides) |
| Request/retry presentation | Shared sequence model and four reveal steps | [Open](/playground?example=presentation) |
| Shared architecture views | Overview/detail filtering with stable identities | [Open](/playground?example=models) |

For automatically generated diagrams, see [generation from analysis](/guide/code-analysis).
Its canonical JSON example describes this repository's Cargo workspace with
an overview and per-package dependency/target views; it uses the APIs and CLI
rather than the DSL playground.

## Decision: where should a request go?

```layup
diagram main "Choose a support route" type=graph layout=auto {
  node urgent "Is the service unavailable?" style=decision palette=yellow
  node account "Is this an account issue?" style=decision palette=blue
  node incident "Page the on-call team" style=terminal palette=red
  node billing "Contact account support" style=terminal palette=green
  node help "Use the help center" style=terminal palette=purple
  ::urgent -> ::incident "Yes"
  ::urgent -> ::account "No"
  ::account -> ::billing "Yes"
  ::account -> ::help "No"
}
```

## Sequence: show an asynchronous handoff

```layup
diagram main "Accept work asynchronously" type=sequence {
  participant browser "Browser"
  participant api "API"
  participant worker "Worker"
  message connection-1 ::browser -> ::api "POST /jobs"
  message connection-2 ::api -> ::worker "Queue job" delivery=async
  message connection-3 ::api -> ::browser "202 Accepted" type=reply
}
```

## State: show a lifecycle

```layup
diagram main "A connection lifecycle" type=state-machine flow-direction=right {
  initial start
  state offline "Offline"
  state online "Online"
  final closed
  transition connection-1 ::start -> ::offline
  transition connection-2 ::offline -> ::online "connect"
  transition connection-3 ::online -> ::offline "disconnect"
  transition connection-4 ::online -> ::closed "close"
}
```

The [repository examples](https://github.com/hxyulin/layup/tree/main/examples)
contain the canonical fixtures used by the editor, tests, and visual galleries.
They describe fictional systems and are intended to teach the diagram language.
