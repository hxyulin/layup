use layup::{
    parser::{Arg, Stmt, Value, parse},
    views::{Resolved, resolve},
};

const SOURCE: &str = r#"model "Shared system" layout=auto direction=down {
  style service node blue
  service api "API"
  group backend "Backend" {
    node worker "Worker" { code "run(job)" }
    node cache "Cache"
  }
  node client "Client"
  client -> api "request"
  api -> worker "enqueue"
  worker -> cache "lookup"
  view overview "Overview" direction=right {
    include client api backend
  }
  view detail "Worker detail" direction=left text-direction=rtl {
    include api worker
  }
}"#;

fn resolved(source: &str, selector: Option<&str>) -> Resolved {
    resolve(&parse(source).unwrap(), selector).unwrap()
}

fn root(result: &Resolved) -> &layup::parser::Item {
    let [Stmt::Item(root)] = result.statements.as_slice() else {
        panic!("expected one root");
    };
    root
}

fn ids(stmts: &[Stmt]) -> Vec<String> {
    let mut out = Vec::new();
    for stmt in stmts {
        if let Stmt::Item(item) = stmt {
            if [
                "service", "node", "group", "state", "initial", "final", "choice",
            ]
            .contains(&item.head.as_str())
            {
                let id = item
                    .args
                    .iter()
                    .find_map(|a| match a {
                        Arg::Value(Value::Ident(s)) => Some(s.clone()),
                        Arg::Attr(k, v) if k == "id" => Some(v.as_text()),
                        _ => None,
                    })
                    .unwrap();
                out.push(id);
            }
            out.extend(ids(item.body.as_deref().unwrap_or(&[])));
        }
    }
    out
}

fn edges(stmts: &[Stmt]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Edge(e) => out.push((e.from.clone(), e.to.clone())),
            Stmt::Item(i) => out.extend(edges(i.body.as_deref().unwrap_or(&[]))),
        }
    }
    out
}

#[test]
fn ordinary_diagrams_pass_through_and_reject_view_selectors() {
    let source = "diagram \"Plain\" { node api \"API\" }";
    let parsed = parse(source).unwrap();
    let actual = resolve(&parsed, None).unwrap();
    assert_eq!(format!("{:?}", actual.statements), format!("{parsed:?}"));
    assert!(actual.views.is_empty());
    assert!(actual.selected_view.is_none());
    assert!(
        resolve(&parsed, Some("overview"))
            .unwrap_err()
            .msg
            .contains("no model")
    );
}

#[test]
fn authored_first_view_is_default_and_container_include_expands_subtree() {
    let actual = resolved(SOURCE, None);
    assert_eq!(actual.selected_view.as_deref(), Some("overview"));
    assert_eq!(
        actual
            .views
            .iter()
            .map(|v| v.id.as_str())
            .collect::<Vec<_>>(),
        ["overview", "detail"]
    );
    assert_eq!(
        ids(root(&actual).body.as_ref().unwrap()),
        ["api", "backend", "worker", "cache", "client"]
    );
    assert_eq!(edges(root(&actual).body.as_ref().unwrap()).len(), 3);
    assert_eq!(root(&actual).head, "diagram");
    assert_eq!(
        root(&actual).args[0],
        Arg::Value(Value::Str("Overview".into()))
    );
}

#[test]
fn selecting_descendant_retains_ancestors_without_siblings_and_filters_edges() {
    let actual = resolved(SOURCE, Some("detail"));
    let body = root(&actual).body.as_ref().unwrap();
    assert_eq!(ids(body), ["api", "backend", "worker"]);
    assert_eq!(edges(body), [("api".into(), "worker".into())]);
    let Stmt::Item(group) = &body[2] else {
        panic!()
    };
    let Stmt::Item(worker) = &group.body.as_ref().unwrap()[0] else {
        panic!()
    };
    assert_eq!(worker.body.as_ref().unwrap().len(), 1);
    assert_eq!(worker.head_span.line, 5);
    assert!(
        root(&actual)
            .args
            .contains(&Arg::Attr("layout".into(), Value::Ident("auto".into())))
    );
    assert!(
        root(&actual)
            .args
            .contains(&Arg::Attr("direction".into(), Value::Ident("left".into())))
    );
    assert!(root(&actual).args.contains(&Arg::Attr(
        "text-direction".into(),
        Value::Ident("rtl".into())
    )));
}

#[test]
fn row_filtering_preserves_order_and_matching_authored_weights() {
    let source = r#"model "Rows" {
      row 1:3:2 { node a "A"; node b "B"; node c "C" }
      view pair { include a c }
    }"#;
    let actual = resolved(source, None);
    let Stmt::Item(row) = &root(&actual).body.as_ref().unwrap()[0] else {
        panic!()
    };
    assert_eq!(row.args, [Arg::Weights(vec![1.0, 2.0])]);
    assert_eq!(ids(row.body.as_ref().unwrap()), ["a", "c"]);
}

#[test]
fn duplicate_refs_unknown_refs_and_selector_have_precise_diagnostics() {
    let cases = [
        (
            "model \"X\" { node api; view a { include api api } }",
            "view/duplicate-include",
            "api",
        ),
        (
            "model \"X\" { node api; view a { include apj } }",
            "view/include-target",
            "apj",
        ),
        (
            "model \"X\" { node api; view a { include api }; view a { include api } }",
            "view/duplicate-id",
            "a",
        ),
        (
            "model \"X\" { node api; node api; view a { include api } }",
            "semantic/duplicate-id",
            "api",
        ),
        (
            "model \"X\" { node api; api -> missing; view a { include api } }",
            "view/edge-target",
            "missing",
        ),
    ];
    for (source, code, highlighted) in cases {
        let error = resolve(&parse(source).unwrap(), None).unwrap_err();
        assert_eq!(error.code, code);
        let span = error.span.unwrap();
        assert_eq!(&source[span.start..span.end], highlighted);
        if code.contains("duplicate") {
            assert_eq!(error.related.len(), 1);
        }
        if code == "view/include-target" {
            assert!(error.help.unwrap().contains("api"));
        }
    }
    let error = resolve(&parse(SOURCE).unwrap(), Some("overvew")).unwrap_err();
    assert_eq!(error.code, "view/unknown");
    assert!(error.help.unwrap().contains("overview"));
}

#[test]
fn every_authored_view_is_validated_even_when_another_is_selected() {
    let source =
        "model \"X\" { node api; view good { include api }; view bad { include missing } }";
    assert_eq!(
        resolve(&parse(source).unwrap(), Some("good"))
            .unwrap_err()
            .code,
        "view/include-target"
    );
}

#[test]
fn shared_nodes_require_explicit_ids_and_invalid_view_bodies_are_rejected() {
    let cases = [
        (
            "model \"X\" { node \"API\"; view a { include api } }",
            "view/explicit-id",
        ),
        ("model \"X\" { node api }", "view/missing"),
        ("model \"X\" { node api; view a {} }", "view/empty"),
        (
            "model \"X\" { node api; view a { node second } }",
            "view/directive",
        ),
        (
            "model \"X\" { node api; view a { include api tone=blue } }",
            "view/include-argument",
        ),
        (
            "model \"X\" { node api; view a { include api {} } }",
            "view/directive",
        ),
        (
            "model \"X\" { node api; view a { api -> api } }",
            "view/directive",
        ),
        ("model \"X\" {}; model \"Y\" {}", "view/model-count"),
        ("node outside; model \"X\" {}", "view/model-document"),
    ];
    for (source, code) in cases {
        let error = resolve(&parse(source).unwrap(), None).unwrap_err();
        assert_eq!(error.code, code, "{source}");
        assert!(error.span.is_some());
    }
}

#[test]
fn unicode_and_quoted_id_attributes_keep_source_spans() {
    let source = "model \"系统\" { node id=\"服务 API\" \"入口\"; view 图 \"说明\" { include \"服务 API\" } }";
    let actual = resolved(source, Some("图"));
    let Stmt::Item(node) = &root(&actual).body.as_ref().unwrap()[0] else {
        panic!()
    };
    assert_eq!(&source[node.head_span.start..node.head_span.end], "node");
    assert_eq!(actual.views[0].title, "说明");
    assert_eq!(actual.selected_view.as_deref(), Some("图"));
}

#[test]
fn selected_view_steps_replace_shared_steps_without_mutating_other_views() {
    let source = r#"model "Steps" {
      node a; node b
      step shared "Shared" { show a b }
      view first { include a b; step local "Local" { show a } }
      view second { include b }
    }"#;
    for (selector, expected) in [("first", "local"), ("second", "shared")] {
        let actual = resolved(source, Some(selector));
        let body = root(&actual).body.as_ref().unwrap();
        let steps: Vec<_> = body
            .iter()
            .filter_map(|s| match s {
                Stmt::Item(i) if i.head == "step" => Some(i),
                _ => None,
            })
            .collect();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].args[0], Arg::Value(Value::Ident(expected.into())));
    }
}

#[test]
fn model_and_view_slide_options_are_inherited_with_view_precedence() {
    let actual = resolved(
        "model \"Slide\" slide=\"16:9\" { node api; view a slide=\"4:3\" { include api } }",
        None,
    );
    assert!(
        root(&actual)
            .args
            .contains(&Arg::Attr("slide".into(), Value::Str("4:3".into())))
    );
}

#[test]
fn native_compilation_uses_selected_view_and_preserves_authored_references() {
    let result = layup::compile_with_options(
        SOURCE,
        &layup::CompileOptions {
            view: Some("detail".into()),
        },
        &layup::text::Fonts::default(),
    )
    .unwrap();
    assert_eq!(result.selected_view.as_deref(), Some("detail"));
    assert_eq!(result.views.len(), 2);
    assert_eq!(result.diagram.title, "Worker detail");
    assert_eq!(result.diagram.direction, layup::model::Direction::Left);
    assert_eq!(
        result
            .scene
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        ["api", "backend", "worker"]
    );
    assert_eq!(result.diagram.edges.len(), 1);
    assert_eq!(result.diagram.edges[0].from, "api");
    assert_eq!(result.diagram.edges[0].to, "worker");
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

#[test]
fn formatting_keeps_every_view_semantically_identical() {
    let formatted = layup::format::format(SOURCE).unwrap();
    assert_eq!(layup::format::format(&formatted).unwrap(), formatted);
    for view in ["overview", "detail"] {
        let options = layup::CompileOptions {
            view: Some(view.into()),
        };
        let fonts = layup::text::Fonts::default();
        let before = layup::compile_with_options(SOURCE, &options, &fonts).unwrap();
        let after = layup::compile_with_options(&formatted, &options, &fonts).unwrap();
        assert_eq!(before.diagram.title, after.diagram.title);
        assert_eq!(before.scene.nodes.len(), after.scene.nodes.len());
        for (a, b) in before.scene.nodes.iter().zip(&after.scene.nodes) {
            assert_eq!(
                (&a.id, &a.kind, a.rect, &a.parent),
                (&b.id, &b.kind, b.rect, &b.parent)
            );
        }
        assert_eq!(before.scene.edges.len(), after.scene.edges.len());
        for (a, b) in before.scene.edges.iter().zip(&after.scene.edges) {
            assert_eq!((&a.from, &a.to, &a.points), (&b.from, &b.to, &b.points));
        }
    }
}

#[test]
fn view_local_steps_compile_and_shared_steps_with_excluded_targets_fail() {
    let source = r#"model "Steps" {
      node a; node b
      step shared { show a b }
      view first { include a b; step local { show a } }
      view second { include b }
    }"#;
    let fonts = layup::text::Fonts::default();
    let first = layup::compile_with_options(
        source,
        &layup::CompileOptions {
            view: Some("first".into()),
        },
        &fonts,
    )
    .unwrap();
    assert_eq!(first.presentation.steps.len(), 1);
    assert_eq!(first.presentation.steps[0].id, "local");
    assert_eq!(first.presentation.steps[0].visible_nodes, ["a"]);
    let error = layup::compile_with_options(
        source,
        &layup::CompileOptions {
            view: Some("second".into()),
        },
        &fonts,
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "presentation/node");
    let span = error.span.unwrap();
    assert_eq!(&source[span.start..span.end], "a");
}

#[test]
fn selected_composite_view_retains_state_scope_and_semantic_validation() {
    let source = r#"model "Machine" mode=state-machine {
      initial start
      state outer "Outer" {
        initial entry
        state active "Active"
        entry -> active
      }
      final done
      start -> outer
      outer -> done
      view valid { include start outer done }
      view invalid { include active }
    }"#;
    assert!(layup::compile(source).is_ok());
    let invalid = layup::compile_with_options(
        source,
        &layup::CompileOptions {
            view: Some("invalid".into()),
        },
        &layup::text::Fonts::default(),
    );
    assert!(
        invalid.is_err(),
        "a view must not manufacture missing initial states"
    );
}

#[test]
fn edge_ids_stay_stable_across_views_and_skip_explicit_reservations() {
    let source = r#"model "Stable edges" layout=auto {
      node a; node b; node c
      a -> b
      b -> c
      a -> c id=edge-1
      view all { include a b c }
      view pair { include b c }
    }"#;
    let fonts = layup::text::Fonts::default();
    let all = layup::compile(source).unwrap();
    assert_eq!(
        all.scene
            .edges
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        ["edge-2", "edge-3", "edge-1"]
    );
    let pair = layup::compile_with_options(
        source,
        &layup::CompileOptions {
            view: Some("pair".into()),
        },
        &fonts,
    )
    .unwrap();
    assert_eq!(pair.scene.edges.len(), 1);
    assert_eq!(pair.scene.edges[0].id, "edge-3");
    assert_eq!(pair.diagram.edges[0].id, "edge-3");
}

#[test]
fn excluded_edges_do_not_hide_duplicate_explicit_edge_ids() {
    let source = "model \"Edges\" { node a; node b; node c; a -> b id=shared; b -> c id=shared; view a { include a } }";
    let error = resolve(&parse(source).unwrap(), None).unwrap_err();
    assert_eq!(error.code, "semantic/duplicate-edge-id");
    assert_eq!(error.related.len(), 1);
    let span = error.span.unwrap();
    assert_eq!(&source[span.start..span.end], "id=shared");
}

#[test]
fn sequence_views_filter_participants_and_keep_message_ids_inside_fragments() {
    let source = r#"model "Protocol" mode=sequence {
      participant client "Client"
      participant api "API"
      participant audit "Audit"
      client -> audit "log"
      loop "Retry" { client -> api "request"; api -> client "response" return }
      view all { include client api audit }
      view request { include client api }
    }"#;
    let compiled = layup::compile_with_options(
        source,
        &layup::CompileOptions {
            view: Some("request".into()),
        },
        &layup::text::Fonts::default(),
    )
    .unwrap();
    assert_eq!(compiled.diagram.mode, layup::model::Mode::Sequence);
    assert_eq!(
        compiled
            .scene
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        ["client", "api"]
    );
    assert_eq!(
        compiled
            .scene
            .edges
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        ["edge-2", "edge-3"]
    );
    assert_eq!(compiled.sequence.unwrap().annotations[0].kind, "loop");
}

#[test]
fn custom_styles_do_not_turn_reserved_structural_statements_into_nodes() {
    let source = r#"model "Rows" {
      style row blue
      row { node a; node b }
      view single { include a }
    }"#;
    let compiled = layup::compile(source).unwrap();
    assert_eq!(compiled.scene.nodes.len(), 1);
    assert_eq!(compiled.scene.nodes[0].id, "a");
}

#[test]
fn sequence_views_filter_targeted_notes_with_excluded_participants() {
    let source = r#"model "Notes" mode=sequence {
      participant a "Client"
      participant b "API"
      participant c "Audit"
      note "Global context"
      note "Selected API" over=b
      note "Excluded audit" over=c
      note "Cross-view note" from=a to=c
      loop "Retry" { note "Internal audit" over=c; a -> b "Request" }
      view full { include a b c }
      view request { include a b }
    }"#;
    let selected = layup::compile_with_options(
        source,
        &layup::CompileOptions {
            view: Some("request".into()),
        },
        &layup::text::Fonts::default(),
    )
    .unwrap();
    let info = selected.sequence.unwrap();
    assert_eq!(
        info.annotations
            .iter()
            .map(|a| a.label.as_str())
            .collect::<Vec<_>>(),
        ["Global context", "Selected API", "Retry"]
    );
    assert_eq!(selected.scene.edges.len(), 1);
    let full = layup::compile(source).unwrap();
    assert_eq!(full.sequence.unwrap().annotations.len(), 6);
}

#[test]
fn excluded_sequence_notes_do_not_hide_unknown_targets_or_malformed_syntax() {
    let unknown = "model \"Notes\" mode=sequence { participant client; participant api; note \"Bad\" over=apj; view a { include client } }";
    let error = layup::compile(unknown).err().unwrap();
    assert_eq!(error.code, "sequence/note-target");
    assert!(error.help.unwrap().contains("api"));
    let span = error.span.unwrap();
    assert_eq!(&unknown[span.start..span.end], "over=apj");

    let malformed = "model \"Notes\" mode=sequence { participant client; participant audit; note \"Half\" from=audit; view a { include client } }";
    assert_eq!(
        layup::compile(malformed).err().unwrap().code,
        "sequence/note-span"
    );
}

#[test]
fn note_filtering_can_leave_a_sequence_fragment_invalid_without_rewriting_it() {
    let source = "model \"Notes\" mode=sequence { participant client; participant audit; loop \"Audit\" { note \"Excluded\" over=audit }; view a { include client } }";
    assert_eq!(
        layup::compile(source).err().unwrap().code,
        "sequence/empty-fragment"
    );
}
