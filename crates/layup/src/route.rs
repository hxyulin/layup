//! Orthogonal edge routing over the laid-out scene.
//!
//! Every edge leaves a node through one side and enters the other through
//! one side. Sides come from the relative position of the two nodes unless
//! the author pinned ports, and the router tries side exits when the
//! obvious one would cut through a neighbor. Aligned nodes get a single
//! straight segment; others get a three-segment route whose middle segment
//! sits in the first clear channel nearest the midpoint. Channels and
//! ports avoid edges routed earlier, so declaration order is the tie
//! breaker. `via=right|left` wraps an edge around the outside of the
//! content, in lanes the layout reserved for it.

use std::collections::BTreeMap;

use crate::Warning;
use crate::layout::{EdgePath, Item, Rect, Scene};
use crate::model::{Diagram, Edge, Side};
use crate::style::{ArrowColor, Tone};
use crate::text::{Font, width};

const CLEARANCE: f64 = 4.0;
const SPREAD: f64 = 20.0;
const EDGE_GAP: f64 = 6.0;

#[derive(Clone)]
struct Plan {
    a: usize,
    b: usize,
    s0: Side,
    s1: Side,
    p0: (f64, f64),
    p1: (f64, f64),
    outside: Option<(Side, usize)>,
}

type Seg = ((f64, f64), (f64, f64));

pub fn route_all(d: &Diagram, scene: &mut Scene, warnings: &mut Vec<Warning>) {
    let mut via_count = [0usize; 2];
    let via_total = [
        d.edges
            .iter()
            .filter(|e| e.via == Some(Side::Right))
            .count(),
        d.edges.iter().filter(|e| e.via == Some(Side::Left)).count(),
    ];
    let mut plans: Vec<Option<Plan>> = Vec::new();
    for e in &d.edges {
        let (Some(a), Some(b)) = (scene.node(&e.from), scene.node(&e.to)) else {
            plans.push(None);
            continue;
        };
        let obstacles = obstacles(scene, a, b);
        let mut plan = plan_direct(
            e,
            a,
            b,
            scene.nodes[a].rect,
            scene.nodes[b].rect,
            &obstacles,
        );
        if let Some(side @ (Side::Right | Side::Left)) = e.via {
            let slot = if side == Side::Right { 0 } else { 1 };
            plan.outside = Some((side, via_count[slot]));
            via_count[slot] += 1;
        }
        plans.push(Some(plan));
    }
    spread_ports(d, &mut plans);

    let mut taken: Vec<Seg> = Vec::new();
    let mut taken_bus: Vec<bool> = Vec::new();
    for (e, plan) in d.edges.iter().zip(plans) {
        let Some(plan) = plan else { continue };
        let style = &d.arrows[&e.kind];
        let tone = match (e.tone, style.color) {
            (Some(t), _) => t,
            (None, ArrowColor::Tone(t)) => t,
            (None, ArrowColor::Inherit) => scene.nodes[plan.b].tone,
        };
        let dashed = e.dashed.unwrap_or(style.dashed);
        let obstacles = obstacles(scene, plan.a, plan.b);
        let avoid: Vec<Seg> = taken
            .iter()
            .zip(&taken_bus)
            .filter(|(_, bus)| !(e.bus && **bus))
            .map(|(s, _)| *s)
            .collect();
        let (ra, rb) = (scene.nodes[plan.a].rect, scene.nodes[plan.b].rect);
        let points = match plan.outside {
            Some((side, k)) => {
                let total = if side == Side::Right {
                    via_total[0]
                } else {
                    via_total[1]
                };
                route_outside(scene, ra, rb, side, k, total, &obstacles, &avoid)
            }
            None => route_with_retries(&plan, &obstacles, &avoid),
        };
        for (i, seg) in points.windows(2).enumerate() {
            let (p, q) = (seg[0], seg[1]);
            if let Some(hit) = obstacles.iter().find(|o| segment_hits(p, q, o)) {
                let name = scene
                    .nodes
                    .iter()
                    .find(|n| n.rect == *hit)
                    .map(|n| n.id.clone())
                    .unwrap_or_default();
                warnings.push(Warning { line: Some(e.line), msg: format!("edge {} -> {} segment {} crosses node `{name}`; try `via=right`, ports (`from=`/`to=`) or a different row", e.from, e.to, i + 1) });
            }
        }
        let chip_text = e.label.clone().or_else(|| {
            if e.labeled {
                style.chip.clone().or_else(|| style.label.clone())
            } else {
                None
            }
        });
        let chip = chip_text.and_then(|t| place_chip(scene, &points, &t, tone, plan.a, plan.b));
        for seg in points.windows(2) {
            taken.push((seg[0], seg[1]));
            taken_bus.push(e.bus);
        }
        scene.edges.push(EdgePath {
            from: e.from.clone(),
            to: e.to.clone(),
            kind: e.kind.clone(),
            points,
            tone,
            dashed,
            head_end: e.head_at_end,
            head_start: e.head_at_start,
            chip,
            bus: e.bus,
            line: e.line,
        });
    }
    let lowest = scene
        .edges
        .iter()
        .flat_map(|e| e.points.iter().map(|p| p.1))
        .fold(0.0, f64::max);
    if lowest + 24.0 > scene.height {
        scene.height = (lowest + 24.0).ceil();
    }
}

/// Edges that leave or enter one node through the same point are moved
/// apart along that side, in declaration order, unless every one is `bus`.
/// A straight edge keeps both ends together so it stays straight.
fn spread_ports(d: &Diagram, plans: &mut [Option<Plan>]) {
    let mut groups: BTreeMap<(usize, u8, i64), Vec<(usize, bool)>> = BTreeMap::new();
    for (i, p) in plans.iter().enumerate() {
        let Some(p) = p else { continue };
        if p.outside.is_some() {
            continue;
        }
        groups
            .entry((p.a, side_code(p.s0), along(p.p0, p.s0).round() as i64))
            .or_default()
            .push((i, false));
        groups
            .entry((p.b, side_code(p.s1), along(p.p1, p.s1).round() as i64))
            .or_default()
            .push((i, true));
    }
    for members in groups.values() {
        if members.len() < 2 || members.iter().all(|(i, _)| d.edges[*i].bus) {
            continue;
        }
        let n = members.len() as f64;
        for (k, (i, is_target)) in members.iter().enumerate() {
            let offset = (k as f64 - (n - 1.0) / 2.0) * SPREAD;
            let p = plans[*i].as_mut().unwrap();
            let straight = vertical(p.s0) == vertical(p.s1)
                && (along(p.p0, p.s0) - along(p.p1, p.s1)).abs() < 0.5;
            if straight {
                p.p0 = shift(p.p0, p.s0, offset);
                p.p1 = shift(p.p1, p.s1, offset);
            } else if *is_target {
                p.p1 = shift(p.p1, p.s1, offset);
            } else {
                p.p0 = shift(p.p0, p.s0, offset);
            }
        }
    }
}

fn side_code(s: Side) -> u8 {
    match s {
        Side::Top => 0,
        Side::Bottom => 1,
        Side::Left => 2,
        Side::Right => 3,
    }
}

fn along(p: (f64, f64), s: Side) -> f64 {
    if vertical(s) { p.0 } else { p.1 }
}

fn shift(p: (f64, f64), s: Side, by: f64) -> (f64, f64) {
    if vertical(s) {
        (p.0 + by, p.1)
    } else {
        (p.0, p.1 + by)
    }
}

fn obstacles(scene: &Scene, a: usize, b: usize) -> Vec<Rect> {
    let anc_a = scene.ancestors(a);
    let anc_b = scene.ancestors(b);
    scene
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            *i != a
                && *i != b
                && !anc_a.contains(i)
                && !anc_b.contains(i)
                && !scene.is_descendant(*i, a)
                && !scene.is_descendant(*i, b)
        })
        .map(|(_, n)| n.rect)
        .collect()
}

fn segment_hits(p: (f64, f64), q: (f64, f64), r: &Rect) -> bool {
    let r = r.grow(-0.5);
    let (x1, x2) = (p.0.min(q.0), p.0.max(q.0));
    let (y1, y2) = (p.1.min(q.1), p.1.max(q.1));
    x2 > r.x && x1 < r.right() && y2 > r.y && y1 < r.bottom()
}

fn clear(points: &[(f64, f64)], obstacles: &[Rect]) -> bool {
    points.windows(2).all(|s| {
        !obstacles
            .iter()
            .any(|o| segment_hits(s[0], s[1], &o.grow(CLEARANCE)))
    })
}

/// True when some segment of `points` runs along an earlier edge's segment.
fn overlaps_taken(points: &[(f64, f64)], taken: &[Seg]) -> bool {
    points.windows(2).any(|s| {
        taken
            .iter()
            .any(|t| collinear_overlap(s[0], s[1], t.0, t.1))
    })
}

pub fn collinear_overlap(p: (f64, f64), q: (f64, f64), r: (f64, f64), s: (f64, f64)) -> bool {
    let v1 = (p.1 - q.1).abs() > (p.0 - q.0).abs();
    let v2 = (r.1 - s.1).abs() > (r.0 - s.0).abs();
    if v1 != v2 {
        return false;
    }
    let (f1, a0, a1, f2, b0, b1) = if v1 {
        (
            p.0,
            p.1.min(q.1),
            p.1.max(q.1),
            r.0,
            r.1.min(s.1),
            r.1.max(s.1),
        )
    } else {
        (
            p.1,
            p.0.min(q.0),
            p.0.max(q.0),
            r.1,
            r.0.min(s.0),
            r.0.max(s.0),
        )
    };
    (f1 - f2).abs() < EDGE_GAP && a1.min(b1) - a0.max(b0) > 8.0
}

fn port(r: Rect, side: Side, along: f64) -> (f64, f64) {
    match side {
        Side::Top => (along, r.y),
        Side::Bottom => (along, r.bottom()),
        Side::Left => (r.x, along),
        Side::Right => (r.right(), along),
    }
}

fn vertical(side: Side) -> bool {
    matches!(side, Side::Top | Side::Bottom)
}

fn overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> Option<(f64, f64)> {
    let lo = a0.max(b0);
    let hi = a1.min(b1);
    (hi - lo >= 24.0).then_some((lo, hi))
}

/// Picks a coordinate inside the overlap, preferring the centers.
fn pick(lo: f64, hi: f64, ca: f64, cb: f64) -> f64 {
    if ca >= lo && ca <= hi {
        ca
    } else if cb >= lo && cb <= hi {
        cb
    } else {
        (lo + hi) / 2.0
    }
}

fn plan_direct(e: &Edge, a: usize, b: usize, ra: Rect, rb: Rect, obstacles: &[Rect]) -> Plan {
    let below = rb.y >= ra.bottom() - 1.0;
    let above = rb.bottom() <= ra.y + 1.0;
    let right = rb.x >= ra.right() - 1.0;
    let left = rb.right() <= ra.x + 1.0;
    let candidates: Vec<(Side, Side)> = match (e.from_side, e.to_side) {
        (Some(s0), Some(s1)) => vec![(s0, s1)],
        (Some(s0), None) => vec![(s0, facing(rb, ra, s0))],
        (None, Some(s1)) => vec![(facing(ra, rb, s1), s1)],
        (None, None) => {
            let toward = if rb.cx() >= ra.cx() {
                Side::Right
            } else {
                Side::Left
            };
            let back = if rb.cx() >= ra.cx() {
                Side::Left
            } else {
                Side::Right
            };
            if below {
                vec![
                    (Side::Bottom, Side::Top),
                    (toward, Side::Top),
                    (Side::Bottom, back),
                    (toward, back),
                ]
            } else if above {
                vec![
                    (Side::Top, Side::Bottom),
                    (toward, Side::Bottom),
                    (Side::Top, back),
                    (toward, back),
                ]
            } else if right {
                vec![
                    (Side::Right, Side::Left),
                    (Side::Top, Side::Left),
                    (Side::Bottom, Side::Left),
                ]
            } else if left {
                vec![
                    (Side::Left, Side::Right),
                    (Side::Top, Side::Right),
                    (Side::Bottom, Side::Right),
                ]
            } else {
                vec![(Side::Bottom, Side::Top)]
            }
        }
    };
    let mut first = None;
    for (s0, s1) in candidates {
        let (p0, p1) = ports(ra, rb, s0, s1);
        let plan = Plan {
            a,
            b,
            s0,
            s1,
            p0,
            p1,
            outside: None,
        };
        let points = connect(p0, s0, p1, s1, obstacles, &[]);
        if clear(&points, obstacles) {
            return plan;
        }
        first.get_or_insert(plan);
    }
    first.unwrap()
}

/// The side of `b` that faces `a` when `a` leaves through `s0`.
fn facing(b: Rect, a: Rect, s0: Side) -> Side {
    if vertical(s0) {
        if b.cy() >= a.cy() {
            Side::Top
        } else {
            Side::Bottom
        }
    } else if b.cx() >= a.cx() {
        Side::Left
    } else {
        Side::Right
    }
}

fn ports(a: Rect, b: Rect, s0: Side, s1: Side) -> ((f64, f64), (f64, f64)) {
    if vertical(s0) && vertical(s1) {
        match overlap(a.x, a.right(), b.x, b.right()) {
            Some((lo, hi)) => {
                let x = pick(lo, hi, a.cx(), b.cx());
                (port(a, s0, x), port(b, s1, x))
            }
            None => (port(a, s0, a.cx()), port(b, s1, b.cx())),
        }
    } else if !vertical(s0) && !vertical(s1) {
        match overlap(a.y, a.bottom(), b.y, b.bottom()) {
            Some((lo, hi)) => {
                let y = pick(lo, hi, a.cy(), b.cy());
                (port(a, s0, y), port(b, s1, y))
            }
            None => (port(a, s0, a.cy()), port(b, s1, b.cy())),
        }
    } else {
        let p0 = if vertical(s0) {
            port(a, s0, a.cx())
        } else {
            port(a, s0, a.cy())
        };
        let p1 = if vertical(s1) {
            port(b, s1, b.cx())
        } else {
            port(b, s1, b.cy())
        };
        (p0, p1)
    }
}

/// Routes the plan; if the result runs along an earlier edge, nudges the
/// ports and tries again before accepting the overlap.
fn route_with_retries(plan: &Plan, obstacles: &[Rect], avoid: &[Seg]) -> Vec<(f64, f64)> {
    let base = connect(plan.p0, plan.s0, plan.p1, plan.s1, obstacles, avoid);
    if !overlaps_taken(&base, avoid) {
        return base;
    }
    let straight = vertical(plan.s0) == vertical(plan.s1)
        && (along(plan.p0, plan.s0) - along(plan.p1, plan.s1)).abs() < 0.5;
    for offset in [SPREAD, -SPREAD, 2.0 * SPREAD, -2.0 * SPREAD] {
        let tries: Vec<((f64, f64), (f64, f64))> = if straight {
            vec![(
                shift(plan.p0, plan.s0, offset),
                shift(plan.p1, plan.s1, offset),
            )]
        } else {
            vec![
                (shift(plan.p0, plan.s0, offset), plan.p1),
                (plan.p0, shift(plan.p1, plan.s1, offset)),
                (
                    shift(plan.p0, plan.s0, offset),
                    shift(plan.p1, plan.s1, offset),
                ),
            ]
        };
        for (p0, p1) in tries {
            let pts = connect(p0, plan.s0, p1, plan.s1, obstacles, avoid);
            if !overlaps_taken(&pts, avoid) && clear(&pts, obstacles) {
                return pts;
            }
        }
    }
    base
}

fn connect(
    p0: (f64, f64),
    s0: Side,
    p1: (f64, f64),
    s1: Side,
    obstacles: &[Rect],
    avoid: &[Seg],
) -> Vec<(f64, f64)> {
    let ok = |pts: &[(f64, f64)]| clear(pts, obstacles) && !overlaps_taken(pts, avoid);
    match (vertical(s0), vertical(s1)) {
        (true, true) => {
            if (p0.0 - p1.0).abs() < 0.5 {
                return vec![p0, p1];
            }
            let (lo, hi) = (p0.1.min(p1.1) + 10.0, p0.1.max(p1.1) - 10.0);
            let mid = (p0.1 + p1.1) / 2.0;
            let ych = scan(mid, lo, hi, |y| ok(&[p0, (p0.0, y), (p1.0, y), p1])).unwrap_or(mid);
            vec![p0, (p0.0, ych), (p1.0, ych), p1]
        }
        (false, false) => {
            if (p0.1 - p1.1).abs() < 0.5 {
                return vec![p0, p1];
            }
            let (lo, hi) = (p0.0.min(p1.0) + 10.0, p0.0.max(p1.0) - 10.0);
            let mid = (p0.0 + p1.0) / 2.0;
            let xch = scan(mid, lo, hi, |x| ok(&[p0, (x, p0.1), (x, p1.1), p1])).unwrap_or(mid);
            vec![p0, (xch, p0.1), (xch, p1.1), p1]
        }
        (true, false) => {
            let l = vec![p0, (p0.0, p1.1), p1];
            if ok(&l) {
                return l;
            }
            // Leave vertically, turn toward the target in a clear channel,
            // then drop or rise to the target's row and enter from the side.
            let dir = if p1.1 >= p0.1 { 1.0 } else { -1.0 };
            let sx = if p1.0 >= p0.0 { -1.0 } else { 1.0 };
            let (dy, dx) = ((p1.1 - p0.1).abs() - 10.0, (p1.0 - p0.0).abs() - 10.0);
            let found = search2(
                p0.1 + dir * 10.0,
                dir,
                dy,
                p1.0 + sx * 10.0,
                sx,
                dx,
                |y, x| ok(&[p0, (p0.0, y), (x, y), (x, p1.1), p1]),
            );
            let (y, x) = found.unwrap_or(((p0.1 + p1.1) / 2.0, (p0.0 + p1.0) / 2.0));
            vec![p0, (p0.0, y), (x, y), (x, p1.1), p1]
        }
        (false, true) => {
            let l = vec![p0, (p1.0, p0.1), p1];
            if ok(&l) {
                return l;
            }
            // Leave sideways into a clear column, run up or down it, then
            // cross to the target's column in a clear channel and enter
            // vertically.
            let sx = if p1.0 >= p0.0 { 1.0 } else { -1.0 };
            let dir = if p1.1 >= p0.1 { -1.0 } else { 1.0 };
            let (dy, dx) = ((p1.1 - p0.1).abs() - 10.0, (p1.0 - p0.0).abs() - 10.0);
            let found = search2(
                p0.0 + sx * 10.0,
                sx,
                dx,
                p1.1 + dir * 10.0,
                dir,
                dy,
                |x, y| ok(&[p0, (x, p0.1), (x, y), (p1.0, y), p1]),
            );
            let (x, y) = found.unwrap_or(((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0));
            vec![p0, (x, p0.1), (x, y), (p1.0, y), p1]
        }
    }
}

/// Searches two coordinates, the first stepping outward from `a0` and the
/// second from `b0`, returning the first pair that satisfies `ok`. The
/// outer coordinate is the one nearest its start, so routes hug the node
/// they leave and the channel nearest the node they enter.
fn search2(
    a0: f64,
    da: f64,
    amax: f64,
    b0: f64,
    db: f64,
    bmax: f64,
    ok: impl Fn(f64, f64) -> bool,
) -> Option<(f64, f64)> {
    let mut a = 0.0;
    while a <= amax.max(0.0) {
        let mut b = 0.0;
        while b <= bmax.max(0.0) {
            let (x, y) = (a0 + da * a, b0 + db * b);
            if ok(x, y) {
                return Some((x, y));
            }
            b += 6.0;
        }
        a += 6.0;
    }
    None
}

/// Tries `mid` first, then values fanning out from it inside `[lo, hi]`.
fn scan(mid: f64, lo: f64, hi: f64, ok: impl Fn(f64) -> bool) -> Option<f64> {
    if lo > hi {
        return ok(mid).then_some(mid);
    }
    let mid = mid.clamp(lo, hi);
    if ok(mid) {
        return Some(mid);
    }
    let mut d = 6.0;
    while mid - d >= lo || mid + d <= hi {
        for y in [mid - d, mid + d] {
            if y >= lo && y <= hi && ok(y) {
                return Some(y);
            }
        }
        d += 6.0;
    }
    None
}

/// Leaves `a` on the side away from `b`, swings out to a lane beside the
/// content, and comes back in to `b` from the side facing `a`. Lanes,
/// horizontal runs and entry points are staggered so several such edges
/// nest without touching: the first declared edge is the outermost.
#[allow(clippy::too_many_arguments)]
fn route_outside(
    scene: &Scene,
    a: Rect,
    b: Rect,
    side: Side,
    k: usize,
    total: usize,
    obstacles: &[Rect],
    avoid: &[Seg],
) -> Vec<(f64, f64)> {
    let b_above = b.cy() < a.cy();
    let n = total.max(1) as f64;
    let outer = (total.saturating_sub(1).saturating_sub(k)) as f64;
    let inner = k as f64;
    let sign = if side == Side::Right { 1.0 } else { -1.0 };
    let lane = if side == Side::Right {
        scene.content_right + 25.0 + 30.0 * outer
    } else {
        scene.content_left - 25.0 - 30.0 * outer
    };
    let dir = if b_above { 1.0 } else { -1.0 };
    let (x0, y0) = if b_above {
        (a.cx(), a.bottom())
    } else {
        (a.cx(), a.y)
    };
    let x1 = b.cx() + sign * SPREAD * ((n - 1.0) / 2.0 - inner);
    let y1 = if b_above { b.bottom() } else { b.y };
    let ok = |pts: &[(f64, f64)]| clear(pts, obstacles) && !overlaps_taken(pts, avoid);
    let first = scan_from(y0 + dir * 26.0, dir, 600.0, |y| {
        ok(&[(x0, y0), (x0, y), (lane, y)])
    })
    .unwrap_or(y0 + dir * 26.0)
        + dir * 25.0 * outer;
    let second = scan_from(y1 + dir * 26.0, dir, 600.0, |y| {
        ok(&[(lane, y), (x1, y), (x1, y1)])
    })
    .unwrap_or(y1 + dir * 26.0)
        + dir * 25.0 * inner;
    vec![
        (x0, y0),
        (x0, first),
        (lane, first),
        (lane, second),
        (x1, second),
        (x1, y1),
    ]
}

fn scan_from(start: f64, dir: f64, max: f64, ok: impl Fn(f64) -> bool) -> Option<f64> {
    let mut d = 0.0;
    while d <= max {
        let y = start + dir * d;
        if ok(y) {
            return Some(y);
        }
        d += 6.0;
    }
    None
}

/// Places the label chip on the longest segment: centered on a horizontal
/// one, rotated along a vertical one when it fits, otherwise beside it
/// without a border. The chip slides along its segment until it covers no
/// node, section title or earlier chip.
fn place_chip(
    scene: &Scene,
    points: &[(f64, f64)],
    text: &str,
    tone: Tone,
    a: usize,
    b: usize,
) -> Option<Item> {
    let (i, len) = points
        .windows(2)
        .enumerate()
        .map(|(i, s)| (i, (s[1].0 - s[0].0).abs() + (s[1].1 - s[0].1).abs()))
        .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap())?;
    let (p, q) = (points[i], points[i + 1]);
    let w = width(text, Font::SansBold, 11.5, 0.0) + 14.0;
    let h = 21.0;
    let horizontal = (p.1 - q.1).abs() < 0.5;
    let (rotate, bordered, beside) = if horizontal {
        (false, len >= w + 16.0, len < w + 16.0)
    } else {
        (len >= w + 20.0, len >= w + 20.0, len < w + 20.0)
    };
    // Boxes the chip must not cover: leaf nodes (not the containers the edge
    // legitimately passes over), titles, and chips already placed.
    let leaf = |i: usize| !scene.nodes.iter().any(|n| n.parent == Some(i));
    let mut blocked: Vec<Rect> = scene
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != a && *i != b && leaf(*i))
        .map(|(_, n)| n.rect)
        .collect();
    blocked.extend(scene.keepout.iter().copied());
    blocked.extend(scene.edges.iter().filter_map(|e| {
        if let Some(Item::Chip { rect, .. }) = &e.chip {
            Some(*rect)
        } else {
            None
        }
    }));
    let rect_at = |t: f64| {
        let (mx, my) = (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
        if beside {
            if horizontal {
                Rect {
                    x: mx - w / 2.0,
                    y: my - h - 2.0,
                    w,
                    h,
                }
            } else {
                Rect {
                    x: mx + 4.0,
                    y: my - h / 2.0,
                    w,
                    h,
                }
            }
        } else if rotate {
            Rect {
                x: mx - h / 2.0,
                y: my - w / 2.0,
                w: h,
                h: w,
            }
        } else {
            Rect {
                x: mx - w / 2.0,
                y: my - h / 2.0,
                w,
                h,
            }
        }
    };
    let free = |r: &Rect| {
        !blocked
            .iter()
            .any(|o| r.x < o.right() && r.right() > o.x && r.y < o.bottom() && r.bottom() > o.y)
    };
    let along = if rotate || horizontal { w } else { h };
    let margin = (along / 2.0 + 8.0) / len.max(1.0);
    let mut chosen = rect_at(0.5);
    if !free(&chosen) {
        let mut d = 0.05;
        while d <= 0.5 - margin {
            for cand in [0.5 - d, 0.5 + d] {
                let r = rect_at(cand);
                if free(&r) {
                    chosen = r;
                    d = 1.0;
                    break;
                }
            }
            d += 0.05;
        }
    }
    // Rotated chips are stored unrotated around their center so the emitter
    // can apply the transform; convert back to the horizontal box.
    let rect = if rotate {
        Rect {
            x: chosen.cx() - w / 2.0,
            y: chosen.cy() - h / 2.0,
            w,
            h,
        }
    } else {
        chosen
    };
    Some(Item::Chip {
        rect,
        text: text.into(),
        tone,
        rotate,
        bordered,
    })
}
