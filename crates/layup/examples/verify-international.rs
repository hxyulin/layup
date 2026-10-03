//! Browser fixture generator. Run `just international-test`.
use layup::{
    Theme,
    layout::Item,
    text::{Font, runs_width_with_fonts},
};
use std::{fmt::Write, fs};

fn main() {
    fs::create_dir_all("out/international").unwrap();
    let mut fonts = layup::text::Fonts::new();
    if let Some(path) = std::env::args().nth(1) {
        fonts.add_fallback(fs::read(path).unwrap()).unwrap();
    }
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>Layup international layout</title><style>body{background:#eee;margin:24px;font:16px sans-serif}svg{max-width:100%;height:auto;background:white;margin-bottom:24px}</style>",
    );
    for path in [
        "examples/international.layup",
        "examples/right-to-left.layup",
        "examples/directions.layup",
    ] {
        let source = fs::read_to_string(path).unwrap();
        let c = layup::compile_with_fonts(&source, &fonts).unwrap();
        assert!(c.warnings.is_empty(), "{path}: {:?}", c.warnings);
        let mut svg = layup::svg::render(&c, Theme::Light);
        let expected: Vec<_> = c
            .scene
            .items
            .iter()
            .map(|p| &p.item)
            .chain(c.scene.edges.iter().filter_map(|e| e.chip.as_ref()))
            .filter_map(|item| match item {
                Item::Text(t) => Some(runs_width_with_fonts(
                    &t.runs,
                    if t.weight >= 600 {
                        Font::SansBold
                    } else {
                        Font::Sans
                    },
                    t.size,
                    &fonts,
                )),
                Item::Chip { text, .. } => Some(layup::text::width_with_fonts(
                    text,
                    Font::Sans,
                    12.0,
                    0.0,
                    &fonts,
                )),
                _ => None,
            })
            .collect();
        let exact: Vec<_> = c
            .scene
            .items
            .iter()
            .map(|p| &p.item)
            .chain(c.scene.edges.iter().filter_map(|e| e.chip.as_ref()))
            .filter_map(|item| match item {
                Item::Text(t) => Some(t.runs.iter().all(|r| {
                    r.text
                        .chars()
                        .all(|ch| layup::text::supports_with_fonts(ch, Font::Sans, &fonts))
                })),
                Item::Chip { text, .. } => Some(
                    text.chars()
                        .all(|ch| layup::text::supports_with_fonts(ch, Font::Sans, &fonts)),
                ),
                _ => None,
            })
            .collect();
        let mut position = 0;
        for (measured, exact) in expected.into_iter().zip(exact) {
            let offset = svg[position..].find("<text ").unwrap() + position + 6;
            let attribute = format!("data-exact=\"{exact}\" data-measured-width=\"{measured}\" ");
            svg.insert_str(offset, &attribute);
            position = offset + attribute.len();
        }
        let _ = write!(html, "<h2>{path}</h2>{svg}");
    }
    fs::write("out/international/index.html", html).unwrap();
    println!("out/international/index.html");
}
