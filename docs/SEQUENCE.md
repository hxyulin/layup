# Sequence diagrams

Use `type=sequence` to show calls and responses in authored chronological
order. Participants occupy columns; time always advances downward. The
existing graph router is replaced by a lifeline layout, while text shaping,
fonts, themes, links, diagnostics and SVG/HTML output share the normal engine.

```text
diagram main "Create a job" type=sequence {
  actor user "User"
  participant browser "Browser"
  participant api "API"
  participant worker "Worker"
  message connection-1 ::user -> ::browser "Submit"
  message request ::browser -> ::api "POST /jobs"
  message connection-3 ::api -> ::api "Validate"
  note "Authorize before dispatch." over=::api
  message enqueue ::api -> ::worker "Queue work" delivery=async
  message connection-5 ::api -> ::browser "202 Accepted" type=reply
}
```

Participants and actors require explicit IDs and accept labels, palettes,
roles, links, alignment, `font-family=mono|sans` and text direction.
`node-style` declarations customize compatible headers through `style=`.
Participant bodies are unsupported. Authored declaration order determines
columns; `participant-direction=left` reverses them.

Messages use `message ID FROM -> TO ["Caption"]`. `delivery=async` gives
a call an open arrowhead; `type=reply` gives a reply a dashed line.
Styles, palettes, paint properties and `label=style` provide presentation.
A self-message forms a loop back to its lifeline. Left arrows reverse
sender/receiver endpoints. Source/target/route-side hints are rejected
because messages follow lifelines.
Graph placement hints and explicit legends do not apply to sequence diagrams.

`note "Text" over=api` places a note over one participant.
`note "Text" between=[browser, api]` spans two participants. Without a target,
the note spans the diagram. Notes, messages and headers wrap with the same
font measurements and Unicode line breaking as other diagrams. Hard newlines
are preserved; Arabic/Hebrew shaping, RTL text and user-supplied CJK fonts
remain available. Automatic width grows for long headers and messages;
`width=N` fixes a canvas and enables wrapping within the available space.

## Combined fragments

`loop` and `optional` have a quoted label and a nonempty event body. `alternatives` contains
at least two labeled `branch` blocks. Fragments can nest:

```text
diagram main "Poll for a result" type=sequence {
  participant browser "Browser"
  participant api "API"
  loop "Until complete" {
    message poll ::browser -> ::api "GET /jobs/:id"
    alternatives "Job status" {
      branch "Complete" {
        message connection-2 ::api -> ::browser "200 + result" type=reply
      }
      branch "Pending" {
        message connection-3 ::api -> ::browser "200 + pending" type=reply
      }
    }
  }
  optional "Show a notification" {
    note "The browser displays the completed result." over=::browser
  }
}
```

These blocks organize displayed events. They do not evaluate conditions,
repeat messages during playback or simulate runtime behavior. Activation
bars, participant creation/destruction, parallel fragments and timing axes
are possible extensions after concrete use cases.

Sequence diagrams support [slide viewports, presentation steps and named
views](PRESENTATION.md). Participant headers and lifelines reveal together;
explicit `show connections=[…]` steps control individual messages. Notes and fragment
frames remain context throughout the presentation. Filtering a shared model
must preserve valid, nonempty fragments and complete alternatives.

See [the request sequence](../examples/sequence.layup),
[multilingual labels](../examples/sequence-international.layup), and
[the combined presentation model](../examples/presentation-model.layup).
Run `just sequence-test` to render a Chromium gallery and check text
containment, message ordering, arrow styles and playback.
