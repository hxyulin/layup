# Architecture diagrams

Use architecture diagrams to explain services, packages, interface boundaries,
or data flow. Cards can carry the code and prose that make a relationship
meaningful to a developer.

```layup source
diagram main "Job processing" type=graph layout=auto flow-direction=right {
  node client "Client" {
    code "POST /jobs"
  }
  group backend "Backend" {
    node api "Job API" {
      text "Validate requests"
    }
    node queue "Queue"
    node worker "Worker" {
      code "process(job)"
    }
    ::backend.api -> ::backend.queue "enqueue"
    ::backend.queue -> ::backend.worker "consume"
  }
  node store "Object storage"
  ::client -> ::backend.api "submit"
  ::backend.worker -> ::store "persist"
}
```

## Model the boundaries people need to understand

`node` is the easiest starting point. A `node` with nested children becomes a
hollow container. `group` gives an explicit neutral container; `package` and
`crate` add package/role typography. Templates selected by `style=trait|type|module|interface` are
compact interface boxes with semantic colors.

Use a container when it represents a boundary, not merely to force spacing.
Qualified references let edges cross container boundaries while retaining their
actual endpoints. Weighted rows are useful for side-by-side subsystems with
different amounts of detail.

## Give relationships a vocabulary

```text
edge implements implementation -> interface style=impl label=style
edge uses module -> dependency style=uses label=style
edge exports public_api -> implementation style=exports label=style
```

Edge styles provide consistent color, dashing, and optional default captions.
Graph diagrams build an automatic legend from used connection styles. Define a custom
`edge-style` when your domain needs a relationship such as `publishes` or `reads`.
Labels describe the interaction; they do not execute behavior.

## Try a service-layer example

<Playground preset="architecture" />

Try adding a service inside an existing group. Switch flow direction and
compare automatic placement with an authored row. For overview/detail
diagrams of the same system, use [shared models and views](/guide/models).

This is a software-diagram vocabulary, with no prescribed C4 or UML notation.
ER schemas and specialized swimlanes are possible future diagram types; the
current primitives can organize related services and interfaces today.
