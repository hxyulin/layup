use layup::{compile, model};

#[test]
fn public_model_builder_accepts_the_same_document_preprocessing() {
    for source in [
        include_str!("../../../examples/presentation-model.layup"),
        include_str!("../../../examples/model-views.layup"),
        include_str!("../../../examples/slides.layup"),
        include_str!("../../../examples/language-v1.layup"),
    ] {
        let typed = model::build(source).unwrap();
        let compiled = compile(source).unwrap();
        assert_eq!(typed.mode, compiled.diagram.mode);
        assert_eq!(typed.title, compiled.diagram.title);
        assert_eq!(typed.edges.len(), compiled.scene.edges.len());
        assert_eq!(
            typed.edges.iter().map(|e| &e.id).collect::<Vec<_>>(),
            compiled
                .scene
                .edges
                .iter()
                .map(|e| &e.id)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn explicit_empty_edge_ids_fail_instead_of_becoming_generated_ids() {
    let source = "diagram \"IDs\" { node a; node b; a -> b id=\"\" }";
    let error = compile(source).err().unwrap();
    assert_eq!(error.code, "semantic/edge-id");
    assert!(error.span.is_some());
}

#[test]
fn view_filtering_does_not_make_shared_declarations_unused() {
    let source = r#"model "Views" {
        style service node blue
        style abandoned node
        arrow query green
        arrow abandoned gray
        service api "API"
        node store "Store"
        api -query-> store
        view full { include api store }
        view storage { include store }
    }"#;
    let options = layup::CompileOptions {
        view: Some("storage".into()),
        ..Default::default()
    };
    let diagnostics =
        layup::lint::lint_with_options(source, &options, &layup::text::Fonts::default());
    let declarations: Vec<_> = diagnostics
        .iter()
        .filter(|d| matches!(d.code, "lint/unused-style" | "lint/unused-arrow"))
        .collect();
    assert_eq!(declarations.len(), 2);
    assert!(declarations.iter().all(|d| d.message.contains("abandoned")));
}
