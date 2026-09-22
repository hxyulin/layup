//! Post-layout checks that the layout and router cannot see locally:
//! collinear edge overlap (two dashed edges on one line look solid) and
//! label chips sitting on nodes.

use crate::Warning;
use crate::layout::{Item, Scene};

type Seg = (usize, (f64, f64), (f64, f64));

pub fn check(scene: &Scene, warnings: &mut Vec<Warning>) {
    let segs: Vec<Seg> = scene
        .edges
        .iter()
        .enumerate()
        .flat_map(|(i, e)| e.points.windows(2).map(move |s| (i, s[0], s[1])))
        .collect();
    for (n, &(i, p, q)) in segs.iter().enumerate() {
        for &(j, r, s) in &segs[n + 1..] {
            if i == j {
                continue;
            }
            if collinear_overlap(p, q, r, s) {
                let (a, b) = (&scene.edges[i], &scene.edges[j]);
                if a.bus && b.bus {
                    continue;
                }
                warnings.push(Warning { line: Some(b.line), msg: format!("edges {} -> {} and {} -> {} overlap on a shared line; use `via` or ports to separate them", a.from, a.to, b.from, b.to) });
            }
        }
    }
    for e in &scene.edges {
        if let Some(Item::Chip { rect, text, .. }) = &e.chip {
            for n in &scene.nodes {
                let inside = rect.x >= n.rect.x - 1.0
                    && rect.right() <= n.rect.right() + 1.0
                    && rect.y >= n.rect.y - 1.0
                    && rect.bottom() <= n.rect.bottom() + 1.0;
                let is_endpoint = n.id == e.from || n.id == e.to;
                let is_container = scene
                    .nodes
                    .iter()
                    .any(|m| m.parent == Some(scene.node(&n.id).unwrap()));
                if inside && !is_endpoint && !is_container {
                    warnings.push(Warning {
                        line: Some(e.line),
                        msg: format!(
                            "label \"{text}\" on {} -> {} sits on node `{}`",
                            e.from, e.to, n.id
                        ),
                    });
                }
            }
        }
    }
}

use crate::route::collinear_overlap;
