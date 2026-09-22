//! Turns the generic item tree into a typed diagram: nodes with resolved
//! styles, rows with weights, sections, edges with resolved kinds.

use std::collections::{BTreeMap, BTreeSet};

use crate::Error;
use crate::parser::{Arg, EdgeStmt, Item, Stmt, Value, parse};
use crate::style::{
    AUTO_CYCLE, Align, ArrowColor, ArrowStyle, NodeStyle, Shape, Tone, arrow_presets, presets,
};

#[derive(Debug, Clone)]
pub struct Diagram {
    pub title: String,
    pub note: Option<String>,
    pub desc: Option<String>,
    pub width: f64,
    /// True when the author wrote `width=`; otherwise the model picks one.
    pub width_set: bool,
    pub preset: Preset,
    pub blocks: Vec<Block>,
    pub edges: Vec<Edge>,
    pub kinds: BTreeMap<String, NodeStyle>,
    pub arrows: BTreeMap<String, ArrowStyle>,
    pub legend: Option<Legend>,
}

/// Diagram-wide defaults. `clean` (the default) fills in tones, edge
/// colors, legend, width and gutters; `manual` selects explicit-only
/// behavior for hand-tuned diagrams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Preset {
    #[default]
    Clean,
    Manual,
}

impl Preset {
    fn parse(s: &str) -> Option<Preset> {
        Some(match s {
            "clean" | "auto" => Preset::Clean,
            "explicit" | "manual" => Preset::Manual,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Legend {
    /// Node kinds to show; empty means every labeled kind that is used.
    pub nodes: Vec<String>,
    /// Arrow kinds to show; empty means every kind that is used.
    pub arrows: Vec<String>,
    pub bottom: bool,
}

#[derive(Debug, Clone)]
pub enum Block {
    Node(Node),
    Row(Row),
    Section(Section),
    Text(String),
    Divider { text: Option<String>, tone: Tone },
    Gap(f64),
}

#[derive(Debug, Clone)]
pub struct Section {
    pub label: String,
    /// `band`: uppercase letter-spaced label, no rule. `section`: title with a rule above.
    pub band: bool,
    pub children: Vec<Block>,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub weights: Vec<f64>,
    /// `None` is an empty cell.
    pub cells: Vec<Option<Block>>,
    pub gutter: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum Line {
    Code(String),
    Sub(String),
    Text(String),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub style: NodeStyle,
    pub title: Option<String>,
    /// Bracketed prefix such as `re-exported` in `"[re-exported] CAN frame types"`.
    pub tag: Option<String>,
    pub role: Option<String>,
    pub lines: Vec<Line>,
    pub children: Vec<Block>,
    pub gutter: Option<f64>,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    fn parse(s: &str) -> Option<Side> {
        Some(match s {
            "top" => Side::Top,
            "bottom" => Side::Bottom,
            "left" => Side::Left,
            "right" => Side::Right,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
    pub head_at_end: bool,
    pub head_at_start: bool,
    pub via: Option<Side>,
    pub from_side: Option<Side>,
    pub to_side: Option<Side>,
    pub dashed: Option<bool>,
    pub tone: Option<Tone>,
    /// Show the kind's default chip text (`impl`, `exports`, ...).
    pub labeled: bool,
    /// Share a port and channel with other `bus` edges instead of spreading.
    pub bus: bool,
    pub line: usize,
}

const FLAG_WORDS: &[&str] = &[
    "hollow", "filled", "center", "left", "mono", "sans", "dashed", "solid", "inherit", "top",
    "bottom", "auto", "caps",
];

pub fn build(src: &str) -> Result<Diagram, Error> {
    let stmts = parse(src)?;
    let mut b = Builder {
        kinds: presets(),
        arrows: arrow_presets(),
        ids: BTreeSet::new(),
        reserved: BTreeSet::new(),
        edges: Vec::new(),
        legend: None,
        legend_off: false,
    };
    let (root, body) = match stmts.as_slice() {
        [Stmt::Item(item)] if item.head == "diagram" => {
            (Some(item), item.body.clone().unwrap_or_default())
        }
        _ => (None, stmts.clone()),
    };
    reserve_explicit_ids(&body, &mut b.reserved);
    let mut d = Diagram {
        title: String::new(),
        note: None,
        desc: None,
        width: 900.0,
        width_set: false,
        preset: Preset::Clean,
        blocks: Vec::new(),
        edges: Vec::new(),
        kinds: BTreeMap::new(),
        arrows: BTreeMap::new(),
        legend: None,
    };
    if let Some(root) = root {
        for a in &root.args {
            match a {
                Arg::Value(Value::Str(s)) => d.title = s.clone(),
                Arg::Attr(k, v) if k == "width" => {
                    d.width = num(v, root.line)?;
                    d.width_set = true;
                }
                Arg::Attr(k, v) if k == "title" => d.title = v.as_text(),
                Arg::Attr(k, v) if k == "preset" => {
                    d.preset = Preset::parse(&v.as_text()).ok_or_else(|| {
                        Error::at(
                            root.line,
                            format!("unknown preset `{}`; use clean or manual", v.as_text()),
                        )
                    })?;
                }
                other => {
                    return Err(Error::at(
                        root.line,
                        format!("unexpected argument {} on `diagram`", show(other)),
                    ));
                }
            }
        }
    }
    // Declarations first so nodes anywhere can use kinds declared later in the file.
    for s in &body {
        if let Stmt::Item(it) = s {
            match it.head.as_str() {
                "style" => b.style(it)?,
                "arrow" => b.arrow(it)?,
                _ => {}
            }
        }
    }
    d.blocks = b.blocks(&body, &mut d)?;
    d.edges = b.edges;
    d.kinds = b.kinds;
    d.arrows = b.arrows;
    d.legend = b.legend;
    if d.preset == Preset::Clean {
        assign_auto_tones(&mut d.blocks);
        fill_default_gutters(&mut d.blocks);
        auto_width(&mut d);
        auto_edge_tones(&mut d);
        if d.legend.is_none() && !b.legend_off && needs_legend(&d) {
            d.legend = Some(Legend {
                nodes: Vec::new(),
                arrows: Vec::new(),
                bottom: false,
            });
        }
    }
    if d.title.is_empty() {
        return Err(Error::new(
            "the diagram has no title: write `diagram \"Title\" { ... }`",
        ));
    }
    for e in &d.edges {
        for id in [&e.from, &e.to] {
            if !b.ids.contains(id) {
                return Err(Error::at(
                    e.line,
                    format!("edge refers to unknown node `{id}`"),
                ));
            }
        }
    }
    Ok(d)
}

struct Builder {
    kinds: BTreeMap<String, NodeStyle>,
    arrows: BTreeMap<String, ArrowStyle>,
    ids: BTreeSet<String>,
    /// Ids written explicitly anywhere in the file; auto-generated ids avoid them.
    reserved: BTreeSet<String>,
    edges: Vec<Edge>,
    legend: Option<Legend>,
    legend_off: bool,
}

impl Builder {
    fn blocks(&mut self, stmts: &[Stmt], d: &mut Diagram) -> Result<Vec<Block>, Error> {
        let mut out = Vec::new();
        for s in stmts {
            match s {
                Stmt::Edge(e) => self.edge(e)?,
                Stmt::Item(it) => match it.head.as_str() {
                    "title" => d.title = one_string(it)?,
                    "note" | "subtitle" => d.note = Some(one_string(it)?),
                    "desc" => d.desc = Some(one_string(it)?),
                    "width" => {
                        d.width = one_num(it)?;
                        d.width_set = true;
                    }
                    "preset" => {
                        let name = one_string(it)?;
                        d.preset = Preset::parse(&name).ok_or_else(|| {
                            Error::at(
                                it.line,
                                format!("unknown preset `{name}`; use clean or manual"),
                            )
                        })?;
                    }
                    "style" | "arrow" => {}
                    "legend" => match legend(it)? {
                        Some(l) => self.legend = Some(l),
                        None => self.legend_off = true,
                    },
                    _ => {
                        if let Some(bl) = self.block(it, d)? {
                            out.push(bl);
                        }
                    }
                },
            }
        }
        Ok(out)
    }

    /// Blocks that may appear inside sections, rows and container nodes.
    fn block(&mut self, it: &Item, d: &mut Diagram) -> Result<Option<Block>, Error> {
        Ok(Some(match it.head.as_str() {
            "section" | "band" => {
                let label = it.args.iter().find_map(|a| {
                    if let Arg::Value(Value::Str(s)) = a {
                        Some(s.clone())
                    } else {
                        None
                    }
                });
                let label = label.ok_or_else(|| {
                    Error::at(it.line, format!("`{}` needs a quoted label", it.head))
                })?;
                let children = self.blocks(&it.body.clone().unwrap_or_default(), d)?;
                Block::Section(Section {
                    label,
                    band: it.head == "band",
                    children,
                })
            }
            "row" => {
                let mut weights = Vec::new();
                let mut gutter = None;
                for a in &it.args {
                    match a {
                        Arg::Weights(w) => weights = w.clone(),
                        Arg::Value(Value::Num(n)) => weights = vec![*n],
                        Arg::Attr(k, v) if k == "gutter" => gutter = Some(num(v, it.line)?),
                        other => {
                            return Err(Error::at(
                                it.line,
                                format!("unexpected argument {} on `row`", show(other)),
                            ));
                        }
                    }
                }
                let mut cells = Vec::new();
                for s in it.body.as_deref().unwrap_or(&[]) {
                    match s {
                        Stmt::Edge(e) => self.edge(e)?,
                        Stmt::Item(c) if c.head == "gap" => cells.push(None),
                        Stmt::Item(c) => cells.push(self.block(c, d)?),
                    }
                }
                if !weights.is_empty() && weights.len() != cells.len() {
                    return Err(Error::at(
                        it.line,
                        format!(
                            "row has {} weights but {} cells",
                            weights.len(),
                            cells.len()
                        ),
                    ));
                }
                Block::Row(Row {
                    weights,
                    cells,
                    gutter,
                })
            }
            "text" => Block::Text(one_string(it)?),
            "divider" => {
                let mut text = None;
                let mut tone = Tone::Gray;
                for a in &it.args {
                    match a {
                        Arg::Value(Value::Str(s)) => text = Some(s.clone()),
                        Arg::Value(Value::Ident(t)) if Tone::parse(t).is_some() => {
                            tone = Tone::parse(t).unwrap()
                        }
                        Arg::Attr(k, v) if k == "tone" => tone = tone_value(v, it.line)?,
                        other => {
                            return Err(Error::at(
                                it.line,
                                format!("unexpected argument {} on `divider`", show(other)),
                            ));
                        }
                    }
                }
                Block::Divider { text, tone }
            }
            "gap" => Block::Gap(
                it.args
                    .iter()
                    .find_map(|a| {
                        if let Arg::Value(Value::Num(n)) = a {
                            Some(*n)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(24.0),
            ),
            kind if self.kinds.contains_key(kind) => Block::Node(self.node(it, d)?),
            other => {
                return Err(Error::at(
                    it.line,
                    format!(
                        "unknown block `{other}`; known: section, band, row, text, divider, gap, legend, style, arrow, and node kinds {}",
                        self.kind_list()
                    ),
                ));
            }
        }))
    }

    fn kind_list(&self) -> String {
        self.kinds
            .keys()
            .map(|k| format!("`{k}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn node(&mut self, it: &Item, d: &mut Diagram) -> Result<Node, Error> {
        let mut style = self.kinds[&it.head].clone();
        let mut id = None;
        let mut title = None;
        let mut role = style.role.clone();
        let mut gutter = None;
        let mut tag = None;
        for a in &it.args {
            match a {
                Arg::Value(Value::Ident(w)) => {
                    if w == "auto" {
                        style.auto = true;
                    } else if let Some(t) = Tone::parse(w) {
                        style.tone = t;
                        style.auto = false;
                    } else if !apply_flag(&mut style, w) {
                        if id.is_some() {
                            return Err(Error::at(
                                it.line,
                                format!("`{w}` is neither a tone, a flag, nor a second id"),
                            ));
                        }
                        id = Some(w.clone());
                    }
                }
                Arg::Value(Value::Str(s)) => {
                    if title.is_some() {
                        return Err(Error::at(it.line, "a node takes one quoted title"));
                    }
                    title = Some(s.clone());
                }
                Arg::Value(Value::Num(_)) | Arg::Weights(_) => {
                    return Err(Error::at(it.line, "numbers are not valid node arguments"));
                }
                Arg::Attr(k, v) => match k.as_str() {
                    "id" => id = Some(v.as_text()),
                    "title" | "name" => title = Some(v.as_text()),
                    "tone" => {
                        if v.as_text() == "auto" {
                            style.auto = true;
                        } else {
                            style.tone = tone_value(v, it.line)?;
                            style.auto = false;
                        }
                    }
                    "role" => role = Some(v.as_text()),
                    "tag" => tag = Some(v.as_text()),
                    "gutter" => gutter = Some(num(v, it.line)?),
                    "align" => style.align = align_value(v, it.line)?,
                    _ => return Err(Error::at(it.line, format!("unknown node attribute `{k}`"))),
                },
            }
        }
        if let Some(t) = &title
            && tag.is_none()
            && let Some((tg, rest)) = split_tag(t)
        {
            tag = Some(tg);
            title = Some(rest);
        }
        let mut lines = Vec::new();
        let mut children = Vec::new();
        for s in it.body.as_deref().unwrap_or(&[]) {
            match s {
                Stmt::Edge(e) => self.edge(e)?,
                Stmt::Item(c) => match c.head.as_str() {
                    "name" | "title" => title = Some(one_string(c)?),
                    "code" => lines.push(Line::Code(one_string(c)?)),
                    "sub" => lines.push(Line::Sub(one_string(c)?)),
                    "text" if c.body.is_none() => lines.push(Line::Text(one_string(c)?)),
                    "role" => role = Some(one_string(c)?),
                    "tag" => tag = Some(one_string(c)?),
                    _ => {
                        if let Some(bl) = self.block(c, d)? {
                            children.push(bl);
                        }
                    }
                },
            }
        }
        let id = match id {
            Some(id) => id,
            None => {
                let base = slug(title.as_deref().unwrap_or(&it.head));
                let mut candidate = base.clone();
                let mut n = 2;
                while self.ids.contains(&candidate) || self.reserved.contains(&candidate) {
                    candidate = format!("{base}-{n}");
                    n += 1;
                }
                candidate
            }
        };
        if !self.ids.insert(id.clone()) {
            return Err(Error::at(it.line, format!("duplicate node id `{id}`")));
        }
        if title.is_none() && lines.iter().all(|l| !matches!(l, Line::Code(_))) && id != it.head {
            title = Some(id.clone());
        }
        // A generic node with nested children draws as a hollow container
        // so newcomers don't need to learn package/crate/group on day one.
        if !children.is_empty()
            && matches!(style.shape, Shape::Card | Shape::Api)
            && matches!(it.head.as_str(), "card" | "node" | "api")
        {
            style.shape = Shape::Container;
            style.hollow = true;
        }
        Ok(Node {
            id,
            kind: it.head.clone(),
            style,
            title,
            tag,
            role,
            lines,
            children,
            gutter,
            line: it.line,
        })
    }

    fn style(&mut self, it: &Item) -> Result<(), Error> {
        let mut args = it.args.iter();
        let name = match args.next() {
            Some(Arg::Value(Value::Ident(n))) => n.clone(),
            _ => {
                return Err(Error::at(
                    it.line,
                    "`style` needs a kind name: `style api tone=purple`",
                ));
            }
        };
        let mut style = self
            .kinds
            .get(&name)
            .cloned()
            .unwrap_or_else(|| self.kinds["card"].clone());
        for a in args {
            match a {
                Arg::Value(Value::Ident(w)) => {
                    if w == "auto" {
                        style.auto = true;
                    } else if let Some(t) = Tone::parse(w) {
                        style.tone = t;
                        style.auto = false;
                    } else if let Some(base) = self.kinds.get(w) {
                        let (tone, hollow, auto) = (style.tone, style.hollow, style.auto);
                        style = base.clone();
                        style.tone = tone;
                        style.hollow = hollow;
                        style.auto = auto;
                    } else if !apply_flag(&mut style, w) {
                        return Err(Error::at(it.line, format!("unknown style word `{w}`")));
                    }
                }
                Arg::Value(Value::Str(s)) => style.label = Some(s.clone()),
                Arg::Attr(k, v) => match k.as_str() {
                    "tone" => {
                        if v.as_text() == "auto" {
                            style.auto = true;
                        } else {
                            style.tone = tone_value(v, it.line)?;
                            style.auto = false;
                        }
                    }
                    "base" => {
                        let base = self.kinds.get(&v.as_text()).ok_or_else(|| {
                            Error::at(it.line, format!("unknown base kind `{}`", v.as_text()))
                        })?;
                        style = base.clone();
                    }
                    "shape" => {
                        style.shape = match v.as_text().as_str() {
                            "card" => Shape::Card,
                            "api" => Shape::Api,
                            "package" => Shape::Package,
                            "container" => Shape::Container,
                            other => {
                                return Err(Error::at(
                                    it.line,
                                    format!(
                                        "unknown shape `{other}`; use card, api, package or container"
                                    ),
                                ));
                            }
                        };
                        if style.shape == Shape::Api {
                            style.align = Align::Center;
                        }
                    }
                    "role" => style.role = Some(v.as_text()),
                    "label" => style.label = Some(v.as_text()),
                    "align" => style.align = align_value(v, it.line)?,
                    _ => return Err(Error::at(it.line, format!("unknown style attribute `{k}`"))),
                },
                Arg::Value(Value::Num(_)) | Arg::Weights(_) => {
                    return Err(Error::at(it.line, "numbers are not valid style arguments"));
                }
            }
        }
        self.kinds.insert(name, style);
        Ok(())
    }

    fn arrow(&mut self, it: &Item) -> Result<(), Error> {
        let mut args = it.args.iter();
        let name = match args.next() {
            Some(Arg::Value(Value::Ident(n))) => n.clone(),
            _ => {
                return Err(Error::at(
                    it.line,
                    "`arrow` needs a kind name: `arrow impl orange \"implements\"`",
                ));
            }
        };
        let mut style = self
            .arrows
            .get(&name)
            .cloned()
            .unwrap_or_else(|| self.arrows["default"].clone());
        for a in args {
            match a {
                Arg::Value(Value::Ident(w)) => match w.as_str() {
                    "dashed" => style.dashed = true,
                    "solid" => style.dashed = false,
                    "inherit" => style.color = ArrowColor::Inherit,
                    w if Tone::parse(w).is_some() => {
                        style.color = ArrowColor::Tone(Tone::parse(w).unwrap())
                    }
                    other => {
                        return Err(Error::at(it.line, format!("unknown arrow word `{other}`")));
                    }
                },
                Arg::Value(Value::Str(s)) => style.label = Some(s.clone()),
                Arg::Attr(k, v) => match k.as_str() {
                    "tone" => style.color = ArrowColor::Tone(tone_value(v, it.line)?),
                    "label" => style.label = Some(v.as_text()),
                    "chip" => style.chip = Some(v.as_text()),
                    _ => return Err(Error::at(it.line, format!("unknown arrow attribute `{k}`"))),
                },
                _ => return Err(Error::at(it.line, "numbers are not valid arrow arguments")),
            }
        }
        self.arrows.insert(name, style);
        Ok(())
    }

    fn edge(&mut self, e: &EdgeStmt) -> Result<(), Error> {
        let kind = e.kind.clone().unwrap_or_else(|| "default".into());
        if !self.arrows.contains_key(&kind) {
            return Err(Error::at(
                e.line,
                format!("unknown edge kind `{kind}`; declare it with `arrow {kind} ...`"),
            ));
        }
        let (from, to, head_at_end, head_at_start) = match (e.left, e.right) {
            (false, true) => (e.from.clone(), e.to.clone(), true, false),
            (true, false) => (e.to.clone(), e.from.clone(), true, false),
            (true, true) => (e.from.clone(), e.to.clone(), true, true),
            (false, false) => (e.from.clone(), e.to.clone(), false, false),
        };
        let mut edge = Edge {
            from,
            to,
            kind,
            label: None,
            head_at_end,
            head_at_start,
            via: None,
            from_side: None,
            to_side: None,
            dashed: None,
            tone: None,
            labeled: false,
            bus: false,
            line: e.line,
        };
        for a in &e.args {
            match a {
                Arg::Value(Value::Str(s)) => edge.label = Some(s.clone()),
                Arg::Value(Value::Ident(w)) => match w.as_str() {
                    "dashed" => edge.dashed = Some(true),
                    "solid" => edge.dashed = Some(false),
                    "labeled" => edge.labeled = true,
                    "bus" => edge.bus = true,
                    w if Tone::parse(w).is_some() => edge.tone = Tone::parse(w),
                    other => {
                        return Err(Error::at(
                            e.line,
                            format!(
                                "unknown edge word `{other}`; use dashed, solid, labeled, bus or a tone"
                            ),
                        ));
                    }
                },
                Arg::Attr(k, v) => match k.as_str() {
                    "via" => edge.via = Some(side_value(v, e.line)?),
                    "from" => edge.from_side = Some(side_value(v, e.line)?),
                    "to" => edge.to_side = Some(side_value(v, e.line)?),
                    "label" => edge.label = Some(v.as_text()),
                    "tone" => edge.tone = Some(tone_value(v, e.line)?),
                    _ => return Err(Error::at(e.line, format!("unknown edge attribute `{k}`"))),
                },
                _ => return Err(Error::at(e.line, "numbers are not valid edge arguments")),
            }
        }
        if e.left && edge.from_side.is_some() {
            // `a <- b from=top` reads from b's point of view; swap the ports.
            std::mem::swap(&mut edge.from_side, &mut edge.to_side);
        }
        self.edges.push(edge);
        Ok(())
    }
}

/// Explicit ids are the first bare word on a node that is neither a tone nor
/// a flag, or an `id=` attribute. Declared kinds are unknown at this point,
/// so every item with a body or a string argument is treated as a node.
fn reserve_explicit_ids(stmts: &[Stmt], out: &mut BTreeSet<String>) {
    for s in stmts {
        let Stmt::Item(it) = s else { continue };
        if matches!(
            it.head.as_str(),
            "style"
                | "arrow"
                | "legend"
                | "preset"
                | "row"
                | "section"
                | "band"
                | "text"
                | "divider"
                | "gap"
                | "note"
                | "desc"
                | "title"
                | "width"
                | "code"
                | "sub"
                | "name"
                | "role"
                | "tag"
        ) {
            if let Some(body) = &it.body {
                reserve_explicit_ids(body, out);
            }
            continue;
        }
        let mut id = None;
        for a in &it.args {
            match a {
                Arg::Value(Value::Ident(w))
                    if Tone::parse(w).is_none()
                        && !FLAG_WORDS.contains(&w.as_str())
                        && id.is_none() =>
                {
                    id = Some(w.clone())
                }
                Arg::Attr(k, v) if k == "id" => id = Some(v.as_text()),
                _ => {}
            }
        }
        if let Some(id) = id {
            out.insert(id);
        }
        if let Some(body) = &it.body {
            reserve_explicit_ids(body, out);
        }
    }
}

/// Assigns `AUTO_CYCLE` tones in document order to nodes that omit a tone.
fn assign_auto_tones(blocks: &mut [Block]) {
    let mut next = 0usize;
    fn walk(blocks: &mut [Block], next: &mut usize) {
        for b in blocks.iter_mut() {
            match b {
                Block::Node(n) => {
                    if n.style.auto {
                        n.style.tone = AUTO_CYCLE[*next % AUTO_CYCLE.len()];
                        *next += 1;
                    }
                    walk(&mut n.children, next);
                }
                Block::Row(r) => {
                    for c in r.cells.iter_mut().flatten() {
                        walk(std::slice::from_mut(c), next);
                    }
                }
                Block::Section(s) => walk(&mut s.children, next),
                _ => {}
            }
        }
    }
    walk(blocks, &mut next);
}

/// Default row gutter for diagrams that don't set one (16px under clean;
/// explicit `gutter=` always wins).
fn fill_default_gutters(blocks: &mut [Block]) {
    fn walk(blocks: &mut [Block]) {
        for b in blocks.iter_mut() {
            match b {
                Block::Node(n) => walk(&mut n.children),
                Block::Row(r) => {
                    if r.gutter.is_none() {
                        r.gutter = Some(16.0);
                    }
                    for c in r.cells.iter_mut().flatten() {
                        walk(std::slice::from_mut(c));
                    }
                }
                Block::Section(s) => walk(&mut s.children),
                _ => {}
            }
        }
    }
    walk(blocks);
}

/// Widen roomy diagrams that never set `width=`: three or more cells in a
/// row, or three levels of nesting, gets the wide 1400px canvas.
fn auto_width(d: &mut Diagram) {
    if d.width_set {
        return;
    }
    fn walk(blocks: &[Block], depth: usize, wide: &mut usize, deep: &mut usize) {
        *deep = (*deep).max(depth);
        for b in blocks {
            match b {
                Block::Node(n) => {
                    if !n.children.is_empty() {
                        walk(&n.children, depth + 1, wide, deep);
                    }
                }
                Block::Row(r) => {
                    *wide = (*wide).max(r.cells.len());
                    for c in r.cells.iter().flatten() {
                        walk(std::slice::from_ref(c), depth, wide, deep);
                    }
                }
                Block::Section(s) => walk(&s.children, depth, wide, deep),
                _ => {}
            }
        }
    }
    let (mut wide, mut deep) = (0usize, 0usize);
    walk(&d.blocks, 0, &mut wide, &mut deep);
    if wide >= 3 || deep >= 3 {
        d.width = 1400.0;
    }
}

/// Bare `->` edges take the source node's tone so they stay visible on a
/// colorful diagram. Typed edges (`-impl->`, …) and explicit tones win.
fn auto_edge_tones(d: &mut Diagram) {
    fn collect(blocks: &[Block], out: &mut BTreeMap<String, Tone>) {
        for b in blocks {
            match b {
                Block::Node(n) => {
                    out.insert(n.id.clone(), n.style.tone);
                    collect(&n.children, out);
                }
                Block::Row(r) => {
                    for c in r.cells.iter().flatten() {
                        collect(std::slice::from_ref(c), out);
                    }
                }
                Block::Section(s) => collect(&s.children, out),
                _ => {}
            }
        }
    }
    let mut tones = BTreeMap::new();
    collect(&d.blocks, &mut tones);
    for e in d.edges.iter_mut() {
        if e.kind == "default"
            && e.tone.is_none()
            && let Some(t) = tones.get(&e.from)
        {
            e.tone = Some(*t);
        }
    }
}

/// Auto-legend when the diagram uses at least one typed edge kind.
fn needs_legend(d: &Diagram) -> bool {
    d.edges.iter().any(|e| e.kind != "default")
}

fn legend(it: &Item) -> Result<Option<Legend>, Error> {
    let mut l = Legend {
        nodes: Vec::new(),
        arrows: Vec::new(),
        bottom: false,
    };
    for a in &it.args {
        match a {
            Arg::Value(Value::Ident(w)) if w == "bottom" => l.bottom = true,
            Arg::Value(Value::Ident(w)) if w == "top" || w == "auto" => {}
            Arg::Value(Value::Ident(w)) if w == "off" || w == "none" => return Ok(None),
            Arg::Attr(k, v) if k == "nodes" => l.nodes = list(v),
            Arg::Attr(k, v) if k == "arrows" => l.arrows = list(v),
            other => {
                return Err(Error::at(
                    it.line,
                    format!("unexpected argument {} on `legend`", show(other)),
                ));
            }
        }
    }
    Ok(Some(l))
}

fn list(v: &Value) -> Vec<String> {
    v.as_text()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn apply_flag(style: &mut NodeStyle, w: &str) -> bool {
    match w {
        "hollow" => style.hollow = true,
        "filled" => style.hollow = false,
        "center" => style.align = Align::Center,
        "left" => style.align = Align::Left,
        "mono" => style.mono = true,
        "sans" => style.mono = false,
        _ => return FLAG_WORDS.contains(&w),
    }
    true
}

fn split_tag(title: &str) -> Option<(String, String)> {
    let rest = title.strip_prefix('[')?;
    let end = rest.find(']')?;
    let tag = rest[..end].trim();
    if tag.is_empty() {
        return None;
    }
    Some((tag.to_string(), rest[end + 1..].trim_start().to_string()))
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "node".into() } else { out }
}

fn one_string(it: &Item) -> Result<String, Error> {
    match it.args.as_slice() {
        [Arg::Value(v)] => Ok(v.as_text()),
        _ => Err(Error::at(
            it.line,
            format!("`{}` takes exactly one quoted string", it.head),
        )),
    }
}

fn one_num(it: &Item) -> Result<f64, Error> {
    match it.args.as_slice() {
        [Arg::Value(Value::Num(n))] => Ok(*n),
        _ => Err(Error::at(
            it.line,
            format!("`{}` takes exactly one number", it.head),
        )),
    }
}

fn num(v: &Value, line: usize) -> Result<f64, Error> {
    match v {
        Value::Num(n) => Ok(*n),
        other => Err(Error::at(
            line,
            format!("expected a number, found `{}`", other.as_text()),
        )),
    }
}

fn tone_value(v: &Value, line: usize) -> Result<Tone, Error> {
    Tone::parse(&v.as_text()).ok_or_else(|| {
        Error::at(
            line,
            format!(
                "unknown tone `{}`; use gray, blue, green, yellow, purple, orange or red",
                v.as_text()
            ),
        )
    })
}

fn align_value(v: &Value, line: usize) -> Result<Align, Error> {
    match v.as_text().as_str() {
        "left" => Ok(Align::Left),
        "center" => Ok(Align::Center),
        other => Err(Error::at(line, format!("unknown alignment `{other}`"))),
    }
}

fn side_value(v: &Value, line: usize) -> Result<Side, Error> {
    Side::parse(&v.as_text()).ok_or_else(|| {
        Error::at(
            line,
            format!(
                "unknown side `{}`; use top, bottom, left or right",
                v.as_text()
            ),
        )
    })
}

fn show(a: &Arg) -> String {
    match a {
        Arg::Value(v) => format!("`{}`", v.as_text()),
        Arg::Attr(k, v) => format!("`{k}={}`", v.as_text()),
        Arg::Weights(w) => format!(
            "`{}`",
            w.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(":")
        ),
    }
}

impl Node {
    pub fn is_container(&self) -> bool {
        matches!(self.style.shape, Shape::Package | Shape::Container) || !self.children.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_nodes_rows_and_edges() {
        let d = build(
            r#"
diagram "T" {
  band "Firmware" {
    row 1:2 {
      card a blue { sub "x" }
      card "Tier 1 — prebuilt" green { code "init()" }
    }
  }
  trait t "[re-exported] SPI error contract" { code "spi::ErrorType" }
  a -> tier-1-prebuilt "built from"
  t -exports-> a via=right
}
"#,
        )
        .unwrap();
        assert_eq!(d.title, "T");
        let Block::Section(s) = &d.blocks[0] else {
            panic!()
        };
        let Block::Row(r) = &s.children[0] else {
            panic!()
        };
        assert_eq!(r.weights, vec![1.0, 2.0]);
        let Some(Block::Node(n)) = &r.cells[1] else {
            panic!()
        };
        assert_eq!(n.id, "tier-1-prebuilt");
        let Block::Node(t) = &d.blocks[1] else {
            panic!()
        };
        assert_eq!(t.tag.as_deref(), Some("re-exported"));
        assert_eq!(t.title.as_deref(), Some("SPI error contract"));
        assert_eq!(d.edges.len(), 2);
        assert_eq!(d.edges[1].via, Some(Side::Right));
    }

    #[test]
    fn auto_ids_avoid_explicit_ones() {
        let d =
            build("diagram \"T\" { package \"hal\" { crate hal { } }\n hal -> hal-2 }").unwrap();
        let Block::Node(p) = &d.blocks[0] else {
            panic!()
        };
        assert_eq!(p.id, "hal-2");
        let Block::Node(c) = &p.children[0] else {
            panic!()
        };
        assert_eq!(c.id, "hal");
    }

    #[test]
    fn rejects_unknown_edge_target() {
        let err = build("diagram \"T\" { card a\n a -> zz }").unwrap_err();
        assert!(err.to_string().contains("unknown node `zz`"), "{err}");
    }

    #[test]
    fn custom_kinds() {
        let d = build("diagram \"T\" { style fn trait tone=orange \"function\"\n fn f \"Free function\" { code \"foo()\" } }").unwrap();
        let Block::Node(n) = &d.blocks[0] else {
            panic!()
        };
        assert_eq!(n.style.shape, Shape::Api);
        assert_eq!(n.style.tone, Tone::Orange);
        assert_eq!(d.kinds["fn"].label.as_deref(), Some("function"));
    }

    #[test]
    fn auto_tones_cycle_in_order() {
        let d = build(
            "diagram \"T\" { node a \"A\"\n node b \"B\"\n card c red { sub \"x\" } \n node d \"D\" }",
        )
        .unwrap();
        let tones: Vec<Tone> = d
            .blocks
            .iter()
            .map(|b| {
                let Block::Node(n) = b else { panic!() };
                n.style.tone
            })
            .collect();
        assert_eq!(
            tones,
            vec![Tone::Blue, Tone::Green, Tone::Red, Tone::Yellow]
        );
    }

    #[test]
    fn node_with_children_becomes_container() {
        let d = build("diagram \"T\" { node g \"Group\" { node a \"A\" } }").unwrap();
        let Block::Node(g) = &d.blocks[0] else {
            panic!()
        };
        assert_eq!(g.style.shape, Shape::Container);
        assert!(g.style.hollow);
    }

    #[test]
    fn bare_edge_takes_source_tone() {
        let d = build("diagram \"T\" { node a \"A\"\n node b \"B\"\n a -> b }").unwrap();
        assert_eq!(d.edges[0].tone, Some(Tone::Blue));
    }

    #[test]
    fn typed_edges_trigger_auto_legend() {
        let d =
            build("diagram \"T\" { node a \"A\"\n node b \"B\"\n a -uses-> b labeled }").unwrap();
        assert!(d.legend.is_some());
        let d2 = build("diagram \"T\" { node a \"A\"\n node b \"B\"\n a -> b }").unwrap();
        assert!(d2.legend.is_none());
        let d3 = build(
            "diagram \"T\" { legend off\n node a \"A\"\n node b \"B\"\n a -uses-> b labeled }",
        )
        .unwrap();
        assert!(d3.legend.is_none());
    }

    #[test]
    fn wide_rows_get_wide_canvas() {
        let d =
            build("diagram \"T\" { row { node a \"A\"\n node b \"B\"\n node c \"C\" } }").unwrap();
        assert_eq!(d.width, 1400.0);
        let d2 =
            build("diagram \"T\" width=900 { row { node a \"A\"\n node b \"B\"\n node c \"C\" } }")
                .unwrap();
        assert_eq!(d2.width, 900.0);
    }

    #[test]
    fn manual_preset_keeps_explicit_defaults() {
        let d = build("diagram \"T\" preset=manual { card a \"A\"\n card b \"B\" }").unwrap();
        let Block::Node(a) = &d.blocks[0] else {
            panic!()
        };
        assert_eq!(a.style.tone, Tone::Gray);
        assert!(d.legend.is_none());
        assert_eq!(d.width, 900.0);
    }

    #[test]
    fn default_gutter_is_sixteen() {
        let d = build("diagram \"T\" { row { node a \"A\"\n node b \"B\" } }").unwrap();
        let Block::Row(r) = &d.blocks[0] else {
            panic!()
        };
        assert_eq!(r.gutter, Some(16.0));
    }
}
