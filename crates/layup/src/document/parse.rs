use std::collections::BTreeMap;

use super::{
    Annotation, Attribute, Attributes, Body, Diagram, Document, Edge, GraphStatement, Kind, Node,
    Reference, Value, error,
    lex::{self, Located, Token},
};
use crate::{Error, diagnostic::Span};

pub(super) fn parse(source: &str) -> Result<Document, Error> {
    let mut p = Parser {
        tokens: lex::lex(source)?
            .into_iter()
            .filter(|t| !matches!(t.token, Token::Comment | Token::BlockComment))
            .collect(),
        position: 0,
        depth: 0,
    };
    p.lines();
    p.word("layup")?;
    let revision = p.take();
    if revision.token != Token::Number(1.0) {
        return Err(Error::located(
            revision.span,
            "document/version",
            "supported source revision is `layup 1`",
        ));
    }
    p.end()?;
    let mut diagrams = Vec::new();
    let mut ids = BTreeMap::new();
    while p.peek() != &Token::Eof {
        let annotations = p.annotations()?;
        let start = p.word("diagram")?;
        let (id, id_span) = p.name()?;
        if let Some(previous) = ids.insert(id.clone(), id_span) {
            return Err(error(id_span, format!("duplicate diagram `{id}`"))
                .with_related(previous, "first declaration"));
        }
        let title = p.label().unwrap_or_else(|| id.clone());
        let attributes = p.attributes()?;
        let kind = attributes
            .get("kind")
            .ok_or_else(|| error(start, "diagram requires `kind=graph`"))?;
        if kind.value != Value::Word("graph".into()) && kind.value != Value::String("graph".into())
        {
            return Err(Error::located(
                kind.span,
                "document/unsupported-kind",
                "this revision-one checkpoint supports `kind=graph`; other diagram grammars are not implemented yet",
            ));
        }
        let (body, end) = p.block()?;
        diagrams.push(Diagram {
            id,
            title,
            attributes,
            annotations,
            body: Body::Graph(body),
            span: start.join(end),
        });
        p.end()?;
    }
    if diagrams.is_empty() {
        return Err(error(p.span(), "document requires at least one diagram"));
    }
    Ok(Document {
        version: 1,
        diagrams,
    })
}

struct Parser {
    tokens: Vec<Located>,
    position: usize,
    depth: usize,
}
impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.position].token
    }
    fn span(&self) -> Span {
        self.tokens[self.position].span
    }
    fn take(&mut self) -> Located {
        let t = self.tokens[self.position].clone();
        if t.token != Token::Eof {
            self.position += 1;
        }
        t
    }
    fn expect(&mut self, token: Token) -> Result<Span, Error> {
        if self.peek() != &token {
            return Err(error(self.span(), format!("expected {token:?}")));
        }
        Ok(self.take().span)
    }
    fn word(&mut self, word: &str) -> Result<Span, Error> {
        self.expect(Token::Word(word.into()))
    }
    fn lines(&mut self) {
        while self.peek() == &Token::Newline {
            self.take();
        }
    }
    fn end(&mut self) -> Result<(), Error> {
        if !matches!(self.peek(), Token::Newline | Token::Close | Token::Eof) {
            return Err(error(self.span(), "expected a statement boundary"));
        }
        self.lines();
        Ok(())
    }
    fn enter(&mut self) -> Result<(), Error> {
        if self.depth >= 128 {
            return Err(error(self.span(), "maximum syntax nesting is 128"));
        }
        self.depth += 1;
        Ok(())
    }
    fn name(&mut self) -> Result<(String, Span), Error> {
        let t = self.take();
        match t.token {
            Token::Word(s) if !["true", "false", "null"].contains(&s.as_str()) => Ok((s, t.span)),
            Token::String(s) if !s.is_empty() => Ok((s, t.span)),
            _ => Err(error(
                t.span,
                "expected a nonempty identifier; quote literal true, false or null names",
            )),
        }
    }
    fn label(&mut self) -> Option<String> {
        if let Token::String(s) = self.peek() {
            let s = s.clone();
            self.take();
            Some(s)
        } else {
            None
        }
    }
    fn reference(&mut self) -> Result<Reference, Error> {
        let root = self.peek() == &Token::Root;
        if root {
            self.take();
        }
        let mut segments = vec![self.name()?.0];
        while self.peek() == &Token::Dot {
            self.take();
            segments.push(self.name()?.0);
        }
        Ok(Reference { root, segments })
    }
    fn value(&mut self) -> Result<Value, Error> {
        match self.peek().clone() {
            Token::ListOpen => {
                self.enter()?;
                self.take();
                self.lines();
                let mut values = Vec::new();
                while self.peek() != &Token::ListClose {
                    values.push(self.value()?);
                    self.lines();
                    if self.peek() == &Token::ListClose {
                        break;
                    }
                    self.expect(Token::Comma)?;
                    self.lines();
                }
                self.take();
                self.depth -= 1;
                Ok(Value::List(values))
            }
            Token::Open => {
                self.enter()?;
                self.take();
                self.lines();
                let mut values = BTreeMap::new();
                while self.peek() != &Token::Close {
                    let (key, span) = self.name()?;
                    self.expect(Token::Colon)?;
                    self.lines();
                    if values.insert(key.clone(), self.value()?).is_some() {
                        return Err(error(span, format!("duplicate record key `{key}`")));
                    }
                    self.lines();
                    if self.peek() == &Token::Close {
                        break;
                    }
                    self.expect(Token::Comma)?;
                    self.lines();
                }
                self.take();
                self.depth -= 1;
                Ok(Value::Record(values))
            }
            Token::Word(w) if ["true", "false", "null"].contains(&w.as_str()) => {
                self.take();
                Ok(match w.as_str() {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    _ => Value::Null,
                })
            }
            Token::Root => Ok(Value::Reference(self.reference()?)),
            Token::Word(_) | Token::String(_) => {
                let first = self.take();
                if self.peek() == &Token::Dot {
                    let segment = match first.token {
                        Token::Word(s) | Token::String(s) => s,
                        _ => unreachable!(),
                    };
                    if segment.is_empty() {
                        return Err(error(first.span, "reference segments cannot be empty"));
                    }
                    let mut segments = vec![segment];
                    while self.peek() == &Token::Dot {
                        self.take();
                        segments.push(self.name()?.0);
                    }
                    Ok(Value::Reference(Reference {
                        root: false,
                        segments,
                    }))
                } else {
                    Ok(match first.token {
                        Token::Word(s) => Value::Word(s),
                        Token::String(s) => Value::String(s),
                        _ => unreachable!(),
                    })
                }
            }
            Token::Number(n) => {
                self.take();
                Ok(Value::Number(n))
            }
            _ => Err(error(self.span(), "expected a value")),
        }
    }
    fn attribute(&mut self, result: &mut Attributes) -> Result<(), Error> {
        let (key, span) = self.name()?;
        self.expect(Token::Equal)?;
        let value = self.value()?;
        let end = self.tokens[self.position - 1].span;
        if result
            .insert(
                key.clone(),
                Attribute {
                    value,
                    span: span.join(end),
                },
            )
            .is_some()
        {
            return Err(error(span, format!("duplicate attribute `{key}`")));
        }
        Ok(())
    }
    fn attributes(&mut self) -> Result<Attributes, Error> {
        let mut result = BTreeMap::new();
        while matches!(self.peek(), Token::Word(_) | Token::String(_)) {
            self.attribute(&mut result)?;
        }
        Ok(result)
    }
    fn annotations(&mut self) -> Result<Vec<Annotation>, Error> {
        let mut result = Vec::new();
        while self.peek() == &Token::At {
            let start = self.take().span;
            let (name, _) = self.name()?;
            self.expect(Token::ParenOpen)?;
            self.lines();
            let mut arguments = BTreeMap::new();
            while self.peek() != &Token::ParenClose {
                self.attribute(&mut arguments)?;
                self.lines();
                if self.peek() == &Token::ParenClose {
                    break;
                }
                self.expect(Token::Comma)?;
                self.lines();
            }
            let end = self.take().span;
            result.push(Annotation {
                name,
                arguments,
                span: start.join(end),
            });
            self.end()?;
        }
        Ok(result)
    }
    fn block(&mut self) -> Result<(Vec<GraphStatement>, Span), Error> {
        self.expect(Token::Open)?;
        self.enter()?;
        self.lines();
        let mut body = Vec::new();
        while self.peek() != &Token::Close {
            if self.peek() == &Token::Eof {
                return Err(error(self.span(), "unclosed block"));
            }
            body.push(self.statement()?);
            self.end()?;
        }
        let end = self.take().span;
        self.depth -= 1;
        Ok((body, end))
    }
    fn edge(
        &mut self,
        start: Span,
        id: Option<String>,
        from: Reference,
        annotations: Vec<Annotation>,
    ) -> Result<GraphStatement, Error> {
        let arrow = match self.take().token {
            Token::Arrow(s) => s,
            _ => {
                return Err(error(
                    self.tokens[self.position - 1].span,
                    "expected ->, <-, <-> or --",
                ));
            }
        };
        let to = self.reference()?;
        let label = self.label();
        let attributes = self.attributes()?;
        Ok(GraphStatement::Edge(Edge {
            id,
            from,
            to,
            arrow,
            label,
            attributes,
            annotations,
            span: start.join(self.tokens[self.position - 1].span),
        }))
    }
    fn statement(&mut self) -> Result<GraphStatement, Error> {
        let annotations = self.annotations()?;
        let start = self.span();
        let saved = self.position;
        if let Ok(from) = self.reference()
            && matches!(self.peek(), Token::Arrow(_))
        {
            return self.edge(start, None, from, annotations);
        }
        self.position = saved;
        let (head, _) = self.name()?;
        if head == "edge" {
            let id = self.name()?.0;
            let from = self.reference()?;
            return self.edge(start, Some(id), from, annotations);
        }
        if !annotations.is_empty()
            && !["node", "group", "package", "crate"].contains(&head.as_str())
        {
            return Err(error(
                annotations[0].span,
                "annotations must precede a diagram, node or edge in the same block",
            ));
        }
        match head.as_str() {
            "node" | "group" | "package" | "crate" => {
                let id = self.name()?.0;
                let title = self.label().unwrap_or_else(|| id.clone());
                let attributes = self.attributes()?;
                let body = if self.peek() == &Token::Open {
                    self.block()?.0
                } else {
                    Vec::new()
                };
                Ok(GraphStatement::Node(Node {
                    declaration: head,
                    id,
                    title,
                    attributes,
                    annotations,
                    body,
                    span: start.join(self.tokens[self.position - 1].span),
                }))
            }
            "node-kind" | "edge-kind" => {
                let id = self.name()?.0;
                let attributes = self.attributes()?;
                let kind = Kind {
                    id,
                    attributes,
                    span: start.join(self.tokens[self.position - 1].span),
                };
                Ok(if head == "node-kind" {
                    GraphStatement::NodeKind(kind)
                } else {
                    GraphStatement::EdgeKind(kind)
                })
            }
            "row" => {
                let attributes = self.attributes()?;
                let (body, end) = self.block()?;
                Ok(GraphStatement::Row {
                    attributes,
                    body,
                    span: start.join(end),
                })
            }
            "text" | "code" | "tag" => {
                let text = self
                    .label()
                    .ok_or_else(|| error(self.span(), "content requires one quoted string"))?;
                Ok(GraphStatement::Content {
                    kind: head,
                    text,
                    span: start.join(self.tokens[self.position - 1].span),
                })
            }
            _ => Err(error(
                start,
                format!("unsupported graph statement `{head}` in this checkpoint"),
            )),
        }
    }
}
