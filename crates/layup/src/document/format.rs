use super::lex::{self, Token};
use crate::Error;

pub(super) fn format(source: &str) -> Result<String, Error> {
    super::parse(source)?;
    let tokens = lex::lex(source)?;
    let mut output = String::new();
    let mut line = String::new();
    let mut blocks = 0usize;
    let mut collections = 0usize;
    let mut braces = Vec::new();
    let mut previous = Token::Newline;
    fn flush(output: &mut String, line: &mut String, depth: usize) {
        if !line.is_empty() {
            output.push_str(&"  ".repeat(depth));
            output.push_str(line.trim_end());
            output.push('\n');
            line.clear();
        }
    }
    for located in tokens {
        let raw = &source[located.span.start..located.span.end];
        match &located.token {
            Token::Eof => flush(&mut output, &mut line, blocks + collections),
            Token::Newline => {
                flush(&mut output, &mut line, blocks + collections);
                previous = Token::Newline;
            }
            Token::Comment => {
                if !line.is_empty() {
                    line.push_str("  ");
                }
                line.push_str(raw.trim_end());
                flush(&mut output, &mut line, blocks + collections);
                previous = Token::Newline;
            }
            Token::BlockComment => {
                if !line.is_empty() && !line.ends_with(char::is_whitespace) {
                    line.push(' ');
                }
                line.push_str(raw);
                line.push(' ');
                // Keep the previous syntax token: a comment between `=` and a
                // record must not make its brace look like a diagram block.
            }
            Token::Open => {
                let block = collections == 0
                    && !matches!(
                        previous,
                        Token::Equal | Token::Colon | Token::Comma | Token::ListOpen
                    );
                braces.push(block);
                if !line.is_empty()
                    && !line.ends_with(char::is_whitespace)
                    && !matches!(previous, Token::Equal | Token::Colon | Token::ListOpen)
                {
                    line.push(' ');
                }
                line.push('{');
                if block {
                    flush(&mut output, &mut line, blocks);
                    blocks += 1;
                } else {
                    collections += 1;
                }
                previous = located.token;
            }
            Token::Close => {
                let block = braces.pop().expect("validated braces");
                if block {
                    flush(&mut output, &mut line, blocks);
                    blocks -= 1;
                } else {
                    collections -= 1;
                }
                line.push('}');
                previous = located.token;
            }
            Token::ListOpen | Token::ParenOpen => {
                if located.token == Token::ListOpen
                    && !line.is_empty()
                    && !line.ends_with(char::is_whitespace)
                    && !matches!(previous, Token::Equal | Token::Colon | Token::ListOpen)
                {
                    line.push(' ');
                }
                collections += 1;
                line.push_str(raw);
                previous = located.token;
            }
            Token::ListClose | Token::ParenClose => {
                collections -= 1;
                line.push_str(raw);
                previous = located.token;
            }
            Token::Equal | Token::Dot | Token::Colon | Token::Comma => {
                line.push_str(raw);
                previous = located.token;
            }
            _ => {
                if !line.is_empty()
                    && !line.ends_with(char::is_whitespace)
                    && !matches!(
                        previous,
                        Token::Equal
                            | Token::Dot
                            | Token::Root
                            | Token::At
                            | Token::ParenOpen
                            | Token::ListOpen
                            | Token::Open
                    )
                {
                    line.push(' ');
                }
                line.push_str(raw);
                previous = located.token;
            }
        }
    }
    Ok(output)
}
