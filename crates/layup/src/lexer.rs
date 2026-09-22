//! Tokenizer for the layup language.
//!
//! Newlines are significant: they terminate statements. Identifiers may
//! contain `-`, `.`, `:` and `/` so crate names and paths can be written bare;
//! a `-` only continues an identifier when another identifier character
//! follows it, which keeps `a -> b` and `a-> b` unambiguous.

use crate::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Str(String),
    Num(f64),
    /// `->`, `<-`, `<->`, `--`, and the kinded forms `-impl->`, `<-impl-`, `-impl-`.
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
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
}

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let push = |out: &mut Vec<Token>, tok: Tok, line: usize| out.push(Token { tok, line });
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\n' => {
                push(&mut out, Tok::Newline, line);
                line += 1;
                i += 1;
            }
            ' ' | '\t' | '\r' => i += 1,
            ';' => {
                push(&mut out, Tok::Newline, line);
                i += 1;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '{' => {
                push(&mut out, Tok::LBrace, line);
                i += 1;
            }
            '}' => {
                push(&mut out, Tok::RBrace, line);
                i += 1;
            }
            '=' => {
                push(&mut out, Tok::Eq, line);
                i += 1;
            }
            ':' => {
                push(&mut out, Tok::Colon, line);
                i += 1;
            }
            '"' => {
                let (s, len, newlines) = lex_string(&chars[i..], line)?;
                push(&mut out, Tok::Str(s), line);
                line += newlines;
                i += len;
            }
            '-' | '<' => {
                let (tok, len) = lex_arrow(&chars[i..], line)?;
                push(&mut out, tok, line);
                i += len;
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let n = text
                    .parse::<f64>()
                    .map_err(|_| Error::syntax(line, format!("bad number `{text}`")))?;
                push(&mut out, Tok::Num(n), line);
            }
            c if is_ident_start(c) => {
                let start = i;
                i += 1;
                while i < chars.len() && ident_continues(&chars, i) {
                    i += 1;
                }
                push(&mut out, Tok::Ident(chars[start..i].iter().collect()), line);
            }
            other => {
                return Err(Error::syntax(
                    line,
                    format!("unexpected character `{other}`"),
                ));
            }
        }
    }
    push(&mut out, Tok::Newline, line);
    Ok(out)
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '.' | ':' | '/')
}

fn ident_continues(chars: &[char], i: usize) -> bool {
    let c = chars[i];
    if is_ident_char(c) {
        return true;
    }
    c == '-' && chars.get(i + 1).is_some_and(|n| is_ident_char(*n))
}

fn lex_string(chars: &[char], line: usize) -> Result<(String, usize, usize), Error> {
    let mut s = String::new();
    let mut i = 1;
    let mut newlines = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => return Ok((s, i + 1, newlines)),
            '\\' => {
                let e = chars.get(i + 1).copied().unwrap_or('\\');
                s.push(match e {
                    'n' => '\n',
                    't' => '\t',
                    other => other,
                });
                i += 2;
            }
            c => {
                if c == '\n' {
                    newlines += 1;
                }
                s.push(c);
                i += 1;
            }
        }
    }
    Err(Error::syntax(line, "unterminated string"))
}

/// Parses one of `->`, `<-`, `<->`, `--`, `-k->`, `<-k-`, `<-k->`, `-k-`.
fn lex_arrow(chars: &[char], line: usize) -> Result<(Tok, usize), Error> {
    let mut i = 0;
    let mut left = false;
    if chars[i] == '<' {
        left = true;
        i += 1;
    }
    if chars.get(i) != Some(&'-') {
        return Err(Error::syntax(line, "expected `-` after `<`"));
    }
    i += 1;
    let mut kind = None;
    if chars.get(i).is_some_and(|c| is_ident_start(*c)) {
        let start = i;
        while i < chars.len() && (is_ident_char(chars[i]) || chars[i] == '-') && chars[i] != '-' {
            i += 1;
        }
        kind = Some(chars[start..i].iter().collect::<String>());
        if chars.get(i) != Some(&'-') {
            return Err(Error::syntax(line, "expected `-` to close the edge kind"));
        }
        i += 1;
    } else if chars.get(i) == Some(&'-') {
        i += 1;
    }
    let right = if chars.get(i) == Some(&'>') {
        i += 1;
        true
    } else {
        false
    };
    if !left && !right && kind.is_none() && i < 2 {
        return Err(Error::syntax(line, "stray `-`"));
    }
    Ok((Tok::Arrow { kind, left, right }, i))
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
