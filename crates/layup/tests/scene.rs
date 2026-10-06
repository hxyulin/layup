use layup::{
    diagnostic::Span,
    layout::{Anchor, Ink, Item, Placed, Rect, TextItem},
    style::Tone,
    text::{Direction, Run},
};

const SOURCE: &str = r#"diagram "Architecture" layout=auto {
  group backend "Backend" {
    node api "API" href="https://example.com/docs?a=1&b=2" {
      code "fn handle()"
    }
    decision accepted "Accepted?"
  }
  terminal done "Done"
  api -> accepted "request" id=request dashed tone=purple
  accepted -> done "yes"
}"#;

#[test]
fn export_carries_version_original_geometry_references_and_source_spans() {
    let compiled = layup::compile(SOURCE).unwrap();
    let json = layup::scene::export(&compiled).unwrap();
    assert!(
        json.starts_with(
            "{\"version\":1,\"units\":\"svg-user-units\",\"coordinateSystem\":\"scene\""
        )
    );
    assert!(json.contains("\"title\":\"Architecture\",\"mode\":\"graph\""));
    assert!(json.contains("\"selectedView\":null,\"views\":[]"));
    assert!(json.contains("\"parentId\":\"backend\""));
    assert!(json.contains("\"id\":\"request\",\"from\":\"api\",\"to\":\"accepted\""));
    assert!(json.contains("\"style\":{\"tone\":\"purple\",\"dashed\":true,\"headStart\":false,\"headEnd\":true,\"bus\":false,\"asynchronous\":false}"));
    assert!(json.contains("\"nodeId\":\"api\",\"drawing\":"));
    assert!(json.contains("\"type\":\"diamond\""));
    assert!(json.contains("\"type\":\"rounded\""));
    assert!(json.contains("\"href\":\"https://example.com/docs?a=1&b=2\""));
    for node in &compiled.scene.nodes {
        let fragment = &SOURCE[node.span.start..node.span.end];
        assert!(fragment.starts_with(&node.kind));
        assert!(json.contains(&format!(
            "\"span\":{{\"start\":{},\"end\":{},\"line\":{}",
            node.span.start, node.span.end, node.span.line
        )));
    }
    for edge in &compiled.scene.edges {
        assert!(SOURCE[edge.span.start..edge.span.end].contains("->"));
        assert!(json.contains(&format!(
            "\"points\":[{{\"x\":{},\"y\":{}",
            edge.points[0].0, edge.points[0].1
        )));
    }
}

#[test]
fn slide_transform_is_separate_from_original_scene_geometry() {
    let compiled = layup::compile(
        "diagram \"Slide\" slide=wide slide-padding=64 min-font-size=20 { node api \"API\" }",
    )
    .unwrap();
    let json = layup::scene::export(&compiled).unwrap();
    let slide = compiled.slide.as_ref().unwrap();
    assert!(json.contains("\"viewport\":{\"width\":1920,\"height\":1080,\"slide\":{"));
    assert!(json.contains(&format!(
        "\"scale\":{},\"offsetX\":{},\"offsetY\":{},\"padding\":64",
        slide.scale, slide.offset_x, slide.offset_y
    )));
    let r = compiled.scene.nodes[0].rect;
    assert!(json.contains(&format!(
        "\"rect\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        r.x, r.y, r.w, r.h
    )));
}

#[test]
fn all_drawing_variants_preserve_text_style_direction_and_nested_groups() {
    let mut compiled = layup::compile("diagram \"Items\" { node a }").unwrap();
    let r = Rect {
        x: 1.1234567890123457,
        y: 2.5,
        w: 3.25,
        h: 4.75,
    };
    compiled.scene.items = vec![Placed {
        node: Some(0),
        item: Item::Group(vec![
            Item::Box {
                rect: r,
                tone: Tone::Blue,
                hollow: true,
                white: false,
                rx: 0.5,
                stroke_width: 1.25,
            },
            Item::StateMarker {
                rect: r,
                tone: Tone::Green,
                final_state: true,
            },
            Item::Diamond {
                rect: r,
                tone: Tone::Red,
                hollow: false,
                stroke_width: 1.0,
            },
            Item::Strip { rect: r, rx: 0.5 },
            Item::Rule {
                x1: 1.0,
                y1: 2.0,
                x2: 3.0,
                y2: 4.0,
                tone: None,
                dashed: true,
            },
            Item::Text(TextItem {
                x: 5.0,
                y: 6.0,
                anchor: Anchor::End,
                direction: Direction::Rtl,
                runs: vec![Run {
                    text: "مرحبا 中文 \"quote\"\n\\path\t\u{0000}".into(),
                    code: true,
                    tag: true,
                }],
                size: 13.25,
                weight: 650,
                mono: true,
                ink: Ink::Tone(Tone::Purple),
                letter_spacing: -0.125,
            }),
            Item::Chip {
                rect: r,
                text: "</script>".into(),
                tone: Tone::Orange,
                rotate: true,
                bordered: false,
            },
            Item::Sample {
                x: 8.0,
                y: 9.0,
                tone: Tone::Yellow,
                dashed: false,
            },
        ]),
    }];
    let json = layup::scene::export(&compiled).unwrap();
    for variant in [
        "group",
        "box",
        "state-marker",
        "diamond",
        "strip",
        "rule",
        "text",
        "chip",
        "sample",
    ] {
        assert!(json.contains(&format!("\"type\":\"{variant}\"")));
    }
    assert!(json.contains("\"x\":1.1234567890123457"));
    assert!(json.contains("\"anchor\":\"end\",\"direction\":\"rtl\""));
    assert!(json.contains(r#"مرحبا 中文 \"quote\"\n\\path\u0009\u0000"#));
    assert!(json.contains("\"code\":true,\"tag\":true"));
    assert!(json.contains("\"size\":13.25,\"weight\":650,\"mono\":true,\"ink\":{\"tone\":\"purple\"},\"letterSpacing\":-0.125"));
    assert!(json.contains("\"text\":\"</script>\""));
}

#[test]
fn export_identifies_fonts_without_serializing_payloads_or_file_paths() {
    let bytes = include_bytes!("../fonts/IBMPlexSans-Regular.ttf");
    let mut fonts = layup::text::Fonts::new();
    fonts.add_fallback(&bytes[..]).unwrap();
    let compiled =
        layup::compile_with_fonts("diagram \"字体\" { node a \"中文\" }", &fonts).unwrap();
    let json = layup::scene::export(&compiled).unwrap();
    assert!(json.contains("\"sans\":\"Layup Sans\",\"mono\":\"Layup Mono\""));
    assert!(json.contains("\"bundledFallbacks\":[\"Layup Arabic\",\"Layup Hebrew\"]"));
    assert!(json.contains("\"fallbacks\":[\"Layup User "));
    assert!(json.contains("\"systemCjk\":true"));
    assert!(!json.contains("data:font"));
    assert!(!json.contains("base64"));
    assert!(!json.contains("IBMPlexSans-Regular.ttf"));
    assert!(json.len() < bytes.len());
}

#[test]
fn selected_views_and_presentation_are_exported_together() {
    let source = r#"model "System" layout=auto {
      node a; node b; a -> b
      view overview "Overview" { include a b; step first { show a }; step second { show b; highlight b } }
      view detail "Detail" { include b; step only { show b } }
    }"#;
    let compiled = layup::compile_with_options(
        source,
        &layup::CompileOptions {
            view: Some("detail".into()),
            ..Default::default()
        },
        &layup::text::Fonts::default(),
    )
    .unwrap();
    let json = layup::scene::export(&compiled).unwrap();
    assert!(json.contains(
        "\"selectedView\":\"detail\",\"views\":[{\"id\":\"overview\",\"title\":\"Overview\""
    ));
    assert!(json.contains("\"presentation\":{\"version\":1,\"steps\":[{\"id\":\"only\""));
    assert!(json.contains("\"visibleNodes\":[\"b\"]"));
    assert!(json.contains("\"edges\":[]"));
}

#[test]
fn exported_warnings_include_available_source_ranges() {
    let mut compiled = layup::compile("diagram \"Warnings\" { node api }").unwrap();
    compiled.warnings.push(layup::Warning {
        line: Some(1),
        msg: "A \"warning\"\nwith context".into(),
    });
    let json = layup::scene::export(&compiled).unwrap();
    assert!(
        json.contains(
            "\"diagnostics\":[{\"line\":1,\"message\":\"A \\\"warning\\\"\\nwith context\""
        )
    );
    assert!(json.contains("\"severity\":\"warning\",\"code\":\"layout\",\"span\":{\"start\":"));
}

#[test]
fn nonfinite_geometry_is_rejected_at_every_nesting_level() {
    let mut compiled = layup::compile("diagram \"Finite\" { node a; node b; a -> b }").unwrap();
    let original = compiled.scene.width;
    compiled.scene.width = f64::NAN;
    assert!(layup::scene::export(&compiled).is_err());
    compiled.scene.width = original;
    compiled.scene.edges[0].points[0].0 = f64::INFINITY;
    assert!(layup::scene::export(&compiled).is_err());
    compiled.scene.edges[0].points[0].0 = 1.0;
    compiled.scene.items.push(Placed {
        node: None,
        item: Item::Group(vec![Item::Sample {
            x: f64::NEG_INFINITY,
            y: 0.0,
            tone: Tone::Gray,
            dashed: false,
        }]),
    });
    assert!(layup::scene::export(&compiled).is_err());
}

#[test]
fn invalid_scene_references_fail_without_panicking() {
    let mut compiled = layup::compile("diagram \"References\" { node a; node b; a -> b }").unwrap();
    compiled.scene.nodes[0].parent = Some(usize::MAX);
    assert!(layup::scene::export(&compiled).is_err());
    compiled.scene.nodes[0].parent = None;
    compiled.scene.edges[0].to = "missing".into();
    assert!(layup::scene::export(&compiled).is_err());
    compiled.scene.edges[0].to = "b".into();
    compiled.scene.items.push(Placed {
        node: Some(usize::MAX),
        item: Item::Group(vec![]),
    });
    assert!(layup::scene::export(&compiled).is_err());
}

#[test]
fn empty_source_ranges_remain_explicit_in_mutated_scenes() {
    let mut compiled = layup::compile("diagram \"Ranges\" { node a }").unwrap();
    compiled.scene.nodes[0].span = Span::default();
    let json = layup::scene::export(&compiled).unwrap();
    assert!(json.contains(
        "\"span\":{\"start\":0,\"end\":0,\"line\":0,\"column\":0,\"endLine\":0,\"endColumn\":0}"
    ));
}

#[test]
fn sequence_metadata_exposes_message_identity_async_style_and_fragment_geometry() {
    let source = r#"diagram "Protocol" mode=sequence {
      actor client "Client"
      participant api "API"
      loop "Retry" { client -> api "request" async id=request }
      note "Process" over=api
      api -> client "response" return
    }"#;
    let mut compiled = layup::compile(source).unwrap();
    let json = layup::scene::export(&compiled).unwrap();
    assert!(json.contains("\"mode\":\"sequence\""));
    assert!(json.contains("\"asynchronous\":true"));
    assert!(json.contains("\"sequence\":{\"participants\":[\"client\",\"api\"],\"messages\":[{\"id\":\"request\",\"asynchronous\":true"));
    assert!(json.contains("\"annotations\":[{\"kind\":\"loop\",\"label\":\"Retry\",\"rect\":{"));
    compiled.sequence.as_mut().unwrap().annotations[0].rect.w = f64::INFINITY;
    assert!(layup::scene::export(&compiled).is_err());
}
