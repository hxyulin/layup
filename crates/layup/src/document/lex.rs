use crate::{Error, diagnostic::Span};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Token {
    Word(String),
    String(String),
    Integer(super::Integer),
    Float(f64),
    Opaque(String),
    Invalid,
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

pub(super) struct Lexer<'a> {
    source: &'a str,
    pub offset: usize,
    line: usize,
    column: usize,
}
impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            line: 1,
            column: 1,
        }
    }
    fn advance_to(&mut self, end: usize) {
        for ch in self.source[self.offset..end].chars() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        self.offset = end;
    }
    pub fn next(&mut self) -> Result<Located, Error> {
        let source = self.source;
        let mut offset = self.offset;
        let mut line = self.line;
        let mut column = self.column;
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
                || (c == '.' && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit))
                || (c == '-'
                    && (rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
                        || (rest.as_bytes().get(1) == Some(&b'.')
                            && rest.as_bytes().get(2).is_some_and(u8::is_ascii_digit))))
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
                let raw = &rest[..length];
                let numeric_span = Span {
                    end: start.start + length,
                    end_column: start.column + length,
                    ..start
                };
                let number = if raw.contains(['.', 'e', 'E']) {
                    Token::Float(
                        raw.parse::<f64>()
                            .ok()
                            .filter(|n| n.is_finite())
                            .ok_or_else(|| super::error(numeric_span, "expected a finite float"))?,
                    )
                } else {
                    let value = if raw.starts_with('-') {
                    raw.parse::<i64>().map(super::Integer::Signed)
                } else { raw.parse::<u64>().map(super::Integer::Unsigned) }
                    .map_err(|_| super::error(numeric_span, "integer is outside the signed/unsigned 64-bit range; quote larger identifiers"))?;
                    Token::Integer(value)
                };
                (number, length)
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
                        _ => {
                            return Err(super::error(
                                Span {
                                    end: start.start + c.len_utf8(),
                                    end_column: start.column + 1,
                                    ..start
                                },
                                format!("unexpected character `{c}`"),
                            ));
                        }
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
            self.offset = offset;
            self.line = line;
            self.column = column;
            return Ok(Located {
                token,
                span: Span {
                    end: offset,
                    end_line: line,
                    end_column: column,
                    ..start
                },
            });
        }
        self.offset = offset;
        self.line = line;
        self.column = column;
        Ok(Located {
            token: Token::Eof,
            span: Span {
                start: offset,
                end: offset,
                line,
                column,
                end_line: line,
                end_column: column,
            },
        })
    }
}

/// Boundary-only scanner: foreign syntax is never sent through the body lexer.
fn opaque_end(source: &str, start: usize) -> Result<usize, Error> {
    let mut offset = start + 1;
    let mut depth = 1usize;
    let bytes = source.as_bytes();
    let span = Span {
        start,
        end: start + 1,
        ..Default::default()
    };
    while offset < bytes.len() {
        if bytes[offset..].starts_with(b"//") {
            offset += source[offset..].find('\n').unwrap_or(bytes.len() - offset);
        } else if bytes[offset..].starts_with(b"/*") {
            offset += block_comment_length(&source[offset..])
                .map_err(|message| super::error(span, message))?;
        } else if bytes[offset] == b'"' {
            offset += 1;
            loop {
                if offset >= bytes.len() {
                    return Err(super::error(span, "unterminated string in opaque diagram"));
                }
                match bytes[offset] {
                    b'"' => {
                        offset += 1;
                        break;
                    }
                    b'\\' => {
                        offset += 1;
                        if offset < bytes.len() {
                            offset += 1;
                        }
                    }
                    _ => offset += 1,
                }
            }
        } else {
            match bytes[offset] {
                b'{' => {
                    depth += 1;
                    if depth > 128 {
                        return Err(super::error(span, "maximum opaque block nesting is 128"));
                    }
                }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(offset + 1);
                    }
                }
                _ => {}
            }
            offset += 1;
        }
    }
    Err(super::error(span, "unclosed opaque diagram block"))
}

pub(super) fn document_tokens(source: &str) -> Result<Vec<Located>, Error> {
    let (tokens, errors) = document_tokens_recovering(source);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    Ok(tokens)
}
pub(super) fn document_tokens_recovering(source: &str) -> (Vec<Located>, Vec<Error>) {
    let mut lexer = Lexer::new(source);
    let mut result: Vec<Located> = Vec::new();
    let mut header = None;
    let mut depth = 0usize;
    let mut errors = Vec::new();
    let mut in_body = false;
    let mut previous = Token::Eof;
    loop {
        let mut located = match lexer.next() {
            Ok(token) => token,
            Err(error) => {
                let mut span = error.span.as_deref().copied().unwrap_or_default();
                let mut end = span.end.max(
                    span.start
                        + source[span.start..]
                            .chars()
                            .next()
                            .map_or(0, char::len_utf8),
                );
                if error.msg.contains("unterminated") || error.msg.contains("block-comment nesting")
                {
                    end = source.len();
                } else if source[span.start..].starts_with('"') {
                    let mut escaped = false;
                    for (index, ch) in source[span.start..].char_indices().skip(1) {
                        end = span.start + index + ch.len_utf8();
                        if escaped {
                            escaped = false;
                        } else if ch == '\\' {
                            escaped = true;
                        } else if ch == '"' {
                            break;
                        }
                    }
                }
                lexer.advance_to(end);
                span.end = end;
                span.end_line = lexer.line;
                span.end_column = lexer.column;
                errors.push(error);
                Located {
                    token: Token::Invalid,
                    span,
                }
            }
        };
        match &located.token {
            Token::Word(word) if word == "diagram" && depth == 0 && header.is_none() => {
                header = Some(result.len())
            }
            Token::Open if header.is_some() && depth == 0 && previous != Token::Equal => {
                let tokens = result[header.unwrap()..]
                    .iter()
                    .filter(|t| {
                        !matches!(
                            t.token,
                            Token::Comment | Token::BlockComment | Token::Newline
                        )
                    })
                    .collect::<Vec<_>>();
                let supported = tokens.windows(3).enumerate().any(|(index, window)| {
                    matches!(&window[0].token, Token::Word(word) if word == "type" || word == "kind")
                        && window[1].token == Token::Equal
                        && matches!(&window[2].token, Token::Word(word) | Token::String(word) if word == "graph")
                        && !matches!(tokens.get(index + 3).map(|t| &t.token), Some(Token::Dot))
                });
                if !supported {
                    let (end, valid) = match opaque_end(source, located.span.start) {
                        Ok(end) => (end, true),
                        Err(mut error) => {
                            error.span = Some(Box::new(located.span));
                            error.line = Some(located.span.line);
                            errors.push(error);
                            (source.len(), false)
                        }
                    };
                    located.token = if valid {
                        Token::Opaque(source[located.span.start..end].into())
                    } else {
                        Token::Invalid
                    };
                    lexer.advance_to(end);
                    located.span.end = end;
                    located.span.end_line = lexer.line;
                    located.span.end_column = lexer.column;
                    header = None;
                } else {
                    depth += 1;
                    in_body = true;
                }
            }
            Token::Open | Token::ParenOpen | Token::ListOpen => depth += 1,
            Token::Close | Token::ParenClose | Token::ListClose => {
                depth = depth.saturating_sub(1);
                if depth == 0 && in_body && matches!(located.token, Token::Close) {
                    header = None;
                    in_body = false;
                }
            }
            _ => {}
        }
        let eof = located.token == Token::Eof;
        if !matches!(
            located.token,
            Token::Comment | Token::BlockComment | Token::Newline
        ) {
            previous = located.token.clone();
        }
        result.push(located);
        if eof {
            break;
        }
    }
    (result, errors)
}
