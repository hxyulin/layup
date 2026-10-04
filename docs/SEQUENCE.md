# Sequence diagrams

Use `mode=sequence` to show calls and responses in authored chronological
order. Participants occupy columns; time always advances downward. The
existing graph router is replaced by a lifeline layout, while text shaping,
fonts, themes, links, diagnostics and SVG/HTML output share the normal engine.

```text
diagram "Create a job" mode=sequence {
  actor user "User"
  participant browser "Browser"
  participant api "API"
  participant worker "Worker"

  user -> browser "Submit"
  browser -> api "POST /jobs" id=request
  api -> api "Validate"
  note "Authorize before dispatch." over=api
  api -> worker "Queue work" async id=enqueue
  api -> browser "202 Accepted" return
}
```

Participants and actors require explicit IDs and accept a title, tone, role,
link, alignment, `mono`/`sans` and text direction. `style participant ...` and
`style actor ...` customize header styles. Bracketed tags remain part of the
header text. Actors use a participant header with an `actor` role.
Participant bodies are not supported. Declare participants in the desired
column order; `direction=left` reverses columns and `direction=right` is the
default. Up/down graph directions do not apply to a sequence.

Messages use the existing edge syntax, including typed arrows, labels, tones,
`labeled`, `dashed` and explicit IDs. `async` adds an open arrowhead; `return`
adds a dashed line. A self-message forms a visible loop back to its lifeline.
Left arrows reverse the sender/receiver endpoints as in a graph. Routing
ports (`from=`, `to=`, `via=`) are rejected because messages follow lifelines.
Graph placement hints and explicit legends do not apply to sequence diagrams.

`note "Text" over=api` places a note over one participant.
`note "Text" from=browser to=api` spans two participants. Without a target,
the note spans the diagram. Notes, messages and headers wrap with the same
font measurements and Unicode line breaking as other diagrams. Hard newlines
are preserved; Arabic/Hebrew shaping, RTL text and user-supplied CJK fonts
remain available. Automatic width grows for long headers and messages;
`width=N` fixes a canvas and enables wrapping within the available space.

## Combined fragments

`loop` and `opt` have a quoted label and a nonempty event body. `alt` contains
at least two labeled `branch` blocks. Fragments can nest:

```text
diagram "Poll for a result" mode=sequence {
  participant browser "Browser"
  participant api "API"
  loop "Until complete" {
    browser -> api "GET /jobs/:id" id=poll
    alt "Job status" {
      branch "Complete" { api -> browser "200 + result" return }
      branch "Pending" { api -> browser "200 + pending" return }
    }
  }
  opt "Show a notification" {
    note "The browser displays the completed result." over=browser
  }
}
```

These blocks organize displayed events. They do not evaluate conditions,
repeat messages during playback or simulate runtime behavior. Activation
bars, participant creation/destruction, parallel fragments and timing axes
are possible extensions after concrete use cases.

Sequence diagrams support [slide viewports, presentation steps and named
views](PRESENTATION.md). Participant headers and lifelines reveal together;
explicit `show-edge` steps control individual messages. Notes and fragment
frames remain context throughout the presentation. Filtering a shared model
must preserve valid, nonempty fragments and complete alternatives.

See [the request sequence](../examples/sequence.layup),
[multilingual labels](../examples/sequence-international.layup), and
[the combined presentation model](../examples/presentation-model.layup).
Run `just sequence-test` to render a Chromium gallery and check text
containment, message ordering, arrow styles and playback.
