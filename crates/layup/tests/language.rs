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
    let ds = lint(src);
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
    for (src, bad, help) in [
        (
            r#"diagram "T" { node worker; worker -> wokrer }"#,
            "wokrer",
            "worker",
        ),
        (r#"diagram "T" { node a aling=left }"#, "aling", "align"),
        (r#"diagram "T" widht=900 { node a }"#, "widht", "width"),
        (r#"diagram "T" { decison q }"#, "decison", "decision"),
    ] {
        let e = compile(src).err().unwrap();
        let s = e.span.unwrap();
        assert_eq!(&src[s.start..s.end], bad);
        assert!(e.help.unwrap().contains(help));
    }
    let src = r#"diagram "T" { node a; node a; a -> a }"#;
    let e = compile(src).err().unwrap();
    assert_eq!(e.code, "semantic/duplicate-id");
    assert_eq!(e.span.unwrap().start, src.find("node a; a").unwrap() + 5);
}

#[test]
fn invalid_geometry_numbers_are_rejected_before_layout() {
    for src in [
        r#"diagram "T" width=0 {}"#,
        r#"diagram "T" { width -1 }"#,
        r#"diagram "T" { gap -4 }"#,
        r#"diagram "T" { row 0:1 { node a; node b } }"#,
        r#"diagram "T" { row -1 { node a } }"#,
        r#"diagram "T" { row 1e308:1e308 { node a; node b } }"#,
        r#"diagram "T" { node a gutter=-1 }"#,
    ] {
        assert_eq!(compile(src).err().unwrap().code, "semantic/number", "{src}");
    }
}

#[test]
fn formatter_preserves_comments_strings_and_rendered_semantics_and_is_idempotent() {
    let src = r#"// Overview
 diagram "国际" mode = state-machine direction = right{ // machine
initial root;state parent "متصل"{initial start; state a "A"{sub "entry / f(\"x\"); path C:\\tmp"}; // action
final done;start->a;a->done "完成"}; root ->parent // entry
}
"#;
    let formatted = format(src).unwrap();
    assert_eq!(format(&formatted).unwrap(), formatted);
    assert!(formatted.contains("// Overview\ndiagram"));
    assert!(
        formatted.contains("{  // machine\n  initial root\n"),
        "{formatted}"
    );
    assert!(formatted.contains(r#""entry / f(\"x\"); path C:\\tmp""#));
    let a = compile(src).unwrap();
    let b = compile(&formatted).unwrap();
    assert!(a.warnings.is_empty() && b.warnings.is_empty());
    assert!(
        normalized_svg(&a) == normalized_svg(&b),
        "formatting changed rendered geometry or text"
    );
    for src in [
        "// only a comment\n",
        "diagram \"T\" {\n\n // comment\n node a\n\n\n node b\n}\n",
        "node a { code \"first\r\n  second\" }",
        "diagram \"T\" {}",
        "",
    ] {
        let f = format(src).unwrap();
        assert_eq!(format(&f).unwrap(), f, "{src:?}");
    }
}

fn normalized_svg(compiled: &layup::Compiled) -> String {
    let svg = layup::svg::render(compiled, layup::Theme::Light);
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
    let src = r#"diagram "T" {
        style unused tone=purple
        arrow unused-arrow dashed
        node a blue blue
        node b
        a -> b "go" bus
        a -> b "go" bus
        a -> b "retry" bus
    }"#;
    let ds = lint(src);
    for code in [
        "lint/unused-style",
        "lint/unused-arrow",
        "lint/overridden-flag",
        "lint/duplicate-transition",
    ] {
        assert!(ds.iter().any(|d| d.code == code), "{ds:?}");
    }
    assert_eq!(
        ds.iter()
            .filter(|d| d.code == "lint/duplicate-transition")
            .count(),
        1
    );
    assert!(ds.iter().all(|d| d.severity == Severity::Warning));
    assert!(lint(r#"diagram "T" { style special base=state; style used base=special; used a; arrow event blue; a -event-> a }"#).iter().all(|d| !d.code.starts_with("lint/unused")));
}

#[test]
fn lint_ignores_directive_bodies_when_locating_real_transitions() {
    let source = "diagram \"T\" {\n title \"T\" { arrow unused blue; a -> b }\n node a\n node b\n a -> b\n a -> b\n}";
    let diagnostics = lint(source);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "lint/ignored-body");
    let duplicate = &diagnostics[1];
    assert_eq!(duplicate.code, "lint/duplicate-transition");
    assert_eq!(duplicate.line, Some(6));
    assert_eq!(duplicate.related[0].0.line, 5);
}

#[test]
fn terminal_excerpts_account_for_cjk_and_tabs_and_keep_machine_columns_scalar() {
    let src = "diagram \"T\" {\n\tnode 中文 aling=left\n}";
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
