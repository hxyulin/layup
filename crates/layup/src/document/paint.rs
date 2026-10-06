//! Validated, theme-aware paint channels. Arbitrary CSS never enters output.
use super::{Attributes, Value, compile::fail};
use crate::{Error, diagnostic::Span};
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Paint {
    pub palette_origin: Option<PaletteOrigin>,
    pub fill_color: Option<Color>,
    pub stroke_color: Option<Color>,
    pub text_color: Option<Color>,
    pub background_color: Option<Color>,
    pub stroke_style: Option<StrokeStyle>,
    pub stroke_width: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum Color {
    Literal(String),
    Token(String),
    Themed { light: Box<Color>, dark: Box<Color> },
    None,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PaletteOrigin {
    Source,
    Target,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StrokeStyle {
    Solid,
    Dashed,
    Dotted,
}

pub(super) fn is_paint(key: &str) -> bool {
    [
        "fill-color",
        "stroke-color",
        "text-color",
        "background-color",
        "stroke-width",
        "stroke-style",
    ]
    .contains(&key)
}
fn color(value: &Value, span: Span, none: bool) -> Result<Option<Color>, Error> {
    match value {
        Value::Choice(s) | Value::String(s) if s == "auto" => Ok(None),
        Value::Choice(s) | Value::String(s) if s == "none" && none => Ok(Some(Color::None)),
        Value::Choice(s) | Value::String(s) => {
            if ["background", "text", "muted", "code", "frame"].contains(&s.as_str()) {
                return Ok(Some(Color::Token(s.clone())));
            }
            let hex = s.strip_prefix('#').is_some_and(|v| {
                [3, 4, 6, 8].contains(&v.len()) && v.bytes().all(|b| b.is_ascii_hexdigit())
            });
            if hex
                || [
                    "black",
                    "white",
                    "red",
                    "green",
                    "blue",
                    "yellow",
                    "orange",
                    "purple",
                    "gray",
                    "grey",
                    "transparent",
                    "navy",
                    "teal",
                    "aqua",
                    "lime",
                    "maroon",
                    "olive",
                    "silver",
                    "fuchsia",
                ]
                .contains(&s.as_str())
            {
                Ok(Some(Color::Literal(s.clone())))
            } else {
                Err(fail(
                    span,
                    "document/color",
                    "color must be a named color, hex literal, theme token, auto, or a {light, dark} pair",
                ))
            }
        }
        Value::Record(values)
            if values.len() == 2 && values.contains_key("light") && values.contains_key("dark") =>
        {
            let channel = |v| {
                color(v, span, none)?.ok_or_else(|| {
                    fail(
                        span,
                        "document/color",
                        "auto applies to the whole channel, not one theme",
                    )
                })
            };
            Ok(Some(Color::Themed {
                light: Box::new(channel(&values["light"])?),
                dark: Box::new(channel(&values["dark"])?),
            }))
        }
        _ => Err(fail(span, "document/color", "invalid paint channel")),
    }
}
impl Paint {
    pub(super) fn diagram(attrs: &Attributes) -> Result<Self, Error> {
        let mut paint = Self::default();
        if let Some(attr) = attrs.get("background-color") {
            paint.background_color = color(&attr.value, attr.span, true)?;
        }
        Ok(paint)
    }
    pub(super) fn parse(attrs: &Attributes, edge: bool) -> Result<Self, Error> {
        let mut paint = Self::default();
        for (key, attr) in attrs {
            match key.as_str() {
                "fill-color" if !edge => paint.fill_color = color(&attr.value, attr.span, true)?,
                "stroke-color" => paint.stroke_color = color(&attr.value, attr.span, true)?,
                "text-color" => paint.text_color = color(&attr.value, attr.span, false)?,
                "background-color" => {
                    return Err(fail(
                        attr.span,
                        "document/color",
                        "background-color belongs to the diagram canvas",
                    ));
                }
                "fill-color" => {
                    return Err(fail(
                        attr.span,
                        "document/color",
                        "connections accept stroke-color and text-color",
                    ));
                }
                "stroke-style" => {
                    paint.stroke_style = Some(match &attr.value {
                        Value::Choice(s) | Value::String(s) => match s.as_str() {
                            "solid" => StrokeStyle::Solid,
                            "dashed" => StrokeStyle::Dashed,
                            "dotted" => StrokeStyle::Dotted,
                            _ => {
                                return Err(fail(
                                    attr.span,
                                    "document/stroke",
                                    "stroke-style must be solid, dashed or dotted",
                                ));
                            }
                        },
                        _ => {
                            return Err(fail(
                                attr.span,
                                "document/stroke",
                                "stroke-style requires a choice",
                            ));
                        }
                    })
                }
                "stroke-width" => {
                    let number = match &attr.value {
                        Value::Integer(n) => n.as_f64(),
                        Value::Float(n) => *n,
                        _ => {
                            return Err(fail(
                                attr.span,
                                "document/stroke",
                                "stroke-width requires a number",
                            ));
                        }
                    };
                    if number < 0. {
                        return Err(fail(
                            attr.span,
                            "document/stroke",
                            "stroke-width cannot be negative",
                        ));
                    }
                    paint.stroke_width = Some(number);
                }
                "palette" if edge => {
                    if let Value::Choice(s) | Value::String(s) = &attr.value {
                        paint.palette_origin = match s.as_str() {
                            "source" => Some(PaletteOrigin::Source),
                            "target" | "auto" => Some(PaletteOrigin::Target),
                            _ => None,
                        };
                    }
                }
                _ => {}
            }
        }
        Ok(paint)
    }
}
impl Color {
    pub(crate) fn css(&self, dark: bool) -> String {
        match self {
            Self::Literal(s) => s.clone(),
            Self::Token(s) => format!(
                "var(--{})",
                match s.as_str() {
                    "background" => "bg",
                    "text" => "ink",
                    "code" => "codeink",
                    other => other,
                }
            ),
            Self::Themed { light, dark: night } => if dark { night } else { light }.css(dark),
            Self::None => "none".into(),
        }
    }
}
