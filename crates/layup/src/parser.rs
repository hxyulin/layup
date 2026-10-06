//! Generic statement parser with source spans and bounded recovery. Node and
//! edge semantics remain in `model`; tooling can inspect partial syntax.
use crate::{
    Error,
    diagnostic::Span,
    lexer::{Tok, Token, lex_recovering},
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Ident(String),
    Str(String),
    Num(f64),
}
impl Value {
    pub fn as_text(&self) -> String {
        match self {
            Self::Ident(s) | Self::Str(s) => s.clone(),
            Self::Num(n) => n.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Value(Value),
    Attr(String, Value),
    Weights(Vec<f64>),
}
#[derive(Debug, Clone)]
pub struct Item {
    pub head: String,
    pub args: Vec<Arg>,
    pub body: Option<Vec<Stmt>>,
    pub line: usize,
    pub span: Span,
    pub head_span: Span,
    pub arg_spans: Vec<Span>,
}
#[derive(Debug, Clone)]
pub struct EdgeStmt {
    pub from: String,
    pub to: String,
    pub kind: Option<String>,
    pub left: bool,
    pub right: bool,
    pub args: Vec<Arg>,
    pub line: usize,
    pub span: Span,
    pub from_span: Span,
    pub to_span: Span,
    pub arrow_span: Span,
    pub arg_spans: Vec<Span>,
}
#[derive(Debug, Clone)]
pub enum Stmt {
    Item(Item),
    Edge(EdgeStmt),
}
impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Self::Item(i) => i.span,
            Self::Edge(e) => e.span,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Parsed {
    pub statements: Vec<Stmt>,
    pub errors: Vec<Error>,
}

pub(crate) fn parse(src: &str) -> Result<Vec<Stmt>, Error> {
    let parsed = parse_recovering(src);
    match parsed.errors.into_iter().next() {
        Some(e) => Err(e),
        None => Ok(parsed.statements),
    }
}

pub(crate) fn parse_recovering(src: &str) -> Parsed {
    let lexed = lex_recovering(src);
    let mut p = Parser {
        toks: lexed
            .tokens
            .into_iter()
            .filter(|t| !matches!(t.tok, Tok::Comment(_)))
            .collect(),
        pos: 0,
        errors: lexed.errors,
        eof: lexed.eof,
    };
    let statements = p.block_body(None, 0);
    p.errors
        .sort_by_key(|e| e.span.as_ref().map_or(usize::MAX, |s| s.start));
    Parsed {
        statements,
        errors: p.errors,
    }
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    errors: Vec<Error>,
    eof: Span,
}
const MAX_DEPTH: usize = 128;
impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.tok)
    }
    fn span(&self) -> Span {
        self.toks.get(self.pos).map_or(self.eof, |t| t.span)
    }
    fn previous(&self) -> Span {
        self.toks
            .get(self.pos.saturating_sub(1))
            .map_or(self.span(), |t| t.span)
    }
    fn bump(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn error(&self, code: &'static str, msg: impl Into<String>) -> Error {
        Error::located(self.span(), code, format!("syntax: {}", msg.into()))
    }
    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(Tok::Newline)) {
            self.pos += 1;
        }
    }
    fn block_body(&mut self, open: Option<Span>, depth: usize) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek() {
                None => {
                    if let Some(span) = open {
                        self.errors.push(
                            Error::located(span, "parse/unclosed-block", "syntax: unclosed `{`")
                                .with_help("add a matching `}` to close this block"),
                        );
                    }
                    return stmts;
                }
                Some(Tok::RBrace) if open.is_some() => {
                    self.pos += 1;
                    return stmts;
                }
                _ => {
                    let start = self.pos;
                    match self.statement(depth) {
                        Ok(s) => stmts.push(s),
                        Err(e) => {
                            if e.code != "parse/invalid-token" {
                                self.errors.push(e);
                            }
                            self.synchronize(start, open.is_some());
                        }
                    }
                }
            }
        }
    }
    /// Stop at the next sibling statement, respecting braces in the rejected
    /// statement. Do not consume the enclosing block's closing delimiter.
    fn synchronize(&mut self, start: usize, nested: bool) {
        let mut depth = 0usize;
        for t in &self.toks[start..self.pos] {
            match t.tok {
                Tok::LBrace => depth += 1,
                Tok::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        while let Some(t) = self.peek() {
            match t {
                Tok::Newline if depth == 0 => {
                    self.pos += 1;
                    return;
                }
                Tok::RBrace if depth == 0 => {
                    if !nested {
                        self.pos += 1;
                    }
                    return;
                }
                Tok::LBrace => depth += 1,
                Tok::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos += 1;
                        return;
                    }
                }
                _ => {}
            }
            self.pos += 1;
        }
    }
    fn statement(&mut self, depth: usize) -> Result<Stmt, Error> {
        let start = self.span();
        let head = match self.peek() {
            Some(Tok::Ident(s)) => s.clone(),
            Some(Tok::Invalid) => return Err(self.error("parse/invalid-token", "invalid token")),
            Some(t) => {
                return Err(self.error(
                    "parse/statement",
                    format!("expected a statement, found {}", describe(t)),
                ));
            }
            None => unreachable!(),
        };
        self.bump();
        if matches!(self.peek(), Some(Tok::Arrow { .. })) {
            return self.edge(head, start);
        }
        let (args, arg_spans) = self.args()?;
        let body = if matches!(self.peek(), Some(Tok::LBrace)) {
            let open = self.span();
            self.bump();
            if depth >= MAX_DEPTH {
                return Err(Error::located(
                    open,
                    "parse/depth",
                    format!("syntax: blocks may nest at most {MAX_DEPTH} levels"),
                )
                .with_help("split deeply nested content into sibling groups"));
            }
            Some(self.block_body(Some(open), depth + 1))
        } else {
            None
        };
        let end = self.previous();
        self.end_statement()?;
        Ok(Stmt::Item(Item {
            head,
            args,
            body,
            line: start.line,
            span: start.join(end),
            head_span: start,
            arg_spans,
        }))
    }
    fn edge(&mut self, from: String, start: Span) -> Result<Stmt, Error> {
        let arrow = self.bump().unwrap();
        let Tok::Arrow { kind, left, right } = arrow.tok else {
            unreachable!()
        };
        let to_span = self.span();
        let to = match self.peek() {
            Some(Tok::Ident(s)) => s.clone(),
            Some(Tok::Invalid) => return Err(self.error("parse/invalid-token", "invalid token")),
            other => {
                return Err(self
                    .error(
                        "parse/edge-target",
                        format!(
                            "expected a node id after the arrow, found {}",
                            describe_opt(other)
                        ),
                    )
                    .with_help("write both endpoints, for example `source -> target`"));
            }
        };
        self.bump();
        let (args, arg_spans) = self.args()?;
        let end = self.previous();
        self.end_statement()?;
        Ok(Stmt::Edge(EdgeStmt {
            from,
            to,
            kind,
            left,
            right,
            args,
            line: start.line,
            span: start.join(end),
            from_span: start,
            to_span,
            arrow_span: arrow.span,
            arg_spans,
        }))
    }
    fn end_statement(&mut self) -> Result<(), Error> {
        match self.peek() {
            Some(Tok::Newline) => {
                self.pos += 1;
                Ok(())
            }
            None | Some(Tok::RBrace) => Ok(()),
            Some(other) => Err(self
                .error(
                    "parse/statement-end",
                    format!("unexpected {} at end of statement", describe(other)),
                )
                .with_help("separate statements with a newline or `;`")),
        }
    }
    fn args(&mut self) -> Result<(Vec<Arg>, Vec<Span>), Error> {
        let mut args = Vec::new();
        let mut spans = Vec::new();
        let mut attrs = BTreeMap::new();
        loop {
            let start = self.span();
            let arg = match self.peek() {
                Some(Tok::Ident(name)) => {
                    let name = name.clone();
                    self.bump();
                    if matches!(self.peek(), Some(Tok::Eq)) {
                        self.bump();
                        let value = self.value()?;
                        if let Some(first) = attrs.insert(name.clone(), start) {
                            return Err(Error::located(
                                start,
                                "parse/duplicate-attribute",
                                format!("syntax: attribute `{name}` is specified more than once"),
                            )
                            .with_related(first, "first attribute is here")
                            .with_help("keep one value for this attribute"));
                        }
                        Arg::Attr(name, value)
                    } else {
                        Arg::Value(Value::Ident(name))
                    }
                }
                Some(Tok::Str(s)) => {
                    let s = s.clone();
                    self.bump();
                    Arg::Value(Value::Str(s))
                }
                Some(Tok::Num(n)) => {
                    let n = *n;
                    self.bump();
                    if matches!(self.peek(), Some(Tok::Colon)) {
                        let mut weights = vec![n];
                        while matches!(self.peek(), Some(Tok::Colon)) {
                            self.bump();
                            match self.peek() {
                                Some(Tok::Num(n)) => {
                                    weights.push(*n);
                                    self.bump();
                                }
                                Some(Tok::Invalid) => {
                                    return Err(self.error("parse/invalid-token", "invalid token"));
                                }
                                _ => {
                                    return Err(self
                                        .error("parse/weight", "expected a number after `:`")
                                        .with_help("row weights look like `row 1:2:1 { ... }`"));
                                }
                            }
                        }
                        Arg::Weights(weights)
                    } else {
                        Arg::Value(Value::Num(n))
                    }
                }
                Some(Tok::Invalid) => {
                    return Err(self.error("parse/invalid-token", "invalid token"));
                }
                _ => return Ok((args, spans)),
            };
            args.push(arg);
            spans.push(start.join(self.previous()));
        }
    }
    fn value(&mut self) -> Result<Value, Error> {
        let value = match self.peek() {
            Some(Tok::Ident(s)) => Value::Ident(s.clone()),
            Some(Tok::Str(s)) => Value::Str(s.clone()),
            Some(Tok::Num(n)) => Value::Num(*n),
            Some(Tok::Invalid) => return Err(self.error("parse/invalid-token", "invalid token")),
            other => {
                return Err(self
                    .error(
                        "parse/attribute-value",
                        format!("expected a value after `=`, found {}", describe_opt(other)),
                    )
                    .with_help("provide an identifier, a quoted string or a number after `=`"));
            }
        };
        self.bump();
        Ok(value)
    }
}
fn describe(t: &Tok) -> String {
    match t {
        Tok::Ident(s) => format!("`{s}`"),
        Tok::Str(s) => format!("\"{s}\""),
        Tok::Num(n) => format!("`{n}`"),
        Tok::Arrow { .. } => "an arrow".into(),
        Tok::LBrace => "`{`".into(),
        Tok::RBrace => "`}`".into(),
        Tok::Eq => "`=`".into(),
        Tok::Colon => "`:`".into(),
        Tok::Newline => "end of line".into(),
        Tok::Comment(_) => "a comment".into(),
        Tok::Invalid => "an invalid token".into(),
    }
}
fn describe_opt(t: Option<&Tok>) -> String {
    t.map(describe).unwrap_or_else(|| "end of input".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_nested_items_and_edges() {
        let stmts=parse("diagram \"T\" width=900 {\n row 3:7 {\n card a blue { sub \"x\" }\n card b\n }\n a -impl-> b \"impl\" via=right\n}").unwrap();
        let Stmt::Item(d) = &stmts[0] else { panic!() };
        assert_eq!(d.head, "diagram");
        let body = d.body.as_ref().unwrap();
        let Stmt::Item(row) = &body[0] else { panic!() };
        assert_eq!(row.args[0], Arg::Weights(vec![3.0, 7.0]));
        assert_eq!(row.body.as_ref().unwrap().len(), 2);
        let Stmt::Edge(e) = &body[1] else { panic!() };
        assert_eq!(e.kind.as_deref(), Some("impl"));
        assert_eq!(e.args.len(), 2);
    }
}
