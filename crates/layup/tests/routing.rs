mod support;
use layup::{Compiled, compile};

fn assert_routes(c: &Compiled) {
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    for edge in &c.scene.edges {
        assert!(edge.points.len() >= 2);
        for p in &edge.points {
            assert!(p.0 >= 0.0 && p.0 <= c.scene.width);
            assert!(p.1 >= 0.0 && p.1 <= c.scene.height);
        }
        for pair in edge.points.windows(2) {
            let (p, q) = (pair[0], pair[1]);
            assert!(p.0 == q.0 || p.1 == q.1);
            assert_ne!(p, q);
            for node in &c.scene.nodes {
                let r = node.rect.grow(-0.5);
                assert!(
                    !(p.0.max(q.0) > r.x
                        && p.0.min(q.0) < r.right()
                        && p.1.max(q.1) > r.y
                        && p.1.min(q.1) < r.bottom()),
                    "edge {} -> {} crosses {}",
                    edge.from,
                    edge.to,
                    node.id
                );
            }
        }
    }
}

#[test]
fn skipped_nodes_and_return_edges_route_without_hints() {
    for src in [SKIP_NODE, RETURN_PATH] {
        let c = compile(src).unwrap();
        assert_routes(&c);
        let again = compile(src).unwrap();
        for (a, b) in c.scene.edges.iter().zip(again.scene.edges) {
            assert_eq!(a.points, b.points);
        }
    }
}

#[test]
fn pinned_sides_are_preserved_by_fallback() {
    let c = compile(PINNED_PORTS).unwrap();
    assert_routes(&c);
    let edge = &c.scene.edges[0];
    let source = c.scene.nodes[support::node(&c.scene, "source").unwrap()].rect;
    let target = c.scene.nodes[support::node(&c.scene, "target").unwrap()].rect;
    assert_eq!(edge.points[0].0, source.right());
    assert!(edge.points[1].0 > source.right());
    assert!(edge.points[edge.points.len() - 2].0 > target.right());
    assert_eq!(edge.points.last().unwrap().0, target.right());
}

#[test]
fn simple_routes_keep_their_straight_path() {
    let c = compile(
        r#"diagram main "Straight" type=graph {
  node a
  node b
  ::a -> ::b
}"#,
    )
    .unwrap();
    assert_routes(&c);
    assert_eq!(c.scene.edges[0].points.len(), 2);
}

#[test]
fn explicit_outside_lanes_remain_explicit() {
    let c = compile(
        r#"diagram main "Outside" type=graph {
  node a
  node b
  ::b -> ::a route-side=right
}"#,
    )
    .unwrap();
    assert!(
        c.scene.edges[0]
            .points
            .iter()
            .any(|p| p.0 > c.scene.content_right)
    );
}

#[test]
fn self_edges_have_a_real_loop_outside_the_node() {
    let c = compile(
        r#"diagram main "Loop" type=graph {
  node a
  ::a -> ::a
}"#,
    )
    .unwrap();
    assert_routes(&c);
    assert!(c.scene.edges[0].points.len() >= 4);
}

const SKIP_NODE: &str = r#"diagram main "Skip a busy stage" type=graph {
  node input "Input"
  node busy "Independent work"
  node output "Output"
  ::input -> ::output "bypass"
}"#;

const RETURN_PATH: &str = r#"diagram main "A return path" type=graph {
  node start "Start"
  node work "Work"
  node finish "Finish"
  ::start -> ::work
  ::work -> ::finish
  ::finish -> ::start "retry"
}"#;

const PINNED_PORTS: &str = r#"diagram main "Keep explicit ports" type=graph {
  node source "Source"
  node obstacle "Obstacle"
  node target "Target"
  ::source -> ::target "around" source-side=right target-side=right
}"#;

#[test]
fn repeated_skip_edges_use_separate_lanes() {
    let c = compile(
        r#"diagram main "Lanes" type=graph {
  node a
  node obstacle
  node b
  ::a -> ::b
  ::a -> ::b
  ::a -> ::b
}"#,
    )
    .unwrap();
    assert_routes(&c);
}
