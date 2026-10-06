# Presentation diagrams and scene data

Layup can fit a completed diagram into a slide viewport, reveal it in authored
steps, and select named views from a shared model. These features work with
architecture graphs, decision trees, state machines and
[sequence diagrams](SEQUENCE.md), using the same Rust engine in the CLI and
JavaScript/WASM package.

[presentation-model.layup](../examples/presentation-model.layup) combines a
request/retry sequence, two named views, a widescreen viewport and four reveal
steps:

```sh
layup render examples/presentation-model.layup --view overview
layup render examples/presentation-model.layup --view walkthrough --html
layup compile examples/presentation-model.layup --view walkthrough -o scene.json
```

## Slide sizing

Add `slide size=wide` to a diagram or model to fit the finished scene into a
1920 × 1080 viewport. `slide size=standard` uses 1440 × 1080. Quoted `"16:9"` and
`"4:3"` are aliases; `slide size={width: 1200, height: 1800}` sets explicit dimensions. Units are
SVG user units, independent of the size at which a browser displays the SVG.

```text
diagram main "Pipeline" type=graph layout=auto flow-direction=right {
  slide size=wide
  node api "API"
  node worker "Worker"
  ::api -> ::worker "dispatch"
}
```

The complete scene, including titles and routed edges, scales uniformly and
centers inside the viewport. Padding defaults to 48 units and can be changed
with `padding=N`. The engine preserves the original layout, font
measurements, edge routes and aspect ratio; unused space forms a margin.
Use `width=N` to control the layout canvas separately from slide dimensions.

`min-font-size=N` checks readability after fitting, with a default of 18 slide
units. The check includes nested message captions, code, role text and label
chips. A warning reports the smallest resulting size and how many text items
fall below the threshold. `--strict` treats that warning as an error. This
threshold refers to the SVG slide, before a projector or browser resizes it.
For dense diagrams, use smaller views or simplify the content.

## Progressive reveal

Direct children of a diagram can define named steps:

```text
diagram main "Dispatch" type=graph layout=auto {
  node api "API"
  node worker "Worker"
  edge dispatch ::api -> ::worker "dispatch"
  step overview "Meet the services" {
    show objects=[::api, ::worker]
    highlight objects=[::api]
    speaker-note "The API accepts work for the worker."
  }
  step request "Dispatch a job" {
    show connections=[dispatch]
    highlight connections=[dispatch]
  }
}
```

`show` and `show connections=[…]` accumulate across steps. `highlight` and
`highlight connections=[…]` apply only to the current step and require visible targets.
Showing a container includes its subtree; showing a descendant retains its
ancestor frames. Edges normally appear when both endpoints are visible. Once
an edge is named by `show connections=[…]` anywhere in the plan, it appears only after
its explicit reveal and after both endpoints are visible. This lets sequence
messages appear one at a time while participant lifelines remain visible.

Named connections use `edge ID`, `message ID` or `transition ID` in their
respective grammars. Opaque renderer IDs are assigned before view selection.
Use authored names for steps and `authoredId` metadata for integrations.

Static SVG and HTML initially display the complete diagram. The HTML viewer's
**Present** button starts the plan; **Previous**, **Next** and **All** control
it. Arrow keys, Page Up/Down and Space navigate steps during presentation;
Home selects the first step, End the last. Inline Markdown diagrams gain the
same controls when `@hxyulin/layup/client` is loaded, including fullscreen.
Step titles, status and plain-text notes appear alongside the controls.
Hidden node links leave keyboard navigation until revealed. Motion respects
the viewer's reduced-motion preference.

Embed HTML accepts `?step=NAME` for an initial step. An iframe host can send:

```js
frame.contentWindow.postMessage({ layup: 'step', id: 'request' }, '*');
frame.contentWindow.postMessage({ layup: 'step', action: 'next' }, '*');
frame.contentWindow.postMessage({ layup: 'step', action: 'all' }, '*');
```

The viewer reports `{ layup: 'step', index, id, count, title, note }` when the
step changes. `index` is zero-based; the complete view has no selected step.
The host should verify the message's origin and source before using it.
Existing focus/select/pan/zoom controls remain available. Reveal is an
authored presentation plan; it does not execute state-machine guards or
automatically time a trace.

## Shared models and named views

A model declares nodes and relationships once. Each named view includes the
nodes it needs and can override diagram attributes:

```text
diagram main "Job system" type=graph layout=auto {
  slide size=wide
  node client "Client"
  node api "API"
  group backend "Backend" {
    node worker "Worker"
    node store "Store"
  }
  edge request ::client -> ::api
  edge dispatch ::api -> ::backend.worker
  edge save ::backend.worker -> ::backend.store
  view overview "System overview" flow-direction=right {
    include ::client ::api ::backend
  }
  view storage "Storage detail" flow-direction=down {
    include ::backend.worker ::backend.store
  }
}
```

Shared nodes require explicit IDs. Including a container selects its subtree;
including a descendant keeps its ancestor wrappers while excluding siblings.
Edges survive when both endpoints survive. Authored rows retain the order and
weights of included children. Declarations, source locations and node/edge
IDs remain shared. Model attributes are inherited; view attributes override
them. The first authored view is the default; select another using `--view`,
Rust `CompileOptions::view`, or JavaScript `{ view: 'storage' }`.

Views can define their own steps; a view's plan replaces the model's shared
steps when supplied. Shared steps must still refer to nodes and edges in the
selected view. All view names, include references and shared edge references
are checked before filtering. Other semantic and layout checks apply to the
selected result. Check each view in CI to validate every rendered variant.
Unused-style/arrow lint rules consider uses across the shared source, so a
declaration used by another view remains useful.
For state-machine views, the selected states must retain a valid initial
scope. Sequence fragments must retain nonempty event bodies and complete
alternative branches. Notes tied to excluded participants are omitted.

The Markdown plugin accepts a default `view` option and a fence override:

````markdown
```layup view=storage
diagram main "Job system" type=graph {
  node worker "Worker"
  node store "Store"
  ::worker -> ::store
  view overview {
    include ::worker ::store
  }
  view storage {
    include ::store
  }
}
```
````

## Scene export

`layup compile input.layup` writes scene JSON to stdout. `-o scene.json` saves
it to a file. Font options, view selection and `--strict` use the same
compilation as rendering. Warnings go to stderr; strict failure suppresses
output and preserves an existing output file.

```js
import { load } from '@hxyulin/layup';
const layup = await load();
const scene = layup.compile(source, { view: 'walkthrough', fonts: [] });
console.log(scene.nodes, scene.edges, scene.presentation.steps);
```

```rust
let fonts = layup::text::Fonts::default();
let options = layup::CompileOptions { view: Some("walkthrough".into()), ..Default::default() };
let compiled = layup::compile_with_options(source, &options, &fonts)?;
let json = layup::scene::export(&compiled)?;
# Ok::<(), layup::Error>(())
```

The contract has `version: 1`, `units: 'svg-user-units'` and
`coordinateSystem: 'scene'`. Coordinates retain their original floating-point
precision. Consumers should check the version before interpreting the data.

| Field | Contents |
| --- | --- |
| `width`, `height`, `margin`, `contentLeft`, `contentRight` | Original layout canvas and bounds |
| `viewport` | Output dimensions and optional slide transform |
| `nodes` | Stable IDs, kinds, parent IDs, rectangles, outlines, tones, links and source spans |
| `edges` | IDs, endpoints, path points, styles, captions and source spans |
| `items` | Ordered drawing operations, including text runs and nested groups |
| `keepout` | Reserved layout rectangles |
| `views`, `selectedView` | Authored view metadata and current selection |
| `presentation` | Versioned cumulative visibility/highlight plan |
| `sequence` | Participants, ordered message IDs and note/fragment geometry, or `null` |
| `fonts` | Font family identifiers and fallback policy |
| `diagnostics` | Compilation warnings with available source locations |

For a slide, apply `x' = offsetX + scale * x` and
`y' = offsetY + scale * y` from `viewport.slide` once. Sizes and stroke widths
scale by the same factor. Without a slide, coordinates are already viewport
coordinates. Text positions use SVG baselines and physical `start`, `middle`
or `end` anchors; direction and styled runs remain explicit. The TypeScript
definitions describe every drawing variant.

Source spans use UTF-8 byte offsets and one-based Unicode scalar columns.
JSON does not include font bytes, base64 data or font file paths. An alternate
renderer supplies the named fonts; system CJK still uses estimated metrics
unless font bytes were supplied at compilation. SVG remains the portable
format with embedded fonts. PNG/PDF renderers and executed transition traces
can build on this scene contract later.
