use layup::{
    Compiled, Theme, compile,
    geometry::Outline,
    layout::{Anchor, Item, Rect},
    model::Side,
    text::{Font, runs_width_with_fonts},
};

fn rect(c: &Compiled, id: &str) -> Rect {
    c.scene.nodes[c.scene.node(id).unwrap()].rect
}

fn assert_geometry(c: &Compiled) {
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    for edge in &c.scene.edges {
        for (id, end) in [(&edge.from, false), (&edge.to, true)] {
            let outline = c.scene.nodes[c.scene.node(id).unwrap()].outline;
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
            r#"diagram "Route" layout=auto width=1400 direction={direction} {{
            decision root "Urgent?"; decision next "Account issue?"; terminal incident "Incident"
            terminal account "Account support"; terminal help "Help center"
            root -> next "No"; root -> incident "Yes"; next -> account "Yes"; next -> help "No"
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
            r#"diagram "Route" layout=auto width=1400 direction={direction} {{
            decision root "Urgent?"; decision next "Account issue?"; terminal incident "Incident"
            terminal account "Account support"; terminal help "Help center"
            next -> help "No"; root -> incident "Yes"; next -> account "Yes"; root -> next "No"
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
        r#"diagram "International" layout=auto direction=left width=1200 {
        decision question "申请资料是否完整？" { sub "请确认联系方式。" }
        terminal yes "是"; terminal no "否"
        question -> yes "是"; question -> no "否"
    }"#,
    )
    .unwrap();
    assert_geometry(&c);
    let rtl = compile(
        r#"diagram "RTL" layout=auto text-direction=rtl {
        decision q "هل تمت الموافقة؟" { sub "يرجى التحقق من الطلب" }
        terminal yes "قبول"; terminal no "رفض"
        q -> yes "نعم"; q -> no "لا"
    }"#,
    )
    .unwrap();
    assert_geometry(&rtl);
    assert!(rtl.scene.items.iter().any(|p| matches!(&p.item, Item::Text(t) if p.node.is_some() && t.direction==layup::text::Direction::Rtl)));
}

#[test]
fn styles_links_and_explicit_ports_work_with_decisions() {
    let c = compile(
        r#"diagram "Custom" layout=auto direction=right width=1100 {
        style question shape=diamond tone=yellow
        question q "Ready?" href="https://example.com/ready" align=right
        terminal yes "Continue"; terminal no "Wait"
        q -> yes "Yes" from=right to=left
        q -> no "No" from=bottom to=left
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
        r#"diagram "Merge" layout=auto { decision q "Ready?"; process a "Prepare"; process b "Retry"; terminal end "Done"; q -> a "Yes"; q -> b "No"; a -> end; b -> end }"#,
        r#"diagram "Loop" layout=auto direction=right { decision q "Ready?"; process retry "Try again"; terminal end "Done"; q -> retry "No"; retry -> q; q -> end "Yes" }"#,
        r#"diagram "Self" { decision q "Ready?"; q -> q "Retry" }"#,
        r#"diagram "Hints" layout=auto { decision q "Ready?"; terminal a "Done" after=q; terminal b "Wait" same-layer=a; q -> a "Yes"; q -> b "No" }"#,
    ] {
        assert_geometry(&compile(src).unwrap());
    }
}

#[test]
fn decision_trees_can_live_inside_authored_containers() {
    let c=compile(r#"diagram "Nested" layout=auto { group flow "Approval" { decision q "Ready?"; terminal a "Yes"; terminal b "No"; q -> a "Yes"; q -> b "No" } }"#).unwrap();
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    let parent = c.scene.node("flow").unwrap();
    for id in ["q", "a", "b"] {
        assert_eq!(
            c.scene.nodes[c.scene.node(id).unwrap()].parent,
            Some(parent)
        );
    }
}

#[test]
fn compact_shapes_reject_nested_blocks_with_a_source_line() {
    for kind in ["decision", "process", "terminal"] {
        let error = compile(&format!(
            "diagram \"Invalid\" {{\n  {kind} q {{ node child }}\n}}"
        ))
        .err()
        .unwrap();
        assert_eq!(error.line, Some(2));
        assert!(error.msg.contains("cannot contain child blocks"));
    }
}

#[test]
fn multiway_decisions_keep_all_outcomes_in_one_rank() {
    let c = compile(
        r#"diagram "Classify" layout=auto width=1400 {
        decision q "Priority?"
        terminal p0 "Immediate"; terminal p1 "Urgent"; terminal p2 "Normal"; terminal p3 "Later"
        q -> p0 "P0"; q -> p1 "P1"; q -> p2 "P2"; q -> p3 "P3"
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
    let c = compile("diagram \"Too small\" layout=auto width=300 {\n decision q \"Ready?\"; terminal yes \"Yes\"\n q -> yes \"A branch caption much wider than the entire canvas and impossible to place\"\n}").unwrap();
    assert!(
        c.warnings
            .iter()
            .any(|w| w.line == Some(3) && w.msg.contains("no clear space for label"))
    );
}
