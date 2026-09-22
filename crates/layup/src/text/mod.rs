//! Text measurement from the same bundled fonts embedded in SVG output.
//! Uses unkerned glyph advances; SVG disables kerning and optional ligatures.
//! Missing glyphs use a conservative fallback estimate. Complex-script shaping
//! and system fallback fonts are not measured.

mod fonts;
pub(crate) use fonts::stylesheet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    Sans,
    SansBold,
    Mono,
}

/// Width in px, including letter spacing, from the bundled font's advances.
pub fn width(s: &str, font: Font, size: f64, letter_spacing_em: f64) -> f64 {
    let face = fonts::face(font);
    let units: f64 = s
        .chars()
        .map(|c| {
            face.glyph_index(c)
                .and_then(|g| face.glyph_hor_advance(g))
                .map(|w| f64::from(w) / f64::from(face.units_per_em()))
                .unwrap_or(if c as u32 >= 0x2e80 { 1.0 } else { 0.6 })
        })
        .sum();
    (units + letter_spacing_em * s.chars().count() as f64) * size
}

/// A run of text in a single font; a line is a sequence of runs.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub text: String,
    pub code: bool,
    /// A bracketed tag prefix such as `[re-exported]`, drawn in accent ink.
    pub tag: bool,
}

impl Run {
    pub fn sans(text: impl Into<String>) -> Run {
        Run {
            text: text.into(),
            code: false,
            tag: false,
        }
    }
    pub fn tag(text: impl Into<String>) -> Run {
        Run {
            text: text.into(),
            code: false,
            tag: true,
        }
    }
}

/// Splits `s` on backtick spans into sans and code runs.
pub fn runs(s: &str) -> Vec<Run> {
    let mut out = Vec::new();
    let mut code = false;
    for (i, part) in s.split('`').enumerate() {
        if i > 0 {
            code = !code;
        }
        if !part.is_empty() {
            out.push(Run {
                text: part.to_string(),
                code,
                tag: false,
            });
        }
    }
    if s.matches('`').count() % 2 == 1 {
        // Unbalanced backtick: treat the trailing part as literal text.
        if let Some(last) = out.last_mut() {
            last.code = false;
        }
    }
    out
}

pub fn runs_width(runs: &[Run], sans: Font, size: f64) -> f64 {
    runs.iter()
        .map(|r| width(&r.text, if r.code { Font::Mono } else { sans }, size, 0.0))
        .sum()
}

/// Greedy word wrap that keeps backtick spans intact and prefers to break at
/// ` · ` separators before breaking between words.
pub fn wrap(s: &str, sans: Font, size: f64, max_width: f64) -> Vec<Vec<Run>> {
    let words = split_words(s);
    let mut lines: Vec<Vec<Run>> = Vec::new();
    let mut current: Vec<Run> = Vec::new();
    let mut current_w = 0.0;
    let space_w = width(" ", sans, size, 0.0);
    for w in words {
        let ww = runs_width(&w, sans, size);
        let extra = if current.is_empty() { ww } else { space_w + ww };
        if !current.is_empty() && current_w + extra > max_width {
            lines.push(std::mem::take(&mut current));
            current_w = 0.0;
        }
        if !current.is_empty() {
            push_run(&mut current, Run::sans(" "));
            current_w += space_w;
        }
        for r in w {
            push_run(&mut current, r);
        }
        current_w += ww;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn push_run(line: &mut Vec<Run>, r: Run) {
    if let Some(last) = line.last_mut()
        && last.code == r.code
    {
        last.text.push_str(&r.text);
        return;
    }
    line.push(r);
}

/// Words as run lists. A backtick span is a single unbreakable word together
/// with any punctuation glued to it.
fn split_words(s: &str) -> Vec<Vec<Run>> {
    let mut words: Vec<Vec<Run>> = Vec::new();
    let mut current: Vec<Run> = Vec::new();
    for run in runs(s) {
        if run.code {
            current.push(run);
            continue;
        }
        let mut first = true;
        for piece in run.text.split(' ') {
            if !first && !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            first = false;
            if !piece.is_empty() {
                current.push(Run::sans(piece));
            }
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_font_widths() {
        assert_eq!(width("", Font::Sans, 12.0, 0.0), 0.0);
        assert!((width("ab", Font::Mono, 10.0, 0.0) - 12.0).abs() < 0.01);
        assert_eq!(
            width("iii", Font::Mono, 12.0, 0.0),
            width("WWW", Font::Mono, 12.0, 0.0)
        );
        assert!(width("WWW", Font::Sans, 12.0, 0.0) > width("iii", Font::Sans, 12.0, 0.0));
        for font in [Font::Sans, Font::SansBold, Font::Mono] {
            assert!(
                (width("abc", font, 10.0, 0.1) - width("abc", font, 10.0, 0.0) - 3.0).abs() < 1e-9
            );
            assert!(fonts::face(font).glyph_index('é').is_some());
        }
    }

    #[test]
    fn wraps_and_keeps_code_spans() {
        let lines = wrap(
            "Deployed firmware `into_*` still means",
            Font::Sans,
            12.0,
            90.0,
        );
        assert!(lines.len() >= 2);
        let flat: Vec<Run> = lines.iter().flatten().cloned().collect();
        assert!(flat.iter().any(|r| r.code && r.text == "into_*"));
    }

    #[test]
    fn single_line_when_it_fits() {
        let lines = wrap("pid · attitude · ins", Font::Sans, 12.5, 400.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1);
    }
}
