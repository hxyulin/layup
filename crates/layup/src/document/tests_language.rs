use crate as layup;
use layup::{
    compile,
    diagnostic::Severity,
    format::format,
    lexer::{Tok, lex, lex_lossless},
    lint::lint,
    parser::{Stmt, parse, parse_recovering},
};

#[test]
fn utf8_spans_cover_exact_source_and_track_crlf_tabs_and_multiline_strings() {
    let src = "// 注释\r\ndiagram \"标题\" {\r\n\tstate 状态 \"مرحبا\n世界\"\r\n}";
    let tokens = lex_lossless(src).unwrap();
    for t in &tokens {
        assert!(src.is_char_boundary(t.span.start) && src.is_char_boundary(t.span.end));
        assert!(t.span.end >= t.span.start);
    }
    let id = tokens
        .iter()
        .find(|t| matches!(&t.tok,Tok::Ident(s) if s=="状态"))
        .unwrap();
    assert_eq!(&src[id.span.start..id.span.end], "状态");
    assert_eq!((id.span.line, id.span.column), (3, 8));
    let text = tokens
        .iter()
        .find(|t| matches!(&t.tok,Tok::Str(s) if s.contains("مرحبا")))
        .unwrap();
    assert_eq!((text.span.line, text.span.end_line), (3, 4));
    assert_eq!(
        (
            tokens.last().unwrap().span.line,
            tokens.last().unwrap().span.column
        ),
        (5, 2)
    );
}

#[test]
fn identifiers_arrows_and_numbers_have_unambiguous_boundaries() {
    let tokens=lex("net::Transport http-client src/api href=https://example.com a->b a--b a<-b a <-> b\n row 1:2.5:1e2 gutter=.5\n gap -2\n a -data-flow-> b; b <-数据- a; a -data-flow- b").unwrap();
    assert!(
        tokens
            .iter()
            .any(|t| matches!(&t.tok,Tok::Ident(s) if s=="https://example.com"))
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t.tok,Tok::Num(n) if n==100.0))
    );
    assert!(tokens.iter().any(|t| matches!(t.tok,Tok::Num(n) if n==0.5)));
    assert!(
        tokens
            .iter()
            .any(|t| matches!(&t.tok,Tok::Arrow {kind:Some(s),right:true,..} if s=="data-flow"))
    );
    assert!(
        parse(r#"diagram "T" { arrow data-flow blue; node a; node b; a -data-flow-> b }"#).is_ok()
    );
}

#[test]
fn strings_preserve_valid_escapes_and_reject_silent_data_loss() {
    let token = lex(r#""quote: \"; slash: \\; newline: \n; tab: \t; return: \r""#)
        .unwrap()
        .remove(0);
    assert_eq!(
        token.tok,
        Tok::Str("quote: \"; slash: \\; newline: \n; tab: \t; return: \r".into())
    );
    let e = parse("diagram \"T\" {\n node a \"bad\\q\"\n}").unwrap_err();
    assert_eq!(e.code, "lex/escape");
    assert_eq!(
        (
            e.span.as_ref().unwrap().line,
            e.span.as_ref().unwrap().column
        ),
        (2, 13)
    );
    assert!(e.help.unwrap().contains("literal backslash"));
    assert_eq!(parse("diagram \"T").unwrap_err().code, "lex/string");
    for n in ["1..2", "1e", "1e999", "12px"] {
        assert_eq!(lex(n).unwrap_err().code, "lex/number");
    }
}

#[test]
fn recovery_collects_sibling_errors_without_swallowing_the_enclosing_block() {
    let src = "diagram \"T\" {\n node a href=\n a ->\n row 1: { node invalid }\n node good \"Good\"\n}\nnode outside";
    let p = parse_recovering(src);
    assert_eq!(p.errors.len(), 3, "{:?}", p.errors);
    assert_eq!(
        p.errors.iter().map(|e| e.line.unwrap()).collect::<Vec<_>>(),
        [2, 3, 4]
    );
    let Stmt::Item(root) = &p.statements[0] else {
        panic!()
    };
    assert!(root.body.as_ref().unwrap().iter().any(|s|matches!(s,Stmt::Item(i) if i.head=="node" && i.args[0]==layup::parser::Arg::Value(layup::parser::Value::Ident("good".into())))));
    assert_eq!(p.statements.len(), 2);
    let ds = lint(
        "diagram main type=graph {\nnode a href=\na ->\nrow weights=[1, nope] { node b } bad=\nnode good\n}",
    );
    assert_eq!(ds.len(), 3);
    assert!(ds.iter().all(|d| d.severity == Severity::Error));
}

#[test]
fn lexical_failures_do_not_cascade_and_eof_has_a_real_source_position() {
    let source = "row 1:1e999 { node a }\nnode b href=";
    let errors = parse_recovering(source).errors;
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(errors[0].code, "lex/number");
    assert_eq!(errors[1].code, "parse/attribute-value");
    let span = errors[1].span.as_ref().unwrap();
    assert_eq!((span.start, span.end), (source.len(), source.len()));
    assert_eq!((span.line, span.column), (2, 13));
}

#[test]
fn duplicate_attributes_and_unclosed_blocks_have_precise_context() {
    let src = "diagram \"T\" width=900 width=1200 {}";
    let e = parse(src).unwrap_err();
    assert_eq!(e.code, "parse/duplicate-attribute");
    assert_eq!(
        &src[e.span.as_ref().unwrap().start..e.span.as_ref().unwrap().end],
        "width"
    );
    assert_eq!(e.related.len(), 1);
    assert!(e.related[0].0.start < e.span.unwrap().start);
    let e = parse("diagram \"T\" {\n node a\n").unwrap_err();
    assert_eq!(e.code, "parse/unclosed-block");
    assert_eq!(e.span.unwrap().column, 13);
}

#[test]
fn nesting_is_bounded_and_invalid_input_never_panics() {
    let deep = format!("{}{}", "group x {\n".repeat(150), "}\n".repeat(150));
    assert!(
        parse_recovering(&deep)
            .errors
            .iter()
            .any(|e| e.code == "parse/depth")
    );
    let alphabet = [
        '{', '}', '=', ';', ':', '-', '<', '>', '"', '\\', '\n', '\r', '\t', 'a', '中', '\u{301}',
        '0', '@',
    ];
    let mut state = 43_u64;
    for _ in 0..600 {
        let mut text = String::new();
        for _ in 0..80 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            text.push(alphabet[(state >> 32) as usize % alphabet.len()]);
        }
        let _ = parse_recovering(&text);
        let _ = format(&text);
        let _ = lint(&text);
    }
}

#[test]
fn semantic_errors_point_at_misspellings_and_offer_known_symbols() {
    let source = "diagram main type=graph { node worker; worker -> wokrer }";
    let error = compile(source).err().unwrap();
    let span = error.span.unwrap();
    assert_eq!(&source[span.start..span.end], "wokrer");
    assert!(error.help.unwrap().contains("worker"));
    let source = "diagram main type=graph { node a; node a }";
    let error = compile(source).err().unwrap();
    assert_eq!(error.code, "document/duplicate-id");
    assert_eq!(error.related.len(), 1);
    let source = "diagram main type=graph widht=900 { node a }";
    let error = compile(source).err().unwrap();
    let span = error.span.unwrap();
    assert_eq!(&source[span.start..span.end], "widht");
    assert!(error.help.unwrap().contains("width"));
}

#[test]
fn invalid_geometry_numbers_are_rejected_before_layout() {
    for source in [
        "diagram main type=graph width=0 {}",
        "diagram main type=graph { gap size=-4 }",
        "diagram main type=graph { row weights=[0, 1] { node a; node b } }",
        "diagram main type=graph { row weights=[1e308, 1e308] { node a; node b } }",
        "diagram main type=graph { node a stroke-width=-1 }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn formatter_preserves_comments_strings_and_rendered_semantics_and_is_idempotent() {
    let source = r#"// Overview
diagram main "国际" type=state-machine flow-direction=right{ // machine
initial root;state parent "متصل"{initial start; state a "A"{text "entry / f(\"x\"); path C:\\tmp"}; // action
final done;start->a;a->done "完成"}; root ->parent // entry
}
"#;
    let formatted = format(source).unwrap();
    assert_eq!(format(&formatted).unwrap(), formatted);
    for comment in ["// Overview", "// machine", "// action", "// entry"] {
        assert!(formatted.contains(comment));
    }
    assert!(formatted.contains(r#""entry / f(\"x\"); path C:\\tmp""#));
    let before = compile(source).unwrap();
    let after = compile(&formatted).unwrap();
    assert!(before.warnings.is_empty() && after.warnings.is_empty());
    assert_eq!(normalized_svg(&before), normalized_svg(&after));
    assert!(format("").is_err());
}

fn normalized_svg(compiled: &layup::Compiled) -> String {
    let mut svg = layup::svg::render(compiled, layup::Theme::Light);
    if let Some(document) = &compiled.document {
        fn semantic(value: &mut serde_json::Value) {
            match value {
                serde_json::Value::Object(fields) => {
                    if fields
                        .get("span")
                        .is_some_and(|span| span.get("start").is_some())
                    {
                        fields.remove("span");
                        fields.remove("valueSpan");
                        fields.remove("nameSpan");
                    }
                    if fields.contains_key("arguments") && fields.contains_key("name") {
                        fields.remove("raw");
                    }
                    for (key, value) in fields.iter_mut() {
                        if key != "metadata" && !(key == "value" && value.get("type").is_some()) {
                            semantic(value);
                        }
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
        let original = serde_json::to_string(document).unwrap();
        let mut normalized = serde_json::to_value(document).unwrap();
        semantic(&mut normalized);
        // Source locations/spelling change after formatting; retain every semantic value.
        svg = svg.replace(
            &layup::svg::esc(&original),
            &layup::svg::esc(&normalized.to_string()),
        );
    }
    // The document namespace includes authored source line numbers. Ignore
    // only that namespace while comparing every rendered shape and glyph.
    let start = svg.find("layup-").unwrap();
    let end = start + "layup-".len() + 8;
    svg.replace(&svg[start..end], "layup-namespace")
}

#[test]
fn formatting_existing_diagrams_is_idempotent_and_preserves_rendering() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut directories = vec![root.join("examples"), root.join("docs")];
    let mut checked = 0;
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().unwrap() != "node_modules"
                    && path.file_name().unwrap() != ".vitepress"
                {
                    directories.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "layup") {
                let source = std::fs::read_to_string(&path).unwrap();
                let formatted = format(&source).unwrap();
                assert_eq!(format(&formatted).unwrap(), formatted, "{}", path.display());
                assert!(
                    normalized_svg(&compile(&source).unwrap())
                        == normalized_svg(&compile(&formatted).unwrap()),
                    "formatting changed rendering of {}",
                    path.display()
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 20, "fixture discovery missed the diagrams");
}

#[test]
fn lint_rules_distinguish_duplicates_from_distinct_transitions() {
    let source = r#"diagram main type=graph {
        node-style unused palette=purple
        edge-style unused-arrow stroke-style=dashed
        node a palette=blue
        node b
        a -> b "go" bus=true
        a -> b "go" bus=true
        a -> b "retry" bus=true
    }"#;
    let diagnostics = lint(source);
    for code in [
        "lint/unused-style",
        "lint/unused-arrow",
        "lint/duplicate-transition",
    ] {
        assert!(
            diagnostics.iter().any(|d| d.code == code),
            "{diagnostics:?}"
        );
    }
    assert_eq!(
        diagnostics
            .iter()
            .filter(|d| d.code == "lint/duplicate-transition")
            .count(),
        1
    );
    assert!(diagnostics.iter().all(|d| d.severity == Severity::Warning));
    assert!(
        lint("diagram main type=graph { node a palette=blue palette=green }")
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
    assert!(lint("diagram main type=graph { node-style special; node-style used base=special; node a style=used; edge-style event palette=blue; a -> a style=event }").iter().all(|d| !d.code.starts_with("lint/unused")));
}

#[test]
fn lint_rejects_ignored_directive_bodies_and_locates_real_connections() {
    let invalid = "diagram main type=graph { node a { code \"literal\" { node hidden } } }";
    assert!(lint(invalid).iter().any(|d| d.severity == Severity::Error));
    let source = "diagram main type=graph {\nnode a\nnode b\na -> b\na -> b\n}";
    let diagnostics = lint(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "lint/duplicate-transition");
    assert_eq!(diagnostics[0].line, Some(5));
    assert_eq!(diagnostics[0].related[0].0.line, 4);
}

#[test]
fn terminal_excerpts_account_for_cjk_and_tabs_and_keep_machine_columns_scalar() {
    let src = "diagram main \"T\" type=graph {\n\tnode 中文 aling=left\n}";
    let e = compile(src).err().unwrap();
    let d = layup::diagnostic::Diagnostic::from_error(&e);
    assert_eq!(d.span.unwrap().column, 10);
    let shown = d.display(src, "diagram.layup", 4);
    assert!(shown.contains("diagram.layup:6:10:"));
    assert!(
        shown.contains(&format!("| {}^^^^^", " ".repeat(14))),
        "{shown}"
    );
    assert!(d.json().contains("\"column\":10"));
}
