# Sequence diagrams

Use `mode=sequence` to explain the order of calls, responses, retries, and
asynchronous work. Participant columns are authored; time always advances
downward.

```layup source
diagram "Create a job" mode=sequence {
  actor user "User"
  participant api "API"
  participant worker "Worker"
  user -> api "Submit" id=request
  api -> api "Validate"
  note "Authorize before dispatch." over=api
  api -> worker "Queue work" async id=enqueue
  api -> user "202 Accepted" return
}
```

Participants require explicit IDs and can have titles, tones, roles, links,
alignment, `mono` headers, and text direction. Actors use the same header with
an `actor` role. Participant bodies are not supported.

## Messages and notes

| Syntax | Meaning |
| --- | --- |
| `a -> b "Call"` | Message from A to B |
| `a -> b "Notify" async` | Open asynchronous arrowhead |
| `b -> a "Response" return` | Dashed return arrow |
| `a -> a "Validate"` | Self-call loop |
| `note "Text" over=a` | Note over one lifeline |
| `note "Text" from=a to=b` | Note spanning two participants |
| `note "Text"` | Note spanning the diagram |

Existing typed arrows, colors, labels, `labeled`, and IDs remain available.
Repeated messages are separate chronological events, so retry calls do not
trigger duplicate-transition lint warnings. `direction=left` reverses columns;
`right` is the default. Up/down flow and graph routing ports do not apply.

## Loops, options, and alternatives

```layup source
diagram "Poll for a result" mode=sequence {
  participant browser "Browser"
  participant api "API"
  loop "Until complete" {
    browser -> api "GET /jobs/:id"
    alt "Job status" {
      branch "Complete" { api -> browser "200 + result" return }
      branch "Pending" { api -> browser "200 + pending" return }
    }
  }
  opt "Notify the user" {
    note "Display the completed result." over=browser
  }
}
```

Fragments have a quoted label and nonempty event body. `alt` needs at least
two `branch` blocks. Nesting reserves room for headings and frames; deeply
nested fragments can need a wider canvas.

<Playground preset="sequence" />

Labels, notes, and roles wrap with measured fonts. CJK and Arabic/Hebrew work
with the same fallback policy as graphs. These fragments organize the
explanation; they do not repeat events during playback. Activation bars,
participant creation/destruction, parallel fragments, and timing axes are
future work.

For a talk, keep participant lifelines visible and reveal individual messages
with explicit `show-edge` steps. Try the
[request/retry presentation](/playground?example=presentation).
