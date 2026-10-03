//! Text measurement from the same bundled fonts embedded in SVG output.
//! Shapes Unicode text with the same bundled fonts used in SVG output.
//! Kerning and optional ligatures stay disabled for consistent Latin metrics.

mod fonts;
mod subset;
pub(crate) use fonts::stylesheet;
use std::sync::Arc;
use unicode_bidi::{BidiInfo, Direction as BidiDirection};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

/// Optional user fonts, tried after the bundled Latin, Arabic and Hebrew faces.
/// Font bytes are shared with compiled scenes and embedded without modification.
#[derive(Clone, Default)]
pub struct Fonts {
    pub(crate) fallbacks: Vec<Arc<[u8]>>,
}

impl std::fmt::Debug for Fonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fonts")
            .field(
                "fallbacks",
                &self
                    .fallbacks
                    .iter()
                    .map(|bytes| (font_family(bytes), bytes.len()))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

pub(crate) fn font_family(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, byte| {
        (h ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("Layup User {hash:x}")
}

impl Fonts {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_fallback(&mut self, bytes: impl Into<Arc<[u8]>>) -> Result<(), crate::Error> {
        let bytes = bytes.into();
        if bytes.starts_with(b"ttcf") {
            return Err(crate::Error::new(
                "font collections are not supported; provide a standalone OpenType or TrueType face",
            ));
        }
        rustybuzz::Face::from_slice(&bytes, 0).ok_or_else(|| {
            crate::Error::new("fallback font must be a valid OpenType or TrueType font")
        })?;
        self.fallbacks.push(bytes);
        Ok(())
    }
    fn faces(&self) -> Vec<rustybuzz::Face<'_>> {
        self.fallbacks
            .iter()
            .map(|bytes| rustybuzz::Face::from_slice(bytes, 0).expect("validated fallback font"))
            .collect()
    }
}

/// CJK uses conventional em-width estimates when no user font covers it.
pub fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x1100..=0x11ff | 0x2e80..=0xa4cf | 0xa960..=0xa97f | 0xac00..=0xd7ff | 0xf900..=0xfaff | 0xfe10..=0xfe1f | 0xfe30..=0xfe4f | 0xff01..=0xff60 | 0xffe0..=0xffe6 | 0x1b000..=0x1b2ff | 0x20000..=0x323af)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Auto,
    Ltr,
    Rtl,
}

impl Direction {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "ltr" => Some(Self::Ltr),
            "rtl" => Some(Self::Rtl),
            _ => None,
        }
    }
    pub fn resolve(self, text: &str) -> Self {
        if self == Self::Auto {
            if unicode_bidi::get_base_direction(text) == BidiDirection::Rtl {
                Self::Rtl
            } else {
                Self::Ltr
            }
        } else {
            self
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    Sans,
    SansBold,
    Mono,
}

/// Whether a character is covered by a bundled face or is a layout control.
pub fn supports(c: char, font: Font) -> bool {
    fonts::supported(font, c)
}

pub fn supports_with_fonts(c: char, font: Font, supplied: &Fonts) -> bool {
    supports(c, font)
        || supplied
            .faces()
            .iter()
            .any(|face| face.glyph_index(c).is_some())
}

/// Width in px, including letter spacing, shaped with bundled faces.
pub fn width(s: &str, font: Font, size: f64, letter_spacing_em: f64) -> f64 {
    width_with_fonts(s, font, size, letter_spacing_em, &Fonts::default())
}

pub fn width_with_fonts(
    s: &str,
    font: Font,
    size: f64,
    letter_spacing_em: f64,
    supplied: &Fonts,
) -> f64 {
    let custom = supplied.faces();
    let select = |c| {
        let primary = fonts::select(font, c);
        if fonts::shaping_face(primary).glyph_index(c).is_some() {
            return primary;
        }
        custom
            .iter()
            .position(|face| face.glyph_index(c).is_some())
            .map_or(primary, |i| fonts::FaceId(7 + i))
    };
    let bidi = BidiInfo::new(s, None);
    let mut units = 0.0;
    for para in &bidi.paragraphs {
        let (levels, ranges) = bidi.visual_runs(para, para.range.clone());
        for range in ranges {
            let rtl = levels[range.start].is_rtl();
            let part = &s[range];
            let mut start = 0;
            let mut selected = select(part.chars().next().unwrap());
            let mut script = Script::Common;
            for (i, c) in part.char_indices() {
                let next_font = select(c);
                let next_script = c.script();
                let strong = !matches!(next_script, Script::Common | Script::Inherited);
                if i > start
                    && (selected != next_font
                        || (strong && script != Script::Common && script != next_script))
                {
                    let face = if selected.0 < 7 {
                        fonts::shaping_face(selected)
                    } else {
                        &custom[selected.0 - 7]
                    };
                    units += shaped_width(&part[start..i], face, rtl);
                    start = i;
                    script = Script::Common;
                }
                selected = next_font;
                if strong {
                    script = next_script;
                }
            }
            let face = if selected.0 < 7 {
                fonts::shaping_face(selected)
            } else {
                &custom[selected.0 - 7]
            };
            units += shaped_width(&part[start..], face, rtl);
        }
    }
    (units + letter_spacing_em * s.graphemes(true).count() as f64) * size
}

fn shaped_width(s: &str, face: &rustybuzz::Face<'_>, rtl: bool) -> f64 {
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(s);
    buffer.set_direction(if rtl {
        rustybuzz::Direction::RightToLeft
    } else {
        rustybuzz::Direction::LeftToRight
    });
    buffer.guess_segment_properties();
    let features = ["kern=0", "liga=0", "clig=0"].map(|s| s.parse().unwrap());
    let shaped = rustybuzz::shape(face, &features, buffer);
    let mut advance: f64 = shaped
        .glyph_positions()
        .iter()
        .map(|p| f64::from(p.x_advance))
        .sum();
    // Unknown glyphs remain visible with viewer fallback, but their metrics
    // cannot be deterministic. Report them separately at compilation time.
    for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
        if info.glyph_id == 0 {
            let c = s[info.cluster as usize..].chars().next().unwrap();
            advance += (if c as u32 >= 0x2e80 { 1.0 } else { 0.6 })
                * f64::from(face.units_per_em())
                - f64::from(pos.x_advance);
        }
    }
    advance / f64::from(face.units_per_em())
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
                text: part.replace('\t', " "),
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
    runs_width_with_fonts(runs, sans, size, &Fonts::default())
}

pub fn runs_width_with_fonts(runs: &[Run], sans: Font, size: f64, supplied: &Fonts) -> f64 {
    runs.iter()
        .map(|r| {
            width_with_fonts(
                &r.text,
                if r.code { Font::Mono } else { sans },
                size,
                0.0,
                supplied,
            )
        })
        .sum()
}

/// Greedy Unicode line wrapping. Backtick spans remain unbreakable; CJK
/// punctuation, nonbreaking spaces and grapheme clusters follow UAX #14.
pub fn wrap(s: &str, sans: Font, size: f64, max_width: f64) -> Vec<Vec<Run>> {
    wrap_with_fonts(s, sans, size, max_width, &Fonts::default())
}

pub fn wrap_with_fonts(
    s: &str,
    sans: Font,
    size: f64,
    max_width: f64,
    supplied: &Fonts,
) -> Vec<Vec<Run>> {
    let runs = runs(s);
    let plain: String = runs.iter().map(|r| r.text.as_str()).collect();
    let mut at = 0;
    let spans: Vec<_> = runs
        .iter()
        .map(|r| {
            let start = at;
            at += r.text.len();
            (start..at, r)
        })
        .collect();
    let mut lines = Vec::new();
    let (mut start, mut previous) = (0, 0);
    for (end, opportunity) in unicode_linebreak::linebreaks(&plain) {
        let mandatory = opportunity == unicode_linebreak::BreakOpportunity::Mandatory;
        if !mandatory
            && spans
                .iter()
                .any(|(range, run)| run.code && range.start < end && end < range.end)
        {
            continue;
        }
        let candidate = slice_runs(&plain, &spans, start, end);
        if previous > start && runs_width_with_fonts(&candidate, sans, size, supplied) > max_width {
            lines.push(slice_runs(&plain, &spans, start, previous));
            start = previous;
        }
        previous = end;
        if mandatory {
            let line = slice_runs(&plain, &spans, start, end);
            if !line.is_empty() || end < plain.len() || plain[start..end].contains('\n') {
                lines.push(line);
            }
            start = end;
        }
    }
    lines
}

fn slice_runs(
    plain: &str,
    spans: &[(std::ops::Range<usize>, &Run)],
    start: usize,
    end: usize,
) -> Vec<Run> {
    let trim = |c: char| c.is_whitespace() && !matches!(c, '\u{a0}' | '\u{202f}');
    let piece = plain[start..end].trim_matches(trim);
    let start = start + plain[start..end].len() - plain[start..end].trim_start_matches(trim).len();
    let end = start + piece.len();
    let mut line = Vec::new();
    for (range, run) in spans {
        let (a, b) = (start.max(range.start), end.min(range.end));
        if a < b {
            let mut r = (*run).clone();
            r.text = plain[a..b].to_string();
            push_run(&mut line, r);
        }
    }
    line
}

fn push_run(line: &mut Vec<Run>, r: Run) {
    if let Some(last) = line.last_mut()
        && last.code == r.code
        && last.tag == r.tag
    {
        last.text.push_str(&r.text);
        return;
    }
    line.push(r);
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
