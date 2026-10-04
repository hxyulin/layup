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

## Decision: where should a request go?

```layup
diagram "Choose a support route" layout=auto {
  decision urgent "Is the service unavailable?" yellow
  decision account "Is this an account issue?" blue
  terminal incident "Page the on-call team" red
  terminal billing "Contact account support" green
  terminal help "Use the help center" purple
  urgent -> incident "Yes"
  urgent -> account "No"
  account -> billing "Yes"
  account -> help "No"
}
```

## Sequence: show an asynchronous handoff

```layup
diagram "Accept work asynchronously" mode=sequence {
  participant browser "Browser"
  participant api "API"
  participant worker "Worker"
  browser -> api "POST /jobs"
  api -> worker "Queue job" async
  api -> browser "202 Accepted" return
}
```

## State: show a lifecycle

```layup
diagram "A connection lifecycle" mode=state-machine direction=right {
  initial start
  state offline "Offline"
  state online "Online"
  final closed
  start -> offline
  offline -> online "connect"
  online -> offline "disconnect"
  online -> closed "close"
}
```

The [repository examples](https://github.com/hxyulin/layup/tree/main/examples)
contain the canonical fixtures used by the editor, tests, and visual galleries.
They describe fictional systems and are intended to teach the diagram language.
