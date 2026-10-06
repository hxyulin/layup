mod support;
use layup::{CompileOptions, Compiled, Error, text::Fonts};
fn selected(source: &str, view: &str) -> Result<Compiled, Error> {
    layup::compile_with_options(
        source,
        &CompileOptions {
            view: Some(view.into()),
            ..Default::default()
        },
        &Fonts::default(),
    )
}
const SOURCE: &str = r#"diagram system "System" type=graph layout=auto {
  node api
  group backend { node worker; node cache }
  edge work api -> backend.worker
  edge lookup backend.worker -> backend.cache
  view overview { include api backend }
  view detail { include api backend.worker }
}"#;
#[test]
fn default_view_expands_containers_and_detail_retains_ancestors() {
    let overview = layup::compile(SOURCE).unwrap();
    assert_eq!(overview.selected_view.as_deref(), Some("overview"));
    assert_eq!(overview.scene.nodes.len(), 4);
    let detail = selected(SOURCE, "detail").unwrap();
    assert_eq!(detail.scene.nodes.len(), 3);
    assert_eq!(detail.scene.edges.len(), 1);
    let worker = support::node(&detail.scene, "worker").unwrap();
    assert_eq!(
        detail.scene.nodes[worker].parent,
        support::node(&detail.scene, "backend")
    );
    assert_eq!(overview.scene.edges[0].id, detail.scene.edges[0].id);
    assert_eq!(
        detail.document.unwrap().objects["object:[\"backend\",\"worker\"]"]
            .path
            .as_ref()
            .unwrap(),
        &["backend", "worker"]
    );
}
#[test]
fn unselected_content_and_views_remain_validated() {
    for invalid in [
        SOURCE.replace("include api backend.worker", "include missing"),
        SOURCE.replace(
            "edge lookup backend.worker -> backend.cache",
            "edge lookup backend.worker -> missing",
        ),
        SOURCE.replace("edge lookup", "edge work"),
        SOURCE.replace("view detail", "view overview"),
    ] {
        assert!(selected(&invalid, "overview").is_err(), "{invalid}");
    }
    assert!(
        selected(SOURCE, "missing")
            .err()
            .unwrap()
            .msg
            .contains("unknown view")
    );
}
#[test]
fn annotations_on_views_and_steps_are_retained() {
    let source = r#"diagram main type=graph {
      node api
      @company.view(flag=true) view detail {
        include api
        @company.step(id=4) step reveal { show objects=[api] }
      }
    }"#;
    let compiled = layup::compile(source).unwrap();
    let targets = &compiled.document.as_ref().unwrap().targets;
    assert_eq!(
        targets
            .iter()
            .map(|t| t.category.as_str())
            .collect::<Vec<_>>(),
        ["view", "step"]
    );
}
#[test]
fn ordered_steps_resolve_object_and_connection_namespaces_independently() {
    let source = r#"diagram main type=sequence {
      participant api; participant worker
      message api api -> worker "Dispatch" delivery=async
      message reply worker -> api "Done" type=reply
      view detail {
        include api worker
        step start { show objects=[api, worker] connections=[api]; highlight connections=[api] }
        step end { show connections=[reply]; speaker-note "Done" }
      }
    }"#;
    let compiled = layup::compile(source).unwrap();
    assert_eq!(compiled.presentation.steps.len(), 2);
    assert_eq!(
        compiled.presentation.steps[0].visible_edges,
        ["relationship:\"api\""]
    );
    assert_eq!(compiled.presentation.steps[1].visible_edges.len(), 2);
    assert_eq!(compiled.presentation.steps[1].note.as_deref(), Some("Done"));
    assert!(compiled.scene.edges[0].asynchronous);
    assert!(compiled.scene.edges[1].dashed);
}
#[test]
fn view_slide_configuration_overrides_shared_configuration() {
    let source = r#"diagram main type=graph {
      slide size=wide padding=32 min-font-size=1
      node api
      view full { include api }
      view portrait { slide size={width: 720, height: 1280} padding=20 min-font-size=1; include api }
    }"#;
    assert_eq!(selected(source, "full").unwrap().viewport(), (1920., 1080.));
    let portrait = selected(source, "portrait").unwrap();
    assert_eq!(portrait.viewport(), (720., 1280.));
    assert_eq!(portrait.slide.unwrap().padding, 20.);
}
#[test]
fn formatting_keeps_all_selected_views_and_steps() {
    let source = include_str!("fixtures/diagrams/examples/presentation-model.layup");
    let formatted = layup::format::format(source).unwrap();
    assert_eq!(layup::format::format(&formatted).unwrap(), formatted);
    for view in ["overview", "walkthrough"] {
        let before = selected(source, view).unwrap();
        let after = selected(&formatted, view).unwrap();
        assert_eq!(before.presentation.json(), after.presentation.json());
        assert_eq!(
            before
                .scene
                .edges
                .iter()
                .map(|e| &e.points)
                .collect::<Vec<_>>(),
            after
                .scene
                .edges
                .iter()
                .map(|e| &e.points)
                .collect::<Vec<_>>()
        );
    }
}
