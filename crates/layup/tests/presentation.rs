use layup::{Theme, compile, html, parser, presentation, svg};

#[test]
fn cumulative_steps_include_ancestors_and_auto_edges_but_not_siblings() {
    let c = compile(
        r#"diagram "Steps" {
      node system "System" { node api "API"; node worker "Worker" }
      node external "External"
      api -> external
      api -> worker
      step intro "API entry" { show api; highlight api; note "Start here" }
      step request "Request" { show external }
      step processing "Processing" { show worker; highlight worker }
    }"#,
    )
    .unwrap();
    let steps = &c.presentation.steps;
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].id, "intro");
    assert_eq!(steps[0].note.as_deref(), Some("Start here"));
    assert!(steps[0].visible_nodes.contains(&"system".into()));
    assert!(steps[0].visible_nodes.contains(&"api".into()));
    assert!(!steps[0].visible_nodes.contains(&"worker".into()));
    assert!(steps[0].visible_edges.is_empty());
    assert_eq!(steps[1].visible_edges, ["edge-1"]);
    assert_eq!(steps[2].visible_edges, ["edge-1", "edge-2"]);
    assert!(steps[1].highlight_nodes.is_empty());
}

#[test]
fn showing_a_container_reveals_its_entire_subtree() {
    let c = compile(r#"diagram "Test" { node root { node nested { node leaf }; node peer }; node other; step all { show root } }"#).unwrap();
    let step = &c.presentation.steps[0];
    assert_eq!(step.visible_nodes.len(), 4);
    for id in ["root", "nested", "leaf", "peer"] {
        assert!(step.visible_nodes.contains(&id.into()));
    }
    assert!(!step.visible_nodes.contains(&"other".into()));
}

#[test]
fn explicit_edge_assignments_defer_even_when_endpoints_are_visible() {
    let c = compile(
        r#"diagram "Test" { node a; node b; a -> b "one"; b -> a "two";
      step nodes { show a b }
      step message { show-edge edge-1; highlight-edge edge-1 }
    }"#,
    )
    .unwrap();
    assert_eq!(c.presentation.steps[0].visible_edges, ["edge-2"]);
    assert_eq!(c.presentation.steps[1].visible_edges, ["edge-1", "edge-2"]);
    assert_eq!(c.presentation.steps[1].highlight_edges, ["edge-1"]);
}

#[test]
fn errors_have_the_exact_authored_target_span() {
    for (directive, code, target) in [
        ("show missing", "presentation/node", "missing"),
        ("show-edge edge-9", "presentation/edge", "edge-9"),
        ("highlight a", "presentation/hidden-highlight", "a"),
        ("show-edge edge-1", "presentation/hidden-endpoint", "edge-1"),
    ] {
        let source =
            format!("diagram \"Test\" {{ node a; node b; a -> b; step intro {{ {directive} }} }}");
        let error = compile(&source).err().unwrap();
        assert_eq!(error.code, code);
        let span = error.span.unwrap();
        assert_eq!(&source[span.start..span.end], target);
    }
}

#[test]
fn duplicate_ids_and_nested_steps_have_context() {
    let error =
        compile("diagram \"Test\" { node a; step intro { show a }; step intro { show a } }")
            .err()
            .unwrap();
    assert_eq!(error.code, "presentation/duplicate-step");
    assert_eq!(error.related.len(), 1);
    assert_eq!(
        error.span.as_ref().unwrap().end - error.span.as_ref().unwrap().start,
        "intro".len()
    );
    let error = compile("diagram \"Test\" { node a { step intro { show a } } }")
        .err()
        .unwrap();
    assert_eq!(error.code, "presentation/nested-step");
}

#[test]
fn malformed_directives_are_rejected_instead_of_silently_ignored() {
    for body in [
        "show",
        "note bare",
        "show a { node b }",
        "show a width=5",
        "node a",
        "a -> b",
        "note \"one\"; note \"two\"",
    ] {
        assert!(
            compile(&format!(
                "diagram \"Test\" {{ node a; node b; step intro {{ {body} }} }}"
            ))
            .is_err(),
            "{body}"
        );
    }
    for head in [
        "step",
        "step \"id\"",
        "step id bare",
        "step id \"Title\" extra",
        "step id",
    ] {
        assert!(
            compile(&format!("diagram \"Test\" {{ node a; {head} }}")).is_err(),
            "{head}"
        );
    }
}

#[test]
fn extraction_keeps_the_ordinary_graph_and_complete_static_svg() {
    let source = "diagram \"Test\" { node a; node b; a -> b; step intro { show a } }";
    let mut statements = parser::parse(source).unwrap();
    assert_eq!(presentation::extract(&mut statements).unwrap().len(), 1);
    let c = compile(source).unwrap();
    assert_eq!(c.scene.nodes.len(), 2);
    assert_eq!(c.scene.edges.len(), 1);
    let svg = svg::render(&c, Theme::Light);
    assert!(svg.contains("data-presentation="));
    assert!(svg.contains("data-edge-id=\"edge-1\""));
    assert!(!svg.contains("class=\"presentation-hidden\""));
    let html = html::render(&c, Theme::Light, false);
    assert!(html.contains("createPresentation(svg"));
    assert!(!html.contains("export function createPresentation"));
    assert!(!html.contains("{{PRESENTATION_SCRIPT}}"));
}

#[test]
fn explicitly_named_edges_can_be_targeted_without_node_id_ambiguity() {
    let c = compile(
        r#"diagram "Named edges" {
      node api; node worker; api -> worker id=api;
      step participants { show api worker }
      step request { show-edge api; highlight-edge api }
    }"#,
    )
    .unwrap();
    assert!(c.presentation.steps[0].visible_edges.is_empty());
    assert_eq!(c.presentation.steps[1].visible_edges, ["api"]);
    assert_eq!(c.presentation.steps[1].highlight_edges, ["api"]);
    assert!(svg::render(&c, Theme::Light).contains("data-edge-id=\"api\""));
}
