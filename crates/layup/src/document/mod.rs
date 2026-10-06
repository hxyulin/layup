//! Revision-one document syntax. Shared values and references are parsed here;
//! diagram bodies dispatch to a grammar rather than acquiring dynamic keywords.
mod compile;
mod format;
mod lex;
mod paint;
mod parse;
pub use paint::{Color, Paint, PaletteOrigin, StrokeStyle};

use crate::diagnostic::Diagnostic;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

use crate::{CompileOptions, Compiled, Error, diagnostic::Span, text::Fonts};

pub(crate) use compile::restore_styles;
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
    pub name_span: Span,
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
    Sequence(Vec<GraphStatement>),
    StateMachine(Vec<GraphStatement>),
    Opaque { raw: String, span: Span },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum GraphStatement {
    Layout {
        kind: LayoutKind,
        title: Option<String>,
        attributes: Attributes,
        annotations: Vec<Annotation>,
        body: Vec<GraphStatement>,
        span: Span,
    },
    Configuration {
        kind: ConfigurationKind,
        attributes: Attributes,
        annotations: Vec<Annotation>,
        span: Span,
    },
    Defaults {
        category: DefaultCategory,
        attributes: Attributes,
        annotations: Vec<Annotation>,
        span: Span,
    },
    View(NamedBlock),
    Step(NamedBlock),
    Selection {
        kind: SelectionKind,
        objects: Vec<Reference>,
        object_spans: Vec<Span>,
        connections: Vec<String>,
        connection_spans: Vec<Span>,
        annotations: Vec<Annotation>,
        span: Span,
    },
    Fragment {
        kind: FragmentKind,
        title: String,
        body: Vec<GraphStatement>,
        annotations: Vec<Annotation>,
        span: Span,
    },
    SequenceNote {
        text: String,
        attributes: Attributes,
        annotations: Vec<Annotation>,
        span: Span,
    },
    Port {
        id: String,
        attributes: Attributes,
        annotations: Vec<Annotation>,
        span: Span,
    },

    Node(Node),
    Edge(Edge),
    NodeStyle(Kind),
    EdgeStyle(Kind),
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
    pub label_span: Option<Span>,
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
    pub from_span: Span,
    pub to_span: Span,
    pub arrow_span: Span,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedBlock {
    pub id_span: Span,
    pub id: String,
    pub title: String,
    pub attributes: Attributes,
    pub annotations: Vec<Annotation>,
    pub body: Vec<GraphStatement>,
    pub span: Span,
}
macro_rules! syntax_choices {
    ($name:ident { $($variant:ident),* }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
        #[serde(rename_all = "kebab-case")]
        pub enum $name { $($variant),* }
    };
}
syntax_choices!(LayoutKind {
    Section,
    Band,
    Divider,
    Gap
});
syntax_choices!(ConfigurationKind { Slide, Legend });
syntax_choices!(DefaultCategory {
    Node,
    Edge,
    Participant,
    Message,
    State,
    Transition
});
syntax_choices!(SelectionKind {
    Include,
    Show,
    Highlight
});
syntax_choices!(FragmentKind {
    Loop,
    Optional,
    Alternatives,
    Branch
});

impl Body {
    pub fn statements(&self) -> Option<&[GraphStatement]> {
        match self {
            Self::Graph(body) | Self::Sequence(body) | Self::StateMachine(body) => Some(body),
            Self::Opaque { .. } => None,
        }
    }
}
impl GraphStatement {
    pub fn annotations(&self) -> &[Annotation] {
        match self {
            Self::Node(n) => &n.annotations,
            Self::Edge(e) => &e.annotations,
            Self::NodeStyle(k) | Self::EdgeStyle(k) => &k.annotations,
            Self::View(n) | Self::Step(n) => &n.annotations,
            Self::Row { annotations, .. }
            | Self::Content { annotations, .. }
            | Self::Layout { annotations, .. }
            | Self::Configuration { annotations, .. }
            | Self::Defaults { annotations, .. }
            | Self::Selection { annotations, .. }
            | Self::Fragment { annotations, .. }
            | Self::SequenceNote { annotations, .. }
            | Self::Port { annotations, .. } => annotations,
        }
    }
    pub fn span(&self) -> Span {
        match self {
            Self::Node(n) => n.span,
            Self::Edge(e) => e.span,
            Self::NodeStyle(k) | Self::EdgeStyle(k) => k.span,
            Self::View(n) | Self::Step(n) => n.span,
            Self::Row { span, .. }
            | Self::Content { span, .. }
            | Self::Layout { span, .. }
            | Self::Configuration { span, .. }
            | Self::Defaults { span, .. }
            | Self::Selection { span, .. }
            | Self::Fragment { span, .. }
            | Self::SequenceNote { span, .. }
            | Self::Port { span, .. } => *span,
        }
    }
    pub fn children(&self) -> &[GraphStatement] {
        match self {
            Self::Node(n) => &n.body,
            Self::View(n) | Self::Step(n) => &n.body,
            Self::Row { body, .. } | Self::Layout { body, .. } | Self::Fragment { body, .. } => {
                body
            }
            _ => &[],
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
    let document = parse(source)?;
    compile::build(&document).map_err(|mut error| {
        error.msg = readable(&document, &error.msg);
        locate_error(source, &mut error);
        error
    })
}

pub fn compile(source: &str, options: &CompileOptions, fonts: &Fonts) -> Result<Compiled, Error> {
    let document = parse(source)?;
    let result = compile::compile(&document, options, fonts).map(|mut compiled| {
        for warning in &mut compiled.warnings {
            warning.msg = readable(&document, &warning.msg);
        }
        compiled
    });
    result.map_err(|mut error| {
        error.msg = readable(&document, &error.msg);
        locate_error(source, &mut error);
        error
    })
}

fn locate_error(source: &str, error: &mut Error) {
    if error.span.is_some() {
        return;
    }
    let Ok(tokens) = lex::document_tokens(source) else {
        return;
    };
    let names = error
        .msg
        .split('`')
        .enumerate()
        .filter_map(|(i, name)| (i % 2 == 1).then_some(name.split('=').next().unwrap_or(name)))
        .collect::<Vec<_>>();
    let on_line = tokens
        .iter()
        .filter(|token| {
            (error.line.is_none() || Some(token.span.line) == error.line)
                && !matches!(
                    token.token,
                    lex::Token::Newline
                        | lex::Token::Comment
                        | lex::Token::BlockComment
                        | lex::Token::Eof
                )
        })
        .collect::<Vec<_>>();
    let token = names.iter().find_map(|name| {
        on_line.iter().rev().find(|token|
            matches!(&token.token, lex::Token::Word(word) | lex::Token::String(word) if word == name)
        ).copied()
    }).or_else(|| on_line.first().copied());
    if let Some(token) = token {
        error.span = Some(Box::new(token.span));
        error.line = Some(token.span.line);
    }
}

fn readable(document: &Document, message: &str) -> String {
    fn visit(body: &[GraphStatement], scope: &[String], pairs: &mut Vec<(String, String)>) {
        for statement in body {
            if let GraphStatement::Node(node) = statement {
                let path = [scope.to_vec(), vec![node.id.clone()]].concat();
                pairs.push((
                    format!("object:{}", serde_json::to_string(&path).unwrap()),
                    path.join("."),
                ));
                visit(&node.body, &path, pairs);
            } else {
                visit(statement.children(), scope, pairs);
            }
        }
    }
    let mut pairs = Vec::new();
    for diagram in &document.diagrams {
        if let Some(body) = diagram.body.statements() {
            visit(body, &[], &mut pairs);
        }
    }
    pairs
        .into_iter()
        .fold(message.into(), |message: String, (id, name)| {
            message.replace(&id, &name)
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
            f(statement.annotations(), warnings);
            visit(statement.children(), warnings, f);
        }
    }
    for diagram in &document.diagrams {
        annotation_warnings(&diagram.annotations, &mut document.warnings);
        match &diagram.body {
            Body::Graph(body) | Body::Sequence(body) | Body::StateMachine(body) => {
                visit(body, &mut document.warnings, &annotation_warnings)
            }
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

#[cfg(test)]
mod tests_language;
