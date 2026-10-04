---
layout: home
hero:
  name: Layup
  text: Explain software with diagrams.
  tagline: Write a small text model. Shape the layout, show a decision, trace an interaction, or reveal a system one step at a time.
  image:
    light: /mark.svg
    dark: /mark-dark.svg
    alt: Two connected Layup nodes
  actions:
    - theme: brand
      text: Write your first diagram
      link: /guide/getting-started
    - theme: alt
      text: Open the live playground
      link: /playground
features:
  - title: Software architecture
    details: Cards with code and prose, nested containers, weighted rows, typed relationships, and measured automatic flow.
    link: /diagrams/architecture
  - title: Decisions and behavior
    details: Decision trees, flowcharts, scoped state machines, and chronological sequence diagrams.
    link: /examples
  - title: Present a story
    details: Fit a slide, reveal nodes and messages in authored steps, and reuse one model across several views.
    link: /guide/presentations
  - title: Portable output
    details: Self-contained SVG and HTML, iframe controls, or versioned scene JSON for your own tooling.
    link: /guide/formats
  - title: International text
    details: Independent layout and text directions, Arabic/Hebrew shaping, Unicode wrapping, and system or supplied CJK fonts.
    link: /guide/styling
  - title: One Rust engine
    details: The CLI, Rust API, JavaScript package, Markdown integration, and live editor share the same WASM-capable compiler.
    link: /reference/api
---

<div class="vp-doc" style="max-width:960px;margin:40px auto;padding:0 24px">

## From a request to an explanation

```layup
diagram "How a request reaches storage" layout=auto direction=right {
  node browser "Browser" { code "POST /jobs" }
  node api "API" { sub "Validate and authorize" }
  node worker "Worker" { code "process(job)" }
  node store "Object storage"
  browser -> api "submit"
  api -> worker "dispatch"
  worker -> store "persist"
}
```

Keep related implementation details together in containers. Use a decision
tree to explain a branching rule, a state machine for a lifecycle, or a
sequence to show the order of interactions. Each diagram can become a slide
or an interactive walkthrough.

[Browse rendered examples](/examples) · [Edit an example now](/playground)

::: info Layup v0.3.0
Install with `cargo install layup-cli --version 0.3.0 --locked`, or use
`cargo binstall layup-cli --version 0.3.0` for a prebuilt binary.
The browser playground compiles with the same Rust engine.
:::

</div>
