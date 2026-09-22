//! Single source of font bytes for measurement and rendering.
use super::Font;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::sync::OnceLock;

const SANS: &[u8] = include_bytes!("../../fonts/IBMPlexSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../fonts/IBMPlexSans-SemiBold.ttf");
const MONO: &[u8] = include_bytes!("../../fonts/IBMPlexMono-Regular.ttf");

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

pub(crate) fn stylesheet() -> &'static str {
    static CSS: OnceLock<String> = OnceLock::new();
    CSS.get_or_init(|| {
        let mut css = String::new();
        for (family, weight, data) in [("Layup Sans", "400", SANS), ("Layup Sans", "600 900", BOLD), ("Layup Mono", "400", MONO)] {
            css.push_str(&format!("@font-face{{font-family:'{family}';font-style:normal;font-weight:{weight};src:url(data:font/ttf;base64,{}) format('truetype')}}\n", STANDARD.encode(data)));
        }
        css
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embeds_the_measured_fonts() {
        let css = stylesheet();
        for data in [SANS, BOLD, MONO] {
            assert!(css.contains(&STANDARD.encode(data)));
        }
        assert_eq!(css.matches("@font-face").count(), 3);
    }
}
