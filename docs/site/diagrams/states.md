# State machines

Use a state machine to describe an object's lifecycle: a connection, job,
workflow, or operating mode. `mode=state-machine` enables automatic machine
placement and semantic checks.

```layup source
diagram "Job lifecycle" mode=state-machine direction=right {
  initial start
  state idle "Idle"
  state running "Running" { sub "entry / begin()" }
  final done
  start -> idle
  idle -> running "start / begin()"
  running -> running "tick / update()"
  running -> idle "reset"
  running -> done "finish [completed]"
}
```

`state` draws a rounded state. `initial` is the filled start dot; `final` is
the bullseye. `choice` is a diamond pseudostate, optionally labeled. Put
guards on its outgoing edges.

Transition labels can follow `event [guard] / action`. They are display text:
Layup does not parse an execution language, evaluate guards, or simulate a
machine.

## Validation

Each machine scope needs one initial marker with one outgoing transition.
Transitions cannot enter an initial marker or leave a final marker. Choices
need at least two outgoing transitions. Unreachable states produce warnings.
A continuously running machine can omit a final marker.

All IDs remain globally unique. Structural groups organize states without
creating a machine scope. These checks are opt-in; ordinary graph diagrams
retain their existing semantics.

## Composite states

Nested states create a scope with its own initial marker and a separate
header. Transitions can enter a composite, target a descendant directly, or
leave a nested state. Initial transitions remain in their own scope.

<Playground preset="composite" />

Try moving a transition to a nested state. The compiler checks initial scope
rules and reachability while retaining original endpoints for cross-boundary
routing. A named view must retain a valid initial scope after filtering.

Cycle placement follows declaration order from the initial marker. Return
edges and self-loops stay visible; rearranging transition statements does not
move the states. All four flow directions, authored rows, manual placement,
ports, and hints remain available.

History, parallel regions, fork/join, and executed transition traces are
future extensions. [Progressive reveal](/guide/presentations) can explain an
authored state path today.
