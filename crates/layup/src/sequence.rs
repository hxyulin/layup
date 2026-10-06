//! Event-ordered sequence diagrams. Participant order is authored; time runs
//! downward. This layout shares measured text, drawing items and SVG emitters.
use crate::{
    Error, Warning,
    diagnostic::{Span, quote},
    geometry::Outline,
    layout::{Anchor, EdgePath, Ink, Item, NodeRect, Placed, Rect, Scene, TextItem},
    model::{Block, Diagram, Direction, Mode, Node},
    parser::{Arg, Stmt, Value},
    style::{Align, ArrowColor, Tone},
    text::{self, Font, Fonts, Run},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub struct SequenceInfo {
    pub participants: Vec<String>,
    pub messages: Vec<MessageInfo>,
    pub annotations: Vec<Annotation>,
}
#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub id: String,
    pub asynchronous: bool,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub struct Annotation {
    pub kind: String,
    pub label: String,
    pub rect: Rect,
    pub span: Span,
}
impl SequenceInfo {
    pub fn json(&self) -> String {
        let messages = self
            .messages
            .iter()
            .map(|m| {
                format!(
                    "{{\"id\":{},\"asynchronous\":{},\"line\":{}}}",
                    quote(&m.id),
                    m.asynchronous,
                    m.span.line
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let annotations = self.annotations.iter().map(|a| format!("{{\"kind\":{},\"label\":{},\"rect\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\"line\":{}}}",quote(&a.kind),quote(&a.label),a.rect.x,a.rect.y,a.rect.w,a.rect.h,a.span.line)).collect::<Vec<_>>().join(",");
        format!(
            "{{\"participants\":[{}],\"messages\":[{messages}],\"annotations\":[{annotations}]}}",
            self.participants
                .iter()
                .map(|s| quote(s))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}
enum Event {
    Message {
        index: usize,
        asynchronous: bool,
    },
    Note {
        label: String,
        from: Option<String>,
        to: Option<String>,
        span: Span,
    },
    Fragment {
        kind: String,
        label: String,
        children: Vec<Event>,
        span: Span,
    },
}
pub struct SequenceDocument {
    pub diagram: Diagram,
    events: Vec<Event>,
    reverse: bool,
    span: Span,
}

pub fn is_sequence(statements: &[Stmt]) -> bool {
    matches!(statements, [Stmt::Item(root)] if root.head=="diagram" && root.args.iter().any(|a|matches!(a,Arg::Attr(k,v) if k=="mode" && v.as_text()=="sequence")))
}

pub fn build(statements: &[Stmt]) -> Result<SequenceDocument, Error> {
    let [Stmt::Item(root)] = statements else {
        return Err(Error::new("sequence diagrams need a diagram wrapper"));
    };
    let mut generated = root.clone();
    let mut attrs = Vec::new();
    let mut spans = Vec::new();
    let mut reverse = false;
    for (arg, span) in generated.args.iter().zip(&generated.arg_spans) {
        match arg {
            Arg::Attr(k, _) if k == "mode" => {}
            Arg::Attr(k, v) if k == "layout" => {
                if !matches!(v.as_text().as_str(), "auto" | "manual") {
                    return Err(Error::located(
                        *span,
                        "sequence/layout",
                        "layout must be auto or manual; sequence events always keep authored order",
                    ));
                }
            }
            Arg::Attr(k, v) if k == "direction" => {
                reverse = match v.as_text().as_str() {
                    "right" | "LR" => false,
                    "left" | "RL" => true,
                    _ => {
                        return Err(Error::located(
                            *span,
                            "sequence/direction",
                            "sequence participant direction must be right or left; messages advance downward",
                        ));
                    }
                };
            }
            _ => {
                attrs.push(arg.clone());
                spans.push(*span);
            }
        }
    }
    generated.args = attrs;
    generated.arg_spans = spans;
    let mut body = crate::parser::parse("style participant process\nstyle actor process")?;
    let mut participant_kinds = BTreeMap::new();
    for statement in root.body.as_deref().unwrap_or(&[]) {
        if let Stmt::Item(item) = statement {
            match item.head.as_str() {
                "participant" | "actor" => {
                    if item.body.is_some() {
                        return Err(Error::located(
                            item.head_span,
                            "sequence/participant-body",
                            "participants have a title and attributes, without a nested body",
                        ));
                    }
                    let node = item.clone();
                    // Require authored IDs so messages and presentation steps
                    // remain meaningful when participant labels change.
                    item.args.iter().find_map(|a| match a {
                        Arg::Attr(k,v) if k=="id" => Some(v.as_text()),
                        Arg::Value(Value::Ident(s)) if Tone::parse(s).is_none() && !["auto","mono","sans","hollow","filled","left","center","solid","dashed","caps","inherit","top","bottom"].contains(&s.as_str()) => Some(s.clone()),
                        _ => None,
                    }).ok_or_else(||Error::located(item.head_span,"sequence/participant-id","participant needs an explicit id, for example `participant api \"API\"`"))?;
                    participant_kinds.insert(item.span.start, item.head.clone());
                    body.push(Stmt::Item(node));
                }
                "title" | "subtitle" | "desc" | "width" | "preset" | "style" | "arrow" => {
                    body.push(statement.clone())
                }
                _ => {}
            }
        }
    }
    let mut edges = Vec::new();
    let events = parse_events(root.body.as_deref().unwrap_or(&[]), &mut edges, true, false)?;
    body.extend(edges);
    generated.body = Some(body);
    let mut diagram = crate::model::build_statements(&[Stmt::Item(generated)])?;
    if participant_kinds.is_empty() {
        return Err(Error::located(
            root.head_span,
            "sequence/participants",
            "a sequence needs at least one participant",
        ));
    }
    for block in &mut diagram.blocks {
        if let Block::Node(node) = block {
            if node.id.is_empty() {
                return Err(Error::located(
                    node.span,
                    "sequence/participant-id",
                    "participant id cannot be empty",
                ));
            }
            if let Some(tag) = node.tag.take() {
                node.title = Some(format!(
                    "[{tag}] {}",
                    node.title.as_deref().unwrap_or(&node.id)
                ));
            }
            if !node.hints.is_empty() || node.gutter.is_some() {
                return Err(Error::located(
                    node.span,
                    "sequence/participant-layout",
                    "sequence participants use authored column order; graph placement hints and gutters do not apply",
                ));
            }
            node.kind = participant_kinds
                .get(&node.span.start)
                .ok_or_else(|| {
                    Error::located(
                        node.span,
                        "sequence/participant",
                        "participant source identity is missing",
                    )
                })?
                .clone();
            if node.kind == "actor" && node.role.is_none() {
                node.role = Some("actor".into());
            }
        }
    }
    diagram.mode = Mode::Sequence;
    diagram.direction = if reverse {
        Direction::Left
    } else {
        Direction::Right
    };
    diagram.auto_layout = false;
    let ids: BTreeSet<_> = diagram
        .blocks
        .iter()
        .filter_map(|block| {
            if let Block::Node(node) = block {
                Some(node.id.as_str())
            } else {
                None
            }
        })
        .collect();
    validate_notes(&events, &ids)?;
    Ok(SequenceDocument {
        diagram,
        events,
        reverse,
        span: root.span,
    })
}

fn parse_events(
    statements: &[Stmt],
    edges: &mut Vec<Stmt>,
    root: bool,
    branch: bool,
) -> Result<Vec<Event>, Error> {
    let mut out = Vec::new();
    for statement in statements {
        match statement {
            Stmt::Edge(edge) => {
                let mut edge = edge.clone();
                let mut asynchronous = false;
                let mut args = Vec::new();
                let mut spans = Vec::new();
                for (arg, span) in edge.args.iter().zip(&edge.arg_spans) {
                    match arg {
                        Arg::Value(Value::Ident(flag)) if flag == "async" => asynchronous = true,
                        Arg::Value(Value::Ident(flag)) if flag == "return" => {
                            args.push(Arg::Value(Value::Ident("dashed".into())));
                            spans.push(*span);
                        }
                        Arg::Attr(k, _) if matches!(k.as_str(), "via" | "from" | "to") => {
                            return Err(Error::located(
                                *span,
                                "sequence/ports",
                                "sequence messages follow participant lifelines; routing ports are only used in graph diagrams",
                            ));
                        }
                        _ => {
                            args.push(arg.clone());
                            spans.push(*span);
                        }
                    }
                }
                edge.args = args;
                edge.arg_spans = spans;
                out.push(Event::Message {
                    index: edges.len(),
                    asynchronous,
                });
                edges.push(Stmt::Edge(edge));
            }
            Stmt::Item(item) => match item.head.as_str() {
                "participant" | "actor" | "title" | "subtitle" | "desc" | "width" | "preset"
                | "style" | "arrow"
                    if root => {}
                "note" => {
                    if item.body.is_some() {
                        return Err(Error::located(
                            item.head_span,
                            "sequence/note-body",
                            "a sequence note has text and optional over= or from=/to= participant ids",
                        ));
                    }
                    let mut label = None;
                    let mut from = None;
                    let mut to = None;
                    for (arg, span) in item.args.iter().zip(&item.arg_spans) {
                        match arg {
                            Arg::Value(Value::Str(s)) if label.is_none() => label = Some(s.clone()),
                            Arg::Attr(k, v) if k == "over" && from.is_none() && to.is_none() => {
                                from = Some(v.as_text());
                                to = from.clone();
                            }
                            Arg::Attr(k, v) if k == "from" && from.is_none() => {
                                from = Some(v.as_text())
                            }
                            Arg::Attr(k, v) if k == "to" && to.is_none() => to = Some(v.as_text()),
                            _ => {
                                return Err(Error::located(
                                    *span,
                                    "sequence/note-argument",
                                    "note takes one quoted label, with over=id or from=id to=id",
                                ));
                            }
                        }
                    }
                    if from.is_some() != to.is_some() {
                        return Err(Error::located(
                            item.head_span,
                            "sequence/note-span",
                            "use both from= and to=, or a single over= participant",
                        ));
                    }
                    out.push(Event::Note {
                        label: label.ok_or_else(|| {
                            Error::located(
                                item.head_span,
                                "sequence/note-label",
                                "note needs quoted text",
                            )
                        })?,
                        from,
                        to,
                        span: item.span,
                    });
                }
                "loop" | "opt" | "alt" | "branch" => {
                    if (item.head == "branch") != branch {
                        return Err(Error::located(
                            item.head_span,
                            "sequence/branch",
                            "branches belong directly inside an alt block; alt bodies contain branch blocks",
                        ));
                    }
                    let label = match item.args.as_slice() {
                        [Arg::Value(Value::Str(s))] => s.clone(),
                        _ => {
                            return Err(Error::located(
                                item.head_span,
                                "sequence/fragment-label",
                                "sequence fragments need one quoted label",
                            ));
                        }
                    };
                    let body = item.body.as_ref().ok_or_else(|| {
                        Error::located(
                            item.head_span,
                            "sequence/fragment-body",
                            "sequence fragments need a body",
                        )
                    })?;
                    if item.head == "alt"
                        && (body.len() < 2
                            || body
                                .iter()
                                .any(|s| !matches!(s,Stmt::Item(i) if i.head=="branch")))
                    {
                        return Err(Error::located(
                            item.head_span,
                            "sequence/alternatives",
                            "alt needs at least two `branch \"Condition\" { ... }` blocks",
                        ));
                    }
                    let children = parse_events(body, edges, false, item.head == "alt")?;
                    if children.is_empty() {
                        return Err(Error::located(
                            item.head_span,
                            "sequence/empty-fragment",
                            "sequence fragments must contain an event",
                        ));
                    }
                    out.push(Event::Fragment {
                        kind: item.head.clone(),
                        label,
                        children,
                        span: item.span,
                    });
                }
                _ => {
                    return Err(Error::located(
                        item.head_span,
                        "sequence/statement",
                        format!(
                            "unknown sequence statement `{}`; use participant, actor, messages, note, loop, opt or alt",
                            item.head
                        ),
                    ));
                }
            },
        }
    }
    Ok(out)
}
fn validate_notes(events: &[Event], ids: &BTreeSet<&str>) -> Result<(), Error> {
    for event in events {
        match event {
            Event::Note { from, to, span, .. } => {
                for id in from.iter().chain(to) {
                    if !ids.contains(id.as_str()) {
                        return Err(Error::located(
                            *span,
                            "sequence/note-target",
                            format!("note refers to unknown participant `{id}`"),
                        ));
                    }
                }
            }
            Event::Fragment { children, .. } => validate_notes(children, ids)?,
            _ => {}
        }
    }
    Ok(())
}

pub fn layout(
    document: &SequenceDocument,
    fonts: &Fonts,
    warnings: &mut Vec<Warning>,
) -> Result<(Scene, SequenceInfo), Error> {
    let d = &document.diagram;
    let participants: Vec<&Node> = d
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Node(n) = b {
                Some(n.as_ref())
            } else {
                None
            }
        })
        .collect();
    let count = participants.len();
    let logical: BTreeMap<_, _> = participants
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut slot = 180.0f64;
    for node in &participants {
        slot = slot.max(
            text::width_with_fonts(
                node.title.as_deref().unwrap_or(&node.id),
                participant_font(node),
                14.0,
                0.0,
                fonts,
            ) + 64.0,
        );
    }
    for edge in &d.edges {
        if let Some(label) = label(d, edge) {
            let distance = logical[edge.from.as_str()]
                .abs_diff(logical[edge.to.as_str()])
                .max(1) as f64;
            slot = slot.max(
                (text::runs_width_with_fonts(&text::runs(&label), Font::Sans, 13.0, fonts) + 48.0)
                    / distance,
            );
        }
    }
    let width = if d.width_set {
        d.width
    } else {
        (slot * count as f64 + 80.0).max(900.0).ceil()
    };
    if width < 80.0 + count as f64 * 48.0 || !width.is_finite() {
        return Err(Error::new(format!(
            "sequence width must be at least {} for {count} participants",
            80 + count * 48
        )));
    }
    slot = (width - 80.0) / count as f64;
    let mut scene = Scene {
        fonts: fonts.clone(),
        width,
        height: 0.0,
        margin: 40.0,
        content_left: 40.0,
        content_right: width - 40.0,
        items: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        keepout: Vec::new(),
    };
    let mut y = 36.0;
    let title_lines = lines(&d.title, Font::SansBold, 22.0, width - 80.0, fonts);
    for line in title_lines {
        check_line(
            &line,
            Font::SansBold,
            22.0,
            width - 80.0,
            fonts,
            document.span.line,
            warnings,
        );
        scene.items.push(Placed {
            item: Item::Text(text_item(
                logical_start(40.0, width - 80.0, d.text_direction.resolve(&d.title)).0,
                y,
                logical_start(40.0, width - 80.0, d.text_direction.resolve(&d.title)).1,
                line,
                22.0,
                600,
                Ink::Text,
                d.text_direction.resolve(&d.title),
            )),
            node: None,
        });
        y += 28.0;
    }
    if let Some(note) = &d.note {
        for line in lines(note, Font::Sans, 13.0, width - 80.0, fonts) {
            check_line(
                &line,
                Font::Sans,
                13.0,
                width - 80.0,
                fonts,
                document.span.line,
                warnings,
            );
            scene.items.push(Placed {
                item: Item::Text(text_item(
                    logical_start(40.0, width - 80.0, d.text_direction.resolve(note)).0,
                    y,
                    logical_start(40.0, width - 80.0, d.text_direction.resolve(note)).1,
                    line,
                    13.0,
                    400,
                    Ink::Muted,
                    d.text_direction.resolve(note),
                )),
                node: None,
            });
            y += 19.0;
        }
    }
    y += 18.0;
    let header_y = y;
    let wrapped: Vec<_> = participants
        .iter()
        .map(|n| {
            lines(
                n.title.as_deref().unwrap_or(&n.id),
                participant_font(n),
                14.0,
                (slot - 48.0).max(1.0),
                fonts,
            )
        })
        .collect();
    let roles: Vec<_> = participants
        .iter()
        .map(|n| {
            n.role.as_deref().map_or_else(Vec::new, |role| {
                lines(role, Font::Sans, 11.5, (slot - 48.0).max(1.0), fonts)
            })
        })
        .collect();
    let header_h = wrapped
        .iter()
        .zip(&participants)
        .enumerate()
        .map(|(i, (lines, _))| 32.0 + lines.len() as f64 * 19.0 + roles[i].len() as f64 * 16.0)
        .fold(52.0, f64::max);
    let mut centers = BTreeMap::new();
    for (i, node) in participants.iter().enumerate() {
        let physical = if document.reverse { count - 1 - i } else { i };
        let center = 40.0 + (physical as f64 + 0.5) * slot;
        centers.insert(node.id.clone(), center);
        let rect = Rect {
            x: center - (slot - 24.0) / 2.0,
            y: header_y,
            w: slot - 24.0,
            h: header_h,
        };
        scene.nodes.push(NodeRect {
            id: node.id.clone(),
            kind: node.kind.clone(),
            rect,
            outline: Outline::Rounded { rect, radius: 8.0 },
            parent: None,
            tone: node.style.tone,
            href: node.href.clone(),
            line: node.line,
            span: node.span,
        });
        scene.items.push(Placed {
            item: Item::Box {
                rect,
                tone: node.style.tone,
                hollow: node.style.hollow,
                white: false,
                rx: 8.0,
                stroke_width: 1.2,
            },
            node: Some(i),
        });
        let direction = node
            .text_direction
            .unwrap_or(d.text_direction)
            .resolve(node.title.as_deref().unwrap_or(&node.id));
        let mut baseline = header_y + 23.0;
        for line in &wrapped[i] {
            check_line(
                line,
                participant_font(node),
                14.0,
                rect.w - 24.0,
                fonts,
                node.line,
                warnings,
            );
            let (x, anchor) = match node.style.align {
                Align::Center => (center, Anchor::Middle),
                Align::Left => (rect.x + 12.0, Anchor::Start),
                Align::Right => (rect.right() - 12.0, Anchor::End),
                Align::End => {
                    if direction.resolve(&node.title.clone().unwrap_or_default())
                        == text::Direction::Rtl
                    {
                        (rect.x + 12.0, Anchor::Start)
                    } else {
                        (rect.right() - 12.0, Anchor::End)
                    }
                }
                Align::Start => logical_start(rect.x + 12.0, rect.w - 24.0, direction),
            };
            let mut text = text_item(
                x,
                baseline,
                anchor,
                line.clone(),
                14.0,
                if node.style.mono { 400 } else { 600 },
                Ink::Text,
                direction,
            );
            text.mono = node.style.mono;
            scene.items.push(Placed {
                item: Item::Text(text),
                node: Some(i),
            });
            baseline += 19.0;
        }
        if let Some(role) = &node.role {
            for row in &roles[i] {
                check_line(
                    row,
                    Font::Sans,
                    11.5,
                    rect.w - 24.0,
                    fonts,
                    node.line,
                    warnings,
                );
                scene.items.push(Placed {
                    item: Item::Text(text_item(
                        center,
                        baseline,
                        Anchor::Middle,
                        row.clone(),
                        11.5,
                        400,
                        Ink::Muted,
                        node.text_direction
                            .unwrap_or(d.text_direction)
                            .resolve(role),
                    )),
                    node: Some(i),
                });
                baseline += 16.0;
            }
        }
    }
    let mut info = SequenceInfo {
        participants: participants.iter().map(|n| n.id.clone()).collect(),
        ..Default::default()
    };
    y = header_y + header_h + 34.0;
    place_events(
        &document.events,
        d,
        &centers,
        slot,
        &mut y,
        &mut scene,
        &mut info,
        warnings,
        0,
    )?;
    let bottom = y + 22.0;
    // Keep each participant's header and lifeline in one SVG semantic group.
    let mut items = Vec::new();
    for item in std::mem::take(&mut scene.items) {
        if let Some(index) = item.node
            && !items.iter().any(|p: &Placed| p.node == Some(index))
        {
            items.push(Placed {
                item: Item::Rule {
                    x1: scene.nodes[index].rect.cx(),
                    y1: header_y + header_h,
                    x2: scene.nodes[index].rect.cx(),
                    y2: bottom,
                    tone: Some(scene.nodes[index].tone),
                    dashed: true,
                },
                node: Some(index),
            });
        }
        items.push(item);
    }
    scene.items = items;
    scene.height = bottom + 32.0;
    Ok((scene, info))
}

#[allow(clippy::too_many_arguments)]
fn place_events(
    events: &[Event],
    d: &Diagram,
    centers: &BTreeMap<String, f64>,
    slot: f64,
    y: &mut f64,
    scene: &mut Scene,
    info: &mut SequenceInfo,
    warnings: &mut Vec<Warning>,
    depth: usize,
) -> Result<(), Error> {
    for event in events {
        match event {
            Event::Message {
                index,
                asynchronous,
            } => {
                let edge = &d.edges[*index];
                let x1 = centers[&edge.from];
                let x2 = centers[&edge.to];
                let self_call = edge.from == edge.to;
                let available = if self_call {
                    (slot * 0.65 - 20.0).max(20.0)
                } else {
                    ((x2 - x1).abs() - 30.0).max(20.0)
                };
                let label = label(d, edge);
                let tone = match (edge.tone, d.arrows[&edge.kind].color) {
                    (Some(t), _) | (None, ArrowColor::Tone(t)) => t,
                    (None, ArrowColor::Inherit) => scene.nodes[scene.node(&edge.to).unwrap()].tone,
                };
                let chip = label.map(|label| {
                    let rows = lines(&label, Font::Sans, 13.0, available, &scene.fonts);
                    let label_width = rows
                        .iter()
                        .map(|r| text::runs_width_with_fonts(r, Font::Sans, 13.0, &scene.fonts))
                        .fold(0.0, f64::max);
                    let center = if self_call {
                        x1 + slot * 0.28
                    } else {
                        (x1 + x2) / 2.0
                    };
                    let rect = Rect {
                        x: center - label_width / 2.0 - 7.0,
                        y: *y - 15.0,
                        w: label_width + 14.0,
                        h: rows.len() as f64 * 18.0 + 3.0,
                    };
                    let mut drawing = vec![Item::Box {
                        rect,
                        tone: Tone::Gray,
                        hollow: false,
                        white: true,
                        rx: 3.0,
                        stroke_width: 0.0,
                    }];
                    for (i, row) in rows.iter().enumerate() {
                        check_line(
                            row,
                            Font::Sans,
                            13.0,
                            available,
                            &scene.fonts,
                            edge.line,
                            warnings,
                        );
                        drawing.push(Item::Text(text_item(
                            center,
                            *y + i as f64 * 18.0,
                            Anchor::Middle,
                            row.clone(),
                            13.0,
                            400,
                            Ink::Tone(tone),
                            d.text_direction.resolve(&label),
                        )));
                    }
                    *y += rows.len() as f64 * 18.0;
                    Item::Group(drawing)
                });
                let points = if self_call {
                    vec![
                        (x1, *y),
                        (x1 + slot * 0.65, *y),
                        (x1 + slot * 0.65, *y + 28.0),
                        (x1, *y + 28.0),
                    ]
                } else {
                    vec![(x1, *y), (x2, *y)]
                };
                if self_call && x1 + slot * 0.65 > scene.width - 10.0 {
                    scene.width = x1 + slot * 0.65 + 40.0;
                    scene.content_right = scene.width - 40.0;
                }
                scene.edges.push(EdgePath {
                    id: edge.id.clone(),
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    kind: edge.kind.clone(),
                    points,
                    tone,
                    dashed: edge.dashed.unwrap_or(d.arrows[&edge.kind].dashed),
                    head_end: edge.head_at_end,
                    head_start: edge.head_at_start,
                    chip,
                    bus: false,
                    line: edge.line,
                    span: edge.span,
                    asynchronous: *asynchronous,
                });
                info.messages.push(MessageInfo {
                    id: edge.id.clone(),
                    asynchronous: *asynchronous,
                    span: edge.span,
                });
                *y += if self_call { 66.0 } else { 42.0 };
            }
            Event::Note {
                label,
                from,
                to,
                span,
            } => {
                let (left, right) = match (from, to) {
                    (Some(a), Some(b)) => (
                        centers[a].min(centers[b]) - slot * 0.38,
                        centers[a].max(centers[b]) + slot * 0.38,
                    ),
                    _ => (48.0, scene.width - 48.0),
                };
                let rows = lines(
                    label,
                    Font::Sans,
                    13.0,
                    (right - left - 24.0).max(1.0),
                    &scene.fonts,
                );
                let rect = Rect {
                    x: left,
                    y: *y - 10.0,
                    w: right - left,
                    h: rows.len() as f64 * 19.0 + 20.0,
                };
                let mut drawing = vec![Item::Box {
                    rect,
                    tone: Tone::Yellow,
                    hollow: false,
                    white: false,
                    rx: 4.0,
                    stroke_width: 1.0,
                }];
                for (i, row) in rows.iter().enumerate() {
                    check_line(
                        row,
                        Font::Sans,
                        13.0,
                        rect.w - 24.0,
                        &scene.fonts,
                        span.line,
                        warnings,
                    );
                    drawing.push(Item::Text(text_item(
                        rect.cx(),
                        *y + 10.0 + i as f64 * 19.0,
                        Anchor::Middle,
                        row.clone(),
                        13.0,
                        400,
                        Ink::Text,
                        d.text_direction.resolve(label),
                    )));
                }
                scene.items.push(Placed {
                    item: Item::Group(drawing),
                    node: None,
                });
                info.annotations.push(Annotation {
                    kind: "note".into(),
                    label: label.clone(),
                    rect,
                    span: *span,
                });
                *y += rect.h + 24.0;
            }
            Event::Fragment {
                kind,
                label,
                children,
                span,
            } => {
                let start = *y - 10.0;
                let inset = 48.0 + depth as f64 * 12.0;
                if scene.width - 2.0 * inset <= 40.0 {
                    return Err(Error::located(
                        *span,
                        "sequence/fragment-width",
                        "nested sequence fragments need more horizontal space; increase width or reduce nesting",
                    ));
                }
                let available = scene.width - 2.0 * inset - 20.0;
                let heading = format!("{kind}: {label}");
                let direction = d.text_direction.resolve(&heading);
                let (heading_x, anchor) = logical_start(inset + 10.0, available, direction);
                for row in lines(&heading, Font::SansBold, 12.0, available, &scene.fonts) {
                    check_line(
                        &row,
                        Font::SansBold,
                        12.0,
                        available,
                        &scene.fonts,
                        span.line,
                        warnings,
                    );
                    let label_width =
                        text::runs_width_with_fonts(&row, Font::SansBold, 12.0, &scene.fonts);
                    scene.items.push(Placed {
                        item: Item::Box {
                            rect: Rect {
                                x: if anchor == Anchor::End {
                                    heading_x - label_width - 4.0
                                } else {
                                    heading_x - 4.0
                                },
                                y: *y - 7.0,
                                w: label_width + 8.0,
                                h: 20.0,
                            },
                            tone: Tone::Gray,
                            hollow: false,
                            white: true,
                            rx: 2.0,
                            stroke_width: 0.0,
                        },
                        node: None,
                    });
                    scene.items.push(Placed {
                        item: Item::Text(text_item(
                            heading_x,
                            *y + 7.0,
                            anchor,
                            row,
                            12.0,
                            600,
                            Ink::Muted,
                            direction,
                        )),
                        node: None,
                    });
                    *y += 18.0;
                }
                *y += 20.0;
                place_events(
                    children,
                    d,
                    centers,
                    slot,
                    y,
                    scene,
                    info,
                    warnings,
                    depth + 1,
                )?;
                let rect = Rect {
                    x: inset,
                    y: start,
                    w: scene.width - 2.0 * inset,
                    h: *y - start - 10.0,
                };
                scene.items.push(Placed {
                    item: Item::Box {
                        rect,
                        tone: Tone::Gray,
                        hollow: true,
                        white: false,
                        rx: 3.0,
                        stroke_width: 1.0,
                    },
                    node: None,
                });
                info.annotations.push(Annotation {
                    kind: kind.clone(),
                    label: label.clone(),
                    rect,
                    span: *span,
                });
                *y += 18.0;
            }
        }
    }
    Ok(())
}
fn label(d: &Diagram, e: &crate::model::Edge) -> Option<String> {
    e.label.clone().or_else(|| {
        if e.labeled {
            d.arrows[&e.kind].label.clone()
        } else {
            None
        }
    })
}
fn lines(label: &str, font: Font, size: f64, width: f64, fonts: &Fonts) -> Vec<Vec<Run>> {
    text::wrap_with_fonts(label, font, size, width, fonts)
}
fn participant_font(node: &Node) -> Font {
    if node.style.mono {
        Font::Mono
    } else {
        Font::SansBold
    }
}
fn logical_start(x: f64, width: f64, direction: text::Direction) -> (f64, Anchor) {
    if direction == text::Direction::Rtl {
        (x + width, Anchor::End)
    } else {
        (x, Anchor::Start)
    }
}
#[allow(clippy::too_many_arguments)]
fn text_item(
    x: f64,
    y: f64,
    anchor: Anchor,
    runs: Vec<Run>,
    size: f64,
    weight: u16,
    ink: Ink,
    direction: text::Direction,
) -> TextItem {
    TextItem {
        x,
        y,
        anchor,
        direction,
        runs,
        size,
        weight,
        mono: false,
        ink,
        letter_spacing: 0.0,
    }
}
fn check_line(
    runs: &[Run],
    font: Font,
    size: f64,
    available: f64,
    fonts: &Fonts,
    line: usize,
    warnings: &mut Vec<Warning>,
) {
    if text::runs_width_with_fonts(runs, font, size, fonts) > available + 0.5 {
        warnings.push(Warning {
            line: Some(line),
            msg: "sequence text exceeds its available width; increase width or shorten the label"
                .into(),
        });
    }
}
