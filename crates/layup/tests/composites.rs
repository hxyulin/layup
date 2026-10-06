mod support;
use layup::{
    Compiled, compile,
    layout::{Item, Rect},
};

fn rect(c: &Compiled, id: &str) -> Rect {
    c.scene.nodes[support::node(&c.scene, id).unwrap()].rect
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
                let a = support::node(&c.scene, &e.from).unwrap();
                let b = support::node(&c.scene, &e.to).unwrap();
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
        let connected = support::node(&c.scene, "connected").unwrap();
        let sending = support::node(&c.scene, "sending").unwrap();
        assert_eq!(c.scene.nodes[sending].parent, Some(connected));
        assert_eq!(
            c.scene.nodes[support::node(&c.scene, "waiting").unwrap()].parent,
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
            "initial s; state parent { initial a; state child; a -> child };\n s -> parent.child",
            "initial transition must target",
            3,
        ),
        (
            "initial s; state parent { initial a; state child; final done; a -> child; child -> done }; state outside; s -> parent;\n outside -> parent.done",
            "final marker must be entered",
            3,
        ),
        (
            "initial s; state parent { initial a; state child; a -> child }; s -> parent;\n parent.child -> parent.a",
            "initial marker cannot have incoming",
            3,
        ),
        (
            "initial s; state parent { initial a; node invalid; a -> invalid }; s -> parent",
            "not a statement",
            2,
        ),
    ] {
        let e = compile(&format!(
            "diagram main \"Invalid\" type=state-machine {{\n  {body}\n}}"
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
        let c = compile(&format!(
            r#"diagram main "跨边界" type=state-machine flow-direction={d} {{
  initial root
  state outside "外部"
  state parent "متصل" text-direction=rtl {{
    initial enter
    state ready "准备"
    state work "工作"
    transition connection-1 ::parent.enter -> ::parent.ready
    transition connection-2 ::parent.ready -> ::parent.work "执行"
  }}
  final done
  transition connection-3 ::root -> ::outside
  transition connection-4 ::outside -> ::parent "进入"
  transition connection-5 ::parent.work -> ::outside "退出"
  transition connection-6 ::parent.work -> ::done "结束"
}}"#
        ))
        .unwrap();
        check(&c);
        assert!(
            c.scene
                .edges
                .iter()
                .any(|e| support::authored(&e.from) == "work"
                    && support::authored(&e.to) == "outside")
        );
        assert!(c.scene.edges.iter().any(
            |e| support::authored(&e.from) == "outside" && support::authored(&e.to) == "parent"
        ));
    }
}

#[test]
fn direct_descendant_entry_and_unreachable_composites_are_checked_structurally() {
    let c = compile(
        r#"diagram main "Direct" type=state-machine {
  initial root
  state a
  state parent {
    initial enter
    state b
    state skipped
    transition connection-1 ::parent.enter -> ::parent.skipped
  }
  state unreachable {
    initial dead
    state isolated
    transition connection-2 ::unreachable.dead -> ::unreachable.isolated
  }
  transition connection-3 ::root -> ::a
  transition connection-4 ::a -> ::parent.b
}"#,
    )
    .unwrap();
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
    assert!(
        warnings
            .iter()
            .any(|w| w.msg.contains("unreachable") && w.msg.contains("enter"))
    );
}

#[test]
fn transitions_between_a_composite_and_its_descendants_have_real_ports() {
    for d in ["down", "up", "right", "left"] {
        let c = compile(&format!(
            r#"diagram main "Local transitions" type=state-machine flow-direction={d} {{
  initial root
  state parent "Parent" {{
    initial enter
    state a "A"
    state b "B"
    transition connection-1 ::parent.enter -> ::parent.a
    transition connection-2 ::parent.a -> ::parent.b
  }}
  transition connection-3 ::root -> ::parent
  transition connection-4 ::parent.b -> ::parent "reset"
  transition connection-5 ::parent -> ::parent.a "resume"
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
        r#"diagram main "Manual composite" type=state-machine layout=manual width=1200 {
  node-style online base=state palette=blue
  row weights=[1, 3, 1] {
    initial root
    state parent "Online" style=online {
      row {
        initial enter
        state ready "Ready"
        final finished
      }
      transition connection-1 ::parent.enter -> ::parent.ready
      transition connection-2 ::parent.ready -> ::parent.finished "complete"
    }
    final end
  }
  transition connection-3 ::root -> ::parent
  transition connection-4 ::parent -> ::end "stop"
}"#,
    )
    .unwrap();
    check(&c);
    assert!(!c.diagram.auto_layout);
    assert_eq!(c.scene.width, 1200.0);
    assert_eq!(
        c.scene.nodes[support::node(&c.scene, "ready").unwrap()].parent,
        support::node(&c.scene, "parent")
    );
}

#[test]
fn reordering_nested_transition_statements_preserves_geometry() {
    let a = include_str!("../../../examples/state-composite.layup");
    let mut lines = a.lines().map(str::to_owned).collect::<Vec<_>>();
    let positions = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("      transition "))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let reversed = positions
        .iter()
        .rev()
        .map(|i| lines[*i].clone())
        .collect::<Vec<_>>();
    for (i, line) in positions.into_iter().zip(reversed) {
        lines[i] = line;
    }
    let b = lines.join("\n");
    assert_ne!(a, b);
    let a = compile(a).unwrap();
    let b = compile(&b).unwrap();
    for node in &a.scene.nodes {
        assert_eq!(node.rect, rect(&b, &node.id));
    }
}
