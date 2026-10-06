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
pub mod diagnostic;
pub mod format;
pub mod geometry;
pub mod html;
pub mod input;
pub mod layout;
pub mod lexer;
pub mod lint;
mod machine;
pub mod model;
pub mod parser;
pub mod presentation;
pub mod route;
pub mod scene;
pub mod sequence;
mod sizing;
pub mod slides;
pub mod style;
pub mod svg;
pub mod text;
pub mod views;

use std::fmt;

#[derive(Debug, Clone)]
pub struct Error {
    pub line: Option<usize>,
    pub msg: String,
    /// Boxed to keep errors small without enlarging successful `Result` values.
    pub span: Option<Box<diagnostic::Span>>,
    pub code: &'static str,
    pub help: Option<String>,
    pub related: Vec<(diagnostic::Span, String)>,
}

impl Error {
    pub fn new(msg: impl Into<String>) -> Self {
        Error {
            line: None,
            msg: msg.into(),
            span: None,
            code: "semantic",
            help: None,
            related: Vec::new(),
        }
    }

    pub fn at(line: usize, msg: impl Into<String>) -> Self {
        Error {
            line: Some(line),
            msg: msg.into(),
            span: None,
            code: "semantic",
            help: None,
            related: Vec::new(),
        }
    }

    pub fn syntax(line: usize, msg: impl Into<String>) -> Self {
        Error {
            line: Some(line),
            msg: format!("syntax: {}", msg.into()),
            span: None,
            code: "syntax",
            help: None,
            related: Vec::new(),
        }
    }
    pub fn located(span: diagnostic::Span, code: &'static str, msg: impl Into<String>) -> Self {
        Self {
            line: Some(span.line),
            msg: msg.into(),
            span: Some(Box::new(span)),
            code,
            help: None,
            related: Vec::new(),
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub(crate) fn with_optional_help(mut self, help: Option<String>) -> Self {
        self.help = help;
        self
    }

    pub fn with_related(mut self, span: diagnostic::Span, message: impl Into<String>) -> Self {
        self.related.push((span, message.into()));
        self
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
    /// Original structured input, when compiled through `input::compile`.
    pub input: Option<input::Graph>,
    pub diagram: model::Diagram,
    pub scene: layout::Scene,
    pub warnings: Vec<Warning>,
    pub slide: Option<slides::SlidePlan>,
    pub presentation: presentation::Presentation,
    pub views: Vec<views::ViewInfo>,
    pub selected_view: Option<String>,
    pub sequence: Option<sequence::SequenceInfo>,
}

impl Compiled {
    /// Rendered viewport; scene geometry stays in its original coordinate space.
    pub fn viewport(&self) -> (f64, f64) {
        self.slide
            .as_ref()
            .map_or((self.scene.width, self.scene.height), |s| {
                (s.width, s.height)
            })
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    pub view: Option<String>,
}

pub fn compile(src: &str) -> Result<Compiled, Error> {
    compile_with_fonts(src, &text::Fonts::default())
}

/// Compile with optional fallback font bytes used for measurement and output.
pub fn compile_with_fonts(src: &str, fonts: &text::Fonts) -> Result<Compiled, Error> {
    compile_with_options(src, &CompileOptions::default(), fonts)
}

/// Compile a selected view using the same fonts and semantics as rendering.
pub fn compile_with_options(
    src: &str,
    options: &CompileOptions,
    fonts: &text::Fonts,
) -> Result<Compiled, Error> {
    compile_document(src, options, fonts).map_err(|mut error| {
        if let Ok(tokens) = lexer::lex(src) {
            if error.line.is_none() {
                error.span = tokens
                    .iter()
                    .find(|t| !matches!(t.tok, lexer::Tok::Newline))
                    .map(|t| Box::new(t.span));
                error.line = error.span.as_ref().map(|s| s.line);
            }
            diagnostic::locate(&mut error, &tokens);
        }
        error
    })
}

fn compile_document(
    src: &str,
    options: &CompileOptions,
    fonts: &text::Fonts,
) -> Result<Compiled, Error> {
    compile_statements(&parser::parse(src)?, options, fonts)
}

pub(crate) fn compile_statements(
    statements: &[parser::Stmt],
    options: &CompileOptions,
    fonts: &text::Fonts,
) -> Result<Compiled, Error> {
    let mut resolved = views::resolve(statements, options.view.as_deref())?;
    let steps = presentation::extract(&mut resolved.statements)?;
    let slide = if let [parser::Stmt::Item(root)] = resolved.statements.as_mut_slice() {
        if root.head == "diagram" {
            slides::Slide::take(root)?
        } else {
            None
        }
    } else {
        None
    };
    let mut warnings = Vec::new();
    let sequence_document = if sequence::is_sequence(&resolved.statements) {
        Some(sequence::build(&resolved.statements)?)
    } else {
        None
    };
    let mut diagram = match &sequence_document {
        Some(document) => document.diagram.clone(),
        None => model::build_statements(&resolved.statements)?,
    };
    if diagram.mode == model::Mode::StateMachine {
        machine::check(&diagram, &mut warnings);
    }
    let (mut scene, sequence_info) = if let Some(document) = &sequence_document {
        let (scene, info) = sequence::layout(document, fonts, &mut warnings)?;
        (scene, Some(info))
    } else {
        (
            layout::layout_with_fonts(&diagram, &mut warnings, fonts),
            None,
        )
    };
    if sequence_info.is_none() {
        route::route_all(&diagram, &mut scene, &mut warnings);
    }
    diagram.width = scene.width;
    check::check(&scene, &mut warnings);
    let mut missing = std::collections::BTreeSet::new();
    let mut drawings: Vec<_> = scene
        .items
        .iter()
        .map(|placed| (&placed.item, placed.node.map(|i| scene.nodes[i].line)))
        .chain(
            scene
                .edges
                .iter()
                .filter_map(|edge| edge.chip.as_ref().map(|item| (item, Some(edge.line)))),
        )
        .collect();
    while let Some((item, line)) = drawings.pop() {
        if let layout::Item::Group(children) = item {
            drawings.extend(children.iter().map(|child| (child, line)));
        }
        if let layout::Item::Text(t) = item {
            for run in &t.runs {
                let font = if run.code || t.mono {
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
                            line,
                            msg: format!("character U+{:04X} `{c}` is outside the bundled fonts; viewer fallback metrics are estimated", c as u32),
                        });
                    }
                }
            }
        }
        if let layout::Item::Chip { text: label, .. } = item {
            for c in label.chars() {
                if !text::is_cjk(c)
                    && !text::supports_with_fonts(c, text::Font::Sans, fonts)
                    && missing.insert(c)
                {
                    warnings.push(Warning { line, msg: format!("character U+{:04X} `{c}` is outside the bundled fonts; viewer fallback metrics are estimated", c as u32) });
                }
            }
        }
    }
    let presentation = presentation::resolve(steps, &scene)?;
    let slide = slide.map(|s| s.plan(&scene, &mut warnings));
    Ok(Compiled {
        input: None,
        diagram,
        scene,
        warnings,
        slide,
        presentation,
        views: resolved.views,
        selected_view: resolved.selected_view,
        sequence: sequence_info,
    })
}
