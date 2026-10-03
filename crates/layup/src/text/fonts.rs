//! Single source of font bytes for measurement and rendering.
use super::Font;
use super::subset::subset;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub(super) const SANS: &[u8] = include_bytes!("../../fonts/IBMPlexSans-Regular.ttf");
pub(super) const BOLD: &[u8] = include_bytes!("../../fonts/IBMPlexSans-SemiBold.ttf");
pub(super) const MONO: &[u8] = include_bytes!("../../fonts/IBMPlexMono-Regular.ttf");

pub(super) const ARABIC: &[u8] = include_bytes!("../../fonts/IBMPlexSansArabic-Regular.ttf");
pub(super) const ARABIC_BOLD: &[u8] = include_bytes!("../../fonts/IBMPlexSansArabic-SemiBold.ttf");
pub(super) const HEBREW: &[u8] = include_bytes!("../../fonts/IBMPlexSansHebrew-Regular.ttf");
pub(super) const HEBREW_BOLD: &[u8] = include_bytes!("../../fonts/IBMPlexSansHebrew-SemiBold.ttf");
const DATA: [&[u8]; 7] = [SANS, BOLD, MONO, ARABIC, ARABIC_BOLD, HEBREW, HEBREW_BOLD];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FaceId(pub usize);

pub(super) fn data(id: FaceId) -> &'static [u8] {
    DATA[id.0]
}

pub(super) fn shaping_face(id: FaceId) -> &'static rustybuzz::Face<'static> {
    static FACES: [OnceLock<rustybuzz::Face<'static>>; 7] = [const { OnceLock::new() }; 7];
    FACES[id.0].get_or_init(|| {
        rustybuzz::Face::from_slice(data(id), 0).expect("bundled font must be valid")
    })
}

#[cfg(test)]
pub(super) fn face(font: Font) -> &'static ttf_parser::Face<'static> {
    shaping_face(FaceId(match font {
        Font::Sans => 0,
        Font::SansBold => 1,
        Font::Mono => 2,
    }))
}

pub(super) fn select(font: Font, c: char) -> FaceId {
    let primary = FaceId(match font {
        Font::Sans => 0,
        Font::SansBold => 1,
        Font::Mono => 2,
    });
    if shaping_face(primary).glyph_index(c).is_some() {
        return primary;
    }
    let fallbacks = if font == Font::SansBold {
        [4, 6]
    } else {
        [3, 5]
    };
    for id in fallbacks.map(FaceId) {
        if shaping_face(id).glyph_index(c).is_some() {
            return id;
        }
    }
    primary
}

pub(super) fn supported(font: Font, c: char) -> bool {
    shaping_face(select(font, c)).glyph_index(c).is_some()
        || matches!(
            unicode_bidi::bidi_class(c),
            unicode_bidi::BidiClass::BN
                | unicode_bidi::BidiClass::B
                | unicode_bidi::BidiClass::S
                | unicode_bidi::BidiClass::LRE
                | unicode_bidi::BidiClass::RLE
                | unicode_bidi::BidiClass::LRO
                | unicode_bidi::BidiClass::RLO
                | unicode_bidi::BidiClass::PDF
                | unicode_bidi::BidiClass::LRI
                | unicode_bidi::BidiClass::RLI
                | unicode_bidi::BidiClass::FSI
                | unicode_bidi::BidiClass::PDI
        )
        || matches!(c, '\u{fe0e}' | '\u{fe0f}')
}

/// Embed only used fallback faces. Shaping fonts are kept intact; our small
/// Latin subsetter deliberately drops OpenType layout tables.
pub(crate) fn stylesheet(chars: &BTreeSet<char>, supplied: &super::Fonts) -> String {
    let mut used: [BTreeSet<char>; 7] = std::array::from_fn(|_| BTreeSet::new());
    for &c in chars {
        for font in [Font::Sans, Font::SansBold, Font::Mono] {
            let id = select(font, c);
            if shaping_face(id).glyph_index(c).is_some() {
                used[id.0].insert(c);
            }
        }
    }
    let mut css = String::new();
    for (i, (family, style, weight)) in [
        ("Layup Sans", "Regular", "400"),
        ("Layup Sans", "SemiBold", "600 900"),
        ("Layup Mono", "Regular", "400"),
        ("Layup Arabic", "Regular", "400"),
        ("Layup Arabic", "SemiBold", "600 900"),
        ("Layup Hebrew", "Regular", "400"),
        ("Layup Hebrew", "SemiBold", "600 900"),
    ]
    .into_iter()
    .enumerate()
    {
        if i >= 3 && used[i].is_empty() {
            continue;
        }
        let shaping = matches!(i, 3..=6)
            || used[i].iter().any(|&c| {
                unicode_bidi::bidi_class(c) == unicode_bidi::BidiClass::NSM
                    || matches!(c as u32, 0x1100..=0x11ff | 0xa960..=0xa97f | 0xd7b0..=0xd7ff)
            });
        let (font, mapped) = if shaping {
            (
                data(FaceId(i)).to_vec(),
                used[i].iter().map(|&c| c as u32).collect(),
            )
        } else {
            subset(data(FaceId(i)), &used[i], family, style)
        };
        css.push_str(&format!("@font-face{{font-family:'{family}';font-style:normal;font-weight:{weight};unicode-range:{};src:url(data:font/ttf;base64,{}) format('truetype')}}\n", unicode_range(&mapped), STANDARD.encode(font)));
    }
    for bytes in &supplied.fallbacks {
        let face = rustybuzz::Face::from_slice(bytes, 0).expect("validated fallback font");
        let mapped: Vec<_> = chars
            .iter()
            .filter(|&&c| {
                face.glyph_index(c).is_some()
                    && [Font::Sans, Font::SansBold, Font::Mono]
                        .iter()
                        .any(|&font| !supported(font, c))
            })
            .map(|&c| c as u32)
            .collect();
        if mapped.is_empty() {
            continue;
        }
        let family = super::font_family(bytes);
        css.push_str(&format!("@font-face{{font-family:'{family}';font-style:normal;font-weight:400;unicode-range:{};src:url(data:font/ttf;base64,{})}}\n",unicode_range(&mapped),STANDARD.encode(bytes)));
    }
    css
}

/// Sorted code points as `U+20-22,U+41` ranges.
fn unicode_range(codes: &[u32]) -> String {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for &c in codes {
        match ranges.last_mut() {
            Some((_, end)) if *end + 1 == c => *end = c,
            _ => ranges.push((c, c)),
        }
    }
    let range = |&(a, b): &(u32, u32)| match a == b {
        true => format!("U+{a:X}"),
        false => format!("U+{a:X}-{b:X}"),
    };
    ranges.iter().map(range).collect::<Vec<_>>().join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embeds_one_subset_per_face() {
        let css = stylesheet(&"abc".chars().collect(), &super::super::Fonts::default());
        assert_eq!(css.matches("@font-face").count(), 3);
        assert!(css.len() < 20_000, "{}", css.len());
        assert!(css.contains("unicode-range:U+61-63;"));
    }
}
