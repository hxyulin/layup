//! Flat state-machine validation and layout. Transition labels are display
//! text; guards and actions are never evaluated by the renderer.
use crate::{
    Error, Warning,
    model::{Block, Diagram, Edge, Node},
    style::Shape,
};
use std::collections::{BTreeMap, BTreeSet};

fn nodes(blocks: &[Block]) -> Vec<&Node> {
    let mut result = Vec::new();
    let mut stack: Vec<_> = blocks.iter().rev().collect();
    while let Some(block) = stack.pop() {
        match block {
            Block::Node(n) => {
                if !n.is_container() {
                    result.push(n.as_ref());
                }
                stack.extend(n.children.iter().rev());
            }
            Block::Row(r) => stack.extend(r.cells.iter().rev().flatten()),
            Block::Section(s) => stack.extend(s.children.iter().rev()),
            Block::Tree { nodes, .. } => stack.extend(nodes.iter().rev()),
            Block::Flow { layers, .. } => {
                stack.extend(layers.iter().rev().flat_map(|l| l.iter().rev()))
            }
            _ => {}
        }
    }
    result
}

pub(crate) fn validate(d: &Diagram) -> Result<(), Error> {
    let nodes = nodes(&d.blocks);
    for n in &nodes {
        if !matches!(
            n.style.shape,
            Shape::State | Shape::Initial | Shape::Final | Shape::Choice
        ) {
            return Err(Error::at(
                n.line,
                "state-machine mode requires state, initial, final or choice nodes",
            ));
        }
    }
    let initial: Vec<_> = nodes
        .iter()
        .filter(|n| n.style.shape == Shape::Initial)
        .collect();
    if initial.len() > 1 {
        return Err(Error::at(
            initial[1].line,
            "a flat state machine requires exactly one initial marker",
        ));
    }
    if initial.is_empty() {
        return Err(Error::new(
            "a flat state machine requires exactly one initial marker",
        ));
    }
    let ids: BTreeMap<_, _> = nodes.iter().map(|n| (n.id.as_str(), *n)).collect();
    let mut outgoing = BTreeMap::<&str, usize>::new();
    for e in &d.edges {
        if !e.head_at_end || e.head_at_start {
            return Err(Error::at(
                e.line,
                "state-machine transitions must have one directed arrow",
            ));
        }
        let (Some(a), Some(b)) = (ids.get(e.from.as_str()), ids.get(e.to.as_str())) else {
            return Err(Error::at(
                e.line,
                "transitions must connect states or pseudostates, not structural containers",
            ));
        };
        if a.style.shape == Shape::Final {
            return Err(Error::at(
                e.line,
                "a final marker cannot have outgoing transitions",
            ));
        }
        if b.style.shape == Shape::Initial {
            return Err(Error::at(
                e.line,
                "an initial marker cannot have incoming transitions",
            ));
        }
        *outgoing.entry(e.from.as_str()).or_default() += 1;
    }
    for n in nodes {
        let count = outgoing.get(n.id.as_str()).copied().unwrap_or(0);
        if n.style.shape == Shape::Initial && count != 1 {
            return Err(Error::at(
                n.line,
                "an initial marker requires exactly one outgoing transition",
            ));
        }
        if n.style.shape == Shape::Choice && count < 2 {
            return Err(Error::at(
                n.line,
                "a choice requires at least two outgoing transitions",
            ));
        }
    }
    Ok(())
}

pub(crate) fn check(d: &Diagram, warnings: &mut Vec<Warning>) {
    let nodes = nodes(&d.blocks);
    let Some(initial) = nodes.iter().find(|n| n.style.shape == Shape::Initial) else {
        return;
    };
    let mut graph = BTreeMap::<&str, Vec<&str>>::new();
    for e in &d.edges {
        graph.entry(&e.from).or_default().push(&e.to);
    }
    let mut seen = BTreeSet::new();
    let mut stack = vec![initial.id.as_str()];
    while let Some(id) = stack.pop() {
        if seen.insert(id) {
            stack.extend(graph.get(id).into_iter().flatten().copied());
        }
    }
    for n in nodes {
        if !seen.contains(n.id.as_str()) {
            warnings.push(Warning {
                line: Some(n.line),
                msg: format!(
                    "state or pseudostate `{}` is unreachable from the initial marker",
                    n.id
                ),
            });
        }
    }
}

/// Remove DFS back edges from the placement graph, retaining every original
/// transition for routing. Traversal starts at the initial marker and uses
/// source node order, so edge statement reordering does not move states.
pub(crate) fn layout_edges(d: &Diagram) -> Vec<Edge> {
    let nodes = nodes(&d.blocks);
    let ids: BTreeMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut graph = vec![BTreeSet::new(); nodes.len()];
    for e in &d.edges {
        graph[ids[e.from.as_str()]].insert(ids[e.to.as_str()]);
    }
    let initial = nodes
        .iter()
        .position(|n| n.style.shape == Shape::Initial)
        .unwrap();
    let mut color = vec![0; nodes.len()];
    let mut back = BTreeSet::new();
    for root in std::iter::once(initial).chain(0..nodes.len()) {
        if color[root] != 0 {
            continue;
        }
        color[root] = 1;
        let mut stack = vec![(root, graph[root].iter())];
        while let Some((a, next)) = stack.last_mut() {
            if let Some(&b) = next.next() {
                if color[b] == 1 {
                    back.insert((*a, b));
                } else if color[b] == 0 {
                    color[b] = 1;
                    stack.push((b, graph[b].iter()));
                }
            } else {
                color[*a] = 2;
                stack.pop();
            }
        }
    }
    d.edges
        .iter()
        .filter(|e| !back.contains(&(ids[e.from.as_str()], ids[e.to.as_str()])))
        .cloned()
        .collect()
}
