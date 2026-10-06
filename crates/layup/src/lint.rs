//! Authoring diagnostics, separate from rendering. Syntax recovery collects
//! independent errors; semantic/layout checks only run for valid syntax.
use crate::{
    diagnostic::{Diagnostic, Severity, Span},
    model::Block,
    parser::{Arg, Item, Stmt, Value},
    text::Fonts,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn lint(source: &str) -> Vec<Diagnostic> {
    lint_with_fonts(source, &Fonts::default())
}

pub fn lint_with_fonts(source: &str, fonts: &Fonts) -> Vec<Diagnostic> {
    lint_with_options(source, &crate::CompileOptions::default(), fonts)
}

pub fn lint_with_options(
    source: &str,
    options: &crate::CompileOptions,
    fonts: &Fonts,
) -> Vec<Diagnostic> {
    if crate::document::is_versioned(source) {
        return match crate::compile_with_options(source, options, fonts) {
            Err(error) => vec![Diagnostic::from_error(&error)],
            Ok(compiled) => compiled
                .warnings
                .iter()
                .map(|warning| Diagnostic {
                    severity: Severity::Warning,
                    code: "layout",
                    message: warning.msg.clone(),
                    line: warning.line,
                    span: None,
                    help: None,
                    related: Vec::new(),
                })
                .collect(),
        };
    }
    let parsed = crate::parser::parse_recovering(source);
    if !parsed.errors.is_empty() {
        return parsed.errors.iter().map(Diagnostic::from_error).collect();
    }
    let compiled = match crate::compile_with_options(source, options, fonts) {
        Err(e) => return vec![Diagnostic::from_error(&e)],
        Ok(c) => c,
    };
    let tokens = crate::lexer::lex(source).unwrap();
    let mut diagnostics = Vec::new();
    for warning in &compiled.warnings {
        let mut e = crate::Error::new(&warning.msg);
        e.line = warning.line;
        crate::diagnostic::locate(&mut e, &tokens);
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            code: "layout",
            message: e.msg,
            span: e.span.as_deref().copied(),
            line: e.line,
            help: None,
            related: Vec::new(),
        });
    }
    let mut items = Vec::new();
    let mut edges = Vec::new();
    let mut resolved = crate::views::resolve(&parsed.statements, options.view.as_deref())
        .expect("validated by compilation");
    crate::presentation::extract(&mut resolved.statements).expect("validated by compilation");
    let mut stack: Vec<_> = resolved.statements.iter().rev().collect();
    while let Some(stmt) = stack.pop() {
        match stmt {
            Stmt::Item(i) => {
                items.push(i);
                if !ignores_body(&i.head)
                    && let Some(body) = &i.body
                {
                    stack.extend(body.iter().rev());
                }
            }
            Stmt::Edge(e) => edges.push(e),
        }
    }
    let mut used_kinds = BTreeSet::new();
    let mut stack: Vec<_> = compiled.diagram.blocks.iter().collect();
    while let Some(b) = stack.pop() {
        match b {
            Block::Node(n) => {
                used_kinds.insert(n.kind.as_str());
                stack.extend(&n.children);
            }
            Block::Row(r) => stack.extend(r.cells.iter().flatten()),
            Block::Section(s) => stack.extend(&s.children),
            Block::Flow { layers, .. } => stack.extend(layers.iter().flatten()),
            Block::Tree { nodes, .. } => stack.extend(nodes),
            _ => {}
        }
    }
    for i in &items {
        if i.head == "style" {
            for arg in i.args.iter().skip(1) {
                match arg {
                    Arg::Attr(k, Value::Ident(s) | Value::Str(s)) if k == "base" => {
                        used_kinds.insert(s);
                    }
                    Arg::Value(Value::Ident(s)) if compiled.diagram.kinds.contains_key(s) => {
                        used_kinds.insert(s);
                    }
                    _ => {}
                }
            }
        }
    }
    let mut used_arrows: BTreeSet<_> = compiled
        .diagram
        .edges
        .iter()
        .map(|e| e.kind.as_str())
        .collect();
    if compiled.selected_view.is_some() {
        // Declarations belong to the shared model. A filtered-out use in
        // another view still makes that declaration useful in the source.
        let mut shared: Vec<_> = parsed.statements.iter().collect();
        while let Some(statement) = shared.pop() {
            match statement {
                Stmt::Edge(edge) => {
                    used_arrows.insert(edge.kind.as_deref().unwrap_or("default"));
                }
                Stmt::Item(item) if item.head != "view" && !ignores_body(&item.head) => {
                    if compiled.diagram.kinds.contains_key(&item.head) {
                        used_kinds.insert(item.head.as_str());
                    }
                    if let Some(body) = &item.body {
                        shared.extend(body);
                    }
                }
                _ => {}
            }
        }
    }
    for i in &items {
        if let Some(Arg::Value(Value::Ident(name))) = i.args.first() {
            if i.head == "style" && !used_kinds.contains(name.as_str()) {
                diagnostics.push(warning(
                    "lint/unused-style",
                    format!("style `{name}` is not used"),
                    i.arg_spans[0],
                    Some("remove the declaration or use this kind on a node".into()),
                ));
            }
            if i.head == "arrow" && !used_arrows.contains(name.as_str()) {
                diagnostics.push(warning(
                    "lint/unused-arrow",
                    format!("arrow kind `{name}` is not used"),
                    i.arg_spans[0],
                    Some("remove the declaration or use it on a transition".into()),
                ));
            }
        }
        if i.body.is_some() && ignores_body(&i.head) {
            diagnostics.push(warning(
                "lint/ignored-body",
                format!("the body of `{}` is ignored", i.head),
                i.head_span,
                Some("move its contents into a node, row, group or section".into()),
            ));
        }
        if compiled.diagram.kinds.contains_key(&i.head)
            || matches!(i.head.as_str(), "style" | "arrow")
        {
            lint_flags(i, &mut diagnostics);
        }
    }
    let mut transitions = BTreeMap::new();
    for (e, syntax) in compiled.diagram.edges.iter().zip(edges) {
        if compiled.diagram.mode == crate::model::Mode::Sequence {
            // Repeated calls are separate chronological events, including retries.
            continue;
        }
        let mut normalized = e.clone();
        normalized.line = 0;
        normalized.span = Span::default();
        normalized.id.clear();
        let signature = format!("{normalized:?}");
        if let Some(first) = transitions.get(&signature) {
            let mut d=warning("lint/duplicate-transition",format!("duplicate transition {} -> {}",e.from,e.to),syntax.span,Some("remove the duplicate, or give distinct transitions different labels or routing attributes".into()));
            d.related.push((*first, "first transition is here".into()));
            diagnostics.push(d);
        } else {
            transitions.insert(signature, syntax.span);
        }
    }
    diagnostics.sort_by_key(|d| d.span.map_or(usize::MAX, |s| s.start));
    diagnostics
}

fn warning(code: &'static str, message: String, span: Span, help: Option<String>) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        code,
        message,
        span: Some(span),
        line: Some(span.line),
        help,
        related: Vec::new(),
    }
}
fn ignores_body(head: &str) -> bool {
    matches!(
        head,
        "style"
            | "arrow"
            | "title"
            | "name"
            | "note"
            | "subtitle"
            | "desc"
            | "width"
            | "preset"
            | "legend"
            | "divider"
            | "gap"
            | "text"
            | "code"
            | "sub"
            | "role"
            | "tag"
    )
}
fn lint_flags(item: &Item, diagnostics: &mut Vec<Diagnostic>) {
    let mut flags = BTreeMap::new();
    let skip = usize::from(matches!(item.head.as_str(), "style" | "arrow"));
    for (arg, span) in item.args.iter().zip(&item.arg_spans).skip(skip) {
        let Arg::Value(Value::Ident(flag)) = arg else {
            continue;
        };
        let category = match flag.as_str() {
            "hollow" | "filled" => "fill",
            "mono" | "sans" => "font",
            "left" | "center" => "alignment",
            "dashed" | "solid" => "stroke",
            t if crate::style::Tone::parse(t).is_some() => "tone",
            _ => continue,
        };
        if let Some((first, previous)) = flags.insert(category, (*span, flag)) {
            let mut d = warning(
                "lint/overridden-flag",
                format!("`{flag}` repeats or overrides `{previous}`"),
                *span,
                Some("keep one flag for each styling property".into()),
            );
            d.related.push((first, "earlier flag is here".into()));
            diagnostics.push(d);
        }
    }
}
