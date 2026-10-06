# Sequence diagrams

Use `type=sequence` to explain the order of calls, responses, retries, and
asynchronous work. Participant columns are authored; time always advances
downward.

```layup source
diagram main "Create a job" type=sequence {
  actor user "User"
  participant api "API"
  participant worker "Worker"
  message request ::user -> ::api "Submit"
  message connection-2 ::api -> ::api "Validate"
  note "Authorize before dispatch." over=::api
  message enqueue ::api -> ::worker "Queue work" delivery=async
  message connection-4 ::api -> ::user "202 Accepted" type=reply
}
```

Participants require explicit IDs and can have titles, tones, roles, links,
alignment, `mono` headers, and text direction. Actors use the same header with
an `actor` role. Participant bodies are not supported.

## Messages and notes

| Syntax | Meaning |
| --- | --- |
| `message call a -> b "Call"` | Message from A to B |
| `message notify a -> b "Notify" delivery=async` | Open asynchronous arrowhead |
| `message response b -> a "Response" type=reply` | Dashed return arrow |
| `message validate a -> a "Validate"` | Self-call loop |
| `note "Text" over=a` | Note over one lifeline |
| `note "Text" between=[a, b]` | Note spanning two participants |
| `note "Text"` | Note spanning the diagram |

Messages accept `style`, `palette`, paint properties and positional captions.\nUse `label=style` to request a style’s default caption.
Repeated messages are separate chronological events, so retry calls do not
trigger duplicate-transition lint warnings. `participant-direction=left` reverses columns;
`right` is the default. Up/down flow and graph routing ports do not apply.

## Loops, options, and alternatives

```layup source
diagram main "Poll for a result" type=sequence {
  participant browser "Browser"
  participant api "API"
  loop "Until complete" {
    message connection-1 ::browser -> ::api "GET /jobs/:id"
    alternatives "Job status" {
      branch "Complete" {
        message connection-2 ::api -> ::browser "200 + result" type=reply
      }
      branch "Pending" {
        message connection-3 ::api -> ::browser "200 + pending" type=reply
      }
    }
  }
  optional "Notify the user" {
    note "Display the completed result." over=::browser
  }
}
```

Fragments have a quoted label and nonempty event body. `alternatives` needs at least
two `branch` blocks. Nesting reserves room for headings and frames; deeply
nested fragments can need a wider canvas.

<Playground preset="sequence" />

Labels, notes, and roles wrap with measured fonts. CJK and Arabic/Hebrew work
with the same fallback policy as graphs. These fragments organize the
explanation; they do not repeat events during playback. Activation bars,
participant creation/destruction, parallel fragments, and timing axes are
future work.

For a talk, keep participant lifelines visible and reveal individual messages
with explicit `show connections=[…]` steps. Try the
[request/retry presentation](/playground?example=presentation).
