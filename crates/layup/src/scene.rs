//! Versioned scene JSON for editors, alternate renderers, and presentation tools.
//!
//! Geometry remains in the scene's original SVG user units. A slide viewport
//! supplies a separate transform; consumers apply it exactly once. Font family
//! identifiers are exported, never font bytes or source-file contents.
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Compiled, Error,
    diagnostic::{Diagnostic, Severity, Span, quote},
    geometry::Outline,
    layout::{Anchor, Ink, Item, Rect},
    model::Mode,
};

pub const VERSION: u32 = 1;

/// Export a complete drawing plan. Non-finite geometry and invalid references
/// are errors rather than invalid JSON or silently substituted `null` values.
pub fn export(compiled: &Compiled) -> Result<String, Error> {
    let scene = &compiled.scene;
    let input_nodes: BTreeMap<_, _> = compiled
        .input
        .iter()
        .flat_map(|g| &g.nodes)
        .map(|n| (n.id.as_str(), n))
        .collect();
    let input_edges: BTreeMap<_, _> = compiled
        .input
        .iter()
        .flat_map(|g| &g.edges)
        .map(|e| (e.id.as_str(), e))
        .collect();
    let source_line = |line: usize| {
        if compiled.input.is_some() {
            "null".into()
        } else {
            line.to_string()
        }
    };
    let source_span = |value| {
        if compiled.input.is_some() {
            "null".into()
        } else {
            span(value)
        }
    };
    let ids: BTreeSet<_> = scene.nodes.iter().map(|n| n.id.as_str()).collect();
    if ids.len() != scene.nodes.len() {
        return Err(invalid_reference("scene contains duplicate node IDs"));
    }
    let mut nodes = Vec::new();
    for node in &scene.nodes {
        let parent = match node.parent {
            Some(index) => {
                let parent = scene
                    .nodes
                    .get(index)
                    .ok_or_else(|| invalid_reference("node parent index is outside the scene"))?;
                if parent.id == node.id {
                    return Err(invalid_reference("a scene node cannot be its own parent"));
                }
                quote(&parent.id)
            }
            None => "null".into(),
        };
        nodes.push(object([
            ("id", quote(&node.id)),
            ("kind", quote(&node.kind)),
            ("parentId", parent),
            ("rect", rect(node.rect)?),
            ("outline", outline(node.outline)?),
            ("tone", quote(node.tone.name())),
            ("href", optional(node.href.as_deref())),
            ("line", source_line(node.line)),
            ("span", source_span(node.span)),
            (
                "sourceLocations",
                input_nodes.get(node.id.as_str()).map_or("[]".into(), |n| {
                    serde_json::to_string(&n.source_locations)
                        .expect("source locations are serializable")
                }),
            ),
            (
                "metadata",
                input_nodes.get(node.id.as_str()).map_or("{}".into(), |n| {
                    serde_json::to_string(&n.metadata).expect("JSON metadata is serializable")
                }),
            ),
        ]));
    }
    let edge_ids: BTreeSet<_> = scene.edges.iter().map(|e| e.id.as_str()).collect();
    if edge_ids.len() != scene.edges.len() {
        return Err(invalid_reference("scene contains duplicate edge IDs"));
    }
    let mut edges = Vec::new();
    for edge in &scene.edges {
        if !ids.contains(edge.from.as_str()) || !ids.contains(edge.to.as_str()) {
            return Err(invalid_reference(
                "scene edge endpoints must refer to scene node IDs",
            ));
        }
        edges.push(object([
            ("id", quote(&edge.id)),
            ("from", quote(&edge.from)),
            ("to", quote(&edge.to)),
            ("kind", quote(&edge.kind)),
            (
                "points",
                array(
                    edge.points
                        .iter()
                        .map(|&(x, y)| point(x, y))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ),
            (
                "style",
                object([
                    ("tone", quote(edge.tone.name())),
                    ("dashed", edge.dashed.to_string()),
                    ("headStart", edge.head_start.to_string()),
                    ("headEnd", edge.head_end.to_string()),
                    ("bus", edge.bus.to_string()),
                    ("asynchronous", edge.asynchronous.to_string()),
                ]),
            ),
            (
                "chip",
                edge.chip.as_ref().map_or(Ok("null".into()), drawing)?,
            ),
            ("line", source_line(edge.line)),
            ("span", source_span(edge.span)),
            (
                "sourceLocations",
                input_edges.get(edge.id.as_str()).map_or("[]".into(), |e| {
                    serde_json::to_string(&e.source_locations)
                        .expect("source locations are serializable")
                }),
            ),
            (
                "metadata",
                input_edges.get(edge.id.as_str()).map_or("{}".into(), |e| {
                    serde_json::to_string(&e.metadata).expect("JSON metadata is serializable")
                }),
            ),
        ]));
    }
    let mut items = Vec::new();
    for placed in &scene.items {
        let node = match placed.node {
            Some(index) => quote(
                &scene
                    .nodes
                    .get(index)
                    .ok_or_else(|| invalid_reference("drawing node index is outside the scene"))?
                    .id,
            ),
            None => "null".into(),
        };
        items.push(object([
            ("nodeId", node),
            ("drawing", drawing(&placed.item)?),
        ]));
    }
    let slide = compiled
        .slide
        .as_ref()
        .map(|s| -> Result<_, Error> {
            Ok(object([
                ("scale", number(s.scale)?),
                ("offsetX", number(s.offset_x)?),
                ("offsetY", number(s.offset_y)?),
                ("padding", number(s.padding)?),
                ("contentWidth", number(s.content_width)?),
                ("contentHeight", number(s.content_height)?),
                ("minFontSize", number(s.min_font_size)?),
                (
                    "smallestFontSize",
                    s.smallest_font_size.map_or(Ok("null".into()), number)?,
                ),
            ]))
        })
        .transpose()?
        .unwrap_or_else(|| "null".into());
    let (width, height) = compiled.viewport();
    let views = compiled
        .views
        .iter()
        .map(|v| {
            object([
                ("id", quote(&v.id)),
                ("title", quote(&v.title)),
                ("span", source_span(v.span)),
            ])
        })
        .collect();
    let diagnostics = compiled
        .warnings
        .iter()
        .map(|warning| {
            let location = warning
                .line
                .and_then(|line| scene.nodes.iter().find(|n| n.line == line).map(|n| n.span))
                .or_else(|| {
                    warning.line.and_then(|line| {
                        scene.edges.iter().find(|e| e.line == line).map(|e| e.span)
                    })
                });
            Diagnostic {
                severity: Severity::Warning,
                code: "layout",
                message: warning.msg.clone(),
                span: location,
                line: warning.line,
                help: None,
                related: Vec::new(),
            }
            .json()
        })
        .collect();
    let sequence = compiled
        .sequence
        .as_ref()
        .map(|info| -> Result<_, Error> {
            if info
                .participants
                .iter()
                .any(|id| !ids.contains(id.as_str()))
                || info
                    .messages
                    .iter()
                    .any(|message| !edge_ids.contains(message.id.as_str()))
            {
                return Err(invalid_reference(
                    "sequence metadata targets must exist in the scene",
                ));
            }
            Ok(object([
                (
                    "participants",
                    array(info.participants.iter().map(|id| quote(id)).collect()),
                ),
                (
                    "messages",
                    array(
                        info.messages
                            .iter()
                            .map(|message| {
                                object([
                                    ("id", quote(&message.id)),
                                    ("asynchronous", message.asynchronous.to_string()),
                                    ("line", message.span.line.to_string()),
                                    ("span", span(message.span)),
                                ])
                            })
                            .collect(),
                    ),
                ),
                (
                    "annotations",
                    array(
                        info.annotations
                            .iter()
                            .map(|annotation| -> Result<_, Error> {
                                Ok(object([
                                    ("kind", quote(&annotation.kind)),
                                    ("label", quote(&annotation.label)),
                                    ("rect", rect(annotation.rect)?),
                                    ("line", annotation.span.line.to_string()),
                                    ("span", span(annotation.span)),
                                ]))
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    ),
                ),
            ]))
        })
        .transpose()?
        .unwrap_or_else(|| "null".into());
    Ok(object([
        ("version", VERSION.to_string()),
        ("units", quote("svg-user-units")),
        ("coordinateSystem", quote("scene")),
        ("title", quote(&compiled.diagram.title)),
        (
            "mode",
            quote(match compiled.diagram.mode {
                Mode::Graph => "graph",
                Mode::StateMachine => "state-machine",
                Mode::Sequence => "sequence",
            }),
        ),
        ("width", number(scene.width)?),
        ("height", number(scene.height)?),
        ("margin", number(scene.margin)?),
        ("contentLeft", number(scene.content_left)?),
        ("contentRight", number(scene.content_right)?),
        (
            "viewport",
            object([
                ("width", number(width)?),
                ("height", number(height)?),
                ("slide", slide),
            ]),
        ),
        ("selectedView", optional(compiled.selected_view.as_deref())),
        ("views", array(views)),
        ("nodes", array(nodes)),
        ("edges", array(edges)),
        ("items", array(items)),
        (
            "keepout",
            array(
                scene
                    .keepout
                    .iter()
                    .map(|r| rect(*r))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ),
        (
            "fonts",
            object([
                ("sans", quote("Layup Sans")),
                ("mono", quote("Layup Mono")),
                (
                    "bundledFallbacks",
                    array(vec![quote("Layup Arabic"), quote("Layup Hebrew")]),
                ),
                (
                    "fallbacks",
                    array(
                        scene
                            .fonts
                            .fallbacks
                            .iter()
                            .map(|bytes| quote(&crate::text::font_family(bytes)))
                            .collect(),
                    ),
                ),
                ("systemCjk", "true".into()),
            ]),
        ),
        ("presentation", compiled.presentation.json()),
        ("sequence", sequence),
        ("diagnostics", array(diagnostics)),
        (
            "provenance",
            compiled.input.as_ref().map_or("null".into(), |g| {
                serde_json::to_string(&g.provenance).expect("provenance is serializable")
            }),
        ),
    ]))
}

fn number(value: f64) -> Result<String, Error> {
    if !value.is_finite() {
        return Err(Error::new(
            "scene export requires finite geometry and drawing values",
        ));
    }
    // Rust's shortest round-trip representation retains the original f64,
    // unlike SVG's human-friendly coordinate rounding.
    Ok(value.to_string())
}

fn invalid_reference(message: &str) -> Error {
    Error::new(message)
}

fn object<const N: usize>(fields: [(&str, String); N]) -> String {
    format!(
        "{{{}}}",
        fields
            .into_iter()
            .map(|(key, value)| format!("{}:{value}", quote(key)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn array(values: Vec<String>) -> String {
    format!("[{}]", values.join(","))
}
fn optional(value: Option<&str>) -> String {
    value.map_or_else(|| "null".into(), quote)
}

fn span(value: Span) -> String {
    object([
        ("start", value.start.to_string()),
        ("end", value.end.to_string()),
        ("line", value.line.to_string()),
        ("column", value.column.to_string()),
        ("endLine", value.end_line.to_string()),
        ("endColumn", value.end_column.to_string()),
    ])
}

fn rect(value: Rect) -> Result<String, Error> {
    Ok(object([
        ("x", number(value.x)?),
        ("y", number(value.y)?),
        ("width", number(value.w)?),
        ("height", number(value.h)?),
    ]))
}

fn point(x: f64, y: f64) -> Result<String, Error> {
    Ok(object([("x", number(x)?), ("y", number(y)?)]))
}

fn outline(value: Outline) -> Result<String, Error> {
    Ok(match value {
        Outline::Rectangle(r) => object([("type", quote("rectangle")), ("rect", rect(r)?)]),
        Outline::Rounded { rect: r, radius } => object([
            ("type", quote("rounded")),
            ("rect", rect(r)?),
            ("radius", number(radius)?),
        ]),
        Outline::Diamond(r) => object([
            ("type", quote("diamond")),
            ("rect", rect(r)?),
            (
                "vertices",
                array(
                    value
                        .vertices()
                        .into_iter()
                        .map(|(x, y)| point(x, y))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ),
        ]),
    })
}

fn drawing(value: &Item) -> Result<String, Error> {
    Ok(match value {
        Item::Group(items) => object([
            ("type", quote("group")),
            (
                "items",
                array(items.iter().map(drawing).collect::<Result<Vec<_>, _>>()?),
            ),
        ]),
        Item::Box {
            rect: r,
            tone,
            hollow,
            white,
            rx,
            stroke_width,
        } => object([
            ("type", quote("box")),
            ("rect", rect(*r)?),
            ("tone", quote(tone.name())),
            ("hollow", hollow.to_string()),
            ("white", white.to_string()),
            ("radius", number(*rx)?),
            ("strokeWidth", number(*stroke_width)?),
        ]),
        Item::StateMarker {
            rect: r,
            tone,
            final_state,
        } => object([
            ("type", quote("state-marker")),
            ("rect", rect(*r)?),
            ("tone", quote(tone.name())),
            ("finalState", final_state.to_string()),
        ]),
        Item::Diamond {
            rect: r,
            tone,
            hollow,
            stroke_width,
        } => object([
            ("type", quote("diamond")),
            ("rect", rect(*r)?),
            ("tone", quote(tone.name())),
            ("hollow", hollow.to_string()),
            ("strokeWidth", number(*stroke_width)?),
        ]),
        Item::Strip { rect: r, rx } => object([
            ("type", quote("strip")),
            ("rect", rect(*r)?),
            ("radius", number(*rx)?),
        ]),
        Item::Rule {
            x1,
            y1,
            x2,
            y2,
            tone,
            dashed,
        } => object([
            ("type", quote("rule")),
            ("from", point(*x1, *y1)?),
            ("to", point(*x2, *y2)?),
            (
                "tone",
                tone.map_or_else(|| "null".into(), |t| quote(t.name())),
            ),
            ("dashed", dashed.to_string()),
        ]),
        Item::Text(text) => object([
            ("type", quote("text")),
            ("x", number(text.x)?),
            ("y", number(text.y)?),
            (
                "anchor",
                quote(match text.anchor {
                    Anchor::Start => "start",
                    Anchor::Middle => "middle",
                    Anchor::End => "end",
                }),
            ),
            (
                "direction",
                quote(match text.direction {
                    crate::text::Direction::Auto => "auto",
                    crate::text::Direction::Ltr => "ltr",
                    crate::text::Direction::Rtl => "rtl",
                }),
            ),
            (
                "runs",
                array(
                    text.runs
                        .iter()
                        .map(|run| {
                            object([
                                ("text", quote(&run.text)),
                                ("code", run.code.to_string()),
                                ("tag", run.tag.to_string()),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("size", number(text.size)?),
            ("weight", text.weight.to_string()),
            ("mono", text.mono.to_string()),
            (
                "ink",
                match text.ink {
                    Ink::Text => quote("text"),
                    Ink::Muted => quote("muted"),
                    Ink::Code => quote("code"),
                    Ink::Tone(tone) => object([("tone", quote(tone.name()))]),
                },
            ),
            ("letterSpacing", number(text.letter_spacing)?),
        ]),
        Item::Chip {
            rect: r,
            text,
            tone,
            rotate,
            bordered,
        } => object([
            ("type", quote("chip")),
            ("rect", rect(*r)?),
            ("text", quote(text)),
            ("tone", quote(tone.name())),
            ("rotate", rotate.to_string()),
            ("bordered", bordered.to_string()),
        ]),
        Item::Sample { x, y, tone, dashed } => object([
            ("type", quote("sample")),
            ("x", number(*x)?),
            ("y", number(*y)?),
            ("tone", quote(tone.name())),
            ("dashed", dashed.to_string()),
        ]),
    })
}
