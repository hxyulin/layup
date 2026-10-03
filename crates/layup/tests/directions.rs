use layup::{Compiled, compile, layout::Rect};

fn rect(c: &Compiled, id: &str) -> Rect {
    c.scene.nodes[c.scene.node(id).unwrap()].rect
}
fn precedes(a: Rect, b: Rect, direction: &str) -> bool {
    match direction {
        "down" => a.bottom() < b.y,
        "up" => b.bottom() < a.y,
        "right" => a.right() < b.x,
        "left" => b.right() < a.x,
        _ => unreachable!(),
    }
}

#[test]
fn all_directions_place_branches_and_route_without_rotating_text() {
    for direction in ["down", "up", "right", "left"] {
        let source = format!(
            r#"diagram "Flow" width=1100 layout=auto direction={direction} {{
            node sink "Sink"; node a "A"; node source "Source"; node b "B"
            source -> a; source -> b; a -> sink; b -> sink
        }}"#
        );
        let c = compile(&source).unwrap();
        assert!(c.warnings.is_empty(), "{direction}: {:?}", c.warnings);
        assert!(precedes(rect(&c, "source"), rect(&c, "a"), direction));
        assert!(precedes(rect(&c, "a"), rect(&c, "sink"), direction));
        if matches!(direction, "left" | "right") {
            assert_eq!(rect(&c, "a").x, rect(&c, "b").x);
            assert!(rect(&c, "a").bottom() < rect(&c, "b").y);
        } else {
            assert_eq!(rect(&c, "a").y, rect(&c, "b").y);
            assert!(rect(&c, "a").right() < rect(&c, "b").x);
        }
        for edge in &c.scene.edges {
            for p in &edge.points {
                assert!(p.0 >= 0.0 && p.0 <= c.scene.width && p.1 >= 0.0 && p.1 <= c.scene.height);
            }
            for segment in edge.points.windows(2) {
                assert!(segment[0].0 == segment[1].0 || segment[0].1 == segment[1].1);
            }
        }
        let again = compile(&source).unwrap();
        for id in ["source", "a", "b", "sink"] {
            assert_eq!(rect(&c, id), rect(&again, id));
        }
    }
}

#[test]
fn nested_edges_order_containers_in_every_direction() {
    for direction in ["down", "up", "right", "left"] {
        let c = compile(&format!(
            r#"diagram "Nested" width=1400 layout=auto direction={direction} {{
            node downstream "Downstream" {{ node sink "Sink" }}
            node upstream "Upstream" {{ node b "B"; node a "A"; a -> b }}
            b -> sink
        }}"#
        ))
        .unwrap();
        assert!(precedes(
            rect(&c, "upstream"),
            rect(&c, "downstream"),
            direction
        ));
        assert!(precedes(rect(&c, "a"), rect(&c, "b"), direction));
    }
}

#[test]
fn authored_rows_and_independent_regions_retain_their_order() {
    for direction in ["up", "right", "left"] {
        let c = compile(&format!(
            r#"diagram "Boundaries" layout=auto direction={direction} {{
            row 2:1 {{ node first "First"; node second "Second" }}
            node a "A"; node b "B"; node c "C"; node d "D"; node isolated "Isolated"
            a -> b; c -> d
        }}"#
        ))
        .unwrap();
        assert!(rect(&c, "first").x < rect(&c, "second").x);
        assert!(rect(&c, "first").w > rect(&c, "second").w);
        assert!(rect(&c, "second").bottom() < rect(&c, "a").y);
        assert!(
            rect(&c, "a").bottom().max(rect(&c, "b").bottom())
                < rect(&c, "c").y.min(rect(&c, "d").y)
        );
        assert!(rect(&c, "c").bottom().max(rect(&c, "d").bottom()) < rect(&c, "isolated").y);
    }
}

#[test]
fn hints_follow_the_flow_axis_and_upward_equality_groups_stay_together() {
    for direction in ["up", "right", "left"] {
        let c = compile(&format!(
            r#"diagram "Hints" layout=auto direction={direction} {{
            node a "A"; node b "B" after=a; node c "C" same-layer=b
        }}"#
        ))
        .unwrap();
        assert!(precedes(rect(&c, "a"), rect(&c, "b"), direction));
    }
    let c = compile(
        r#"diagram "Peers" width=1400 layout=auto direction=up {
        node a; node b same-layer=a; node c same-layer=a; node d same-layer=a
    }"#,
    )
    .unwrap();
    assert_eq!(rect(&c, "a").y, rect(&c, "d").y);
}

#[test]
fn default_direction_and_validation_are_explicit() {
    let source = r#"diagram "T" layout=auto { node a; node b; a -> b }"#;
    let default = compile(source).unwrap();
    let down = compile(&source.replacen("layout=auto", "layout=auto direction=down", 1)).unwrap();
    assert_eq!(rect(&default, "a"), rect(&down, "a"));
    assert!(
        compile(r#"diagram "T" direction=left { node a }"#)
            .err()
            .unwrap()
            .msg
            .contains("layout=auto")
    );
    assert!(compile(r#"diagram "T" layout=auto direction=diagonal {}"#).is_err());
    for alias in ["TB", "TD", "BT", "LR", "RL"] {
        assert!(
            compile(&format!(
                "diagram \"T\" layout=auto direction={alias} {{node a}}"
            ))
            .is_ok()
        );
    }
}
