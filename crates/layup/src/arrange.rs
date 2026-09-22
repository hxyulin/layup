//! Deterministic graph layering within authored block boundaries.
use std::collections::{BTreeMap, BTreeSet};

use crate::Error;
use crate::model::{Block, Edge, HintKind, Row};

/// Only consecutive sibling nodes are rearranged. Explicit rows, sections,
/// dividers and prose are boundaries; their contents are handled recursively.
pub(crate) fn arrange(blocks: &mut Vec<Block>, edges: &[Edge]) -> Result<(), Error> {
    for block in blocks.iter_mut() {
        descend(block, edges)?;
    }
    let mut out = Vec::new();
    let mut run = Vec::new();
    for block in std::mem::take(blocks) {
        if matches!(block, Block::Node(_)) {
            run.push(block);
        } else {
            out.extend(layer(std::mem::take(&mut run), edges)?);
            out.push(block);
        }
    }
    out.extend(layer(run, edges)?);
    *blocks = out;
    Ok(())
}

fn descend(block: &mut Block, edges: &[Edge]) -> Result<(), Error> {
    match block {
        Block::Node(n) => arrange(&mut n.children, edges)?,
        Block::Section(s) => arrange(&mut s.children, edges)?,
        Block::Row(r) => {
            for cell in r.cells.iter_mut().flatten() {
                if let Block::Node(n) = cell
                    && !n.hints.is_empty()
                {
                    return Err(Error::at(
                        n.line,
                        "layout hints cannot reposition explicit row cells",
                    ));
                }
                descend(cell, edges)?;
            }
        }
        _ => {}
    }
    Ok(())
}

// Project descendant endpoints onto the immediate sibling that contains them.
fn owners(block: &Block, owner: usize, ids: &mut BTreeMap<String, usize>) {
    match block {
        Block::Node(n) => {
            ids.insert(n.id.clone(), owner);
            for child in &n.children {
                owners(child, owner, ids);
            }
        }
        Block::Row(r) => {
            for child in r.cells.iter().flatten() {
                owners(child, owner, ids);
            }
        }
        Block::Section(s) => {
            for child in &s.children {
                owners(child, owner, ids);
            }
        }
        _ => {}
    }
}

fn layer(blocks: Vec<Block>, edges: &[Edge]) -> Result<Vec<Block>, Error> {
    let n = blocks.len();
    if n == 0 {
        return Ok(blocks);
    }
    let mut ids = BTreeMap::new();
    for (i, block) in blocks.iter().enumerate() {
        owners(block, i, &mut ids);
    }
    let mut equal = vec![BTreeSet::new(); n];
    let direct: BTreeMap<_, _> = blocks
        .iter()
        .enumerate()
        .filter_map(|(i, b)| {
            if let Block::Node(node) = b {
                Some((node.id.as_str(), i))
            } else {
                None
            }
        })
        .collect();
    let mut below = Vec::new();
    let mut next = vec![None; n];
    let mut prev = vec![None; n];
    for (i, block) in blocks.iter().enumerate() {
        let Block::Node(node) = block else {
            unreachable!()
        };
        for hint in &node.hints {
            let &j = direct.get(hint.target.as_str()).ok_or_else(|| {
                Error::at(
                    node.line,
                    format!(
                        "hint target `{}` must be a peer in the same automatic-layout region",
                        hint.target
                    ),
                )
            })?;
            if i == j {
                return Err(Error::at(
                    node.line,
                    "a layout hint cannot reference its own node",
                ));
            }
            if hint.kind == HintKind::Below {
                below.push((j, i, node.line));
            } else {
                equal[i].insert(j);
                equal[j].insert(i);
                if hint.kind == HintKind::Beside {
                    if next[j].is_some_and(|v| v != i) || prev[i].is_some_and(|v| v != j) {
                        return Err(Error::at(
                            node.line,
                            "conflicting beside hints: each node can have only one immediate neighbor on each side",
                        ));
                    }
                    next[j] = Some(i);
                    prev[i] = Some(j);
                }
            }
        }
    }
    let equality = components(&equal);
    let mut weak = equal.clone();
    let mut graph = equal;
    for edge in edges {
        if let (Some(&a), Some(&b)) = (ids.get(&edge.from), ids.get(&edge.to))
            && a != b
        {
            weak[a].insert(b);
            weak[b].insert(a);
            // Undirected and bidirectional edges connect a region but do not rank it.
            if edge.head_at_end && !edge.head_at_start {
                graph[a].insert(b);
            }
        }
    }
    for &(a, b, _) in &below {
        graph[a].insert(b);
        weak[a].insert(b);
        weak[b].insert(a);
    }
    let islands = components(&weak);
    let mut island_first = vec![n; n];
    for (i, &island) in islands.iter().enumerate() {
        island_first[island] = island_first[island].min(i);
    }
    let component = components(&graph);
    for &(a, b, line) in &below {
        if component[a] == component[b] {
            return Err(Error::at(
                line,
                "below hint conflicts with a same-layer hint, directed cycle, or dependency path",
            ));
        }
    }
    let count = component.iter().max().unwrap() + 1;
    let mut dag = vec![BTreeSet::new(); count];
    let mut incoming = vec![0; count];
    for (a, next) in graph.iter().enumerate() {
        for &b in next {
            let (a, b) = (component[a], component[b]);
            if a != b && dag[a].insert(b) {
                incoming[b] += 1;
            }
        }
    }
    let mut ready: BTreeSet<usize> = (0..count).filter(|&i| incoming[i] == 0).collect();
    let mut rank = vec![0; count];
    while let Some(a) = ready.pop_first() {
        for &b in &dag[a] {
            rank[b] = rank[b].max(rank[a] + 1);
            incoming[b] -= 1;
            if incoming[b] == 0 {
                ready.insert(b);
            }
        }
    }
    // Equality groups are indivisible row units. Beside chains define order
    // inside a unit; otherwise source order remains the tie-breaker.
    let mut chains = Vec::new();
    let mut visited = vec![false; n];
    for (root, predecessor) in prev.iter().enumerate() {
        if predecessor.is_some() {
            continue;
        }
        let mut chain = Vec::new();
        let mut cursor = Some(root);
        while let Some(i) = cursor {
            if visited[i] {
                break;
            }
            visited[i] = true;
            chain.push(i);
            cursor = next[i];
        }
        chains.push(chain);
    }
    if let Some(i) = visited.iter().position(|v| !v) {
        let Block::Node(node) = &blocks[i] else {
            unreachable!()
        };
        return Err(Error::at(node.line, "beside hints form a cycle"));
    }
    let mut groups: BTreeMap<usize, Vec<Vec<usize>>> = BTreeMap::new();
    for chain in chains {
        groups.entry(equality[chain[0]]).or_default().push(chain);
    }
    let mut groups: Vec<Vec<usize>> = groups
        .into_values()
        .map(|mut chains| {
            chains.sort_by_key(|c| *c.iter().min().unwrap());
            chains.into_iter().flatten().collect()
        })
        .collect();
    groups.sort_by_key(|g| *g.iter().min().unwrap());
    let mut levels: BTreeMap<(usize, usize), Vec<Vec<usize>>> = BTreeMap::new();
    for group in groups {
        // Connected regions stay separate. Unconnected peers form one final
        // region, so inserting a note-like node does not resize graph layers.
        let first = group[0];
        let island = if weak[first].is_empty() {
            n
        } else {
            island_first[islands[first]]
        };
        levels
            .entry((island, rank[component[first]]))
            .or_default()
            .push(group);
    }
    let mut slots: Vec<_> = blocks.into_iter().map(Some).collect();
    let mut result = Vec::new();
    for groups in levels.into_values() {
        let mut row = Vec::new();
        for group in groups {
            if !row.is_empty() && row.len() + group.len() > 3 {
                emit_row(&mut result, std::mem::take(&mut row));
            }
            row.extend(group.into_iter().map(|i| slots[i].take()));
        }
        emit_row(&mut result, row);
    }
    Ok(result)
}

fn emit_row(out: &mut Vec<Block>, cells: Vec<Option<Block>>) {
    match cells.len() {
        0 => {}
        1 => out.extend(cells.into_iter().flatten()),
        n => out.push(Block::Row(Row {
            weights: vec![1.0; n],
            cells,
            gutter: Some(24.0),
        })),
    }
}

pub(crate) fn validate_mode(blocks: &[Block], automatic: bool) -> Result<(), Error> {
    for block in blocks {
        match block {
            Block::Node(n) => {
                if !automatic && !n.hints.is_empty() {
                    return Err(Error::at(n.line, "layout hints require layout=auto"));
                }
                validate_mode(&n.children, automatic)?;
            }
            Block::Row(r) => {
                for cell in r.cells.iter().flatten() {
                    validate_mode(std::slice::from_ref(cell), automatic)?;
                }
            }
            Block::Section(s) => validate_mode(&s.children, automatic)?,
            _ => {}
        }
    }
    Ok(())
}

// Iterative Kosaraju: cycles share a rank, without recursing on graph depth.
fn components(graph: &[BTreeSet<usize>]) -> Vec<usize> {
    let n = graph.len();
    let mut reverse = vec![Vec::new(); n];
    for (a, next) in graph.iter().enumerate() {
        for &b in next {
            reverse[b].push(a);
        }
    }
    let mut seen = vec![false; n];
    let mut order = Vec::new();
    for root in 0..n {
        let mut stack = vec![(root, false)];
        while let Some((a, done)) = stack.pop() {
            if done {
                order.push(a);
            } else if !seen[a] {
                seen[a] = true;
                stack.push((a, true));
                for &b in graph[a].iter().rev() {
                    if !seen[b] {
                        stack.push((b, false));
                    }
                }
            }
        }
    }
    let mut component = vec![usize::MAX; n];
    let mut id = 0;
    for &root in order.iter().rev() {
        if component[root] != usize::MAX {
            continue;
        }
        let mut stack = vec![root];
        component[root] = id;
        while let Some(a) = stack.pop() {
            for &b in &reverse[a] {
                if component[b] == usize::MAX {
                    component[b] = id;
                    stack.push(b);
                }
            }
        }
        id += 1;
    }
    component
}
