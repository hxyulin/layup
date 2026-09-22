//! Deterministic graph layering within authored block boundaries.
use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Block, Edge, Row};

/// Only consecutive sibling nodes are rearranged. Explicit rows, sections,
/// dividers and prose are boundaries; their contents are handled recursively.
pub(crate) fn arrange(blocks: &mut Vec<Block>, edges: &[Edge]) {
    for block in blocks.iter_mut() {
        descend(block, edges);
    }
    let mut out = Vec::new();
    let mut run = Vec::new();
    for block in std::mem::take(blocks) {
        if matches!(block, Block::Node(_)) {
            run.push(block);
        } else {
            out.extend(layer(std::mem::take(&mut run), edges));
            out.push(block);
        }
    }
    out.extend(layer(run, edges));
    *blocks = out;
}

fn descend(block: &mut Block, edges: &[Edge]) {
    match block {
        Block::Node(n) => arrange(&mut n.children, edges),
        Block::Section(s) => arrange(&mut s.children, edges),
        Block::Row(r) => {
            for cell in r.cells.iter_mut().flatten() {
                descend(cell, edges);
            }
        }
        _ => {}
    }
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

fn layer(blocks: Vec<Block>, edges: &[Edge]) -> Vec<Block> {
    let n = blocks.len();
    if n < 2 {
        return blocks;
    }
    let mut ids = BTreeMap::new();
    for (i, block) in blocks.iter().enumerate() {
        owners(block, i, &mut ids);
    }
    let mut graph = vec![BTreeSet::new(); n];
    for edge in edges {
        // Undirected and bidirectional edges impose no vertical direction.
        if !edge.head_at_end || edge.head_at_start {
            continue;
        }
        if let (Some(&a), Some(&b)) = (ids.get(&edge.from), ids.get(&edge.to))
            && a != b
        {
            graph[a].insert(b);
        }
    }
    let component = components(&graph);
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
    let mut levels: BTreeMap<usize, Vec<Block>> = BTreeMap::new();
    for (i, block) in blocks.into_iter().enumerate() {
        levels.entry(rank[component[i]]).or_default().push(block);
    }
    let mut result = Vec::new();
    for level in levels.into_values() {
        // Bound row density. Large layers wrap in source order.
        let mut iter = level.into_iter();
        loop {
            let cells: Vec<_> = iter.by_ref().take(3).map(Some).collect();
            match cells.len() {
                0 => break,
                1 => result.extend(cells.into_iter().flatten()),
                n => result.push(Block::Row(Row {
                    weights: vec![1.0; n],
                    cells,
                    gutter: Some(24.0),
                })),
            }
        }
    }
    result
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
