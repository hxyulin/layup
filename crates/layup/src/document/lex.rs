use crate::{Error, diagnostic::Span};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Token {
    Word(String),
    String(String),
    Number(f64),
    Arrow(String),
    Open,
    Close,
    ListOpen,
    ListClose,
    ParenOpen,
    ParenClose,
    Equal,
    Colon,
    Root,
    Dot,
    Comma,
    At,
    Newline,
    Comment,
    BlockComment,
    Eof,
}
#[derive(Debug, Clone)]
pub(super) struct Located {
    pub token: Token,
    pub span: Span,
}

/// Scan trivia without interpreting comment contents as language tokens.
/// Byte scanning is safe here: only ASCII delimiter boundaries are returned.
pub(super) fn block_comment_length(source: &str) -> Result<usize, &'static str> {
    let bytes = source.as_bytes();
    let mut depth = 1usize;
    let mut offset = 2;
    while offset < bytes.len() {
        match bytes.get(offset..offset + 2) {
            Some(b"/*") => {
                depth += 1;
                if depth > 128 {
                    return Err("maximum block-comment nesting is 128");
                }
                offset += 2;
            }
            Some(b"*/") => {
                depth -= 1;
                offset += 2;
                if depth == 0 {
                    return Ok(offset);
                }
            }
            _ => offset += 1,
        }
    }
    Err("unterminated block comment; expected `*/`")
}

pub(super) fn lex(source: &str) -> Result<Vec<Located>, Error> {
    let mut result = Vec::new();
    let mut offset = 0;
    let mut line = 1;
    let mut column = 1;
    while offset < source.len() {
        let rest = &source[offset..];
        let c = rest.chars().next().unwrap();
        let start = Span {
            start: offset,
            end: offset,
            line,
            column,
            end_line: line,
            end_column: column,
        };
        let (token, length) = if c == ' ' || c == '\t' || c == '\r' {
            offset += c.len_utf8();
            column += 1;
            continue;
        } else if rest.starts_with("//") {
            (Token::Comment, rest.find('\n').unwrap_or(rest.len()))
        } else if rest.starts_with("/*") {
            let length = block_comment_length(rest).map_err(|message| {
                Error::located(
                    Span {
                        end: start.start + 2,
                        end_column: start.column + 2,
                        ..start
                    },
                    "document/comment",
                    message,
                )
            })?;
            (Token::BlockComment, length)
        } else if c == '"' {
            let mut value = String::new();
            let mut chars = rest.char_indices().skip(1);
            let mut end = None;
            while let Some((i, ch)) = chars.next() {
                match ch {
                    '"' => {
                        end = Some(i + 1);
                        break;
                    }
                    '\\' => {
                        let Some((_, escaped)) = chars.next() else {
                            break;
                        };
                        value.push(match escaped {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            _ => {
                                return Err(super::error(
                                    start,
                                    format!("unsupported escape `\\{escaped}`"),
                                ));
                            }
                        });
                    }
                    ch => value.push(ch),
                }
            }
            (
                Token::String(value),
                end.ok_or_else(|| super::error(start, "unterminated string"))?,
            )
        } else if let Some(arrow) = ["<->", "->", "<-", "--"]
            .into_iter()
            .find(|s| rest.starts_with(s))
        {
            (Token::Arrow(arrow.into()), arrow.len())
        } else if c.is_ascii_digit()
            || (c == '-' && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit))
        {
            let bytes = rest.as_bytes();
            let mut length = usize::from(c == '-');
            while bytes.get(length).is_some_and(u8::is_ascii_digit) {
                length += 1;
            }
            if bytes.get(length) == Some(&b'.') {
                length += 1;
                while bytes.get(length).is_some_and(u8::is_ascii_digit) {
                    length += 1;
                }
            }
            if matches!(bytes.get(length), Some(b'e' | b'E')) {
                length += 1;
                if matches!(bytes.get(length), Some(b'+' | b'-')) {
                    length += 1;
                }
                while bytes.get(length).is_some_and(u8::is_ascii_digit) {
                    length += 1;
                }
            }
            let number = rest[..length]
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .ok_or_else(|| super::error(start, "expected a finite number"))?;
            (Token::Number(number), length)
        } else if unicode_ident::is_xid_start(c) || c == '_' {
            let mut length = c.len_utf8();
            for (i, ch) in rest.char_indices().skip(1) {
                if unicode_ident::is_xid_continue(ch)
                    || ch == '_'
                    || (ch == '-'
                        && rest[i + 1..]
                            .chars()
                            .next()
                            .is_some_and(unicode_ident::is_xid_continue))
                {
                    length = i + ch.len_utf8();
                } else {
                    break;
                }
            }
            (Token::Word(rest[..length].into()), length)
        } else if rest.starts_with("::") {
            (Token::Root, 2)
        } else {
            (
                match c {
                    '{' => Token::Open,
                    '}' => Token::Close,
                    '[' => Token::ListOpen,
                    ']' => Token::ListClose,
                    '(' => Token::ParenOpen,
                    ')' => Token::ParenClose,
                    '=' => Token::Equal,
                    ':' => Token::Colon,
                    '.' => Token::Dot,
                    ',' => Token::Comma,
                    '@' => Token::At,
                    '\n' | ';' => Token::Newline,
                    _ => return Err(super::error(start, format!("unexpected character `{c}`"))),
                },
                c.len_utf8(),
            )
        };
        for ch in source[offset..offset + length].chars() {
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        offset += length;
        result.push(Located {
            token,
            span: Span {
                end: offset,
                end_line: line,
                end_column: column,
                ..start
            },
        });
    }
    result.push(Located {
        token: Token::Eof,
        span: Span {
            start: offset,
            end: offset,
            line,
            column,
            end_line: line,
            end_column: column,
        },
    });
    Ok(result)
}
