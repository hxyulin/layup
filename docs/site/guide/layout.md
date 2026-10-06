# Layout and routing

Choose automatic layout when relationships explain the flow. Choose authored
rows and containers when the explanation depends on a specific grouping or
reading order. You can combine both in one diagram.

## Infer flow from edges

```layup source
diagram main "Request processing" type=graph layout=auto flow-direction=right {
  node gateway "Gateway"
  node auth "Authenticate"
  node validate "Validate"
  node store "Store"
  ::gateway -> ::auth
  ::gateway -> ::validate
  ::auth -> ::store
  ::validate -> ::store
}
```

`flow-direction=down|up|right|left` controls inferred graph flow. Peers occupy the
perpendicular axis; labels stay upright. Declaration order breaks ties.
Disconnected components retain authored region order. Graph direction is
independent from [text direction](/guide/styling#direction-and-unicode).

An automatic canvas starts at 900 units and
grows from measured text, ranks, tree extents, and container padding. `width=N`
fixes the canvas. State machines use specialized cycle placement; sequence
diagrams keep event order instead of graph ranks.

## Author rows and hierarchy

```layup source
diagram main "Read and write paths" type=graph {
  row gap=24 weights=[1, 2] {
    node reader "Reader" {
      code "fetch(key)"
    }
    node writer "Writer" {
      code "put(key, value)"
    }
  }
  node store "Store"
  ::reader -> ::store "read"
  ::writer -> ::store "write"
}
```

Weights split the available row width after gutters. `row weights=[1, 2]` gives the
second child twice the space. Sections, rows, dividers, gaps, and paragraphs
are layout boundaries: automatic placement does not reorder across them.
Nested containers apply the same layout rules to their own children.

`layout=manual` uses authored block flow. Use `width=N` to pin the canvas,
`defaults node palette=gray` to select a common palette, and
`legend visibility=hidden` to suppress a legend. These choices are independent.

## Add a relationship-free constraint

Inside automatic regions:

| Hint | Meaning |
| --- | --- |
| `after=api` | Put this node in a later flow rank |
| `same-rank=api` | Keep this node in the same flow rank |
| `beside=api` | Keep these nodes as peers |

`after=` follows the selected direction. Explicit rows retain authored cell
order even when automatic inference is enabled.

## Guide a route when needed

Edges choose orthogonal paths and attach to the actual node outline. Return
paths and self-loops receive outside space. Obstructed paths can try other
ports and search around nodes; dense graphs can still need author guidance.

```text
api -> store "query" source-side=bottom target-side=top
store -> api "response" route-side=right
```

`source-side=` and `target-side=` pin endpoint sides. `route-side=` reserves an outside side. These
options apply to graph and state-machine routing, not sequence lifelines.
Warnings identify overflow, obstructed routes, overlapping edges, and labels
without enough room. Shorten a caption, widen the canvas, adjust grouping,
or add a hint; use `layup check --strict` before sharing the result.

Try changing the direction and adding a row in the [playground](/playground).
