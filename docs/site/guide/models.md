# Shared models and named views

Define a system once, then choose how much detail to show. Named views keep
IDs, relationships, source locations, and style declarations shared while
allowing different layout and presentation choices.

```layup source
model "Job system" layout=auto {
  node client "Client"
  node api "API"
  group backend "Backend" {
    node worker "Worker"
    node store "Store"
  }
  client -> api id=request
  api -> worker id=dispatch
  worker -> store id=save
  view overview "System overview" direction=right {
    include client api backend
  }
  view storage "Storage detail" direction=down {
    include worker store
  }
}
```

## Selection rules

- Shared nodes require explicit IDs. Each view needs a unique name and an
  `include` list.
- A container include selects its subtree. A descendant include preserves
  ancestor frames without including unselected siblings.
- An edge survives when both endpoints survive. Rows preserve child order
  and the weights of selected cells.
- Model attributes are inherited, then view attributes override them. The
  view's title becomes the diagram title.
- The first authored view is the default. Explicit selection uses `--view`
  or an API option.

Generated edge IDs are assigned before filtering the complete model, so a
relationship retains its ID between views. Prefer explicit IDs for steps and
external integrations that should survive source edits.

## Try an overview and a detail view

<Playground preset="models" />

Switch the **View** selector. Add an attribute such as `direction=left` to
one view, or change a shared node title and inspect both results.

```sh
layup render model.layup --view storage --html
layup check model.layup --view overview --strict
layup check model.layup --view storage --strict
```

```js
const scene = engine.compile(source, { view: 'storage' });
const rendered = engine.render(source, { view: 'storage' });
```

## Per-view steps and validation

A view can declare `step` blocks; when present, its plan replaces the shared
model's plan. Shared steps must still reference targets in the selected
result. Models can inherit slide attributes and let individual views override
the viewport or readability threshold.

All include references, duplicate identities, and shared edge references are
validated before filtering. Other semantic and layout checks apply to the
selected result. Unused declaration linting considers uses across the shared
source. Validate each view in CI to cover all rendered variants.

State-machine views must retain valid initial scopes. Sequence notes tied to
excluded participants disappear, but filtering must preserve nonempty
fragments and complete alternative branches; empty branches are errors.
