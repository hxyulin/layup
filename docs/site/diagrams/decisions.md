# Decisions and flowcharts

Use a decision tree to explain a branching rule. Use a flowchart when branches
merge, retry, or return to earlier processing.

```layup source
diagram main "Validate a request" type=graph layout=auto flow-direction=down {
  node receive "Receive request" style=process
  node valid "Are all requirements met?" style=decision palette=yellow
  node accept "Accept" style=terminal palette=green
  node reject "Request changes" style=terminal palette=orange
  ::receive -> ::valid
  ::valid -> ::accept "Yes"
  ::valid -> ::reject "No"
}
```

| Built-in style | Use |
| --- | --- |
| `decision` | Diamond question with measured, wrapped text |
| `process` | Rounded processing step |
| `terminal` | Capsule for a start, outcome, or endpoint |

These compact shapes accept title, code, and prose, but child nodes belong in
a surrounding group. Arrow endpoints meet sloping or rounded boundaries;
branch labels stay upright.

## Tree layout versus graph layout

An automatic region made entirely of decision/process/terminal nodes uses
subtree lanes when it is a single directed tree containing a decision. A
question stays centered over its descendants, and sibling declaration order
controls branch order. All four graph directions work.

Merges, cycles, explicit placement hints, and mixed card/flowchart regions use
general graph placement. They remain valid diagrams. Put independent trees
in separate groups or sections if their explanations should stay separate.

<Playground preset="decisions" />

Try adding another outcome to a question. Then connect two outcomes to a
shared process to see graph layout take over. If a long branch caption cannot
fit, shorten it, increase `width=`, or guide placement; strict checks make
that tradeoff visible.
