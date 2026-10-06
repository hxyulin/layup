//! Syntax-only formatter: token spellings, comments, and statement order are
//! retained. It never lays out or renders a diagram and refuses invalid syntax.
use crate::{
    Error,
    lexer::{Tok, lex_lossless},
};

pub fn format(source: &str) -> Result<String, Error> {
    if crate::document::is_versioned(source) {
        return crate::document::format(source);
    }
    crate::parser::parse(source)?;
    let tokens = lex_lossless(source)?;
    let mut out = String::new();
    let mut line = String::new();
    let mut depth = 0usize;
    let mut previous: Option<&Tok> = None;
    let mut blank = false;
    let mut line_ended = false;
    let flush = |line: &mut String, out: &mut String, depth: usize| {
        if !line.is_empty() {
            out.push_str(&"  ".repeat(depth));
            out.push_str(line.trim_end());
            out.push('\n');
            line.clear();
        }
    };
    for (i, token) in tokens.iter().enumerate() {
        let raw = &source[token.span.start..token.span.end];
        if !matches!(token.tok, Tok::Newline | Tok::Comment(_)) {
            line_ended = false;
        }
        match &token.tok {
            Tok::Newline => {
                if line_ended {
                    line_ended = false;
                    previous = None;
                    continue;
                }
                if line.is_empty() {
                    // One authored blank line; semicolons are separators only.
                    if raw != ";"
                        && !raw.is_empty()
                        && !out.is_empty()
                        && !blank
                        && !out.ends_with("{\n")
                    {
                        out.push('\n');
                        blank = true;
                    }
                } else {
                    flush(&mut line, &mut out, depth);
                    blank = false;
                }
                previous = None;
            }
            Tok::LBrace => {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push('{');
                let next = tokens.get(i + 1).map(|t| &t.tok);
                if !matches!(next, Some(Tok::RBrace) | Some(Tok::Comment(_))) {
                    flush(&mut line, &mut out, depth);
                    line_ended = true;
                }
                depth += 1;
                previous = Some(&token.tok);
                blank = false;
            }
            Tok::RBrace => {
                let empty = matches!(previous, Some(Tok::LBrace)) && line.ends_with('{');
                if !empty {
                    flush(&mut line, &mut out, depth);
                }
                depth = depth.saturating_sub(1);
                line.push('}');
                previous = Some(&token.tok);
                blank = false;
            }
            Tok::Comment(_) => {
                if !line.is_empty() {
                    line.push_str("  ");
                }
                line.push_str(raw.trim_end());
                // A comment after `{` belongs to the parent header line.
                let indent = if matches!(previous, Some(Tok::LBrace)) {
                    depth.saturating_sub(1)
                } else {
                    depth
                };
                flush(&mut line, &mut out, indent);
                line_ended = true;
                previous = None;
                blank = false;
            }
            Tok::Eq | Tok::Colon => {
                line.push_str(raw);
                previous = Some(&token.tok);
                blank = false;
            }
            _ => {
                if !line.is_empty() && !matches!(previous, Some(Tok::Eq | Tok::Colon)) {
                    line.push(' ');
                }
                line.push_str(raw);
                previous = Some(&token.tok);
                blank = false;
            }
        }
    }
    flush(&mut line, &mut out, depth);
    while out.ends_with("\n\n") {
        out.pop();
    }
    Ok(out)
}
