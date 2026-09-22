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
    let diagram = model::build(src)?;
    let mut warnings = Vec::new();
    let mut scene = layout::layout(&diagram, &mut warnings);
    route::route_all(&diagram, &mut scene, &mut warnings);
    check::check(&scene, &mut warnings);
    Ok(Compiled {
        diagram,
        scene,
        warnings,
    })
}
