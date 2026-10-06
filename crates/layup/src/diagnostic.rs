//! Source locations use UTF-8 byte offsets and 1-based Unicode scalar columns.
//! Editors can use byte spans directly or convert columns to their own units.
use std::fmt::Write as _;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl Span {
    pub fn join(self, end: Self) -> Self {
        Self {
            end: end.end,
            end_line: end.end_line,
            end_column: end.end_column,
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub span: Option<Span>,
    pub line: Option<usize>,
    pub help: Option<String>,
    pub related: Vec<(Span, String)>,
}

impl Diagnostic {
    pub fn from_error(error: &crate::Error) -> Self {
        Self {
            severity: Severity::Error,
            code: error.code,
            message: error.msg.clone(),
            span: error.span.as_deref().copied(),
            line: error.line,
            help: error.help.clone(),
            related: error.related.clone(),
        }
    }

    /// JSON contains byte offsets and scalar columns, plus legacy `line` and
    /// `message` fields. No source text or font payloads are included.
    pub fn json(&self) -> String {
        let span = self.span.map_or("null".to_string(), span_json);
        let line = self.line.map_or("null".to_string(), |l| l.to_string());
        let column = self
            .span
            .map_or("null".to_string(), |s| s.column.to_string());
        let related = self
            .related
            .iter()
            .map(|(s, msg)| format!("{{\"span\":{},\"message\":{}}}", span_json(*s), quote(msg)))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"line\":{line},\"message\":{},\"column\":{column},\"severity\":{},\"code\":{},\"span\":{span},\"help\":{},\"related\":[{related}]}}",
            quote(&self.message),
            quote(self.severity.name()),
            quote(self.code),
            self.help.as_deref().map_or("null".into(), quote)
        )
    }

    /// Render a diagnostic and a source excerpt. `line_offset` maps a fenced
    /// DSL snippet onto the line of its containing Markdown file.
    pub fn display(&self, source: &str, name: &str, line_offset: usize) -> String {
        let severity = self.severity.name();
        let line = self.line;
        let location = match (line, self.span) {
            (Some(l), Some(s)) => format!("{name}:{}:{}", l + line_offset, s.column),
            (Some(l), None) => format!("{name}:{}", l + line_offset),
            _ => name.to_string(),
        };
        let mut out = format!("{location}: {severity}[{}]: {}", self.code, self.message);
        if let Some(span) = self.span
            && let Some(text) = source.split('\n').nth(span.line.saturating_sub(1))
        {
            let text = text.trim_end_matches('\r');
            let expand = |text: &str| -> String {
                let mut out = String::new();
                for c in text.chars() {
                    if c == '\t' {
                        out.extend(std::iter::repeat_n(' ', 4 - out.width() % 4));
                    } else if c.is_control() {
                        out.push('�');
                    } else {
                        out.push(c);
                    }
                }
                out
            };
            let prefix = expand(
                &text
                    .chars()
                    .take(span.column.saturating_sub(1))
                    .collect::<String>(),
            );
            let count = if span.end_line == span.line {
                span.end_column.saturating_sub(span.column).max(1)
            } else {
                text.chars()
                    .count()
                    .saturating_sub(span.column.saturating_sub(1))
                    .max(1)
            };
            let shown_line = span.line + line_offset;
            let _ = write!(
                out,
                "\n {shown_line} | {}\n {} | {}{}",
                expand(text),
                " ".repeat(shown_line.to_string().len()),
                " ".repeat(prefix.width()),
                "^".repeat(
                    expand(
                        &text
                            .chars()
                            .take(span.column.saturating_sub(1) + count)
                            .collect::<String>()
                    )
                    .width()
                    .saturating_sub(prefix.width())
                    .max(1)
                )
            );
        }
        if let Some(help) = &self.help {
            let _ = write!(out, "\n  help: {help}");
        }
        for (span, msg) in &self.related {
            let _ = write!(
                out,
                "\n  note: {name}:{}:{}: {msg}",
                span.line + line_offset,
                span.column
            );
        }
        out
    }
}

fn span_json(s: Span) -> String {
    format!(
        "{{\"start\":{},\"end\":{},\"line\":{},\"column\":{},\"endLine\":{},\"endColumn\":{}}}",
        s.start, s.end, s.line, s.column, s.end_line, s.end_column
    )
}

pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if c.is_control() && (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Attach a token span to an older line-based semantic diagnostic. Select the
/// named token when available, otherwise point at the first token on the line.
pub(crate) fn locate(error: &mut crate::Error, tokens: &[crate::lexer::Token]) {
    if error.span.is_some() {
        return;
    }
    let Some(line) = error.line else {
        return;
    };
    let named: Vec<_> = error
        .msg
        .split('`')
        .enumerate()
        .filter_map(|(i, s)| (i % 2 == 1).then_some(s.split('=').next().unwrap_or(s)))
        .collect();
    let at_line: Vec<_> = tokens
        .iter()
        .filter(|t| {
            t.line == line
                && !matches!(
                    t.tok,
                    crate::lexer::Tok::Newline | crate::lexer::Tok::Comment(_)
                )
        })
        .collect();
    let token = named
        .iter()
        .find_map(|name| {
            at_line
                .iter()
                .rev()
                .find(|t| match &t.tok {
                    crate::lexer::Tok::Ident(s) | crate::lexer::Tok::Str(s) => s == name,
                    crate::lexer::Tok::Arrow { kind: Some(s), .. } => s == name,
                    _ => false,
                })
                .copied()
        })
        .or_else(|| at_line.first().copied());
    if let Some(t) = token {
        error.span = Some(Box::new(t.span));
    }
}

pub(crate) fn suggestion<'a>(
    name: &str,
    choices: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    fn distance(a: &str, b: &str) -> usize {
        let b: Vec<_> = b.chars().collect();
        let mut row: Vec<_> = (0..=b.len()).collect();
        for (i, a) in a.chars().enumerate() {
            let mut last = row[0];
            row[0] = i + 1;
            for (j, &b) in b.iter().enumerate() {
                let old = row[j + 1];
                row[j + 1] = (row[j] + 1).min(old + 1).min(last + usize::from(a != b));
                last = old;
            }
        }
        row[b.len()]
    }
    let limit = if name.chars().count() <= 4 { 1 } else { 2 };
    choices
        .into_iter()
        .map(|c| (distance(name, c), c))
        .filter(|(d, _)| *d <= limit)
        .min_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)))
        .map(|(_, c)| format!("did you mean `{c}`?"))
}
