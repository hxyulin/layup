use layup::{CompileOptions, document, text::Fonts};

fn compile(body: &str) -> Result<layup::Compiled, layup::Error> {
    layup::compile(&format!("layup 1\ndiagram main kind=graph {{\n{body}\n}}"))
}
fn scene(source: &str, diagram: Option<&str>) -> serde_json::Value {
    let compiled = layup::compile_with_options(
        source,
        &CompileOptions {
            diagram: diagram.map(str::to_owned),
            ..Default::default()
        },
        &Fonts::default(),
    )
    .unwrap();
    serde_json::from_str(&layup::scene::export(&compiled).unwrap()).unwrap()
}

#[test]
fn graph_fixture_preserves_scoped_identity_source_spans_and_annotations() {
    let source = include_str!("../../../examples/language-v1.layup");
    let compiled = layup::compile(source).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&layup::scene::export(&compiled).unwrap()).unwrap();
    let nodes = json["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 6);
    assert_eq!(
        nodes
            .iter()
            .filter(|n| n["objectPath"].as_array().unwrap().last().unwrap() == "cache")
            .count(),
        2
    );
    let api = nodes
        .iter()
        .find(|n| n["objectPath"] == serde_json::json!(["backend", "API::run(&self)"]))
        .unwrap();
    let start = api["span"]["start"].as_u64().unwrap() as usize;
    let end = api["span"]["end"].as_u64().unwrap() as usize;
    assert!(source[start..end].starts_with("node \"API::run(&self)\""));
    assert_eq!(api["sourceLocations"][0]["range"]["startLine"], 8);
    assert_eq!(api["metadata"]["analysis"]["public"], true);
    assert_eq!(
        api["metadata"]["analysis"]["extra"],
        serde_json::Value::Null
    );
    assert_eq!(api["tone"], "blue");
    assert_eq!(api["kind"], "service");
    assert!(compiled.diagram.kinds.contains_key("service"));
    assert!(
        !compiled
            .diagram
            .kinds
            .keys()
            .any(|key| key.starts_with("__layup_"))
    );
    assert_eq!(json["document"]["diagramId"], "services");
    assert!(layup::svg::render(&compiled, layup::Theme::Light).contains("data-layup-document"));
}

#[test]
fn literal_dots_and_qualified_paths_are_distinct_and_rows_do_not_scope() {
    let compiled = compile("node \"api.run\" \"Literal\"\ngroup api { node run \"Scoped\" }\nrow { node red \"ID\" }\nedge one red -> \"api.run\"\nedge two ::red -> api.run").unwrap();
    assert_eq!(compiled.scene.edges.len(), 2);
    assert_ne!(compiled.scene.edges[0].to, compiled.scene.edges[1].to);
    assert_eq!(compiled.scene.edges[0].from, compiled.scene.edges[1].from);
}

#[test]
fn resolves_forward_names_and_bases_without_attribute_order_dependencies() {
    let a = compile("node api kind=service\nnode worker\napi -> worker\nnode-kind service tone=red base=later\nnode-kind later base=node font=mono").unwrap();
    let b = compile("node-kind later font=mono base=node\nnode-kind service base=later tone=red\nnode api kind=service\nnode worker\napi -> worker").unwrap();
    assert_eq!(a.scene.nodes[0].tone, b.scene.nodes[0].tone);
    assert_eq!(a.scene.nodes[0].tone, layup::style::Tone::Red);
}

#[test]
fn rejects_ambiguous_names_duplicate_attributes_and_ignored_bodies() {
    for (body, code) in [
        ("node a\nnode a", "document/duplicate-id"),
        ("group a { node a }", "document/shadowing"),
        ("group box { node later }\nnode later", "document/shadowing"),
        ("node \"\"", "document/syntax"),
        ("node a tone=red tone=blue", "document/syntax"),
        ("node a \"First\" \"Second\"", "document/syntax"),
        (
            "node a { code \"fn run()\" { node hidden } }",
            "document/syntax",
        ),
        (
            "node a\nnode b\nedge e a -> b\nedge e b -> a",
            "document/duplicate-edge",
        ),
        (
            "node-kind a base=b\nnode-kind b base=a\nnode x",
            "document/kind-cycle",
        ),
        ("@doc(text=\"orphan\")\nrow {}", "document/syntax"),
        (
            "@doc(text=\"one\")\n@doc(text=\"two\")\nnode a",
            "document/annotation",
        ),
        (
            "@source(uri=\"a\",range={startLine:0,startColumn:1,endLine:1,endColumn:1})\nnode a",
            "document/annotation",
        ),
        (
            "@meta(namespace=x,value={x:1,x:2})\nnode a",
            "document/syntax",
        ),
        ("node a kind=process { node b }", "document/children"),
    ] {
        let error = match compile(body) {
            Err(error) => error,
            Ok(_) => panic!("accepted invalid input: {body}"),
        };
        assert_eq!(error.code, code, "{body}: {error}");
        assert!(error.span.is_some(), "{body}: missing source span");
    }
}

#[test]
fn validates_unselected_diagrams_and_explicitly_rejects_other_grammars() {
    let src = "layup 1\ndiagram first kind=graph { node a }\ndiagram second kind=graph { node b }";
    assert_eq!(scene(src, None)["document"]["diagramId"], "first");
    assert_eq!(
        scene(src, Some("second"))["document"]["diagramId"],
        "second"
    );
    let bad = src.replace("node b", "node b\nb -> missing");
    assert!(
        layup::compile_with_options(
            &bad,
            &CompileOptions {
                diagram: Some("first".into()),
                ..Default::default()
            },
            &Fonts::default()
        )
        .is_err()
    );
    for source in [
        "layup 2\ndiagram x kind=graph {}",
        "layup 1\ndiagram x kind=sequence {}",
        "layup\ndiagram x kind=graph {}",
    ] {
        assert!(layup::compile(source).is_err());
    }
    assert!(layup::compile("diagram \"Legacy\" { node api \"API\" }").is_ok());
}

#[test]
fn left_and_bidirectional_sides_follow_semantic_endpoints() {
    let a = compile("node a\nnode b\nedge e a <- b target-side=left").unwrap();
    assert!(a.diagram.edges[0].from.contains("b"));
    assert_eq!(a.diagram.edges[0].from_side, None);
    assert_eq!(a.diagram.edges[0].to_side, Some(layup::model::Side::Left));
    let b = compile("node a\nnode b\nedge e a <-> b source-side=top target-side=bottom").unwrap();
    assert_eq!(b.diagram.edges[0].from_side, Some(layup::model::Side::Top));
    assert_eq!(b.diagram.edges[0].to_side, Some(layup::model::Side::Bottom));
}

#[test]
fn formatting_preserves_comments_values_references_and_is_idempotent() {
    let source = include_str!("../../../examples/language-v1.layup");
    let formatted = layup::format::format(source).unwrap();
    assert_eq!(layup::format::format(&formatted).unwrap(), formatted);
    assert!(formatted.contains("// Definitions can follow"));
    assert!(formatted.contains("backend.\"API::run(&self)\""));
    let before = scene(source, None);
    let after = scene(&formatted, None);
    assert_eq!(before["document"], after["document"]);
    assert_eq!(before["width"], after["width"]);
    assert_eq!(before["height"], after["height"]);
    assert!(document::parse(&formatted).is_ok());
    assert!(
        layup::lint::lint(source)
            .iter()
            .all(|d| d.severity != layup::diagnostic::Severity::Error)
    );
}

#[test]
fn unicode_names_keep_scalar_columns_and_exact_byte_spans() {
    let source = "// 注释\r\nlayup 1\r\ndiagram 国际 \"مرحبا\" kind=graph textdir=rtl {\r\n node 服务 \"[raw] API\"\r\n node café \"Worker\"\r\n edge 调用 服务->café\r\n}";
    let compiled = layup::compile(source).unwrap();
    assert_eq!(compiled.diagram.text_direction, layup::text::Direction::Rtl);
    let info = compiled.document.as_ref().unwrap();
    assert!(
        info.objects
            .values()
            .any(|n| n.path.as_ref().unwrap() == &["服务"])
    );
    let node = &compiled.scene.nodes[0];
    assert_eq!(node.span.column, 2);
    assert_eq!(
        &source[node.span.start..node.span.end],
        "node 服务 \"[raw] API\""
    );
    let layup::model::Block::Node(node) = &compiled.diagram.blocks[0] else {
        panic!("missing node");
    };
    assert_eq!(node.title.as_deref(), Some("[raw] API"));
    assert_eq!(node.tag, None);
    let formatted = layup::format::format(source).unwrap();
    assert_eq!(layup::compile(&formatted).unwrap().scene.nodes.len(), 2);
    let invalid = source.replace("café\r\n}", "missing\r\n}");
    assert_eq!(
        layup::compile(&invalid).err().unwrap().code,
        "document/reference"
    );
}

#[test]
fn annotation_evidence_is_repeatable_but_does_not_inherit() {
    let compiled = compile("@source(uri=\"one.rs\")\n@source(uri=\"two.rs\")\n@doc(text=\"Container only\")\ngroup outer { node inner }").unwrap();
    let info = compiled.document.unwrap();
    let parent = info
        .objects
        .values()
        .find(|n| n.authored_id.as_deref() == Some("outer"))
        .unwrap();
    let child = info
        .objects
        .values()
        .find(|n| n.authored_id.as_deref() == Some("inner"))
        .unwrap();
    assert_eq!(parent.source_locations.len(), 2);
    assert_eq!(child.source_locations.len(), 0);
    assert_eq!(child.documentation, None);
    for body in [
        "node-kind unused tone=blu\nnode a",
        "edge-kind unused tone=blu\nnode a",
    ] {
        assert!(compile(body).is_err());
    }
}

#[test]
fn deeply_nested_values_and_kind_chains_are_bounded() {
    let body = format!(
        "@meta(namespace=x,value={}null{})\nnode a",
        "[".repeat(140),
        "]".repeat(140)
    );
    assert!(compile(&body).is_err());
    let definitions = (0..140)
        .map(|i| format!("node-kind k{i} base=k{}\n", i + 1))
        .collect::<String>();
    assert!(compile(&format!("{definitions}node-kind k140 base=node\nnode a")).is_err());
}

#[test]
fn inline_and_nested_multiline_comments_are_lossless_trivia() {
    let source = r#"/* Header 注释
      /* nested { @not-an-annotation */
    */
    layup /* language revision */ 1
    diagram main kind=/* grammar */graph {
      @meta(namespace=test, value=/* record */{x: true, items: [1, /* item */2]})
      /* stays attached to the following node */
      node a /* label spans a comment
        without ending the statement */ "A" tone=/* palette */blue // inline
      node b "Strings keep /* markers */ and // markers"
      a/* source */->/* target */b "Call"
    }
    "#;
    let compiled = layup::compile(source).unwrap();
    assert_eq!(compiled.scene.nodes.len(), 2);
    assert_eq!(compiled.scene.edges.len(), 1);
    assert_eq!(compiled.scene.nodes[0].tone, layup::style::Tone::Blue);
    let info = compiled.document.as_ref().unwrap();
    let a = info
        .objects
        .values()
        .find(|n| n.authored_id.as_deref() == Some("a"))
        .unwrap();
    assert_eq!(a.metadata["test"]["items"], serde_json::json!([1, 2]));
    let formatted = layup::format::format(source).unwrap();
    assert_eq!(layup::format::format(&formatted).unwrap(), formatted);
    for comment in [
        "/* language revision */",
        "/* grammar */",
        "/* record */",
        "/* source */",
        "// inline",
        "/* Header 注释\n      /* nested { @not-an-annotation */\n    */",
    ] {
        assert!(formatted.contains(comment), "lost comment: {comment}");
    }
    let after = layup::compile(&formatted).unwrap();
    assert_eq!(compiled.scene.width, after.scene.width);
    assert_eq!(compiled.scene.height, after.scene.height);
    assert_eq!(
        compiled
            .scene
            .nodes
            .iter()
            .map(|n| &n.id)
            .collect::<Vec<_>>(),
        after.scene.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
    );
    assert_eq!(compiled.scene.edges[0].points, after.scene.edges[0].points);
}

#[test]
fn block_comment_diagnostics_and_limits_are_explicit() {
    let source = "layup 1\ndiagram main kind=graph {\n  /* missing close";
    let error = layup::compile(source).err().unwrap();
    assert_eq!(error.code, "document/comment");
    let span = error.span.unwrap();
    assert_eq!(&source[span.start..span.end], "/*");
    assert_eq!((span.line, span.column), (3, 3));
    assert!(error.msg.contains("unterminated"));
    assert!(layup::format::format(source).is_err());
    let too_deep = format!(
        "layup 1\n{}{}\ndiagram main kind=graph {{}}",
        "/*".repeat(129),
        "*/".repeat(129)
    );
    assert_eq!(
        layup::compile(&too_deep).err().unwrap().code,
        "document/comment"
    );
    assert!(compile("no/* comments cannot join identifier tokens */de a").is_err());
}
