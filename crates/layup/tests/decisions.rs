mod support;
use layup::{
    Compiled, Theme, compile,
    geometry::Outline,
    layout::{Anchor, Item, Rect},
    model::Side,
    text::{Font, runs_width_with_fonts},
};

fn rect(c: &Compiled, id: &str) -> Rect {
    c.scene.nodes[support::node(&c.scene, id).unwrap()].rect
}

fn assert_geometry(c: &Compiled) {
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    for edge in &c.scene.edges {
        for (id, end) in [(&edge.from, false), (&edge.to, true)] {
            let outline = c.scene.nodes[support::node(&c.scene, id).unwrap()].outline;
            let i = if end { edge.points.len() - 1 } else { 0 };
            let j = if end { i - 1 } else { 1 };
            let (p, q) = (edge.points[i], edge.points[j]);
            let side = if p.0 == q.0 {
                if q.1 < p.1 { Side::Top } else { Side::Bottom }
            } else if q.0 < p.0 {
                Side::Left
            } else {
                Side::Right
            };
            let along = if p.0 == q.0 { p.0 } else { p.1 };
            let expected = outline.port(side, along);
            assert!(
                (p.0 - expected.0).abs() < 0.001 && (p.1 - expected.1).abs() < 0.001,
                "{id}: {p:?} != {expected:?}"
            );
            assert!(
                !outline.segment_hits(p, q),
                "{id}: edge enters its endpoint interior"
            );
        }
        for segment in edge.points.windows(2) {
            assert!(segment[0].0 == segment[1].0 || segment[0].1 == segment[1].1);
            for n in &c.scene.nodes {
                assert!(
                    !n.outline.segment_hits(segment[0], segment[1]),
                    "{} -> {} crosses {}",
                    edge.from,
                    edge.to,
                    n.id
                );
            }
        }
        assert!(
            edge.points
                .iter()
                .all(|&(x, y)| x >= 0.0 && x <= c.scene.width && y >= 0.0 && y <= c.scene.height)
        );
    }
    for p in &c.scene.items {
        if let (Some(i), Item::Text(t)) = (p.node, &p.item) {
            let outline = c.scene.nodes[i].outline;
            let width = runs_width_with_fonts(
                &t.runs,
                if t.mono {
                    Font::Mono
                } else if t.weight >= 600 {
                    Font::SansBold
                } else {
                    Font::Sans
                },
                t.size,
                &c.scene.fonts,
            );
            let x = match t.anchor {
                Anchor::Start => t.x,
                Anchor::Middle => t.x - width / 2.0,
                Anchor::End => t.x - width,
            };
            for (px, py) in [
                (x, t.y - t.size),
                (x + width, t.y - t.size),
                (x, t.y + 3.0),
                (x + width, t.y + 3.0),
            ] {
                assert!(
                    outline.contains((px, py)),
                    "{} label outside outline at {px},{py}",
                    c.scene.nodes[i].id
                );
            }
        }
    }
}

#[test]
fn nested_decision_trees_use_subtree_lanes_in_every_direction() {
    for direction in ["down", "up", "right", "left"] {
        let c = compile(&format!(
            r#"diagram main "Route" type=graph layout=auto width=1400 flow-direction={direction} {{
  node root "Urgent?" style=decision
  node next "Account issue?" style=decision
  node incident "Incident" style=terminal
  node account "Account support" style=terminal
  node help "Help center" style=terminal
  ::root -> ::next "No"
  ::root -> ::incident "Yes"
  ::next -> ::account "Yes"
  ::next -> ::help "No"
}}"#
        ))
        .unwrap();
        assert_geometry(&c);
        let (n, a, b) = (rect(&c, "next"), rect(&c, "account"), rect(&c, "help"));
        if matches!(direction, "right" | "left") {
            assert!((n.cy() - (a.cy() + b.cy()) / 2.0).abs() < 0.001);
        } else {
            assert!((n.cx() - (a.cx() + b.cx()) / 2.0).abs() < 0.001);
        }
        let again = compile(&format!(
            r#"diagram main "Route" type=graph layout=auto width=1400 flow-direction={direction} {{
  node root "Urgent?" style=decision
  node next "Account issue?" style=decision
  node incident "Incident" style=terminal
  node account "Account support" style=terminal
  node help "Help center" style=terminal
  ::next -> ::help "No"
  ::root -> ::incident "Yes"
  ::next -> ::account "Yes"
  ::root -> ::next "No"
}}"#
        ))
        .unwrap();
        for id in ["root", "next", "incident", "account", "help"] {
            assert_eq!(rect(&c, id), rect(&again, id));
        }
    }
}

#[test]
fn outlines_intersect_slopes_and_rounded_corners() {
    let r = Rect {
        x: 0.0,
        y: 0.0,
        w: 200.0,
        h: 100.0,
    };
    let d = Outline::Diamond(r);
    assert_eq!(d.port(Side::Top, 50.0), (50.0, 25.0));
    assert_eq!(d.port(Side::Right, 25.0), (150.0, 25.0));
    assert!(!d.segment_hits((0.0, 10.0), (60.0, 10.0)));
    assert!(d.segment_hits((0.0, 50.0), (200.0, 50.0)));
    let safe = d.text_area(5.0);
    assert!(d.contains((safe.x, safe.y)) && d.contains((safe.right(), safe.bottom())));
    let rounded = Outline::Rounded {
        rect: r,
        radius: 50.0,
    };
    assert!(!rounded.contains((1.0, 1.0)));
    assert_eq!(rounded.port(Side::Right, 50.0), (200.0, 50.0));
    assert!(rounded.port(Side::Top, 25.0).1 > 0.0);
    let safe = rounded.text_area(1.0);
    assert!(rounded.contains((safe.x, safe.y)) && rounded.contains((safe.right(), safe.bottom())));
}

#[test]
fn unicode_multiline_questions_fit_the_safe_text_area() {
    let c = compile(
        r#"diagram main "International" type=graph layout=auto flow-direction=left width=1200 {
  node question "申请资料是否完整？" style=decision {
    text "请确认联系方式。"
  }
  node yes "是" style=terminal
  node no "否" style=terminal
  ::question -> ::yes "是"
  ::question -> ::no "否"
}"#,
    )
    .unwrap();
    assert_geometry(&c);
    let rtl = compile(
        r#"diagram main "RTL" type=graph layout=auto text-direction=rtl {
  node q "هل تمت الموافقة؟" style=decision {
    text "يرجى التحقق من الطلب"
  }
  node yes "قبول" style=terminal
  node no "رفض" style=terminal
  ::q -> ::yes "نعم"
  ::q -> ::no "لا"
}"#,
    )
    .unwrap();
    assert_geometry(&rtl);
    assert!(rtl.scene.items.iter().any(|p| matches!(&p.item, Item::Text(t) if p.node.is_some() && t.direction==layup::text::Direction::Rtl)));
}

#[test]
fn styles_links_and_explicit_ports_work_with_decisions() {
    let c = compile(
        r#"diagram main "Custom" type=graph layout=auto flow-direction=right width=1100 {
  node-style question shape=diamond palette=yellow
  node q "Ready?" style=question href="https://example.com/ready" text-align=right
  node yes "Continue" style=terminal
  node no "Wait" style=terminal
  ::q -> ::yes "Yes" source-side=right target-side=left
  ::q -> ::no "No" source-side=bottom target-side=left
}"#,
    )
    .unwrap();
    assert_geometry(&c);
    let svg = layup::svg::render(&c, Theme::Dark);
    assert!(svg.contains("<polygon class=\"box"));
    assert!(svg.contains("href=\"https://example.com/ready\""));
    assert!(svg.contains(" rx=\"19.5\""));
}

#[test]
fn merges_loops_and_hints_keep_general_graph_routing() {
    for src in [
        r#"diagram main "Merge" type=graph layout=auto {
  node q "Ready?" style=decision
  node a "Prepare" style=process
  node b "Retry" style=process
  node end "Done" style=terminal
  ::q -> ::a "Yes"
  ::q -> ::b "No"
  ::a -> ::end
  ::b -> ::end
}"#,
        r#"diagram main "Loop" type=graph layout=auto flow-direction=right {
  node q "Ready?" style=decision
  node retry "Try again" style=process
  node end "Done" style=terminal
  ::q -> ::retry "No"
  ::retry -> ::q
  ::q -> ::end "Yes"
}"#,
        r#"diagram main "Self" type=graph {
  node q "Ready?" style=decision
  ::q -> ::q "Retry"
}"#,
        r#"diagram main "Hints" type=graph layout=auto {
  node q "Ready?" style=decision
  node a "Done" style=terminal after=::q
  node b "Wait" style=terminal same-rank=::a
  ::q -> ::a "Yes"
  ::q -> ::b "No"
}"#,
    ] {
        assert_geometry(&compile(src).unwrap());
    }
}

#[test]
fn decision_trees_can_live_inside_authored_containers() {
    let c = compile(
        r#"diagram main "Nested" type=graph layout=auto {
  group flow "Approval" {
    node q "Ready?" style=decision
    node a "Yes" style=terminal
    node b "No" style=terminal
    ::flow.q -> ::flow.a "Yes"
    ::flow.q -> ::flow.b "No"
  }
}"#,
    )
    .unwrap();
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    let parent = support::node(&c.scene, "flow").unwrap();
    for id in ["q", "a", "b"] {
        assert_eq!(
            c.scene.nodes[support::node(&c.scene, id).unwrap()].parent,
            Some(parent)
        );
    }
}

#[test]
fn compact_shapes_reject_nested_blocks_with_a_source_line() {
    for kind in ["decision", "process", "terminal"] {
        let error = compile(&format!(
            "diagram main \"Invalid\" type=graph {{\n  node parent style={kind} {{\n    node child\n  }}\n}}"
        ))
        .err()
        .unwrap();
        assert_eq!(error.line, Some(2));
        assert!(error.msg.contains("cannot contain child diagrams"));
    }
}

#[test]
fn multiway_decisions_keep_all_outcomes_in_one_rank() {
    let c = compile(
        r#"diagram main "Classify" type=graph layout=auto width=1400 {
  node q "Priority?" style=decision
  node p0 "Immediate" style=terminal
  node p1 "Urgent" style=terminal
  node p2 "Normal" style=terminal
  node p3 "Later" style=terminal
  ::q -> ::p0 "P0"
  ::q -> ::p1 "P1"
  ::q -> ::p2 "P2"
  ::q -> ::p3 "P3"
}"#,
    )
    .unwrap();
    assert_geometry(&c);
    for id in ["p1", "p2", "p3"] {
        assert_eq!(rect(&c, "p0").y, rect(&c, id).y);
    }
    assert_eq!(c.scene.edges.iter().filter(|e| e.chip.is_some()).count(), 4);
}

#[test]
fn captions_without_room_report_the_edge_source_line() {
    let c = compile("diagram main \"Too small\" type=graph layout=auto width=300 {\n  node q \"Ready?\" style=decision\n  node yes \"Yes\" style=terminal\n  ::q -> ::yes \"A branch caption much wider than the entire canvas and impossible to place\"\n}").unwrap();
    assert!(c.warnings.iter().any(
        |w| w.line == Some(c.scene.edges[0].line) && w.msg.contains("no clear space for label")
    ));
}
