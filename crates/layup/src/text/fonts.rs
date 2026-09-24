//! Single source of font bytes for measurement and rendering.
use super::Font;
use super::subset::subset;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub(super) const SANS: &[u8] = include_bytes!("../../fonts/IBMPlexSans-Regular.ttf");
pub(super) const BOLD: &[u8] = include_bytes!("../../fonts/IBMPlexSans-SemiBold.ttf");
pub(super) const MONO: &[u8] = include_bytes!("../../fonts/IBMPlexMono-Regular.ttf");

pub(super) fn face(font: Font) -> &'static ttf_parser::Face<'static> {
    static FACES: OnceLock<[ttf_parser::Face<'static>; 3]> = OnceLock::new();
    let faces = FACES.get_or_init(|| {
        [SANS, BOLD, MONO]
            .map(|data| ttf_parser::Face::parse(data, 0).expect("bundled font must be valid"))
    });
    &faces[match font {
        Font::Sans => 0,
        Font::SansBold => 1,
        Font::Mono => 2,
    }]
}

/// `@font-face` rules embedding each bundled font, subset to `chars`.
pub(crate) fn stylesheet(chars: &BTreeSet<char>) -> String {
    let mut css = String::new();
    for (family, style, weight, data) in [
        ("Layup Sans", "Regular", "400", SANS),
        ("Layup Sans", "SemiBold", "600 900", BOLD),
        ("Layup Mono", "Regular", "400", MONO),
    ] {
        let font = subset(data, chars, family, style);
        css.push_str(&format!("@font-face{{font-family:'{family}';font-style:normal;font-weight:{weight};src:url(data:font/ttf;base64,{}) format('truetype')}}\n", STANDARD.encode(font)));
    }
    css
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embeds_one_subset_per_face() {
        let css = stylesheet(&"abc".chars().collect());
        assert_eq!(css.matches("@font-face").count(), 3);
        assert!(css.len() < 20_000, "{}", css.len());
    }
}
