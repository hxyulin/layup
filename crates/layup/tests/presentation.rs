mod support;
use layup::{Theme, compile, html, svg};

#[test]
fn cumulative_steps_include_ancestors_and_auto_edges_but_not_siblings() {
    let c = compile(
        r#"diagram main "Steps" type=graph {
  node system "System" {
    node api "API"
    node worker "Worker"
  }
  node external "External"
  edge request ::system.api -> ::external
  edge processing ::system.api -> ::system.worker
  step intro "API entry" {
    show objects=[::system.api]
    highlight objects=[::system.api]
    speaker-note "Start here"
  }
  step request "Request" {
    show objects=[::external]
  }
  step processing "Processing" {
    show objects=[::system.worker]
    highlight objects=[::system.worker]
  }
}"#,
    )
    .unwrap();
    let steps = &c.presentation.steps;
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].id, "intro");
    assert_eq!(steps[0].note.as_deref(), Some("Start here"));
    assert!(
        steps[0]
            .visible_nodes
            .iter()
            .any(|id| support::authored(id) == "system")
    );
    assert!(
        steps[0]
            .visible_nodes
            .iter()
            .any(|id| support::authored(id) == "api")
    );
    assert!(
        !steps[0]
            .visible_nodes
            .iter()
            .any(|id| support::authored(id) == "worker")
    );
    assert!(steps[0].visible_edges.is_empty());
    assert_eq!(steps[1].visible_edges, ["relationship:\"request\""]);
    assert_eq!(
        steps[2].visible_edges,
        ["relationship:\"request\"", "relationship:\"processing\""]
    );
    assert!(steps[1].highlight_nodes.is_empty());
}

#[test]
fn showing_a_container_reveals_its_entire_subtree() {
    let c = compile(
        r#"diagram main "Test" type=graph {
  node root {
    node nested {
      node leaf
    }
    node peer
  }
  node other
  step all {
    show objects=[::root]
  }
}"#,
    )
    .unwrap();
    let step = &c.presentation.steps[0];
    assert_eq!(step.visible_nodes.len(), 4);
    for id in ["root", "nested", "leaf", "peer"] {
        assert!(
            step.visible_nodes
                .iter()
                .any(|render_id| support::authored(render_id) == id)
        );
    }
    assert!(
        !step
            .visible_nodes
            .iter()
            .any(|render_id| support::authored(render_id) == "other")
    );
}

#[test]
fn explicit_edge_assignments_defer_even_when_endpoints_are_visible() {
    let c = compile(
        r#"diagram main "Test" type=graph {
  node a
  node b
  edge request ::a -> ::b "one"
  edge processing ::b -> ::a "two"
  step nodes {
    show objects=[::a, ::b]
  }
  step message {
    show connections=[request]
    highlight connections=[request]
  }
}"#,
    )
    .unwrap();
    assert_eq!(
        c.presentation.steps[0].visible_edges,
        ["relationship:\"processing\""]
    );
    assert_eq!(
        c.presentation.steps[1].visible_edges,
        ["relationship:\"request\"", "relationship:\"processing\""]
    );
    assert_eq!(
        c.presentation.steps[1].highlight_edges,
        ["relationship:\"request\""]
    );
}

#[test]
fn errors_have_the_exact_authored_target_span() {
    for (directive, code, target) in [
        ("show objects=[missing]", "document/reference", "missing"),
        (
            "show connections=[missing]",
            "document/connection",
            "missing",
        ),
        (
            "highlight objects=[a]",
            "presentation/hidden-highlight",
            "a",
        ),
        (
            "show connections=[request]",
            "presentation/hidden-endpoint",
            "request",
        ),
    ] {
        let source = format!(
            "diagram main \"Test\" type=graph {{\n  node a\n  node b\n  edge request ::a -> ::b\n  step intro {{\n    {directive}\n  }}\n}}"
        );
        let error = compile(&source).err().unwrap();
        assert_eq!(error.code, code);
        let span = error.span.unwrap();
        assert_eq!(&source[span.start..span.end], target);
    }
}

#[test]
fn duplicate_ids_and_nested_steps_have_context() {
    let error =
        compile("diagram main \"Test\" type=graph {\n  node a\n  step intro {\n    show objects=[::a]\n  }\n  step intro {\n    show objects=[::a]\n  }\n}")
            .err()
            .unwrap();
    assert_eq!(error.code, "presentation/duplicate-step");
    assert_eq!(error.related.len(), 1);
    assert_eq!(
        error.span.as_ref().unwrap().end - error.span.as_ref().unwrap().start,
        "intro".len()
    );
    let error = compile("diagram main \"Test\" type=graph {\n  node a {\n    step intro {\n      show objects=[::a]\n    }\n  }\n}")
        .err()
        .unwrap();
    assert_eq!(error.code, "document/syntax");
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
                "diagram main \"Test\" type=graph {{\n  node a\n  node b\n  step intro {{\n    {body}\n  }}\n}}"
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
            compile(&format!(
                "diagram main \"Test\" type=graph {{\n  node a\n  {head}\n}}"
            ))
            .is_err(),
            "{head}"
        );
    }
}

#[test]
fn extraction_keeps_the_ordinary_graph_and_complete_static_svg() {
    let source = "diagram main \"Test\" type=graph {\n  node a\n  node b\n  edge request ::a -> ::b\n  step intro {\n    show objects=[::a]\n  }\n}";
    assert_eq!(layup::document::parse(source).unwrap().diagrams.len(), 1);
    let c = compile(source).unwrap();
    assert_eq!(c.scene.nodes.len(), 2);
    assert_eq!(c.scene.edges.len(), 1);
    let svg = svg::render(&c, Theme::Light);
    assert!(svg.contains("data-presentation="));
    assert!(svg.contains("data-edge-id=\"relationship:&quot;request&quot;\""));
    assert!(!svg.contains("class=\"presentation-hidden\""));
    let html = html::render(&c, Theme::Light, false);
    assert!(html.contains("createPresentation(svg"));
    assert!(!html.contains("export function createPresentation"));
    assert!(!html.contains("{{PRESENTATION_SCRIPT}}"));
}

#[test]
fn explicitly_named_edges_can_be_targeted_without_node_id_ambiguity() {
    let c = compile(
        r#"diagram main "Named edges" type=graph {
  node api
  node worker
  edge api ::api -> ::worker
  step participants {
    show objects=[::api, ::worker]
  }
  step request {
    show connections=[api]
    highlight connections=[api]
  }
}"#,
    )
    .unwrap();
    assert!(c.presentation.steps[0].visible_edges.is_empty());
    assert_eq!(
        c.presentation.steps[1].visible_edges,
        ["relationship:\"api\""]
    );
    assert_eq!(
        c.presentation.steps[1].highlight_edges,
        ["relationship:\"api\""]
    );
    assert!(
        svg::render(&c, Theme::Light).contains("data-edge-id=\"relationship:&quot;api&quot;\"")
    );
}
