use layup::{Compiled, compile, layout::Item, model::Mode};

fn text_rows(item: &Item) -> Vec<String> {
    match item {
        Item::Text(text) => vec![text.runs.iter().map(|r| r.text.as_str()).collect()],
        Item::Group(children) => children.iter().flat_map(text_rows).collect(),
        Item::Chip { text, .. } => vec![text.clone()],
        _ => Vec::new(),
    }
}

fn cx(c: &Compiled, id: &str) -> f64 {
    c.scene.nodes[c.scene.node(id).unwrap()].rect.cx()
}

#[test]
fn authored_events_stay_ordered_and_message_kinds_are_preserved() {
    let source = r#"diagram "Chronology" mode=sequence {
      participant a "Client"; participant b "Service"
      a -> b "Request" id=request
      note "Check credentials" over=b
      b -> b "Validate" id=validate
      loop "Retry if needed" {
        b -> a "Retry" return id=retry
        a -> b "Re-submit" async id=resubmit
      }
      b -> a "Response" return id=response
    }"#;
    let c = compile(source).unwrap();
    assert_eq!(c.diagram.mode, Mode::Sequence);
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    let sequence = c.sequence.as_ref().unwrap();
    assert_eq!(sequence.participants, ["a", "b"]);
    assert_eq!(
        sequence
            .messages
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        ["request", "validate", "retry", "resubmit", "response"]
    );
    for (before, after) in c.scene.edges.iter().zip(c.scene.edges.iter().skip(1)) {
        assert!(before.points.last().unwrap().1 < after.points[0].1);
    }
    assert_eq!(c.scene.edges[1].points.len(), 4);
    assert_eq!(c.scene.edges[1].points.first().unwrap().0, cx(&c, "b"));
    assert_eq!(c.scene.edges[1].points.last().unwrap().0, cx(&c, "b"));
    assert!(c.scene.edges[2].dashed);
    assert!(c.scene.edges[3].asynchronous);
    assert!(!c.scene.edges[3].dashed);
    assert!(c.scene.edges[4].dashed);
    assert!(sequence.messages[3].asynchronous);
    assert!(sequence.annotations.iter().any(|a| a.kind == "note"));
    assert!(sequence.annotations.iter().any(|a| a.kind == "loop"));
    let source_span = c.scene.edges[0].span;
    assert!(source[source_span.start..source_span.end].contains("id=request"));
}

#[test]
fn left_direction_reverses_participants_without_reversing_time() {
    let source = r#"diagram "Order" mode=sequence direction=right {
      participant a "A"; participant b "B"; participant c "C"
      a -> b "First"; b -> c "Second"; c -> a "Third"
    }"#;
    let right = compile(source).unwrap();
    let left = compile(&source.replace("direction=right", "direction=left")).unwrap();
    assert!(cx(&right, "a") < cx(&right, "b") && cx(&right, "b") < cx(&right, "c"));
    assert!(cx(&left, "a") > cx(&left, "b") && cx(&left, "b") > cx(&left, "c"));
    for (a, b) in right.scene.edges.iter().zip(&left.scene.edges) {
        assert_eq!(
            (a.from.as_str(), a.to.as_str()),
            (b.from.as_str(), b.to.as_str())
        );
        assert_eq!(a.points[0].1, b.points[0].1);
    }
}

#[test]
fn alternatives_notes_and_fragments_have_measured_extents() {
    let source = r#"diagram "Fragments" mode=sequence {
      participant a "Browser"; participant b "API"
      note "Overview" from=a to=b
      alt "Response" {
        branch "Success" { b -> a "OK" id=ok }
        branch "Retry" { loop "Backoff" { a -> b "Retry" id=retry } }
      }
      opt "Cleanup" { note "Release resources" over=b; b -> b "Close" id=close }
    }"#;
    let c = compile(source).unwrap();
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    let annotations = &c.sequence.as_ref().unwrap().annotations;
    for a in annotations {
        assert!(a.rect.w > 0.0 && a.rect.h > 0.0);
        assert!(a.rect.x >= 0.0 && a.rect.right() <= c.scene.width);
        assert!(a.rect.y >= 0.0 && a.rect.bottom() <= c.scene.height);
    }
    let alt = annotations.iter().find(|a| a.kind == "alt").unwrap();
    let branches: Vec<_> = annotations.iter().filter(|a| a.kind == "branch").collect();
    assert_eq!(branches.len(), 2);
    for b in branches {
        assert!(b.rect.x > alt.rect.x && b.rect.right() < alt.rect.right());
        assert!(b.rect.y > alt.rect.y && b.rect.bottom() < alt.rect.bottom());
    }
    let note = annotations.iter().find(|a| a.label == "Overview").unwrap();
    assert!(note.rect.x < cx(&c, "a") && note.rect.right() > cx(&c, "b"));
}

#[test]
fn invalid_sequence_structure_has_source_diagnostics() {
    for (body, expected) in [
        ("", "at least one participant"),
        ("participant a; participant a", "duplicate"),
        ("participant a; a -> missing", "unknown node"),
        ("participant a; a -> a id=one; a -> a id=one", "duplicate"),
        ("participant \"Anonymous\"", "explicit id"),
        ("participant a { note \"Nested\" }", "nested body"),
        (
            "participant a; note \"Unknown\" over=missing",
            "unknown participant",
        ),
        ("participant a; note \"Half\" from=a", "both from="),
        ("participant a; note \"Mixed\" over=a to=a", "note takes"),
        ("participant a; note a", "quoted"),
        ("participant a; loop \"Empty\" {}", "event"),
        ("participant a; loop Retry { a -> a }", "quoted label"),
        ("participant a; loop \"Missing body\"", "body"),
        (
            "participant a; branch \"Orphan\" { a -> a }",
            "branches belong",
        ),
        (
            "participant a; alt \"Incomplete\" { branch \"One\" { a -> a } }",
            "at least two",
        ),
        ("participant a; alt \"Wrong\" { a -> a; a -> a }", "branch"),
        (
            "participant a; loop \"Nested declaration\" { participant b }",
            "unknown sequence statement",
        ),
        ("participant a; a -> a via=left", "routing ports"),
    ] {
        let source = format!("// 中文\ndiagram \"Invalid\" mode=sequence {{ {body} }}");
        let error = match compile(&source) {
            Ok(_) => panic!("accepted invalid sequence: {body}"),
            Err(e) => e,
        };
        assert!(error.msg.contains(expected), "{body}: {error}");
        assert_eq!(error.line, Some(2));
        let span = error.span.unwrap();
        assert!(span.end <= source.len() && span.start < span.end, "{body}");
    }
    let error = compile("diagram \"Wrong axis\" mode=sequence direction=down { participant a }")
        .err()
        .unwrap();
    assert_eq!(error.code, "sequence/direction");
}

#[test]
fn international_multiline_labels_and_user_fonts_use_measured_text() {
    let source = include_str!("../../../examples/sequence-international.layup");
    let c = compile(source).unwrap();
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    let first = c.scene.edges[0].chip.as_ref().unwrap();
    let rows = text_rows(first);
    assert_eq!(rows, ["发送请求", "إرسال الطلب"]);
    assert!(c.scene.edges[1].points[0].1 > c.scene.edges[0].points[0].1 + 28.0);
    let long_label = "中".repeat(40);
    let source = format!(
        "diagram \"Font metrics\" mode=sequence {{ participant a \"{long_label}\"; participant b \"B\"; a -> b \"{long_label}\" }}"
    );
    let default = compile(&source).unwrap();
    let mut fonts = layup::text::Fonts::new();
    fonts
        .add_fallback(include_bytes!("fonts/Fallback.ttf").as_slice())
        .unwrap();
    let supplied = layup::compile_with_fonts(&source, &fonts).unwrap();
    assert!(default.warnings.is_empty(), "{:?}", default.warnings);
    assert!(supplied.warnings.is_empty(), "{:?}", supplied.warnings);
    assert!(supplied.scene.width < default.scene.width);
}

#[test]
fn long_messages_expand_automatic_width_and_wrap_authored_width() {
    let label = "A long request label with parameters and context ".repeat(7);
    let source = format!(
        "diagram \"Labels\" mode=sequence {{ participant a; participant b; a -> b \"{label}\" }}"
    );
    let automatic = compile(&source).unwrap();
    assert!(automatic.scene.width > 1400.0);
    assert!(automatic.warnings.is_empty(), "{:?}", automatic.warnings);
    let authored = compile(&source.replace("mode=sequence", "mode=sequence width=900")).unwrap();
    assert_eq!(authored.scene.width, 900.0);
    assert!(authored.scene.height > automatic.scene.height);
    assert!(text_rows(authored.scene.edges[0].chip.as_ref().unwrap()).len() > 1);
    assert!(authored.warnings.is_empty(), "{:?}", authored.warnings);
}

#[test]
fn slides_and_reveal_steps_keep_sequence_message_geometry_and_identity() {
    let source = r#"diagram "Playback" mode=sequence {
      participant client; participant api
      client -> api "Request" id=request
      api -> client "Response" return id=response
    }"#;
    let plain = compile(source).unwrap();
    let authored = source
        .replace("mode=sequence", "mode=sequence slide=wide min-font-size=1")
        .replace("\n    }", "\n      step first { show client api; show-edge request; highlight-edge request }\n      step second { show-edge response; highlight-edge response }\n    }");
    let c = compile(&authored).unwrap();
    assert_eq!(c.presentation.steps.len(), 2);
    assert_eq!(c.presentation.steps[0].visible_edges, ["request"]);
    assert_eq!(
        c.presentation.steps[1].visible_edges,
        ["request", "response"]
    );
    assert_eq!(c.viewport(), (1920.0, 1080.0));
    for (before, after) in plain.scene.edges.iter().zip(&c.scene.edges) {
        assert_eq!(before.points, after.points);
        assert_eq!(before.id, after.id);
        assert_eq!(
            text_rows(before.chip.as_ref().unwrap()),
            text_rows(after.chip.as_ref().unwrap())
        );
    }
    let svg = layup::svg::render(&c, layup::Theme::Light);
    assert!(svg.contains("data-edge-id=\"request\""));
    assert!(svg.contains("data-edge-id=\"response\""));
}

#[test]
fn sequence_examples_compile_without_warnings() {
    for source in [
        include_str!("../../../examples/sequence.layup"),
        include_str!("../../../examples/sequence-international.layup"),
    ] {
        let c = compile(source).unwrap();
        assert!(c.warnings.is_empty(), "{:?}", c.warnings);
        assert!(c.sequence.is_some());
        assert_eq!(
            c.scene.edges.len(),
            c.sequence.as_ref().unwrap().messages.len()
        );
    }
}

#[test]
fn selected_sequence_views_keep_message_identity_and_authored_order() {
    let source = r#"model "Shared interactions" mode=sequence slide=wide min-font-size=1 {
      participant client "Client"; participant api "API"; participant store "Store"
      client -> api "Request" id=request
      api -> store "Query" id=query
      store -> api "Records" return id=records
      api -> client "Response" return id=response
      view overview "Client and API" { include client api }
      view data "Storage access" { include api store }
    }"#;
    let fonts = layup::text::Fonts::default();
    for (view, participants, messages) in [
        (
            "overview",
            vec!["client", "api"],
            vec!["request", "response"],
        ),
        ("data", vec!["api", "store"], vec!["query", "records"]),
    ] {
        let options = layup::CompileOptions {
            view: Some(view.into()),
            ..Default::default()
        };
        let c = layup::compile_with_options(source, &options, &fonts).unwrap();
        assert!(c.warnings.is_empty(), "{:?}", c.warnings);
        let info = c.sequence.as_ref().unwrap();
        assert_eq!(info.participants, participants);
        assert_eq!(
            info.messages
                .iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>(),
            messages
        );
        assert_eq!(c.selected_view.as_deref(), Some(view));
        assert_eq!(c.viewport(), (1920.0, 1080.0));
    }
}

#[test]
fn participant_id_attributes_follow_existing_node_id_precedence() {
    for declaration in [
        "participant \"API\" id=api",
        "participant alias \"API\" id=api",
        "actor alias \"User\" id=api",
        "participant blue \"API\" id=api",
    ] {
        let source = format!(
            "diagram \"Authored IDs\" mode=sequence {{ {declaration}; api -> api \"Local work\" }}"
        );
        let c = compile(&source).unwrap();
        assert_eq!(c.sequence.as_ref().unwrap().participants, ["api"]);
        assert_eq!(c.scene.edges[0].from, "api");
        assert_eq!(c.scene.edges[0].to, "api");
    }
}

#[test]
fn participant_styles_monospace_headers_and_roles_use_measured_layout() {
    let source = r#"diagram "Styled participants" mode=sequence width=900 {
      style participant process mono blue
      style actor process hollow purple
      actor user "User"
      participant api "ConnectionPool<RequestContext>" role="The service owns authorization, validation, processing and persistence for every incoming request."
      user -> api "Call"
    }"#;
    let c = compile(source).unwrap();
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    assert!(c.diagram.kinds["participant"].mono);
    assert!(c.diagram.kinds["actor"].hollow);
    let api = c.scene.node("api").unwrap();
    let texts: Vec<_> = c
        .scene
        .items
        .iter()
        .filter_map(|p| match &p.item {
            Item::Text(t) if p.node == Some(api) => Some(t),
            _ => None,
        })
        .collect();
    let header = texts.iter().find(|t| t.size == 14.0).unwrap();
    assert!(header.mono);
    assert_eq!(header.weight, 400);
    assert!(texts.iter().filter(|t| t.size == 11.5).count() > 1);
    let api_box = c.scene.nodes[api].rect;
    for t in texts {
        assert!(t.y > api_box.y && t.y < api_box.bottom());
    }
}

#[test]
fn rtl_titles_subtitles_and_fragment_headings_anchor_to_the_right() {
    use layup::{layout::Anchor, text::Direction};
    let source = r#"diagram "طلب متعدد الخطوات" mode=sequence text-direction=rtl {
      subtitle "تفاصيل معالجة الطلب"
      participant client "عميل"; participant api "خادم"
      loop "إعادة المحاولة" { client -> api "طلب" }
    }"#;
    let c = compile(source).unwrap();
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    let headings: Vec<_> = c
        .scene
        .items
        .iter()
        .filter_map(|p| match &p.item {
            Item::Text(t) if p.node.is_none() => Some(t),
            _ => None,
        })
        .collect();
    assert!(headings.len() >= 3);
    for heading in headings {
        assert_eq!(heading.direction, Direction::Rtl);
        assert_eq!(heading.anchor, Anchor::End);
        assert!(heading.x > c.scene.width / 2.0 && heading.x < c.scene.width);
    }
}

#[test]
fn invalid_sequence_layout_and_unsupported_legend_are_diagnosed() {
    let invalid = compile("diagram \"Bad layout\" mode=sequence layout=mystery { participant a }")
        .err()
        .unwrap();
    assert_eq!(invalid.code, "sequence/layout");
    assert!(invalid.span.is_some());
    for declaration in ["legend off", "legend arrows", "legend { participant a }"] {
        let source = format!("diagram \"Legend\" mode=sequence {{ participant a; {declaration} }}");
        let invalid = compile(&source).err().unwrap();
        assert!(invalid.msg.contains("legend"));
        assert!(invalid.span.is_some());
    }
}

#[test]
fn deep_fragments_fail_before_their_geometry_becomes_negative() {
    let source = format!(
        "diagram \"Depth\" mode=sequence width=900 {{\nparticipant a\n{}a -> a \"Work\"\n{}}}",
        "loop \"Nested\" {\n".repeat(40),
        "}\n".repeat(40),
    );
    let invalid = compile(&source).err().unwrap();
    assert_eq!(invalid.code, "sequence/fragment-width");
    assert!(invalid.line.unwrap() > 2);
    let span = invalid.span.unwrap();
    assert!(span.start < span.end && span.end <= source.len());
}

#[test]
fn missing_glyphs_in_multiline_message_and_note_groups_are_reported() {
    let source = "diagram \"Glyph coverage\" mode=sequence {\nparticipant a; participant b\na -> b \"Request\\n𐍈\"\nnote \"Context\\n𓀀\" over=b\n}";
    let c = compile(source).unwrap();
    let missing: Vec<_> = c
        .warnings
        .iter()
        .filter(|w| w.msg.contains("outside the bundled fonts"))
        .collect();
    assert_eq!(missing.len(), 2);
    assert!(
        missing
            .iter()
            .any(|w| w.msg.contains("U+10348") && w.line == Some(3))
    );
    assert!(missing.iter().any(|w| w.msg.contains("U+13000")));
    assert_eq!(
        text_rows(c.scene.edges[0].chip.as_ref().unwrap()),
        ["Request", "𐍈"]
    );
}
