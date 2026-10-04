//! Hierarchical state-machine validation and layout. Transition labels are display
//! text; guards and actions are never evaluated by the renderer.
use crate::{
    Error, Warning,
    model::{Block, Diagram, Edge, Node},
    style::Shape,
};
use std::collections::{BTreeMap, BTreeSet};

struct Member<'a> {
    node: &'a Node,
    scope: Option<&'a str>,
}

fn members(blocks: &[Block]) -> Vec<Member<'_>> {
    let mut result = Vec::new();
    let mut stack: Vec<_> = blocks.iter().rev().map(|b| (b, None)).collect();
    while let Some((block, scope)) = stack.pop() {
        match block {
            Block::Node(n) => {
                if !n.is_container() || n.style.shape == Shape::State {
                    result.push(Member {
                        node: n.as_ref(),
                        scope,
                    });
                }
                let child_scope = if n.is_composite() {
                    Some(n.id.as_str())
                } else {
                    scope
                };
                stack.extend(n.children.iter().rev().map(|b| (b, child_scope)));
            }
            Block::Row(r) => stack.extend(r.cells.iter().rev().flatten().map(|b| (b, scope))),
            Block::Section(s) => stack.extend(s.children.iter().rev().map(|b| (b, scope))),
            Block::Tree { nodes, .. } => stack.extend(nodes.iter().rev().map(|b| (b, scope))),
            Block::Flow { layers, .. } => stack.extend(
                layers
                    .iter()
                    .rev()
                    .flat_map(|l| l.iter().rev())
                    .map(|b| (b, scope)),
            ),
            _ => {}
        }
    }
    result
}

pub(crate) fn validate(d: &Diagram) -> Result<(), Error> {
    let members = members(&d.blocks);
    let mut scopes = BTreeMap::from([(None, (None, Vec::new()))]);
    for m in &members {
        let n = m.node;
        if !matches!(
            n.style.shape,
            Shape::State | Shape::Initial | Shape::Final | Shape::Choice
        ) {
            return Err(Error::at(
                n.line,
                "state-machine mode requires state, initial, final or choice nodes",
            ));
        }
        if n.is_composite() {
            scopes
                .entry(Some(n.id.as_str()))
                .or_insert((Some(n.line), Vec::new()));
        }
        if n.style.shape == Shape::Initial {
            scopes
                .entry(m.scope)
                .or_insert((None, Vec::new()))
                .1
                .push(n);
        }
    }
    for (scope, (line, initial)) in scopes {
        if initial.len() != 1 {
            let label = scope.map_or("the root scope".to_string(), |id| {
                format!("composite state `{id}`")
            });
            let message = format!("{label} requires exactly one initial marker");
            return Err(match initial.get(1).map(|n| n.line).or(line) {
                Some(line) => Error::at(line, message),
                None => Error::new(message),
            });
        }
    }
    let ids: BTreeMap<_, _> = members.iter().map(|m| (m.node.id.as_str(), m)).collect();
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
        if a.node.style.shape == Shape::Final {
            return Err(Error::at(
                e.line,
                "a final marker cannot have outgoing transitions",
            ));
        }
        if b.node.style.shape == Shape::Initial {
            return Err(Error::at(
                e.line,
                "an initial marker cannot have incoming transitions",
            ));
        }
        if a.node.style.shape == Shape::Initial && a.scope != b.scope {
            return Err(Error::at(
                e.line,
                "an initial transition must target a state or pseudostate in its own scope",
            ));
        }
        let mut scope = a.scope;
        while scope != b.scope {
            let Some(parent) = scope else {
                break;
            };
            scope = ids[parent].scope;
        }
        if b.node.style.shape == Shape::Final && scope != b.scope {
            return Err(Error::at(
                e.line,
                "a final marker must be entered from its own scope or a descendant scope",
            ));
        }
        *outgoing.entry(e.from.as_str()).or_default() += 1;
    }
    for m in members {
        let n = m.node;
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
    let members = members(&d.blocks);
    let Some(initial) = members
        .iter()
        .find(|m| m.scope.is_none() && m.node.style.shape == Shape::Initial)
    else {
        return;
    };
    let ids: BTreeMap<_, _> = members.iter().map(|m| (m.node.id.as_str(), m)).collect();
    let mut graph = BTreeMap::<&str, Vec<&str>>::new();
    for e in &d.edges {
        graph.entry(&e.from).or_default().push(&e.to);
    }
    // Entering a composite activates its initial marker. Direct entry into
    // a descendant also makes its ancestors structurally reachable.
    for m in &members {
        if m.node.style.shape == Shape::Initial
            && let Some(scope) = m.scope
        {
            graph.entry(scope).or_default().push(&m.node.id);
        }
    }
    let mut seen = BTreeSet::new();
    let mut expanded = BTreeSet::new();
    let mut stack = vec![initial.node.id.as_str()];
    while let Some(id) = stack.pop() {
        if expanded.insert(id) {
            seen.insert(id);
            stack.extend(graph.get(id).into_iter().flatten().copied());
            let mut scope = ids[id].scope;
            while let Some(parent) = scope {
                // Ancestors are active without re-entering their default
                // initial path when a transition targets a descendant.
                seen.insert(parent);
                stack.extend(
                    d.edges
                        .iter()
                        .filter(|e| e.from == parent)
                        .map(|e| e.to.as_str()),
                );
                scope = ids[parent].scope;
            }
        }
    }
    for m in members {
        if !seen.contains(m.node.id.as_str()) {
            warnings.push(Warning {
                line: Some(m.node.line),
                msg: format!(
                    "state or pseudostate `{}` is unreachable from the initial marker",
                    m.node.id
                ),
            });
        }
    }
}

/// Project transitions onto direct members of each scope, then remove DFS
/// back edges for arrangement only. The diagram retains its original edges.
pub(crate) fn layout_edges(d: &Diagram) -> Vec<Edge> {
    let members = members(&d.blocks);
    let ids: BTreeMap<_, _> = members
        .iter()
        .enumerate()
        .map(|(i, m)| (m.node.id.as_str(), i))
        .collect();
    let scopes: BTreeSet<_> = members.iter().map(|m| m.scope).collect();
    let mut result = Vec::new();
    for scope in scopes {
        let peers: Vec<_> = members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.scope == scope)
            .map(|(i, _)| i)
            .collect();
        let owner = |mut i: usize| -> Option<usize> {
            loop {
                if members[i].scope == scope {
                    return Some(i);
                }
                i = *ids.get(members[i].scope?)?;
            }
        };
        let mut graph = vec![BTreeSet::new(); members.len()];
        let mut projected = Vec::new();
        for e in &d.edges {
            if let (Some(a), Some(b)) = (owner(ids[e.from.as_str()]), owner(ids[e.to.as_str()]))
                && a != b
            {
                graph[a].insert(b);
                let mut edge = e.clone();
                edge.from = members[a].node.id.clone();
                edge.to = members[b].node.id.clone();
                projected.push((a, b, edge));
            }
        }
        let initial = peers
            .iter()
            .copied()
            .find(|&i| members[i].node.style.shape == Shape::Initial)
            .unwrap();
        let mut color = vec![0; members.len()];
        let mut back = BTreeSet::new();
        for root in std::iter::once(initial).chain(peers) {
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
        result.extend(
            projected
                .into_iter()
                .filter(|(a, b, _)| !back.contains(&(*a, *b)))
                .map(|(_, _, e)| e),
        );
    }
    result
}
