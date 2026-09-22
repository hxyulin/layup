//! Parses tokens into a generic item tree. The parser knows nothing about
//! node kinds; every statement is either an item (`head args... { body }`)
//! or an edge (`a -kind-> b "label" attrs...`). Interpretation happens in
//! `model`.

use crate::Error;
use crate::lexer::{Tok, Token, lex};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Ident(String),
    Str(String),
    Num(f64),
}

impl Value {
    pub fn as_text(&self) -> String {
        match self {
            Value::Ident(s) | Value::Str(s) => s.clone(),
            Value::Num(n) => format!("{n}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Value(Value),
    Attr(String, Value),
    /// A `3:7` style weight list.
    Weights(Vec<f64>),
}

#[derive(Debug, Clone)]
pub struct Item {
    pub head: String,
    pub args: Vec<Arg>,
    pub body: Option<Vec<Stmt>>,
    pub line: usize,
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
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Item(Item),
    Edge(EdgeStmt),
}

pub fn parse(src: &str) -> Result<Vec<Stmt>, Error> {
    let tokens = lex(src)?;
    let mut p = Parser {
        toks: tokens,
        pos: 0,
    };
    let stmts = p.block_body(None)?;
    Ok(stmts)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks
            .get(self.pos)
            .map(|t| t.tok.clone())
            .as_ref()
            .map(|_| &self.toks[self.pos].tok)
    }

    fn line(&self) -> usize {
        self.toks
            .get(self.pos)
            .map(|t| t.line)
            .unwrap_or_else(|| self.toks.last().map(|t| t.line).unwrap_or(1))
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).map(|t| t.tok.clone());
        self.pos += 1;
        t
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(Tok::Newline)) {
            self.pos += 1;
        }
    }

    /// Parses statements until `}` (when `open_line` is set) or end of input.
    fn block_body(&mut self, open_line: Option<usize>) -> Result<Vec<Stmt>, Error> {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek() {
                None => {
                    if let Some(l) = open_line {
                        return Err(Error::syntax(l, "unclosed `{`"));
                    }
                    return Ok(stmts);
                }
                Some(Tok::RBrace) => {
                    if open_line.is_none() {
                        return Err(Error::syntax(self.line(), "unexpected `}`"));
                    }
                    self.pos += 1;
                    return Ok(stmts);
                }
                Some(Tok::Ident(_)) => stmts.push(self.statement()?),
                Some(other) => {
                    return Err(Error::syntax(
                        self.line(),
                        format!("expected a statement, found {}", describe(other)),
                    ));
                }
            }
        }
    }

    fn statement(&mut self) -> Result<Stmt, Error> {
        let line = self.line();
        let Some(Tok::Ident(head)) = self.bump() else {
            unreachable!()
        };
        if let Some(Tok::Arrow { .. }) = self.peek() {
            return self.edge(head, line);
        }
        let args = self.args()?;
        let body = if matches!(self.peek(), Some(Tok::LBrace)) {
            self.pos += 1;
            Some(self.block_body(Some(line))?)
        } else {
            None
        };
        self.end_statement()?;
        Ok(Stmt::Item(Item {
            head,
            args,
            body,
            line,
        }))
    }

    fn edge(&mut self, from: String, line: usize) -> Result<Stmt, Error> {
        let Some(Tok::Arrow { kind, left, right }) = self.bump() else {
            unreachable!()
        };
        let to = match self.bump() {
            Some(Tok::Ident(s)) => s,
            other => {
                return Err(Error::syntax(
                    line,
                    format!(
                        "expected a node id after the arrow, found {}",
                        describe_opt(other.as_ref())
                    ),
                ));
            }
        };
        let args = self.args()?;
        self.end_statement()?;
        Ok(Stmt::Edge(EdgeStmt {
            from,
            to,
            kind,
            left,
            right,
            args,
            line,
        }))
    }

    fn end_statement(&mut self) -> Result<(), Error> {
        match self.peek() {
            Some(Tok::Newline) => {
                self.pos += 1;
                Ok(())
            }
            None | Some(Tok::RBrace) => Ok(()),
            Some(other) => Err(Error::syntax(
                self.line(),
                format!("unexpected {} at end of statement", describe(other)),
            )),
        }
    }

    fn args(&mut self) -> Result<Vec<Arg>, Error> {
        let mut args = Vec::new();
        loop {
            match self.peek() {
                Some(Tok::Ident(_)) => {
                    let Some(Tok::Ident(name)) = self.bump() else {
                        unreachable!()
                    };
                    if matches!(self.peek(), Some(Tok::Eq)) {
                        self.pos += 1;
                        let v = self.value()?;
                        args.push(Arg::Attr(name, v));
                    } else {
                        args.push(Arg::Value(Value::Ident(name)));
                    }
                }
                Some(Tok::Str(_)) => {
                    let Some(Tok::Str(s)) = self.bump() else {
                        unreachable!()
                    };
                    args.push(Arg::Value(Value::Str(s)));
                }
                Some(Tok::Num(_)) => {
                    let Some(Tok::Num(n)) = self.bump() else {
                        unreachable!()
                    };
                    if matches!(self.peek(), Some(Tok::Colon)) {
                        let mut ws = vec![n];
                        while matches!(self.peek(), Some(Tok::Colon)) {
                            self.pos += 1;
                            match self.bump() {
                                Some(Tok::Num(m)) => ws.push(m),
                                _ => {
                                    return Err(Error::syntax(
                                        self.line(),
                                        "expected a number after `:`",
                                    ));
                                }
                            }
                        }
                        args.push(Arg::Weights(ws));
                    } else {
                        args.push(Arg::Value(Value::Num(n)));
                    }
                }
                _ => return Ok(args),
            }
        }
    }

    fn value(&mut self) -> Result<Value, Error> {
        match self.bump() {
            Some(Tok::Ident(s)) => Ok(Value::Ident(s)),
            Some(Tok::Str(s)) => Ok(Value::Str(s)),
            Some(Tok::Num(n)) => Ok(Value::Num(n)),
            other => Err(Error::syntax(
                self.line(),
                format!(
                    "expected a value after `=`, found {}",
                    describe_opt(other.as_ref())
                ),
            )),
        }
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
        let src = r#"
diagram "T" width=900 {
  row 3:7 {
    card a blue { sub "x" }
    card b
  }
  a -impl-> b "impl" via=right
}
"#;
        let stmts = parse(src).unwrap();
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
