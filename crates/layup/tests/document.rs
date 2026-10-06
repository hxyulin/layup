use layup::{CompileOptions, document, text::Fonts};

fn compile(body: &str) -> Result<layup::Compiled, layup::Error> {
    layup::compile(&format!("layup 1\ndiagram main type=graph {{\n{body}\n}}"))
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
    let a = compile("node api style=service\nnode worker\napi -> worker\nnode-style service palette=red base=later\nnode-style later base=node font-family=mono").unwrap();
    let b = compile("node-style later font-family=mono base=node\nnode-style service base=later palette=red\nnode api style=service\nnode worker\napi -> worker").unwrap();
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
        ("node a palette=red palette=blue", "document/syntax"),
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
            "node-style a base=b\nnode-style b base=a\nnode x",
            "document/kind-cycle",
        ),
        ("@doc(text=\"orphan\")", "document/syntax"),
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
        ("node a style=process { node b }", "document/children"),
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
    let src = "layup 1\ndiagram first type=graph { node a }\ndiagram second type=graph { node b }";
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
        "layup 2\ndiagram x type=graph {}",
        "layup 1\ndiagram x type=sequence {}",
        "layup\ndiagram x type=graph {}",
    ] {
        assert!(layup::compile(source).is_err());
    }
    assert!(layup::compile("diagram \"Legacy\" { node api \"API\" }").is_err());
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
    fn semantic(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(fields) => {
                for key in ["span", "valueSpan", "nameSpan", "raw"] {
                    fields.remove(key);
                }
                for value in fields.values_mut() {
                    semantic(value);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    semantic(value);
                }
            }
            _ => {}
        }
    }
    let mut before_document = before["document"].clone();
    let mut after_document = after["document"].clone();
    semantic(&mut before_document);
    semantic(&mut after_document);
    assert_eq!(before_document, after_document);
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
    let source = "// 注释\r\nlayup 1\r\ndiagram 国际 \"مرحبا\" type=graph text-direction=rtl {\r\n node 服务 \"[raw] API\"\r\n node café \"Worker\"\r\n edge 调用 服务->café\r\n}";
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
        "node-style unused palette=blu\nnode a",
        "edge-style unused palette=blu\nnode a",
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
        .map(|i| format!("node-style k{i} base=k{}\n", i + 1))
        .collect::<String>();
    assert!(compile(&format!("{definitions}node-style k140 base=node\nnode a")).is_err());
}

#[test]
fn inline_and_nested_multiline_comments_are_lossless_trivia() {
    let source = r#"/* Header 注释
      /* nested { @not-an-annotation */
    */
    layup /* language revision */ 1
    diagram main type=/* grammar */graph {
      @meta(namespace=test, value=/* record */{x: true, items: [1, /* item */2]})
      /* stays attached to the following node */
      node a /* label spans a comment
        without ending the statement */ "A" palette=/* palette */blue // inline
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
    let source = "layup 1\ndiagram main type=graph {\n  /* missing close";
    let error = layup::compile(source).err().unwrap();
    assert_eq!(error.code, "document/comment");
    let span = error.span.unwrap();
    assert_eq!(&source[span.start..span.end], "/*");
    assert_eq!((span.line, span.column), (3, 3));
    assert!(error.msg.contains("unterminated"));
    assert!(layup::format::format(source).is_err());
    let too_deep = format!(
        "layup 1\n{}{}\ndiagram main type=graph {{}}",
        "/*".repeat(129),
        "*/".repeat(129)
    );
    assert_eq!(
        layup::compile(&too_deep).err().unwrap().code,
        "document/comment"
    );
    assert!(compile("no/* comments cannot join identifier tokens */de a").is_err());
}

#[test]
fn shared_language_conformance_and_formatting() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/language/conformance.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let parsed = document::parse_recovering(source);
        assert_eq!(
            parsed.errors.is_empty(),
            case["valid"].as_bool().unwrap(),
            "{}: {:?}",
            case["name"],
            parsed.errors
        );
        if parsed.errors.is_empty() {
            let formatted = document::format(source).unwrap();
            assert_eq!(
                document::format(&formatted).unwrap(),
                formatted,
                "{}",
                case["name"]
            );
            assert!(document::parse(&formatted).is_ok(), "{}", case["name"]);
        } else {
            assert!(document::format(source).is_err(), "{}", case["name"]);
        }
    }
}

#[test]
fn optional_revision_asserts_the_same_document_and_rejects_float_revisions() {
    let source = "diagram main type=graph { node api \"API\" }";
    assert_eq!(layup::compile(source).unwrap().scene.nodes.len(), 1);
    let quoted = "diagram \"future\" custom-property={x: 1} type = vendor.timeline {}\ndiagram main type=graph { node api }";
    assert_eq!(
        layup::compile(quoted).unwrap().document.unwrap().diagram_id,
        "main"
    );
    assert!(layup::compile("diagram \"type=graph\" { node api }").is_err());
    assert_eq!(
        layup::compile(&format!("layup 1\n{source}"))
            .unwrap()
            .scene
            .nodes
            .len(),
        1
    );
    for version in ["1.0", "1e0", "2", "true"] {
        assert_eq!(
            layup::compile(&format!("layup {version}\n{source}"))
                .err()
                .unwrap()
                .code,
            "document/version"
        );
    }
}

#[test]
fn annotation_values_are_exact_typed_and_preserved_on_every_target() {
    let source = "diagram main type=graph {\n@company.rank(value=9007199254740993)\n@company.rank(value=18446744073709551615)\nnode api\n@company.layout(value={mode: manual, small: -9223372036854775808, float: 1.0, target: ::api})\nrow { node worker }\n@company.style()\nnode-style service\n}";
    let doc = document::parse(source).unwrap();
    let compiled = layup::compile(source).unwrap();
    let json = serde_json::to_value(compiled.document.unwrap()).unwrap();
    let node = &json["objects"]["object:[\"api\"]"]["annotations"];
    assert_eq!(node.as_array().unwrap().len(), 2);
    assert_eq!(
        node[0]["arguments"]["value"]["value"],
        serde_json::json!({"type":"integer","value":"9007199254740993"})
    );
    assert_eq!(
        node[1]["arguments"]["value"]["value"]["value"],
        "18446744073709551615"
    );
    assert_eq!(json["targets"].as_array().unwrap().len(), 3);
    for span in &doc.value_spans {
        assert!(!source[span.span.start..span.span.end].is_empty());
    }
    let formatted = document::format(source).unwrap();
    assert!(formatted.contains("9007199254740993"));
    assert!(formatted.contains("18446744073709551615"));
    assert!(formatted.contains("-9223372036854775808"));
    assert!(formatted.contains("float: 1.0"));
    let inspected = document::inspect(source);
    let row = &inspected["document"]["diagrams"][0]["body"]["value"][1]["value"];
    assert_eq!(
        row["annotations"][0]["arguments"]["value"]["value"]["value"]["target"]["type"],
        "reference"
    );
}

#[test]
fn opaque_bodies_are_lossless_skipped_and_warn_without_hiding_valid_siblings() {
    let raw = "{\nlaunch => 2027-01-01 ? punctuation\n\"quoted } \\q\"\n/* } /* nested */ */\n{ foreign => syntax }\n}";
    let source = format!(
        "@company.owner(team=design)\ndiagram future type=company.timeline custom-property=true {raw}\ndiagram main type=graph {{ node api }}"
    );
    let parsed = document::parse(&source).unwrap();
    assert!(
        matches!(&parsed.diagrams[0].body, document::Body::Opaque {raw: actual, ..} if actual == raw)
    );
    let formatted = document::format(&source).unwrap();
    assert!(formatted.contains(raw));
    let compiled = layup::compile(&source).unwrap();
    assert_eq!(compiled.scene.nodes.len(), 1);
    assert_eq!(compiled.warnings.len(), 1);
    let info = compiled.document.unwrap();
    assert_eq!(info.diagram_id, "main");
    assert_eq!(info.manifest[0].status, "skipped");
    assert_eq!(info.manifest[0].annotations.len(), 1);
    let selected = CompileOptions {
        diagram: Some("future".into()),
        ..Default::default()
    };
    assert_eq!(
        layup::compile_with_options(&source, &selected, &Fonts::default())
            .err()
            .unwrap()
            .code,
        "diagram/unavailable-selection"
    );
    let lint = layup::lint::lint(&source);
    assert_eq!(lint[0].code, "diagram/unrecognized-type");
    assert!(lint[0].span.is_some());
    assert!(document::parse("diagram future type=unknown {}").is_ok());
    assert_eq!(
        layup::compile("diagram future type=unknown {}")
            .err()
            .unwrap()
            .code,
        "diagram/no-renderable-diagram"
    );
    assert!(
        document::parse("diagram future type=unknown {}\ndiagram future type=graph {}").is_err()
    );
    assert!(layup::compile(&source.replace("node api", "node api width=invalid")).is_err());
}

#[test]
fn typo_hints_are_conservative_and_never_change_extension_meanings() {
    let source = "diagram typo type=garph {}\ndiagram main type=graph {\n@sorce(uri=\"a.rs\")\nnode a\n@vendor.sorce(uri=\"b.rs\")\nnode b\n}";
    let diagnostics = layup::lint::lint(source);
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].code, "diagram/unrecognized-type");
    assert!(diagnostics[0].help.as_deref().unwrap().contains("graph"));
    assert_eq!(diagnostics[1].code, "annotation/possible-typo");
    let info = layup::compile(source).unwrap().document.unwrap();
    assert_eq!(info.objects["object:[\"a\"]"].source_locations.len(), 0);
    assert_eq!(info.objects["object:[\"a\"]"].annotations[0].name, "sorce");
    assert!(compile("@source(unknown=1)\nnode a").is_err());
    assert!(compile("@source(uri=\"a.rs\") row {}").is_err());
}

#[test]
fn recovering_parser_keeps_siblings_and_collects_independent_errors() {
    let source = "diagram main type=graph {\nnode broken width=\nnode valid \"Valid\"\nunknown directive\nnode later\n}\ndiagram next type=graph { node other }";
    let parsed = document::parse_recovering(source);
    assert_eq!(parsed.errors.len(), 2);
    assert_eq!(parsed.document.diagrams.len(), 2);
    let document::Body::Graph(body) = &parsed.document.diagrams[0].body else {
        panic!("graph")
    };
    assert_eq!(body.len(), 2);
    assert!(document::parse(source).is_err());
    assert!(document::format(source).is_err());
    assert_eq!(layup::lint::lint(source).len(), 2);
}

#[test]
fn lexer_recovery_preserves_later_declarations_and_precise_spans() {
    let source =
        "diagram main type=graph {\nnode a \"bad\\q\"\nnode valid\nnode b ?\nnode later\n}";
    let parsed = document::parse_recovering(source);
    assert_eq!(parsed.errors.len(), 2, "{:?}", parsed.errors);
    let document::Body::Graph(body) = &parsed.document.diagrams[0].body else {
        panic!("graph")
    };
    assert!(
        body.iter()
            .any(|item| matches!(item, document::GraphStatement::Node(node) if node.id == "valid"))
    );
    assert!(
        body.iter()
            .any(|item| matches!(item, document::GraphStatement::Node(node) if node.id == "later"))
    );
    let span = parsed.errors[1].span.as_deref().unwrap();
    assert_eq!(&source[span.start..span.end], "?");
    assert_eq!((span.line, span.column), (4, 8));
    assert_eq!(layup::lint::lint(source).len(), 2);
}

#[test]
fn reviewed_style_vocabulary_preserves_inheritance_and_source_ranges() {
    let source = "diagram main type=graph layout=auto flow-direction=right {\nnode-style parent palette=blue\nnode-style service base=parent palette=green font-family=mono\n@source(uri=\"a.rs\",range={start-line:1,start-column:1,end-line:2,end-column:1})\nnode api style=service palette=purple text-direction=ltr\nnode worker\nedge e api -> worker style=calls stroke-style=solid\nedge-style calls palette=blue stroke-style=dashed\n}";
    let compiled = layup::compile(source).unwrap();
    assert_eq!(compiled.scene.nodes[0].tone, layup::style::Tone::Purple);
    let api = &compiled.document.as_ref().unwrap().objects["object:[\"api\"]"];
    assert_eq!(
        api.source_locations[0].range.as_ref().unwrap().start_line,
        1
    );
    assert!(
        layup::compile(&source.replace("palette=purple", "palette=purple palette=blue")).is_err()
    );
}
