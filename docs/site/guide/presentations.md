# Slides and progressive reveal

A diagram for a talk needs readable sizing and a controlled introduction.
Layup supplies both through grouped viewport settings and authored presentation
steps. The same features apply to graphs, decisions, states, and sequences.

## Fit a slide

```layup source
diagram main "A request through the system" type=graph layout=auto flow-direction=right width=480 {
  slide size=wide
  node client "Client" style=process palette=blue
  node api "API" style=process palette=green
  node store "Storage" style=process palette=purple
  ::client -> ::api "Request"
  ::api -> ::store "Query"
}
```

| Attribute | Behavior |
| --- | --- |
| `slide size=wide` or `slide size="16:9"` | 1920 × 1080 viewport |
| `slide size=standard` or `slide size="4:3"` | 1440 × 1080 viewport |
| `slide size={width: 1200, height: 1800}` | Explicit width and height |
| `padding=N` | Interior padding, default 48 |
| `min-font-size=N` | Minimum post-fit font size, default 18 |

The size, padding and readability properties belong to a `slide` statement at
the diagram or view level. The finished scene scales uniformly and centers in the viewport. Layout,
routes, measured text, and aspect ratio remain intact. `width=N` controls the
original layout canvas separately. For a short pipeline, a compact source
width such as `480` avoids fitting a mostly empty 900-unit default canvas.
The slide keeps its 16:9 viewport; change source width to balance the content
inside it. Readability warnings include every text
item, including captions, roles, and chips, after fitting. `--strict` fails
on those warnings. The threshold is in slide units before display resizing.

## Reveal the explanation in steps

```layup source
diagram main "Dispatch a job" type=graph layout=auto flow-direction=right {
  node api "API"
  node worker "Worker"
  edge dispatch ::api -> ::worker "dispatch"
  step overview "Meet the services" {
    show objects=[::api, ::worker]
    highlight objects=[::api]
    speaker-note "The API accepts work for the worker."
  }
  step request "Dispatch work" {
    show connections=[dispatch]
    highlight connections=[dispatch]
  }
}
```

`show objects=[…]` and `show connections=[…]` accumulate. Highlights apply only to the current step
and require visible targets. Showing a container reveals its subtree;
showing a descendant keeps ancestor frames visible.

Edges ordinarily appear when both endpoints are visible. If an edge is
assigned to `show connections=[…]` anywhere, it waits for that explicit reveal and for
both endpoints. Give important edges IDs to keep step references stable.

Static output starts with the complete diagram. **Present** starts the plan;
Previous, Next, and Show all control it. Arrow Left/Right, Page Up/Down,
Space, Home, and End navigate during presentation. Inline docs also support
fullscreen. Notes are plain text; reduced-motion preferences are respected.

## A combined request/retry walkthrough

<Playground preset="presentation" />

Choose the walkthrough view, then select **Present** under the diagram.
Its four steps introduce the request, asynchronous dispatch, polling, and
result. Expand it to see the slide at a useful presentation size.

Sequence notes and fragment frames remain context during reveal. Presentation
steps do not execute state machines or automatically time a trace.

## Drive an iframe

Use `--html --embed` for a page without the toolbar. `?step=request` selects an
initial step. A host can send:

```js
frame.contentWindow.postMessage({ layup: 'step', id: 'request' }, '*');
frame.contentWindow.postMessage({ layup: 'step', action: 'next' }, '*');
frame.contentWindow.postMessage({ layup: 'step', action: 'all' }, '*');
```

The viewer reports `{ layup: 'step', index, id, count, title, note }`.
Indices are zero-based; the complete view has index `-1` and ID `null`.
Existing focus/select/theme/fit messages remain available. Verify the origin
and source of incoming messages in your host.
