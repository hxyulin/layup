use std::collections::BTreeMap;

use super::{
    Annotation, Attribute, Attributes, Body, ConfigurationKind, DefaultCategory, Diagram, Document,
    Edge, FragmentKind, GraphStatement, Kind, LayoutKind, NamedBlock, Node, Reference,
    SelectionKind, Value, error,
    lex::{self, Located, Token},
};
use crate::{Error, diagnostic::Span};

pub(super) fn parse(source: &str) -> Result<Document, Error> {
    let parsed = parse_recovering(source);
    if let Some(error) = parsed.errors.into_iter().next() {
        return Err(error);
    }
    Ok(parsed.document)
}

pub(super) fn parse_recovering(source: &str) -> super::Parsed {
    let mut document = Document {
        version: 1,
        source: source.into(),
        value_spans: Vec::new(),
        diagrams: Vec::new(),
        warnings: Vec::new(),
    };
    let (tokens, errors) = lex::document_tokens_recovering(source);
    let tokens = tokens
        .into_iter()
        .filter(|t| !matches!(t.token, Token::Comment | Token::BlockComment))
        .collect();
    let mut p = Parser {
        tokens,
        position: 0,
        grammar: "graph".into(),
        context: "diagram".into(),
        depth: 0,
        collection_depth: 0,
        value_spans: Vec::new(),
        errors,
        source,
    };
    p.lines();
    if p.peek() == &Token::Word("layup".into()) {
        p.take();
        let revision = p.take();
        if revision.token != Token::Integer(super::Integer::Unsigned(1)) {
            p.errors.push(Error::located(
                revision.span,
                "document/version",
                "supported source revision is the integer `layup 1`",
            ));
        }
        if let Err(error) = p.end() {
            p.errors.push(error);
            p.recover(0, true);
        }
    }
    let mut ids = BTreeMap::new();
    while p.peek() != &Token::Eof {
        let begin = p.position;
        match p.diagram() {
            Ok(diagram) => {
                if let Some(previous) = ids.insert(diagram.id.clone(), diagram.span) {
                    p.errors.push(
                        error(diagram.span, format!("duplicate diagram `{}`", diagram.id))
                            .with_related(previous, "first declaration"),
                    );
                }
                document.diagrams.push(diagram);
                if let Err(error) = p.end() {
                    p.errors.push(error);
                    p.recover(begin, true);
                }
            }
            Err(error) => {
                p.errors.push(error);
                p.recover(begin, true);
            }
        }
        p.depth = 0;
        p.collection_depth = 0;
    }
    if document.diagrams.is_empty() && p.errors.is_empty() {
        p.errors
            .push(error(p.span(), "document requires at least one diagram"));
    }
    p.errors.retain(|e| e.code != "document/invalid-token");
    p.errors
        .sort_by_key(|e| e.span.as_deref().map_or(usize::MAX, |s| s.start));
    p.value_spans.sort_by_key(|v| v.span.start);
    document.value_spans = p.value_spans;
    super::diagnose_extensions(&mut document);
    super::Parsed {
        document,
        errors: p.errors,
    }
}

struct Parser<'a> {
    tokens: Vec<Located>,
    position: usize,
    grammar: String,
    context: String,
    depth: usize,
    errors: Vec<Error>,
    collection_depth: usize,
    value_spans: Vec<super::ValueSpan>,
    source: &'a str,
}
impl Parser<'_> {
    fn diagram(&mut self) -> Result<Diagram, Error> {
        let annotations = self.annotations()?;
        if self.peek() == &Token::Eof
            && let Some(annotation) = annotations.first()
        {
            return Err(error(
                annotation.span,
                "orphan annotation: expected a following diagram in this document",
            ));
        }
        let start = self.word("diagram")?;
        let (id, _) = self.name()?;
        let title = self.label().unwrap_or_else(|| id.clone());
        if title.is_empty() {
            return Err(error(start, "diagram display label cannot be empty"));
        }
        let attributes = self.attributes()?;
        let selector = attributes.get("type").ok_or_else(|| {
            error(
                start,
                "diagram requires `type=graph` or another registered type",
            )
        })?;
        let diagram_type = match &selector.value {
            Value::Choice(s) | Value::String(s) => s.clone(),
            Value::Reference(r) if !r.root => r.segments.join("."),
            _ => {
                return Err(error(
                    selector.value_span,
                    "diagram type must be a registry name",
                ));
            }
        };
        if diagram_type.is_empty() {
            return Err(error(selector.value_span, "diagram type cannot be empty"));
        }
        self.grammar = diagram_type.clone();
        self.context = "diagram".into();
        let (body, end) = if ["graph", "sequence", "state-machine"].contains(&diagram_type.as_str())
        {
            let (body, end) = self.block()?;
            (
                match diagram_type.as_str() {
                    "sequence" => Body::Sequence(body),
                    "state-machine" => Body::StateMachine(body),
                    _ => Body::Graph(body),
                },
                end,
            )
        } else {
            let token = self.take();
            if token.token == Token::Invalid {
                return Err(Error::located(
                    token.span,
                    "document/invalid-token",
                    "invalid body",
                ));
            }
            let Token::Opaque(raw) = token.token else {
                return Err(error(token.span, "expected an opaque diagram block"));
            };
            (
                Body::Opaque {
                    raw,
                    span: token.span,
                },
                token.span,
            )
        };
        Ok(Diagram {
            id,
            title,
            diagram_type,
            type_span: selector.value_span,
            attributes,
            annotations,
            body,
            span: start.join(end),
        })
    }

    /// Re-scan from the failed statement so consumed delimiters cannot hide siblings.
    fn recover(&mut self, begin: usize, document: bool) {
        self.position = begin;
        let mut depth = 0usize;
        while self.peek() != &Token::Eof {
            if self.position > begin
                && depth == 0
                && document
                && self.peek() == &Token::Word("diagram".into())
            {
                return;
            }
            match self.peek() {
                Token::Close if depth == 0 => {
                    if !document {
                        return;
                    }
                }
                Token::Open | Token::ListOpen | Token::ParenOpen => depth += 1,
                Token::Close | Token::ListClose | Token::ParenClose => {
                    depth = depth.saturating_sub(1)
                }
                Token::Newline if depth == 0 => {
                    self.take();
                    self.lines();
                    return;
                }
                _ => {}
            }
            self.take();
        }
    }
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
        if self.peek() == &Token::Invalid {
            return Err(Error::located(
                self.span(),
                "document/invalid-token",
                "invalid token",
            ));
        }
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
        if self.peek() == &Token::Invalid {
            return Err(Error::located(
                self.span(),
                "document/invalid-token",
                "invalid token",
            ));
        }
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
            Token::Invalid => Err(Error::located(
                t.span,
                "document/invalid-token",
                "invalid token",
            )),
            Token::Word(s) if !["true", "false", "null"].contains(&s.as_str()) => Ok((s, t.span)),
            Token::String(s) if !s.is_empty() && !s.chars().any(char::is_control) => {
                Ok((s, t.span))
            }
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
        let start = self.span();
        let value = self.value_inner()?;
        let end = self.tokens[self.position.saturating_sub(1)].span;
        self.value_spans.push(super::ValueSpan {
            span: start.join(end),
            kind: match &value {
                Value::Choice(_) => "choice",
                Value::String(_) => "string",
                Value::Integer(_) => "integer",
                Value::Float(_) => "float",
                Value::Bool(_) => "bool",
                Value::Null => "null",
                Value::List(_) => "list",
                Value::Record(_) => "record",
                Value::Reference(_) => "reference",
            }
            .into(),
        });
        Ok(value)
    }
    fn value_inner(&mut self) -> Result<Value, Error> {
        match self.peek().clone() {
            Token::Invalid => Err(Error::located(
                self.span(),
                "document/invalid-token",
                "invalid token",
            )),
            Token::ListOpen => {
                self.enter()?;
                self.collection_depth += 1;
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
                self.collection_depth -= 1;
                Ok(Value::List(values))
            }
            Token::Open => {
                self.enter()?;
                self.collection_depth += 1;
                self.take();
                self.lines();
                let mut values = BTreeMap::new();
                while self.peek() != &Token::Close {
                    let key_token = self.take();
                    let span = key_token.span;
                    let key = match key_token.token {
                        Token::Word(key) | Token::String(key) => key,
                        _ => return Err(error(span, "record key must be an identifier or string")),
                    };
                    self.lines();
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
                self.collection_depth -= 1;
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
                        Token::Word(s) => Value::Choice(s),
                        Token::String(s) => Value::String(s),
                        _ => unreachable!(),
                    })
                }
            }
            Token::Integer(n) => {
                self.take();
                Ok(Value::Integer(n))
            }
            Token::Float(n) => {
                self.take();
                Ok(Value::Float(n))
            }
            _ => Err(error(self.span(), "expected a value")),
        }
    }
    fn attribute(&mut self, result: &mut Attributes) -> Result<(), Error> {
        let (key, span) = self.name()?;
        if self.collection_depth > 0 {
            self.lines();
        }
        if self.collection_depth == 0
            && [
                "kind",
                "tone",
                "font",
                "align",
                "textdir",
                "stroke",
                "direction",
                "same-layer",
                "via",
                "fill",
                "gutter",
                "below",
            ]
            .contains(&key.as_str())
        {
            return Err(error(
                span,
                format!("obsolete property `{key}`; use the documented explicit property name"),
            ));
        }
        self.expect(Token::Equal)?;
        if self.collection_depth > 0 {
            self.lines();
        }
        let value_start = self.span();
        let value = self.value()?;
        let end = self.tokens[self.position - 1].span;
        if let Some(previous) = result.insert(
            key.clone(),
            Attribute {
                value,
                span: span.join(end),
                value_span: value_start.join(end),
                name_span: span,
            },
        ) {
            return Err(error(span, format!("duplicate attribute `{key}`"))
                .with_related(previous.name_span, "first assignment"));
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
            let mut name = match self.take().token {
                Token::Word(name) => name,
                _ => return Err(error(start, "annotation name must be an identifier")),
            };
            while self.peek() == &Token::Dot {
                self.take();
                name.push('.');
                let segment = self.take();
                let Token::Word(word) = segment.token else {
                    return Err(error(
                        segment.span,
                        "annotation namespace segments must be identifiers",
                    ));
                };
                name.push_str(&word);
            }
            self.expect(Token::ParenOpen)?;
            self.collection_depth += 1;
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
            self.collection_depth -= 1;
            if end.end - start.start > 1_048_576 {
                return Err(error(start, "maximum annotation size is 1 MiB"));
            }
            result.push(Annotation {
                name,
                raw: self.source[start.start..end.end].into(),
                arguments,
                span: start.join(end),
            });
            self.lines();
        }
        Ok(result)
    }
    fn context_block(&mut self, context: &str) -> Result<(Vec<GraphStatement>, Span), Error> {
        let previous = std::mem::replace(&mut self.context, context.into());
        let result = self.block();
        self.context = previous;
        result
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
            let begin = self.position;
            let depth = self.depth;
            let collection_depth = self.collection_depth;
            match self.statement() {
                Ok(statement) => {
                    body.push(statement);
                    if let Err(error) = self.end() {
                        self.errors.push(error);
                        self.recover(begin, false);
                    }
                }
                Err(error) => {
                    self.errors.push(error);
                    self.recover(begin, false);
                }
            }
            self.depth = depth;
            self.collection_depth = collection_depth;
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
        from_span: Span,
        annotations: Vec<Annotation>,
    ) -> Result<GraphStatement, Error> {
        let arrow_span = self.span();
        let arrow = match self.take().token {
            Token::Arrow(s) => s,
            _ => {
                return Err(error(
                    self.tokens[self.position - 1].span,
                    "expected ->, <-, <-> or --",
                ));
            }
        };
        let to_start = self.span();
        let to = self.reference()?;
        let to_span = to_start.join(self.tokens[self.position - 1].span);
        let label = self.label();
        let attributes = self.attributes()?;
        Ok(GraphStatement::Edge(Edge {
            id,
            from_span,
            to_span,
            arrow_span,
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
        if self.grammar != "sequence"
            && !["view", "step"].contains(&self.context.as_str())
            && let Ok(from) = self.reference()
            && matches!(self.peek(), Token::Arrow(_))
        {
            return self.edge(
                start,
                None,
                from,
                start.join(self.tokens[self.position - 1].span),
                annotations,
            );
        }
        self.position = saved;
        if self.peek() == &Token::Close || self.peek() == &Token::Eof {
            return Err(error(
                annotations.first().map_or(start, |a| a.span),
                "orphan annotation: expected a following construct in the same block",
            )
            .with_help("add a target before the closing brace, or remove the annotation"));
        }
        let (head, _) = self.name()?;
        let valid = match self.context.as_str() {
            "step" => ["show", "highlight", "speaker-note"].contains(&head.as_str()),
            "view" => ["include", "step", "slide"].contains(&head.as_str()),
            "alternatives" => head == "branch",
            _ => match self.grammar.as_str() {
                "sequence" => [
                    "participant",
                    "actor",
                    "message",
                    "note",
                    "loop",
                    "optional",
                    "alternatives",
                    "node-style",
                    "edge-style",
                    "defaults",
                    "view",
                    "step",
                    "slide",
                ]
                .contains(&head.as_str()),
                "state-machine" => [
                    "state",
                    "initial",
                    "final",
                    "choice",
                    "transition",
                    "node-style",
                    "edge-style",
                    "defaults",
                    "view",
                    "step",
                    "slide",
                    "legend",
                    "text",
                    "code",
                    "tag",
                    "entry",
                    "exit",
                    "port",
                    "row",
                    "section",
                    "band",
                    "divider",
                    "gap",
                ]
                .contains(&head.as_str()),
                _ => [
                    "node",
                    "group",
                    "package",
                    "crate",
                    "edge",
                    "node-style",
                    "edge-style",
                    "defaults",
                    "row",
                    "section",
                    "band",
                    "divider",
                    "gap",
                    "text",
                    "code",
                    "tag",
                    "port",
                    "view",
                    "step",
                    "slide",
                    "legend",
                ]
                .contains(&head.as_str()),
            },
        };
        let diagram_only = [
            "view",
            "defaults",
            "slide",
            "legend",
            "node-style",
            "edge-style",
            "participant",
            "actor",
        ]
        .contains(&head.as_str());
        if diagram_only && self.context != "diagram" && !(head == "slide" && self.context == "view")
        {
            return Err(error(
                start,
                format!("`{head}` belongs directly to the diagram"),
            ));
        }
        if head == "branch" && self.context != "alternatives" {
            return Err(error(start, "branches belong directly inside alternatives"));
        }
        if head == "step" && self.context != "diagram" && self.context != "view" {
            return Err(error(start, "steps belong directly to a diagram or view"));
        }
        if !valid {
            return Err(error(
                start,
                format!(
                    "`{head}` is not a statement in {} {}",
                    self.grammar, self.context
                ),
            ));
        }
        if ["edge", "message", "transition"].contains(&head.as_str()) {
            let id = self.name()?.0;
            let from_start = self.span();
            let from = self.reference()?;
            return self.edge(
                start,
                Some(id),
                from,
                from_start.join(self.tokens[self.position - 1].span),
                annotations,
            );
        }
        match head.as_str() {
            "node" | "group" | "package" | "crate" | "participant" | "actor" | "state"
            | "initial" | "final" | "choice" => {
                let id = self.name()?.0;
                let label_span = matches!(self.peek(), Token::String(_)).then(|| self.span());
                let title = self.label().unwrap_or_else(|| id.clone());
                let attributes = self.attributes()?;
                let body = if self.peek() == &Token::Open {
                    self.context_block(&head)?.0
                } else {
                    Vec::new()
                };
                Ok(GraphStatement::Node(Node {
                    declaration: head,
                    label_span,
                    id,
                    title,
                    attributes,
                    annotations,
                    body,
                    span: start.join(self.tokens[self.position - 1].span),
                }))
            }
            "node-style" | "edge-style" => {
                let id = self.name()?.0;
                let attributes = self.attributes()?;
                let kind = Kind {
                    annotations,
                    id,
                    attributes,
                    span: start.join(self.tokens[self.position - 1].span),
                };
                Ok(if head == "node-style" {
                    GraphStatement::NodeStyle(kind)
                } else {
                    GraphStatement::EdgeStyle(kind)
                })
            }
            "row" => {
                let attributes = self.attributes()?;
                let (body, end) = self.block()?;
                Ok(GraphStatement::Row {
                    annotations,
                    attributes,
                    body,
                    span: start.join(end),
                })
            }
            "text" | "code" | "tag" | "speaker-note" | "entry" | "exit" => {
                let text = self
                    .label()
                    .ok_or_else(|| error(self.span(), "content requires one quoted string"))?;
                Ok(GraphStatement::Content {
                    annotations,
                    kind: head,
                    text,
                    span: start.join(self.tokens[self.position - 1].span),
                })
            }
            "view" | "step" => {
                let (id, id_span) = self.name()?;
                let title = self.label().unwrap_or_else(|| id.clone());
                let attributes = self.attributes()?;
                let (body, end) = self.context_block(&head)?;
                let block = NamedBlock {
                    id,
                    id_span,
                    title,
                    attributes,
                    annotations,
                    body,
                    span: start.join(end),
                };
                Ok(if head == "view" {
                    GraphStatement::View(block)
                } else {
                    GraphStatement::Step(block)
                })
            }
            "include" | "show" | "highlight" => {
                let mut objects = Vec::new();
                let mut connections = Vec::new();
                let mut object_spans = Vec::new();
                let mut connection_spans = Vec::new();
                if head == "include" {
                    while !matches!(self.peek(), Token::Newline | Token::Close | Token::Eof) {
                        let start = self.span();
                        objects.push(self.reference()?);
                        object_spans.push(start.join(self.tokens[self.position - 1].span));
                    }
                } else {
                    let attributes = self.attributes()?;
                    for (key, attr) in attributes {
                        let Value::List(values) = attr.value else {
                            return Err(error(attr.span, "selection requires a list"));
                        };
                        if values.iter().any(|v| {
                            !matches!(v, Value::Choice(_) | Value::String(_) | Value::Reference(_))
                        }) {
                            return Err(error(attr.value_span, "selection requires identities"));
                        }
                        let spans = self
                            .value_spans
                            .iter()
                            .filter(|v| {
                                v.span.start >= attr.value_span.start
                                    && v.span.end <= attr.value_span.end
                                    && ["choice", "string", "reference"].contains(&v.kind.as_str())
                            })
                            .map(|v| v.span)
                            .collect::<Vec<_>>();
                        for (value, value_span) in values.into_iter().zip(spans) {
                            let reference = match value {
                                Value::Reference(r) => r,
                                Value::Choice(s) | Value::String(s) => Reference {
                                    root: false,
                                    segments: vec![s],
                                },
                                _ => return Err(error(attr.span, "selection requires identities")),
                            };
                            match key.as_str() {
                                "objects" => {
                                    objects.push(reference);
                                    object_spans.push(value_span);
                                }
                                "connections"
                                    if !reference.root && reference.segments.len() == 1 =>
                                {
                                    connections.push(reference.segments[0].clone());
                                    connection_spans.push(value_span);
                                }
                                _ => {
                                    return Err(error(
                                        attr.span,
                                        "selection accepts objects and diagram-wide connections",
                                    ));
                                }
                            }
                        }
                    }
                }
                if objects.is_empty() && connections.is_empty() {
                    return Err(error(start, "selection requires at least one target"));
                }
                Ok(GraphStatement::Selection {
                    kind: match head.as_str() {
                        "include" => SelectionKind::Include,
                        "show" => SelectionKind::Show,
                        _ => SelectionKind::Highlight,
                    },
                    objects,
                    object_spans,
                    connections,
                    connection_spans,
                    annotations,
                    span: start.join(self.tokens[self.position - 1].span),
                })
            }
            "loop" | "optional" | "alternatives" | "branch" => {
                let title = self
                    .label()
                    .ok_or_else(|| error(start, "fragment requires a quoted label"))?;
                let (body, end) = self.context_block(&head)?;
                Ok(GraphStatement::Fragment {
                    kind: match head.as_str() {
                        "loop" => FragmentKind::Loop,
                        "optional" => FragmentKind::Optional,
                        "alternatives" => FragmentKind::Alternatives,
                        _ => FragmentKind::Branch,
                    },
                    title,
                    body,
                    annotations,
                    span: start.join(end),
                })
            }
            "note" => {
                let text = self
                    .label()
                    .ok_or_else(|| error(start, "note requires quoted text"))?;
                let attributes = self.attributes()?;
                Ok(GraphStatement::SequenceNote {
                    text,
                    attributes,
                    annotations,
                    span: start.join(self.tokens[self.position - 1].span),
                })
            }
            "slide" | "legend" => {
                let attributes = self.attributes()?;
                Ok(GraphStatement::Configuration {
                    kind: if head == "slide" {
                        ConfigurationKind::Slide
                    } else {
                        ConfigurationKind::Legend
                    },
                    attributes,
                    annotations,
                    span: start.join(self.tokens[self.position - 1].span),
                })
            }
            "defaults" => {
                let (category, span) = self.name()?;
                let category = match category.as_str() {
                    "node" => DefaultCategory::Node,
                    "edge" => DefaultCategory::Edge,
                    "participant" => DefaultCategory::Participant,
                    "message" => DefaultCategory::Message,
                    "state" => DefaultCategory::State,
                    "transition" => DefaultCategory::Transition,
                    _ => return Err(error(span, "unknown defaults category")),
                };
                let attributes = self.attributes()?;
                Ok(GraphStatement::Defaults {
                    category,
                    attributes,
                    annotations,
                    span: start.join(self.tokens[self.position - 1].span),
                })
            }
            "section" | "band" | "divider" | "gap" => {
                let title = self.label();
                let attributes = self.attributes()?;
                let (body, end) = if head == "section" || head == "band" {
                    self.context_block(&head)?
                } else {
                    (Vec::new(), self.tokens[self.position - 1].span)
                };
                Ok(GraphStatement::Layout {
                    kind: match head.as_str() {
                        "section" => LayoutKind::Section,
                        "band" => LayoutKind::Band,
                        "divider" => LayoutKind::Divider,
                        _ => LayoutKind::Gap,
                    },
                    title,
                    attributes,
                    body,
                    annotations,
                    span: start.join(end),
                })
            }
            "port" => {
                let id = self.name()?.0;
                let attributes = self.attributes()?;
                Ok(GraphStatement::Port {
                    id,
                    attributes,
                    annotations,
                    span: start.join(self.tokens[self.position - 1].span),
                })
            }
            _ => Err(error(
                start,
                format!(
                    "unsupported statement `{head}` in {} {}",
                    self.grammar, self.context
                ),
            )),
        }
    }
}
