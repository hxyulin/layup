//! layup: authored-layout diagrams from a small text language, rendered to
//! self-contained SVG or an interactive HTML page.
//!
//! ```
//! let diagram = layup::compile(r#"diagram "Hello" { node api "API" }"#)?;
//! let svg = layup::svg::render(&diagram, layup::Theme::Light);
//! assert!(svg.contains("@font-face"));
//! # Ok::<(), layup::Error>(())
//! ```
//!
//! Pipeline: `parser` (tokens to items) -> `model` (typed diagram) ->
//! `layout` (block layout with measured text) -> `route` (orthogonal edges)
//! -> `svg` / `html` emitters. `check` reports layout problems.

mod arrange;
pub mod check;
pub mod html;
pub mod layout;
pub mod lexer;
pub mod model;
pub mod parser;
pub mod route;
pub mod style;
pub mod svg;
pub mod text;

use std::fmt;

#[derive(Debug, Clone)]
pub struct Error {
    pub line: Option<usize>,
    pub msg: String,
}

impl Error {
    pub fn new(msg: impl Into<String>) -> Self {
        Error {
            line: None,
            msg: msg.into(),
        }
    }

    pub fn at(line: usize, msg: impl Into<String>) -> Self {
        Error {
            line: Some(line),
            msg: msg.into(),
        }
    }

    pub fn syntax(line: usize, msg: impl Into<String>) -> Self {
        Error {
            line: Some(line),
            msg: format!("syntax: {}", msg.into()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {l}: {}", self.msg),
            None => write!(f, "{}", self.msg),
        }
    }
}

impl std::error::Error for Error {}

/// A non-fatal problem found while laying out or routing.
#[derive(Debug, Clone)]
pub struct Warning {
    pub line: Option<usize>,
    pub msg: String,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {l}: {}", self.msg),
            None => write!(f, "{}", self.msg),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
    /// Light, switching to dark under `prefers-color-scheme: dark`.
    Auto,
}

impl Theme {
    pub fn parse(s: &str) -> Option<Theme> {
        Some(match s {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            "auto" => Theme::Auto,
            _ => return None,
        })
    }
}

/// Everything needed to emit a diagram, plus the warnings found on the way.
pub struct Compiled {
    pub diagram: model::Diagram,
    pub scene: layout::Scene,
    pub warnings: Vec<Warning>,
}

pub fn compile(src: &str) -> Result<Compiled, Error> {
    compile_with_fonts(src, &text::Fonts::default())
}

/// Compile with optional fallback font bytes used for measurement and output.
pub fn compile_with_fonts(src: &str, fonts: &text::Fonts) -> Result<Compiled, Error> {
    let diagram = model::build(src)?;
    let mut warnings = Vec::new();
    let mut scene = layout::layout_with_fonts(&diagram, &mut warnings, fonts);
    route::route_all(&diagram, &mut scene, &mut warnings);
    check::check(&scene, &mut warnings);
    let mut missing = std::collections::BTreeSet::new();
    for placed in &scene.items {
        if let layout::Item::Text(t) = &placed.item {
            for run in &t.runs {
                let font = if run.code {
                    text::Font::Mono
                } else if t.weight >= 600 {
                    text::Font::SansBold
                } else {
                    text::Font::Sans
                };
                for c in run.text.chars() {
                    if !text::is_cjk(c)
                        && !text::supports_with_fonts(c, font, fonts)
                        && missing.insert(c)
                    {
                        warnings.push(Warning {
                            line: placed.node.map(|i| scene.nodes[i].line),
                            msg: format!("character U+{:04X} `{c}` is outside the bundled fonts; viewer fallback metrics are estimated", c as u32),
                        });
                    }
                }
            }
        }
    }
    for chip in scene
        .items
        .iter()
        .map(|p| &p.item)
        .chain(scene.edges.iter().filter_map(|e| e.chip.as_ref()))
    {
        if let layout::Item::Chip { text: label, .. } = chip {
            for c in label.chars() {
                if !text::is_cjk(c)
                    && !text::supports_with_fonts(c, text::Font::Sans, fonts)
                    && missing.insert(c)
                {
                    warnings.push(Warning { line: None, msg: format!("character U+{:04X} `{c}` is outside the bundled fonts; viewer fallback metrics are estimated", c as u32) });
                }
            }
        }
    }
    Ok(Compiled {
        diagram,
        scene,
        warnings,
    })
}
