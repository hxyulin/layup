//! Obstacle-aware fallback for routes the inexpensive channel router cannot fit.
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::*;

type Point = (f64, f64);

pub(super) fn fallback(
    scene: &Scene,
    edge: &Edge,
    plan: &Plan,
    obstacles: &[Rect],
    avoid: &[Seg],
) -> Option<Vec<Point>> {
    let (a, b) = (scene.nodes[plan.a].rect, scene.nodes[plan.b].rect);
    // Ancestor/descendant edges have special boundary semantics; leave those
    // to the original router instead of treating the ancestor as solid.
    if scene.is_descendant(plan.a, plan.b) || scene.is_descendant(plan.b, plan.a) {
        return None;
    }
    let mut blocked: Vec<_> = obstacles.iter().map(|r| r.grow(CLEARANCE)).collect();
    blocked.extend(scene.keepout.iter().map(|r| r.grow(CLEARANCE)));
    blocked.push(a);
    if plan.a != plan.b {
        blocked.push(b);
    }
    let starts = candidates(a, edge.from_side, plan.s0, plan.p0, &blocked, avoid);
    let ends = candidates(b, edge.to_side, plan.s1, plan.p1, &blocked, avoid);
    let mut pairs = Vec::new();
    for (i, &(p, stub)) in starts.iter().enumerate() {
        for (j, &(q, end)) in ends.iter().enumerate() {
            if p == q {
                continue;
            }
            pairs.push((distance(stub, end), i, j));
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    for (_, i, j) in pairs {
        let (p, start) = starts[i];
        let (q, end) = ends[j];
        if let Some(middle) = grid_path(start, end, &blocked, avoid, scene) {
            let mut points = vec![p];
            points.extend(middle);
            points.push(q);
            let points = simplify(points);
            if open(&points, &blocked, avoid) {
                return Some(points);
            }
        }
    }
    None
}

fn candidates(
    r: Rect,
    pinned: Option<Side>,
    preferred: Side,
    point: Point,
    blocked: &[Rect],
    avoid: &[Seg],
) -> Vec<(Point, Point)> {
    let mut result = Vec::new();
    for side in [preferred, Side::Top, Side::Bottom, Side::Left, Side::Right] {
        if pinned.is_some_and(|p| p != side) {
            continue;
        }
        let center = if vertical(side) { r.cx() } else { r.cy() };
        let origin = if side == preferred {
            along(point, side)
        } else {
            center
        };
        for offset in [0.0, SPREAD, -SPREAD, 2.0 * SPREAD, -2.0 * SPREAD] {
            let along = origin + offset;
            let (lo, hi) = if vertical(side) {
                (r.x, r.right())
            } else {
                (r.y, r.bottom())
            };
            if along < lo + 4.0 || along > hi - 4.0 {
                continue;
            }
            let p = port(r, side, along);
            let stub = match side {
                Side::Top => (p.0, p.1 - 8.0),
                Side::Bottom => (p.0, p.1 + 8.0),
                Side::Left => (p.0 - 8.0, p.1),
                Side::Right => (p.0 + 8.0, p.1),
            };
            if open(&[p, stub], blocked, avoid) {
                if !result.contains(&(p, stub)) {
                    result.push((p, stub));
                }
                break;
            }
        }
    }
    result
}

fn distance(a: Point, b: Point) -> f64 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn open(points: &[Point], blocked: &[Rect], avoid: &[Seg]) -> bool {
    points
        .windows(2)
        .all(|s| !blocked.iter().any(|r| segment_hits(s[0], s[1], r)))
        && !overlaps_taken(points, avoid)
}

fn grid_path(
    start: Point,
    end: Point,
    blocked: &[Rect],
    avoid: &[Seg],
    scene: &Scene,
) -> Option<Vec<Point>> {
    let mut xs = vec![start.0, end.0, 8.0, scene.width - 8.0];
    let mut ys = vec![start.1, end.1, 8.0, scene.height - 8.0];
    for r in blocked {
        xs.extend([r.x - 2.0, r.right() + 2.0]);
        ys.extend([r.y - 2.0, r.bottom() + 2.0]);
    }
    // Nearby lanes also separate routes that would otherwise share a trunk.
    for &(p, q) in avoid {
        if (p.0 - q.0).abs() < 0.5 {
            xs.extend([p.0 - EDGE_GAP, p.0 + EDGE_GAP]);
        }
        if (p.1 - q.1).abs() < 0.5 {
            ys.extend([p.1 - EDGE_GAP, p.1 + EDGE_GAP]);
        }
    }
    xs.retain(|x| x.is_finite() && *x >= 4.0 && *x <= scene.width - 4.0);
    ys.retain(|y| y.is_finite() && *y >= 4.0 && *y <= scene.height - 4.0);
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    // Bound worst-case fallback cost for dense diagrams. Existing diagnostics
    // remain visible when no route can be found within this budget.
    let cells = xs.len().checked_mul(ys.len())?;
    if cells > 40_000 {
        return None;
    }
    let index = |p: Point| -> Option<usize> {
        Some(
            ys.binary_search_by(|v| v.total_cmp(&p.1)).ok()? * xs.len()
                + xs.binary_search_by(|v| v.total_cmp(&p.0)).ok()?,
        )
    };
    let first = index(start)?;
    let last = index(end)?;
    let point = |i: usize| (xs[i % xs.len()], ys[i / xs.len()]);
    let mut dist = vec![u64::MAX; cells * 2];
    let mut previous = vec![usize::MAX; cells * 2];
    let mut heap = BinaryHeap::new();
    for dir in 0..2 {
        dist[first * 2 + dir] = 0;
        heap.push(Reverse((0, first * 2 + dir)));
    }
    while let Some(Reverse((cost, state))) = heap.pop() {
        if cost != dist[state] {
            continue;
        }
        let cell = state / 2;
        if cell == last {
            let mut path = Vec::new();
            let mut at = state;
            loop {
                path.push(point(at / 2));
                if previous[at] == usize::MAX {
                    break;
                }
                at = previous[at];
            }
            path.reverse();
            return Some(path);
        }
        let (x, y) = (cell % xs.len(), cell / xs.len());
        let neighbors = [
            (x.checked_sub(1).map(|nx| y * xs.len() + nx), 0),
            ((x + 1 < xs.len()).then_some(cell + 1), 0),
            (y.checked_sub(1).map(|ny| ny * xs.len() + x), 1),
            ((y + 1 < ys.len()).then_some(cell + xs.len()), 1),
        ];
        for (next, dir) in neighbors {
            let Some(next) = next else { continue };
            let (p, q) = (point(cell), point(next));
            if !open(&[p, q], blocked, avoid) {
                continue;
            }
            let candidate = cost
                + (distance(p, q) * 1000.0).round() as u64
                + if state % 2 == dir { 0 } else { 12_000 };
            let target = next * 2 + dir;
            if candidate < dist[target] {
                dist[target] = candidate;
                previous[target] = state;
                heap.push(Reverse((candidate, target)));
            }
        }
    }
    None
}

fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    for p in points {
        if out.last() == Some(&p) {
            continue;
        }
        while out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            if (a.0 == b.0 && b.0 == p.0) || (a.1 == b.1 && b.1 == p.1) {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }
    out
}

pub(super) fn valid_ports(points: &[Point], plan: &Plan, a: Rect, b: Rect) -> bool {
    let Some((&first, rest)) = points.split_first() else {
        return false;
    };
    let Some(&second) = rest.first() else {
        return false;
    };
    let last = points[points.len() - 1];
    let penultimate = points[points.len() - 2];
    let outward = |p: Point, q: Point, r: Rect, side| match side {
        Side::Top => p.1 == r.y && p.0 >= r.x && p.0 <= r.right() && q.1 < p.1,
        Side::Bottom => p.1 == r.bottom() && p.0 >= r.x && p.0 <= r.right() && q.1 > p.1,
        Side::Left => p.0 == r.x && p.1 >= r.y && p.1 <= r.bottom() && q.0 < p.0,
        Side::Right => p.0 == r.right() && p.1 >= r.y && p.1 <= r.bottom() && q.0 > p.0,
    };
    outward(first, second, a, plan.s0) && outward(last, penultimate, b, plan.s1)
}
