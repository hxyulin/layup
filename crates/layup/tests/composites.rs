use layup::{
    Compiled, compile,
    layout::{Item, Rect},
};

fn rect(c: &Compiled, id: &str) -> Rect {
    c.scene.nodes[c.scene.node(id).unwrap()].rect
}
fn check(c: &Compiled) {
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    for n in &c.scene.nodes {
        if let Some(p) = n.parent {
            let r = c.scene.nodes[p].rect;
            assert!(
                n.rect.x > r.x
                    && n.rect.right() < r.right()
                    && n.rect.y > r.y
                    && n.rect.bottom() < r.bottom(),
                "{} outside parent",
                n.id
            );
        }
        assert!(n.rect.w > 0.0 && n.rect.h > 0.0);
    }
    for e in &c.scene.edges {
        for pair in e.points.windows(2) {
            assert!(
                pair[0].0 == pair[1].0 || pair[0].1 == pair[1].1,
                "{} -> {} {:?}",
                e.from,
                e.to,
                e.points
            );
            for (i, n) in c.scene.nodes.iter().enumerate() {
                // Borders of containing states may be crossed on entry/exit.
                let a = c.scene.node(&e.from).unwrap();
                let b = c.scene.node(&e.to).unwrap();
                if c.scene.is_descendant(a, i) || c.scene.is_descendant(b, i) {
                    continue;
                }
                assert!(
                    !n.outline.segment_hits(pair[0], pair[1]),
                    "{} -> {} crosses {}",
                    e.from,
                    e.to,
                    n.id
                );
            }
        }
    }
}

#[test]
fn nested_composites_have_scoped_initials_and_contained_geometry_in_all_directions() {
    let source = include_str!("../../../examples/state-composite.layup");
    for d in ["down", "up", "right", "left"] {
        let c = compile(&source.replace("direction=right", &format!("direction={d}"))).unwrap();
        check(&c);
        let connected = c.scene.node("connected").unwrap();
        let sending = c.scene.node("sending").unwrap();
        assert_eq!(c.scene.nodes[sending].parent, Some(connected));
        assert_eq!(
            c.scene.nodes[c.scene.node("waiting").unwrap()].parent,
            Some(sending)
        );
        assert!(rect(&c, "connected").w > 240.0);
        assert_eq!(c.scene.edges.len(), 12);
        assert!(
            c.scene
                .items
                .iter()
                .any(|p| p.node == Some(connected)
                    && matches!(p.item, Item::Box { hollow: true, .. }))
        );
    }
}

#[test]
fn composite_validation_reports_scope_and_transition_source_lines() {
    for (body, expected, line) in [
        (
            "initial s; state parent { state child }; s -> parent",
            "composite state `parent` requires exactly one initial",
            2,
        ),
        (
            "initial s; state parent { initial a; initial b; state child; a -> child; b -> child }; s -> parent",
            "exactly one initial",
            2,
        ),
        (
            "initial s; state parent { initial a; state child; a -> child };\n s -> child",
            "initial transition must target",
            3,
        ),
        (
            "initial s; state parent { initial a; state child; final done; a -> child; child -> done }; state outside; s -> parent;\n outside -> done",
            "final marker must be entered",
            3,
        ),
        (
            "initial s; state parent { initial a; state child; a -> child }; s -> parent;\n child -> a",
            "initial marker cannot have incoming",
            3,
        ),
        (
            "initial s; state parent { initial a; node invalid; a -> invalid }; s -> parent",
            "requires state, initial",
            2,
        ),
    ] {
        let e = compile(&format!(
            "diagram \"Invalid\" mode=state-machine {{\n{body}\n}}"
        ))
        .err()
        .unwrap();
        assert!(e.msg.contains(expected), "{}", e.msg);
        assert_eq!(e.line, Some(line));
    }
}

#[test]
fn cross_boundary_transitions_retain_original_endpoints() {
    for d in ["down", "up", "right", "left"] {
        let c=compile(&format!(r#"diagram "跨边界" mode=state-machine direction={d} {{
            initial root; state outside "外部"
            state parent "متصل" text-direction=rtl {{ initial enter; state ready "准备"; state work "工作"; enter -> ready; ready -> work "执行" }}
            final done
            root -> outside; outside -> parent "进入"; work -> outside "退出"; work -> done "结束"
        }}"#)).unwrap();
        check(&c);
        assert!(
            c.scene
                .edges
                .iter()
                .any(|e| e.from == "work" && e.to == "outside")
        );
        assert!(
            c.scene
                .edges
                .iter()
                .any(|e| e.from == "outside" && e.to == "parent")
        );
    }
}

#[test]
fn direct_descendant_entry_and_unreachable_composites_are_checked_structurally() {
    let c=compile(r#"diagram "Direct" mode=state-machine {
        initial root; state a; state parent { initial enter; state b; state skipped; enter -> skipped }
        state unreachable { initial dead; state isolated; dead -> isolated }
        root -> a; a -> b
    }"#).unwrap();
    let warnings: Vec<_> = c
        .warnings
        .iter()
        .filter(|w| w.msg.contains("unreachable"))
        .collect();
    assert!(
        !warnings
            .iter()
            .any(|w| w.msg.contains("`parent`") || w.msg.contains("`b`"))
    );
    assert!(warnings.iter().any(|w| w.msg.contains("`unreachable`")));
    assert!(warnings.iter().any(|w| w.msg.contains("`enter`")));
}

#[test]
fn transitions_between_a_composite_and_its_descendants_have_real_ports() {
    for d in ["down", "up", "right", "left"] {
        let c = compile(&format!(
            r#"diagram "Local transitions" mode=state-machine direction={d} {{
            initial root
            state parent "Parent" {{ initial enter; state a "A"; state b "B"; enter -> a; a -> b }}
            root -> parent
            b -> parent "reset"
            parent -> a "resume"
        }}"#
        ))
        .unwrap();
        check(&c);
        for e in &c.scene.edges {
            for (id, p) in [(&e.from, e.points[0]), (&e.to, *e.points.last().unwrap())] {
                let r = rect(&c, id);
                assert!(p.0 >= r.x && p.0 <= r.right() && p.1 >= r.y && p.1 <= r.bottom());
            }
        }
    }
}

#[test]
fn authored_rows_and_custom_kinds_preserve_composite_scopes() {
    let c = compile(
        r#"diagram "Manual composite" mode=state-machine layout=manual width=1200 {
        style online base=state tone=blue
        row 1:3:1 {
            initial root
            online parent "Online" {
                row { initial enter; state ready "Ready"; final finished }
                enter -> ready
                ready -> finished "complete"
            }
            final end
        }
        root -> parent
        parent -> end "stop"
    }"#,
    )
    .unwrap();
    check(&c);
    assert!(!c.diagram.auto_layout);
    assert_eq!(c.scene.width, 1200.0);
    assert_eq!(
        c.scene.nodes[c.scene.node("ready").unwrap()].parent,
        c.scene.node("parent")
    );
}

#[test]
fn reordering_nested_transition_statements_preserves_geometry() {
    let a = include_str!("../../../examples/state-composite.layup");
    let b = a.replace("      begin -> waiting\n      waiting -> retry \"timeout\"\n      retry -> waiting \"retry\"\n      waiting -> delivered \"ack\"", "      waiting -> delivered \"ack\"\n      retry -> waiting \"retry\"\n      waiting -> retry \"timeout\"\n      begin -> waiting").replace("  start -> offline\n  offline -> connected \"connect\"\n  connected -> offline \"disconnect\"\n  connected -> stopped \"shutdown\"", "  connected -> stopped \"shutdown\"\n  connected -> offline \"disconnect\"\n  offline -> connected \"connect\"\n  start -> offline");
    assert_ne!(a, b);
    let a = compile(a).unwrap();
    let b = compile(&b).unwrap();
    for node in &a.scene.nodes {
        assert_eq!(node.rect, rect(&b, &node.id));
    }
}
