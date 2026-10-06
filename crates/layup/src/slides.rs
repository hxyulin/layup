//! Presentation viewports fit the completed scene without changing its layout.
//!
//! All dimensions are SVG user units. A minimum font size refers to the final
//! slide coordinate system, before a browser or presentation app resizes it.

use crate::{
    Error, Warning,
    layout::{Item as DrawItem, Scene},
    parser::{Arg, Item, Value},
};

/// An authored presentation viewport, independent of the diagram's layout width.
#[derive(Debug, Clone, PartialEq)]
pub struct Slide {
    pub width: f64,
    pub height: f64,
    pub padding: f64,
    pub min_font_size: f64,
    /// Source line of the slide attribute for readability diagnostics.
    pub line: usize,
}

/// The uniform transform from scene coordinates into a presentation viewport.
#[derive(Debug, Clone, PartialEq)]
pub struct SlidePlan {
    pub width: f64,
    pub height: f64,
    pub padding: f64,
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    /// Final routed scene dimensions before scaling.
    pub content_width: f64,
    pub content_height: f64,
    pub min_font_size: f64,
    /// Smallest nonempty rendered text item, after scaling.
    pub smallest_font_size: Option<f64>,
}

impl Slide {
    /// Extract slide attributes from an AST root before semantic compilation.
    /// Argument source spans stay aligned. An error leaves the item unchanged.
    pub fn take(root: &mut Item) -> Result<Option<Self>, Error> {
        let get = |name: &str| {
            root.args.iter().enumerate().find_map(|(i, arg)| {
                if let Arg::Attr(key, value) = arg
                    && key == name
                {
                    Some((value, root.arg_spans.get(i).copied().unwrap_or(root.span)))
                } else {
                    None
                }
            })
        };
        let slide = get("slide");
        let padding = get("slide-padding");
        let minimum = get("min-font-size");
        let Some((value, span)) = slide else {
            if let Some((_, span)) = padding.or(minimum) {
                return Err(Error::located(
                    span,
                    "semantic/slide",
                    "slide-padding and min-font-size require a slide viewport",
                )
                .with_help("add slide=wide, slide=standard, or slide=\"1920:1080\""));
            }
            return Ok(None);
        };
        let dimensions = match value.as_text().as_str() {
            "wide" | "16:9" => Some((1920.0, 1080.0)),
            "standard" | "4:3" => Some((1440.0, 1080.0)),
            dimensions => dimensions
                .split_once(':')
                .and_then(|(w, h)| Some((w.parse::<f64>().ok()?, h.parse::<f64>().ok()?))),
        };
        let (width, height) = dimensions
            .filter(|&(w, h)| w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0)
            .ok_or_else(|| {
                Error::located(
                    span,
                    "semantic/slide",
                    "slide must be wide, standard, or two positive finite dimensions",
                )
                .with_help("use slide=wide, slide=standard, or slide=\"1920:1080\"")
            })?;
        let number = |attribute: Option<(&Value, crate::diagnostic::Span)>,
                      default: f64,
                      name: &str,
                      positive: bool|
         -> Result<f64, Error> {
            let Some((value, span)) = attribute else {
                return Ok(default);
            };
            match value {
                Value::Num(n) if n.is_finite() && if positive { *n > 0.0 } else { *n >= 0.0 } => {
                    Ok(*n)
                }
                _ => Err(Error::located(
                    span,
                    "semantic/slide",
                    format!(
                        "{name} must be a {} finite number",
                        if positive { "positive" } else { "nonnegative" }
                    ),
                )),
            }
        };
        let padding_value = number(padding, 48.0, "slide-padding", false)?;
        let min_font_size = number(minimum, 18.0, "min-font-size", true)?;
        if padding_value >= width.min(height) / 2.0 {
            return Err(Error::located(
                padding.map_or(span, |(_, span)| span),
                "semantic/slide",
                "slide-padding must leave positive space inside the viewport",
            )
            .with_help("reduce slide-padding to less than half the smaller slide dimension"));
        }
        let result = Self {
            width,
            height,
            padding: padding_value,
            min_font_size,
            line: span.line,
        };
        let args = std::mem::take(&mut root.args);
        let spans = std::mem::take(&mut root.arg_spans);
        for (i, arg) in args.into_iter().enumerate() {
            if matches!(&arg, Arg::Attr(k, _) if matches!(k.as_str(), "slide" | "slide-padding" | "min-font-size"))
            {
                continue;
            }
            root.args.push(arg);
            root.arg_spans
                .push(spans.get(i).copied().unwrap_or(root.span));
        }
        Ok(Some(result))
    }

    /// Fit the complete, routed scene and warn once if text becomes too small.
    /// Scene dimensions include authored title, legend and routing margins.
    pub fn plan(&self, scene: &Scene, warnings: &mut Vec<Warning>) -> SlidePlan {
        let scale = ((self.width - 2.0 * self.padding) / scene.width)
            .min((self.height - 2.0 * self.padding) / scene.height);
        let offset_x = (self.width - scene.width * scale) / 2.0;
        let offset_y = (self.height - scene.height * scale) / 2.0;
        let mut smallest: Option<f64> = None;
        let mut below = 0usize;
        for item in scene
            .items
            .iter()
            .map(|placed| &placed.item)
            .chain(scene.edges.iter().filter_map(|edge| edge.chip.as_ref()))
        {
            measure_fonts(item, scale, self.min_font_size, &mut smallest, &mut below);
        }
        if below > 0 {
            warnings.push(Warning {
                line: Some(self.line),
                msg: format!(
                    "slide readability: {below} text items are below the minimum font size {:.1} after fitting (smallest {:.1}); split the diagram into views or use a larger slide viewport",
                    self.min_font_size,
                    smallest.unwrap_or(0.0),
                ),
            });
        }
        SlidePlan {
            width: self.width,
            height: self.height,
            padding: self.padding,
            scale,
            offset_x,
            offset_y,
            content_width: scene.width,
            content_height: scene.height,
            min_font_size: self.min_font_size,
            smallest_font_size: smallest,
        }
    }
}

fn measure_fonts(
    item: &DrawItem,
    scale: f64,
    minimum: f64,
    smallest: &mut Option<f64>,
    below: &mut usize,
) {
    let size = match item {
        DrawItem::Group(children)
        | DrawItem::StyledGroup {
            items: children, ..
        } => {
            for child in children {
                measure_fonts(child, scale, minimum, smallest, below);
            }
            None
        }
        DrawItem::Text(text) if text.runs.iter().any(|r| !r.text.trim().is_empty()) => {
            Some(text.size)
        }
        DrawItem::Chip { text, bordered, .. } if !text.trim().is_empty() => {
            Some(if *bordered { 11.5 } else { 12.0 })
        }
        _ => None,
    };
    if let Some(size) = size {
        let size = size * scale;
        *smallest = Some(smallest.map_or(size, |previous| previous.min(size)));
        if size + 1e-9 < minimum {
            *below += 1;
        }
    }
}

#[cfg(test)]
#[path = "slides/tests.rs"]
mod tests;
