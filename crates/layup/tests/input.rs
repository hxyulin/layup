use layup::{CompileOptions, input, scene, text::Fonts};
use serde_json::{Value, json};

fn graph() -> Value {
    json!({"version": 1, "title": "Generated \"API\"", "direction": "right",
        "nodes": [
            {"id": "src/api.rs::API<T>#", "title": "API", "kind": "group", "sourceLocations": [{"uri": "src/api.rs", "symbol": "API<T>", "range": {"startLine": 2, "startColumn": 1, "endLine": 5, "endColumn": 2}}]},
            {"id": "API::run(&self)", "title": "运行 مرحبا", "kind": "function", "parentId": "src/api.rs::API<T>#", "code": ["run(\"value\")"], "href": "https://example.test/src/api.rs#L3", "metadata": {"confidence": 0.8}},
            {"id": "db::save", "title": "Save", "sourceLocations": [{"uri": "src/db.rs"}]}
        ],
        "edges": [{"id": "call@src/api.rs:3", "from": "API::run(&self)", "to": "db::save", "kind": "calls", "sourceLocations": [{"uri": "src/api.rs", "symbol": "run"}], "metadata": {"evidence": "resolved-call"}}],
        "views": [{"id": "overview", "title": "Overview", "include": ["db::save"]}, {"id": "detail", "title": "Detail", "include": ["API::run(&self)", "db::save"]}],
        "provenance": {"analyzer": "fixture", "version": "1"}
    })
}

#[test]
fn structured_input_preserves_symbols_hierarchy_views_and_original_locations() {
    let source = graph().to_string();
    let options = CompileOptions {
        view: Some("detail".into()),
        ..Default::default()
    };
    let compiled = input::compile_json(&source, &options, &Fonts::new()).unwrap();
    let result: Value = serde_json::from_str(&scene::export(&compiled).unwrap()).unwrap();
    assert_eq!(result["selectedView"], "detail");
    assert_eq!(result["provenance"]["analyzer"], "fixture");
    assert_eq!(result["edges"][0]["id"], "call@src/api.rs:3");
    assert_eq!(
        result["edges"][0]["sourceLocations"][0]["uri"],
        "src/api.rs"
    );
    let node = result["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "API::run(&self)")
        .unwrap();
    assert_eq!(node["parentId"], "src/api.rs::API<T>#");
    assert_eq!(node["metadata"]["confidence"], 0.8);
    assert!(node["span"].is_null() && node["line"].is_null());
    let svg = layup::svg::render(&compiled, layup::Theme::Dark);
    assert!(svg.contains("data-layup-analysis=\"1\""));
    assert!(svg.contains("API::run(&amp;self)"));
    assert!(svg.contains("resolved-call"));
    let overview = input::compile_json(&source, &CompileOptions::default(), &Fonts::new()).unwrap();
    assert_eq!(overview.scene.nodes.len(), 1);
    let svg = layup::svg::render(&overview, layup::Theme::Light);
    assert!(!svg.contains("resolved-call"));
}

#[test]
fn invalid_graphs_fail_even_when_a_view_excludes_the_invalid_object() {
    let cases: Vec<fn(&mut Value)> = vec![
        |g| g["version"] = json!(2),
        |g| g["unexpected"] = json!(true),
        |g| g["nodes"][1]["id"] = g["nodes"][0]["id"].clone(),
        |g| g["nodes"][1]["parentId"] = json!("missing"),
        |g| g["nodes"][0]["parentId"] = json!("API::run(&self)"),
        |g| g["edges"][0]["to"] = json!("missing"),
        |g| {
            let duplicate = g["edges"][0].clone();
            g["edges"].as_array_mut().unwrap().push(duplicate);
        },
        |g| g["nodes"][1]["tone"] = json!("invalid"),
        |g| g["nodes"][0]["sourceLocations"][0]["range"]["startLine"] = json!(0),
        |g| g["nodes"][1]["kind"] = json!("view"),
        |g| g["views"][1]["include"] = json!(["missing"]),
    ];
    for change in cases {
        let mut value = graph();
        change(&mut value);
        let error = input::compile_json(
            &value.to_string(),
            &CompileOptions::default(),
            &Fonts::new(),
        )
        .err()
        .expect("invalid graph accepted");
        assert_eq!(error.code, "input/graph");
        assert!(error.span.is_none() && error.line.is_none());
    }
}

#[test]
fn structured_graph_and_dsl_share_geometry_and_routing() {
    let value = json!({"version": 1, "title": "Services", "direction": "right", "nodes": [{"id": "a", "title": "API"}, {"id": "b", "title": "Worker"}], "edges": [{"id": "dispatch", "from": "a", "to": "b", "label": "dispatch"}]});
    let graph = input::compile_json(
        &value.to_string(),
        &CompileOptions::default(),
        &Fonts::new(),
    )
    .unwrap();
    let dsl = layup::compile("diagram main \"Services\" type=graph layout=auto flow-direction=right { node a \"API\"; node b \"Worker\"; edge dispatch a -> b \"dispatch\" style=flow }").unwrap();
    assert_eq!(graph.viewport(), dsl.viewport());
    assert_eq!(graph.scene.nodes[0].rect, dsl.scene.nodes[0].rect);
    assert_eq!(graph.scene.edges[0].points, dsl.scene.edges[0].points);
}
