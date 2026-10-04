# Slides and progressive reveal

A diagram for a talk needs readable sizing and a controlled introduction.
Layup supplies both through viewport attributes and authored presentation
steps. The same features apply to graphs, decisions, states, and sequences.

## Fit a slide

```layup source
diagram "A request through the system" layout=auto direction=right slide=wide {
  process client "Client" blue
  process api "API" green
  process store "Storage" purple
  client -> api "Request"
  api -> store "Query"
}
```

| Attribute | Behavior |
| --- | --- |
| `slide=wide` or `slide="16:9"` | 1920 × 1080 viewport |
| `slide=standard` or `slide="4:3"` | 1440 × 1080 viewport |
| `slide="1200:1800"` | Explicit width and height |
| `slide-padding=N` | Interior padding, default 48 |
| `min-font-size=N` | Minimum post-fit font size, default 18 |

The finished scene scales uniformly and centers in the viewport. Layout,
routes, measured text, and aspect ratio remain intact. `width=N` controls the
original layout canvas separately. Readability warnings include every text
item, including captions, roles, and chips, after fitting. `--strict` fails
on those warnings. The threshold is in slide units before display resizing.

## Reveal the explanation in steps

```layup source
diagram "Dispatch a job" layout=auto direction=right {
  node api "API"
  node worker "Worker"
  api -> worker "dispatch" id=dispatch
  step overview "Meet the services" {
    show api worker
    highlight api
    note "The API accepts work for the worker."
  }
  step request "Dispatch work" {
    show-edge dispatch
    highlight-edge dispatch
  }
}
```

`show` and `show-edge` accumulate. Highlights apply only to the current step
and require visible targets. Showing a container reveals its subtree;
showing a descendant keeps ancestor frames visible.

Edges ordinarily appear when both endpoints are visible. If an edge is
assigned to `show-edge` anywhere, it waits for that explicit reveal and for
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
