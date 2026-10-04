use layup::{
    Compiled, Theme, compile,
    layout::{Item, Rect},
    model::{Mode, Side},
    style::Shape,
};

fn rect(c: &Compiled, id: &str) -> Rect {
    c.scene.nodes[c.scene.node(id).unwrap()].rect
}
fn precedes(a: Rect, b: Rect, d: &str) -> bool {
    match d {
        "down" => a.bottom() < b.y,
        "up" => b.bottom() < a.y,
        "right" => a.right() < b.x,
        "left" => b.right() < a.x,
        _ => unreachable!(),
    }
}
fn assert_routes(c: &Compiled) {
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    for edge in &c.scene.edges {
        for (id, end) in [(&edge.from, false), (&edge.to, true)] {
            let outline = c.scene.nodes[c.scene.node(id).unwrap()].outline;
            let i = if end { edge.points.len() - 1 } else { 0 };
            let j = if end { i - 1 } else { 1 };
            let (p, q) = (edge.points[i], edge.points[j]);
            let (side, along) = if p.0 == q.0 {
                (if q.1 < p.1 { Side::Top } else { Side::Bottom }, p.0)
            } else {
                (if q.0 < p.0 { Side::Left } else { Side::Right }, p.1)
            };
            let expected = outline.port(side, along);
            assert!(
                (p.0 - expected.0).abs() < 0.001 && (p.1 - expected.1).abs() < 0.001,
                "{id}: {p:?} != {expected:?}"
            );
        }
        for pair in edge.points.windows(2) {
            assert!(pair[0].0 == pair[1].0 || pair[0].1 == pair[1].1);
            for node in &c.scene.nodes {
                assert!(
                    !node.outline.segment_hits(pair[0], pair[1]),
                    "{} -> {} crosses {}",
                    edge.from,
                    edge.to,
                    node.id
                );
            }
        }
        assert!(
            edge.points
                .iter()
                .all(|&(x, y)| x >= 0.0 && x <= c.scene.width && y >= 0.0 && y <= c.scene.height)
        );
    }
}

#[test]
fn cyclic_machines_follow_the_flow_in_every_direction() {
    for direction in ["down", "up", "right", "left"] {
        let c=compile(&format!(r#"diagram "Lifecycle" mode=state-machine direction={direction} width=1400 {{
            initial start; state idle "Idle"; state active "Active"; state paused "Paused"; final end
            start -> idle; idle -> active "start"; active -> paused "pause"; paused -> active "resume"
            active -> active "tick"; active -> end "finish [done] / close()"
        }}"#)).unwrap();
        assert_routes(&c);
        assert_eq!(c.diagram.mode, Mode::StateMachine);
        assert!(c.diagram.auto_layout);
        assert!(precedes(rect(&c, "start"), rect(&c, "idle"), direction));
        assert!(precedes(rect(&c, "idle"), rect(&c, "active"), direction));
        assert!(precedes(rect(&c, "active"), rect(&c, "paused"), direction));
        assert_eq!(c.scene.edges.len(), 6);
        assert_eq!(c.scene.edges.iter().filter(|e| e.chip.is_some()).count(), 5);
        let loop_edge = c.scene.edges.iter().find(|e| e.from == e.to).unwrap();
        assert!(loop_edge.points.len() >= 4);
    }
}

#[test]
fn statement_order_does_not_change_cycle_placement() {
    let head = r#"diagram "Cycle" mode=state-machine direction=right { initial s; state a "A"; state b "B"; state c "C"; final end; "#;
    let a = compile(&format!(
        "{head}s -> a; a -> b; b -> c; c -> a; c -> end }}"
    ))
    .unwrap();
    let b = compile(&format!(
        "{head}c -> end; c -> a; b -> c; a -> b; s -> a }}"
    ))
    .unwrap();
    for id in ["s", "a", "b", "c", "end"] {
        assert_eq!(rect(&a, id), rect(&b, id));
    }
    assert_routes(&a);
    assert_routes(&b);
}

#[test]
fn markers_are_circles_and_choices_can_be_labeled_or_unlabeled() {
    for label in ["", "\"Allowed?\""] {
        let c=compile(&format!(r#"diagram "Choice" mode=state-machine width=1400 {{
            initial start; state a "Review"; choice q {label}; state yes "Allow"; state no "Deny"; final end
            start -> a; a -> q "check"; q -> yes "[allowed]"; q -> no "[else]"; yes -> end; no -> end
        }}"#)).unwrap();
        assert_routes(&c);
        assert_eq!(rect(&c, "start").w, 20.0);
        assert_eq!(rect(&c, "end").w, 28.0);
        assert_eq!(
            c.scene
                .items
                .iter()
                .filter(|p| matches!(p.item, Item::StateMarker { .. }))
                .count(),
            2
        );
        let svg = layup::svg::render(&c, Theme::Dark);
        assert!(svg.contains("<circle class=\"box state-final\""));
        assert!(svg.contains("<title>Initial state</title>"));
        assert!(svg.contains("<polygon class=\"box"));
        let texts = c
            .scene
            .items
            .iter()
            .filter(|p| p.node == c.scene.node("q") && matches!(p.item, Item::Text(_)))
            .count();
        assert_eq!(texts, usize::from(!label.is_empty()));
        if label.is_empty() {
            assert_eq!(rect(&c, "q").w, 28.0);
            assert_eq!(rect(&c, "q").h, 28.0);
        }
    }
}

#[test]
fn machine_validation_reports_invalid_transitions_at_the_source_line() {
    for (body, expected) in [
        (
            "initial s; final end\n s -> end; end -> end",
            "final marker cannot have outgoing",
        ),
        (
            "initial s; state a\n s -> a; a -> s",
            "initial marker cannot have incoming",
        ),
        ("initial s; state a\n s -- a", "one directed arrow"),
        ("initial s; state a\n s <-> a", "one directed arrow"),
        (
            "initial s; group g { state a }\n s -> g",
            "not structural containers",
        ),
    ] {
        let error = compile(&format!(
            "diagram \"Invalid\" mode=state-machine {{\n {body}\n}}"
        ))
        .err()
        .unwrap();
        assert_eq!(error.line, Some(3));
        assert!(error.msg.contains(expected), "{}", error.msg);
    }
    for (body, expected) in [
        ("state a", "exactly one initial"),
        (
            "initial s; initial other; state a; s -> a; other -> a",
            "exactly one initial",
        ),
        ("initial s; state a", "exactly one outgoing"),
        (
            "initial s; state a; state b; s -> a; s -> b",
            "exactly one outgoing",
        ),
        (
            "initial s; choice q; final end; s -> q; q -> end",
            "at least two outgoing",
        ),
        ("initial s; node a; s -> a", "requires state, initial"),
    ] {
        let error = compile(&format!(
            "diagram \"Invalid\" mode=state-machine {{ {body} }}"
        ))
        .err()
        .unwrap();
        assert!(error.msg.contains(expected), "{}", error.msg);
    }
}

#[test]
fn unreachable_states_warn_but_running_machines_need_no_final_marker() {
    let c=compile("diagram \"Forever\" mode=state-machine {\n initial s; state active\n state isolated\n s -> active; active -> active \"tick\"\n}").unwrap();
    assert!(
        c.warnings.iter().any(|w| w.line == Some(3)
            && w.msg.contains("isolated")
            && w.msg.contains("unreachable"))
    );
    assert!(c.diagram.auto_layout);
}

#[test]
fn generic_graphs_keep_their_existing_semantics() {
    let c = compile(r#"diagram "Ordinary" { initial start; final end; end -> start }"#).unwrap();
    assert_eq!(c.diagram.mode, Mode::Graph);
    assert!(!c.diagram.auto_layout);
    assert!(c.warnings.iter().all(|w| !w.msg.contains("unreachable")));
    assert!(
        compile(r#"diagram "Bad mode" mode=simulation {}"#)
            .err()
            .unwrap()
            .msg
            .contains("mode must be")
    );
}

#[test]
fn custom_state_kinds_manual_rows_and_unicode_actions_work() {
    let c=compile(r#"diagram "申请" mode=state-machine layout=manual {
        style reviewing base=state tone=yellow
        row { initial start; reviewing review "مراجعة" text-direction=rtl { sub "entry / مراجعة الطلب" }; final end }
        start -> review "提交"; review -> end "通过 / 归档"
    }"#).unwrap();
    assert_routes(&c);
    assert!(!c.diagram.auto_layout);
    assert_eq!(c.diagram.kinds["reviewing"].shape, Shape::State);
    let svg = layup::svg::render(&c, Theme::Light);
    assert!(svg.contains("direction=\"rtl\""));
}

#[test]
fn marker_bodies_are_rejected_clearly() {
    for source in [
        r#"diagram "T" { initial s "Start" }"#,
        r#"diagram "T" { final end { code "done()" } }"#,
    ] {
        assert!(compile(source).is_err());
    }
}

#[test]
fn longer_horizontal_cycles_keep_positive_widths() {
    let mut source = String::from(
        "diagram \"Long cycle\" mode=state-machine direction=right width=1400 { initial start; final end; ",
    );
    for i in 0..12 {
        source.push_str(&format!("state q{i} \"I\"; "));
    }
    source.push_str("start -> q0; ");
    for i in 0..11 {
        source.push_str(&format!("q{i} -> q{}; ", i + 1));
    }
    source.push_str("q11 -> q0; q11 -> end }");
    let c = compile(&source).unwrap();
    assert_routes(&c);
    assert!(
        c.scene.nodes.iter().all(|n| n.rect.w > 0.0
            && n.rect.h > 0.0
            && n.rect.x.is_finite()
            && n.rect.y.is_finite())
    );
}
