//! Revision-one document syntax. Shared values and references are parsed here;
//! diagram bodies dispatch to a grammar rather than acquiring dynamic keywords.
mod compile;
mod format;
mod lex;
mod parse;

use std::collections::BTreeMap;

use crate::{CompileOptions, Compiled, Error, diagnostic::Span, text::Fonts};

pub use compile::{EntityInfo, Info};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Word(String),
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    List(Vec<Value>),
    Record(BTreeMap<String, Value>),
    Reference(Reference),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub root: bool,
    pub segments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub value: Value,
    pub span: Span,
}
pub type Attributes = BTreeMap<String, Attribute>;

#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub name: String,
    pub arguments: Attributes,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub version: u32,
    pub diagrams: Vec<Diagram>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagram {
    pub id: String,
    pub title: String,
    pub attributes: Attributes,
    pub annotations: Vec<Annotation>,
    pub body: Body,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Graph(Vec<GraphStatement>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphStatement {
    Node(Node),
    Edge(Edge),
    NodeKind(Kind),
    EdgeKind(Kind),
    Row {
        attributes: Attributes,
        body: Vec<GraphStatement>,
        span: Span,
    },
    Content {
        kind: String,
        text: String,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub declaration: String,
    pub id: String,
    pub title: String,
    pub attributes: Attributes,
    pub annotations: Vec<Annotation>,
    pub body: Vec<GraphStatement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Kind {
    pub id: String,
    pub attributes: Attributes,
    pub span: Span,
}

/// The first non-comment token selects the dialect, including malformed or
/// unsupported revision headers. Errors must never fall back to legacy parsing.
pub fn is_versioned(source: &str) -> bool {
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
            return rest.strip_prefix("layup").is_some_and(|tail| {
                tail.is_empty()
                    || !tail.starts_with(|c: char| unicode_ident::is_xid_continue(c) || c == '-')
            });
        }
    }
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
            && let Ok(tokens) = lex::lex(source)
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
