//! UTF-8 tokenizer. Newlines and semicolons terminate statements. The
//! lossless entry point retains comments and source slices for tooling.
use crate::{Error, diagnostic::Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Str(String),
    Num(f64),
    Arrow {
        kind: Option<String>,
        left: bool,
        right: bool,
    },
    LBrace,
    RBrace,
    Eq,
    Colon,
    Newline,
    /// Retained by `lex_lossless` and `lex_recovering`, skipped by `lex`.
    Comment(String),
    Invalid,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub errors: Vec<Error>,
    pub eof: Span,
}

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    Ok(lex_lossless(src)?
        .into_iter()
        .filter(|t| !matches!(t.tok, Tok::Comment(_)))
        .collect())
}

pub fn lex_lossless(src: &str) -> Result<Vec<Token>, Error> {
    let result = lex_recovering(src);
    if let Some(e) = result.errors.into_iter().next() {
        Err(e)
    } else {
        Ok(result.tokens)
    }
}

/// Consume malformed tokens and continue, preserving valid tokens and every
/// lexical diagnostic. Unterminated multiline strings consume the remainder.
pub fn lex_recovering(src: &str) -> Lexed {
    let mut scanner = Scanner {
        src,
        pos: 0,
        line: 1,
        column: 1,
    };
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    while let Some(c) = scanner.peek() {
        let start = scanner.here();
        let result = match c {
            ' ' | '\t' | '\r' if !scanner.starts("\r\n") => {
                scanner.bump();
                continue;
            }
            '\n' | '\r' | ';' => {
                scanner.bump();
                Ok(Tok::Newline)
            }
            '/' if scanner.starts("//") => {
                while scanner.peek().is_some_and(|c| c != '\n' && c != '\r') {
                    scanner.bump();
                }
                Ok(Tok::Comment(src[start.start..scanner.pos].to_string()))
            }
            '{' => {
                scanner.bump();
                Ok(Tok::LBrace)
            }
            '}' => {
                scanner.bump();
                Ok(Tok::RBrace)
            }
            '=' => {
                scanner.bump();
                Ok(Tok::Eq)
            }
            ':' => {
                scanner.bump();
                Ok(Tok::Colon)
            }
            '"' => scanner.string(start),
            c if c.is_ascii_digit()
                || (c == '-' || c == '.')
                    && scanner.second().is_some_and(|c| c.is_ascii_digit()) =>
            {
                scanner.number(start)
            }
            '-' | '<' => scanner.arrow(start),
            c if is_ident_start(c) => {
                scanner.bump();
                while scanner.peek().is_some_and(|c| {
                    is_ident_char(c) || c == '-' && scanner.second().is_some_and(is_ident_char)
                }) {
                    scanner.bump();
                }
                Ok(Tok::Ident(src[start.start..scanner.pos].to_string()))
            }
            other => {
                scanner.bump();
                Err(scanner.error(
                    start,
                    "lex/character",
                    format!("unexpected character `{other}`"),
                ))
            }
        };
        match result {
            Ok(tok) => tokens.push(Token {
                tok,
                line: start.line,
                span: start.join(scanner.here()),
            }),
            Err(e) => {
                tokens.push(Token {
                    tok: Tok::Invalid,
                    line: start.line,
                    span: start.join(scanner.here()),
                });
                errors.push(e);
            }
        }
    }
    let span = scanner.here();
    tokens.push(Token {
        tok: Tok::Newline,
        line: span.line,
        span,
    });
    Lexed {
        tokens,
        errors,
        eof: scanner.here(),
    }
}

struct Scanner<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
    column: usize,
}
impl Scanner<'_> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }
    fn second(&self) -> Option<char> {
        self.src[self.pos..].chars().nth(1)
    }
    fn starts(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s)
    }
    fn here(&self) -> Span {
        Span {
            start: self.pos,
            end: self.pos,
            line: self.line,
            column: self.column,
            end_line: self.line,
            end_column: self.column,
        }
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\r' && self.peek() == Some('\n') {
            self.pos += 1;
            self.line += 1;
            self.column = 1;
        } else if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }
    fn error(&self, start: Span, code: &'static str, message: impl Into<String>) -> Error {
        Error::located(
            start.join(self.here()),
            code,
            format!("syntax: {}", message.into()),
        )
    }
    fn string(&mut self, start: Span) -> Result<Tok, Error> {
        self.bump();
        let mut s = String::new();
        let mut invalid = None;
        while let Some(c) = self.peek() {
            if c == '"' {
                self.bump();
                return invalid.map_or(Ok(Tok::Str(s)), Err);
            }
            if c == '\\' {
                let escape = self.here();
                self.bump();
                match self.bump() {
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some(c) => {
                        invalid.get_or_insert_with(||self.error(escape,"lex/escape",format!("unknown escape `\\{c}`")).with_help(r#"use `\\`, `\"`, `\n`, `\r` or `\t`; write a literal backslash as `\\`"#));
                    }
                    None => break,
                }
            } else {
                // Preserve literal string bytes, including CRLF pairs.
                let before = self.pos;
                self.bump();
                s.push_str(&self.src[before..self.pos]);
            }
        }
        Err(self
            .error(start, "lex/string", "unterminated string")
            .with_help("close the string with `\"`; escape embedded quotes with `\\\"`"))
    }
    fn number(&mut self, start: Span) -> Result<Tok, Error> {
        if self.peek() == Some('-') {
            self.bump();
        }
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
        if self.peek() == Some('.') {
            self.bump();
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
            }
        }
        if self.peek().is_some_and(|c| c == 'e' || c == 'E') {
            self.bump();
            if self.peek().is_some_and(|c| c == '+' || c == '-') {
                self.bump();
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
            }
        }
        // Reject malformed numeric suffixes together rather than splitting
        // `1..2` or `12px` into unrelated, misleading parser errors.
        while self
            .peek()
            .is_some_and(|c| c == '.' || is_ident_start(c) || c.is_ascii_digit())
        {
            self.bump();
        }
        let raw = &self.src[start.start..self.pos];
        match raw.parse::<f64>() {
            Ok(n) if n.is_finite() => Ok(Tok::Num(n)),
            _ => Err(self.error(start,"lex/number",format!("bad or non-finite number `{raw}`")).with_help("use a finite decimal number, optionally with an exponent, such as `900`, `12.5` or `1e3`")),
        }
    }
    fn arrow(&mut self, start: Span) -> Result<Tok, Error> {
        let left = self.peek() == Some('<');
        if left {
            self.bump();
        }
        if self.peek() != Some('-') {
            return Err(self.error(start, "lex/arrow", "expected `-` after `<`"));
        }
        self.bump();
        let mut kind = None;
        let after_dash = (self.pos, self.line, self.column);
        if self.peek().is_some_and(is_ident_start) {
            let begin = self.pos;
            while self.peek().is_some_and(|c| {
                is_ident_char(c) || c == '-' && self.second().is_some_and(is_ident_char)
            }) {
                self.bump();
            }
            kind = Some(self.src[begin..self.pos].to_string());
            if self.peek() != Some('-') {
                if left {
                    // Compact untyped `a<-b` leaves its target for the parser.
                    (self.pos, self.line, self.column) = after_dash;
                    return Ok(Tok::Arrow {
                        kind: None,
                        left: true,
                        right: false,
                    });
                }
                return Err(self
                    .error(start, "lex/arrow", "expected `-` to close the edge kind")
                    .with_help("write a typed arrow as `a -kind-> b` or `a -kind- b`"));
            }
            self.bump();
        } else if self.peek() == Some('-') {
            self.bump();
        }
        let right = self.peek() == Some('>');
        if right {
            self.bump();
        }
        if !left && !right && kind.is_none() && self.pos - start.start < 2 {
            return Err(self.error(start, "lex/arrow", "stray `-`"));
        }
        Ok(Tok::Arrow { kind, left, right })
    }
}

fn is_ident_start(c: char) -> bool {
    unicode_ident::is_xid_start(c) || c == '_'
}
fn is_ident_char(c: char) -> bool {
    unicode_ident::is_xid_continue(c) || matches!(c, '_' | '.' | ':' | '/')
}

#[cfg(test)]
mod tests {
    use super::*;
    fn kinds(src: &str) -> Vec<Tok> {
        lex(src).unwrap().into_iter().map(|t| t.tok).collect()
    }
    #[test]
    fn arrows_and_idents() {
        let t = kinds("http-client -impl-> net::Transport \"x\"");
        assert_eq!(t[0], Tok::Ident("http-client".into()));
        assert_eq!(
            t[1],
            Tok::Arrow {
                kind: Some("impl".into()),
                left: false,
                right: true
            }
        );
        assert_eq!(t[2], Tok::Ident("net::Transport".into()));
        assert_eq!(t[3], Tok::Str("x".into()));
    }
    #[test]
    fn dash_before_arrow_ends_ident() {
        let t = kinds("a-> b");
        assert_eq!(t[0], Tok::Ident("a".into()));
        assert!(matches!(t[1], Tok::Arrow { right: true, .. }));
    }
    #[test]
    fn weights_and_comments() {
        let t = kinds("row 3:7 { // hi\n}");
        assert_eq!(t[1], Tok::Num(3.0));
        assert_eq!(t[2], Tok::Colon);
        assert_eq!(t[3], Tok::Num(7.0));
        assert_eq!(t[4], Tok::LBrace);
        assert_eq!(t[5], Tok::Newline);
    }
}
