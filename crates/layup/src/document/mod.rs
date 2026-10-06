//! Revision-one document syntax. Shared values and references are parsed here;
//! diagram bodies dispatch to a grammar rather than acquiring dynamic keywords.
mod compile;
mod format;
mod lex;
mod parse;

use crate::diagnostic::Diagnostic;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

use crate::{CompileOptions, Compiled, Error, diagnostic::Span, text::Fonts};

pub use compile::{EntityInfo, Info, ManifestEntry, TargetInfo};

/// Exact signed/unsigned 64-bit integer; tagged document values serialize as decimal strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integer {
    Signed(i64),
    Unsigned(u64),
}
impl Integer {
    pub fn as_f64(self) -> f64 {
        match self {
            Self::Signed(n) => n as f64,
            Self::Unsigned(n) => n as f64,
        }
    }
}
impl Serialize for Integer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&match self {
            Self::Signed(n) => n.to_string(),
            Self::Unsigned(n) => n.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum Value {
    Choice(String),
    String(String),
    Integer(Integer),
    Float(f64),
    Bool(bool),
    Null,
    List(Vec<Value>),
    Record(BTreeMap<String, Value>),
    Reference(Reference),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub root: bool,
    pub segments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribute {
    pub value: Value,
    pub span: Span,
    pub value_span: Span,
}
pub type Attributes = BTreeMap<String, Attribute>;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub name: String,
    pub raw: String,
    pub arguments: Attributes,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub version: u32,
    pub source: String,
    pub value_spans: Vec<ValueSpan>,
    pub diagrams: Vec<Diagram>,
    #[serde(skip)]
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagram {
    pub id: String,
    pub title: String,
    pub diagram_type: String,
    pub type_span: Span,
    pub attributes: Attributes,
    pub annotations: Vec<Annotation>,
    pub body: Body,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum Body {
    Graph(Vec<GraphStatement>),
    Opaque { raw: String, span: Span },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum GraphStatement {
    Node(Node),
    Edge(Edge),
    NodeKind(Kind),
    EdgeKind(Kind),
    Row {
        annotations: Vec<Annotation>,
        attributes: Attributes,
        body: Vec<GraphStatement>,
        span: Span,
    },
    Content {
        annotations: Vec<Annotation>,
        kind: String,
        text: String,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub declaration: String,
    pub id: String,
    pub title: String,
    pub attributes: Attributes,
    pub annotations: Vec<Annotation>,
    pub body: Vec<GraphStatement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub id: Option<String>,
    pub from: Reference,
    pub to: Reference,
    pub arrow: String,
    pub label: Option<String>,
    pub attributes: Attributes,
    pub annotations: Vec<Annotation>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Kind {
    pub annotations: Vec<Annotation>,
    pub id: String,
    pub attributes: Attributes,
    pub span: Span,
}

/// Recognize named document declarations and explicit revision assertions.
/// An invalid assertion never falls back to the title-only legacy grammar.
pub fn is_document(source: &str) -> bool {
    let mut rest = source;
    loop {
        rest = rest.trim_start();
        if rest.starts_with("//") {
            let Some(end) = rest.find('\n') else {
                return false;
            };
            rest = &rest[end + 1..];
        } else if rest.starts_with("/*") {
            let Ok(end) = lex::block_comment_length(rest) else {
                return false;
            };
            rest = &rest[end..];
        } else {
            if rest.starts_with('@')
                || rest.strip_prefix("layup").is_some_and(|tail| {
                    tail.is_empty()
                        || !tail
                            .starts_with(|c: char| unicode_ident::is_xid_continue(c) || c == '-')
                })
            {
                return true;
            }
            let mut lexer = lex::Lexer::new(rest);
            let next = |lexer: &mut lex::Lexer<'_>| {
                loop {
                    let token = lexer.next().ok()?;
                    if !matches!(
                        token.token,
                        lex::Token::Comment | lex::Token::BlockComment | lex::Token::Newline
                    ) {
                        return Some(token.token);
                    }
                }
            };
            if next(&mut lexer) != Some(lex::Token::Word("diagram".into())) {
                return false;
            }
            match next(&mut lexer) {
                Some(lex::Token::Word(_)) => return next(&mut lexer) != Some(lex::Token::Equal),
                Some(lex::Token::String(_)) => {
                    let mut previous = None;
                    let mut depth = 0usize;
                    while let Some(token) = next(&mut lexer) {
                        match token {
                            lex::Token::Eof => break,
                            lex::Token::Open
                                if depth == 0 && previous != Some(lex::Token::Equal) =>
                            {
                                break;
                            }
                            lex::Token::Open | lex::Token::ParenOpen | lex::Token::ListOpen => {
                                depth += 1
                            }
                            lex::Token::Close | lex::Token::ParenClose | lex::Token::ListClose => {
                                depth = depth.saturating_sub(1)
                            }
                            _ => {}
                        }
                        if depth == 0
                            && token == lex::Token::Equal
                            && matches!(previous, Some(lex::Token::Word(ref word)) if word == "type" || word == "kind")
                        {
                            return true;
                        }
                        previous = Some(token);
                    }
                    return false;
                }
                _ => return false,
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueSpan {
    pub span: Span,
    pub kind: String,
}

/// Parse with sibling recovery; the partial document is never implicitly renderable.
pub fn parse_recovering(source: &str) -> Parsed {
    parse::parse_recovering(source)
}
#[derive(Debug)]
pub struct Parsed {
    pub document: Document,
    pub errors: Vec<Error>,
}

/// Inspect syntax and opaque extension bodies without requiring a renderer.
pub fn inspect(source: &str) -> serde_json::Value {
    let mut parsed = parse_recovering(source);
    if parsed.errors.is_empty()
        && let Err(error) = compile::validate_annotations(&parsed.document)
    {
        parsed.errors.push(error);
    }
    let diagnostics = parsed
        .errors
        .iter()
        .map(Diagnostic::from_error)
        .chain(parsed.document.warnings.iter().cloned())
        .map(|d| serde_json::from_str::<serde_json::Value>(&d.json()).expect("diagnostic JSON"))
        .collect::<Vec<_>>();
    serde_json::json!({ "document": parsed.document, "diagnostics": diagnostics })
}

pub fn parse(source: &str) -> Result<Document, Error> {
    parse::parse(source)
}

/// Resolve and validate the default diagram without measuring text or routing.
pub fn build(source: &str) -> Result<crate::model::Diagram, Error> {
    compile::build(&parse(source)?)
}

pub fn compile(source: &str, options: &CompileOptions, fonts: &Fonts) -> Result<Compiled, Error> {
    compile::compile(&parse(source)?, options, fonts).map_err(|mut error| {
        if error.span.is_none()
            && let Ok(tokens) = lex::document_tokens(source)
        {
            error.span = tokens
                .iter()
                .find(|t| Some(t.span.line) == error.line)
                .map(|t| Box::new(t.span));
        }
        error
    })
}

pub fn format(source: &str) -> Result<String, Error> {
    format::format(source)
}

fn error(span: Span, message: impl Into<String>) -> Error {
    Error::located(span, "document/syntax", message)
}

/// Conservative typo hints only within an unqualified core registry.
fn hint(name: &str, candidates: &[&str]) -> Option<String> {
    if name.contains('.')
        || name.chars().count() < 4
        || name.chars().count()
            > candidates
                .iter()
                .map(|c| c.chars().count())
                .max()
                .unwrap_or(0)
                + 1
    {
        return None;
    }
    let distance = |target: &str| {
        let a: Vec<_> = name.chars().collect();
        let b: Vec<_> = target.chars().collect();
        let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
        for (i, row) in d.iter_mut().enumerate() {
            row[0] = i;
        }
        for (j, cell) in d[0].iter_mut().enumerate() {
            *cell = j;
        }
        for i in 1..=a.len() {
            for j in 1..=b.len() {
                d[i][j] = (d[i - 1][j] + 1)
                    .min(d[i][j - 1] + 1)
                    .min(d[i - 1][j - 1] + usize::from(a[i - 1] != b[j - 1]));
                if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                    d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
                }
            }
        }
        d[a.len()][b.len()]
    };
    let matches = candidates
        .iter()
        .filter(|candidate| **candidate != name && distance(candidate) <= 1)
        .collect::<Vec<_>>();
    (matches.len() == 1).then(|| format!("did you mean `{}`?", matches[0]))
}
fn warning(span: Span, code: &'static str, message: String, help: Option<String>) -> Diagnostic {
    Diagnostic {
        severity: crate::diagnostic::Severity::Warning,
        code,
        message,
        line: Some(span.line),
        span: Some(span),
        help,
        related: Vec::new(),
    }
}
fn diagnose_extensions(document: &mut Document) {
    let annotation_warnings = |annotations: &[Annotation], warnings: &mut Vec<Diagnostic>| {
        for a in annotations {
            if let Some(help) = hint(&a.name, &["source", "doc", "meta"]) {
                warnings.push(warning(
                    a.span,
                    "annotation/possible-typo",
                    format!(
                        "unrecognized annotation @{} is preserved without interpretation",
                        a.name
                    ),
                    Some(help),
                ));
            }
        }
    };
    fn visit(
        body: &[GraphStatement],
        warnings: &mut Vec<Diagnostic>,
        f: &impl Fn(&[Annotation], &mut Vec<Diagnostic>),
    ) {
        for statement in body {
            match statement {
                GraphStatement::Node(node) => {
                    f(&node.annotations, warnings);
                    visit(&node.body, warnings, f);
                }
                GraphStatement::Edge(edge) => f(&edge.annotations, warnings),
                GraphStatement::NodeKind(kind) | GraphStatement::EdgeKind(kind) => {
                    f(&kind.annotations, warnings)
                }
                GraphStatement::Row {
                    annotations, body, ..
                } => {
                    f(annotations, warnings);
                    visit(body, warnings, f);
                }
                GraphStatement::Content { annotations, .. } => f(annotations, warnings),
            }
        }
    }
    for diagram in &document.diagrams {
        annotation_warnings(&diagram.annotations, &mut document.warnings);
        match &diagram.body {
            Body::Graph(body) => visit(body, &mut document.warnings, &annotation_warnings),
            Body::Opaque { .. } => {
                let known =
                    ["sequence", "state-machine", "er"].contains(&diagram.diagram_type.as_str());
                document.warnings.push(warning(diagram.type_span,
                    if known { "diagram/unavailable-type" } else { "diagram/unrecognized-type" },
                    format!("diagram `{}` with type `{}` is preserved and skipped: its body grammar is unavailable", diagram.id, diagram.diagram_type),
                    if known { None } else { hint(&diagram.diagram_type, &["graph", "sequence", "state-machine", "er"]) }));
            }
        }
    }
}
