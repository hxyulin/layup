//! Recovering source diagnostics, followed by semantic and layout validation.
use crate::{
    diagnostic::{Diagnostic, Severity},
    document::{GraphStatement, Value},
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
    let parsed = crate::document::parse_recovering(source);
    let mut diagnostics = parsed
        .errors
        .iter()
        .map(Diagnostic::from_error)
        .collect::<Vec<_>>();
    diagnostics.extend(parsed.document.warnings.iter().cloned());
    if !parsed.errors.is_empty() {
        return diagnostics;
    }
    let compiled = match crate::compile_with_options(source, options, fonts) {
        Err(error) => {
            diagnostics.insert(0, Diagnostic::from_error(&error));
            return diagnostics;
        }
        Ok(compiled) => compiled,
    };
    diagnostics.extend(
        compiled
            .warnings
            .iter()
            .take(
                compiled
                    .warnings
                    .len()
                    .saturating_sub(parsed.document.warnings.len()),
            )
            .map(|warning| Diagnostic {
                severity: Severity::Warning,
                code: "layout",
                message: warning.msg.clone(),
                line: warning.line,
                span: warning.line.and_then(|line| {
                    parsed
                        .document
                        .diagrams
                        .iter()
                        .flat_map(|d| d.body.statements().unwrap_or(&[]))
                        .find(|s| s.span().line == line)
                        .map(GraphStatement::span)
                }),
                help: None,
                related: Vec::new(),
            }),
    );
    for diagram in &parsed.document.diagrams {
        let Some(body) = diagram.body.statements() else {
            continue;
        };
        let mut statements = Vec::new();
        fn visit<'a>(body: &'a [GraphStatement], statements: &mut Vec<&'a GraphStatement>) {
            for s in body {
                statements.push(s);
                if !matches!(s, GraphStatement::View(_) | GraphStatement::Step(_)) {
                    visit(s.children(), statements);
                }
            }
        }
        visit(body, &mut statements);
        let mut node_styles = BTreeSet::new();
        let mut edge_styles = BTreeSet::new();
        for statement in &statements {
            let (attrs, styles) = match statement {
                GraphStatement::Node(node) => (&node.attributes, &mut node_styles),
                GraphStatement::Edge(edge) => (&edge.attributes, &mut edge_styles),
                GraphStatement::NodeStyle(kind) => (&kind.attributes, &mut node_styles),
                GraphStatement::EdgeStyle(kind) => (&kind.attributes, &mut edge_styles),
                GraphStatement::Defaults {
                    category,
                    attributes,
                    ..
                } => (
                    attributes,
                    if matches!(
                        category,
                        crate::document::DefaultCategory::Edge
                            | crate::document::DefaultCategory::Message
                            | crate::document::DefaultCategory::Transition
                    ) {
                        &mut edge_styles
                    } else {
                        &mut node_styles
                    },
                ),
                _ => continue,
            };
            for name in ["style", "base"] {
                if let Some(a) = attrs.get(name)
                    && let Value::Choice(name) | Value::String(name) = &a.value
                {
                    styles.insert(name.clone());
                }
            }
        }
        for statement in statements {
            let (kind, edge) = match statement {
                GraphStatement::NodeStyle(k) => (k, false),
                GraphStatement::EdgeStyle(k) => (k, true),
                _ => continue,
            };
            if !(if edge { &edge_styles } else { &node_styles }).contains(&kind.id) {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    code: if edge {
                        "lint/unused-arrow"
                    } else {
                        "lint/unused-style"
                    },
                    message: format!(
                        "{} style `{}` is not used",
                        if edge { "edge" } else { "node" },
                        kind.id
                    ),
                    span: Some(kind.span),
                    line: Some(kind.span.line),
                    help: Some("remove the declaration or reference it with style=".into()),
                    related: Vec::new(),
                });
            }
        }
    }
    if compiled.diagram.mode != crate::model::Mode::Sequence {
        let mut transitions = BTreeMap::new();
        for edge in &compiled.diagram.edges {
            let mut normalized = edge.clone();
            normalized.line = 0;
            normalized.span = Default::default();
            normalized.id.clear();
            let signature = format!("{normalized:?}");
            if let Some(first) = transitions.insert(signature, edge.span) {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    code: "lint/duplicate-transition",
                    message: "duplicate connection with the same endpoints, label and routing"
                        .into(),
                    span: Some(edge.span),
                    line: Some(edge.span.line),
                    help: Some(
                        "remove the duplicate or give it distinct display/routing properties"
                            .into(),
                    ),
                    related: vec![(first, "first connection".into())],
                });
            }
        }
    }
    diagnostics.sort_by_key(|d| d.span.map_or(usize::MAX, |s| s.start));
    diagnostics
}
