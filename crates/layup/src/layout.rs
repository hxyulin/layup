//! Block layout: measures every block for a given width, then places it.
//!
//! The model is CSS-like block flow. Rows split their width by weights and
//! stretch every cell to the tallest one. Containers pad their children.
//! Text is wrapped with measured widths so nothing is ever placed by hand.

use crate::Warning;
use crate::geometry::Outline;
use crate::model::{Block, Diagram, Direction, Legend, Line, Node, Row, Section};
use crate::style::{Align, ArrowColor, NodeStyle, Shape, Tone};
use crate::text::{
    Font, Fonts, Run, runs, runs_width_with_fonts, width_with_fonts, wrap_with_fonts,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
    pub fn cx(&self) -> f64 {
        self.x + self.w / 2.0
    }
    pub fn cy(&self) -> f64 {
        self.y + self.h / 2.0
    }
    pub fn grow(&self, m: f64) -> Rect {
        Rect {
            x: self.x - m,
            y: self.y - m,
            w: self.w + 2.0 * m,
            h: self.h + 2.0 * m,
        }
    }
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x > self.x && x < self.right() && y > self.y && y < self.bottom()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    Text,
    Muted,
    Code,
    Tone(Tone),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

fn logical_start(x: f64, w: f64, direction: crate::text::Direction, text: &str) -> (f64, Anchor) {
    if direction.resolve(text) == crate::text::Direction::Rtl {
        (x + w, Anchor::End)
    } else {
        (x, Anchor::Start)
    }
}

#[derive(Debug, Clone)]
pub struct TextItem {
    pub x: f64,
    pub y: f64,
    pub anchor: Anchor,
    pub direction: crate::text::Direction,
    pub runs: Vec<Run>,
    pub size: f64,
    pub weight: u16,
    pub mono: bool,
    pub ink: Ink,
    pub letter_spacing: f64,
}

#[derive(Debug, Clone)]
pub enum Item {
    /// A node body. `hollow` draws only the border.
    Box {
        rect: Rect,
        tone: Tone,
        hollow: bool,
        white: bool,
        rx: f64,
        stroke_width: f64,
    },
    StateMarker {
        rect: Rect,
        tone: Tone,
        final_state: bool,
    },
    Diamond {
        rect: Rect,
        tone: Tone,
        hollow: bool,
        stroke_width: f64,
    },
    /// The gray head strip of a package frame.
    Strip {
        rect: Rect,
        rx: f64,
    },
    Rule {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        tone: Option<Tone>,
        dashed: bool,
    },
    Text(TextItem),
    /// Caption on a background chip; `bordered` chips label edges.
    Chip {
        rect: Rect,
        text: String,
        tone: Tone,
        rotate: bool,
        bordered: bool,
    },
    /// Legend arrow sample.
    Sample {
        x: f64,
        y: f64,
        tone: Tone,
        dashed: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Placed {
    pub item: Item,
    /// Index into `Scene::nodes` for items that belong to a node.
    pub node: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct NodeRect {
    pub id: String,
    pub kind: String,
    pub rect: Rect,
    pub outline: Outline,
    pub parent: Option<usize>,
    pub tone: Tone,
    pub href: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct EdgePath {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub points: Vec<(f64, f64)>,
    pub tone: Tone,
    pub dashed: bool,
    pub head_end: bool,
    pub head_start: bool,
    pub chip: Option<Item>,
    pub bus: bool,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub fonts: Fonts,
    pub width: f64,
    pub height: f64,
    pub margin: f64,
    /// Horizontal extent of the block content; outside lanes live beyond it.
    pub content_left: f64,
    pub content_right: f64,
    pub items: Vec<Placed>,
    pub nodes: Vec<NodeRect>,
    pub edges: Vec<EdgePath>,
    /// Text that edge labels must not cover (section titles, captions).
    pub keepout: Vec<Rect>,
}

impl Scene {
    pub fn node(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    pub fn ancestors(&self, mut i: usize) -> Vec<usize> {
        let mut out = Vec::new();
        while let Some(p) = self.nodes[i].parent {
            out.push(p);
            i = p;
        }
        out
    }

    pub fn is_descendant(&self, mut i: usize, of: usize) -> bool {
        while let Some(p) = self.nodes[i].parent {
            if p == of {
                return true;
            }
            i = p;
        }
        false
    }
}

const BLOCK_GAP: f64 = 16.0;
const ROW_GUTTER: f64 = 12.0;
const SECTION_GAP: f64 = 48.0;

struct Metrics {
    margin: f64,
    h1: f64,
    h2: f64,
    bottom: f64,
}

fn metrics(width: f64) -> Metrics {
    if width >= 1400.0 {
        Metrics {
            margin: 50.0,
            h1: 24.0,
            h2: 15.0,
            bottom: 30.0,
        }
    } else {
        Metrics {
            margin: 40.0,
            h1: 19.0,
            h2: 13.0,
            bottom: 24.0,
        }
    }
}

/// A measured text line ready to draw.
#[derive(Debug, Clone)]
struct MLine {
    lines: Vec<Vec<Run>>,
    size: f64,
    weight: u16,
    mono: bool,
    ink: Ink,
    /// Baseline offset of the first line from the cursor.
    ascent: f64,
    /// Cursor advance per line.
    advance: f64,
    /// Extra space before this block of lines.
    before: f64,
    /// Blue bold prefix drawn before the first line.
    tag: Option<String>,
}

enum L<'a> {
    Flow {
        cells: Vec<(L<'a>, f64, f64, f64, f64)>,
        h: f64,
    },
    Node {
        node: &'a Node,
        lines: Vec<MLine>,
        children: Vec<(L<'a>, f64)>,
        natural: f64,
        content_bottom: f64,
    },
    Row {
        cells: Vec<Option<(L<'a>, f64, f64)>>,
        h: f64,
    },
    Section {
        section: &'a Section,
        children: Vec<(L<'a>, f64)>,
        h: f64,
    },
    Text {
        lines: Vec<Vec<Run>>,
        h: f64,
    },
    Divider {
        text: Option<&'a str>,
        tone: Tone,
        h: f64,
    },
    Gap(f64),
    Legend {
        entries: Vec<LegendEntry>,
        rows: Vec<Vec<usize>>,
        h: f64,
    },
}

fn height(l: &L) -> f64 {
    match l {
        L::Node { natural, .. } => *natural,
        L::Flow { h, .. }
        | L::Row { h, .. }
        | L::Section { h, .. }
        | L::Text { h, .. }
        | L::Divider { h, .. }
        | L::Legend { h, .. } => *h,
        L::Gap(h) => *h,
    }
}

struct Ctx<'a> {
    scene: Scene,
    warnings: &'a mut Vec<Warning>,
    parent: Option<usize>,
    text_direction: crate::text::Direction,
    machine_gutter: Option<f64>,
    pack: bool,
}

pub fn layout(d: &Diagram, warnings: &mut Vec<Warning>) -> Scene {
    layout_with_fonts(d, warnings, &Fonts::default())
}

pub fn layout_with_fonts(d: &Diagram, warnings: &mut Vec<Warning>, fonts: &Fonts) -> Scene {
    let mut fitted;
    let d = if d.auto_layout && !d.width_set && d.preset == crate::model::Preset::Clean {
        fitted = d.clone();
        crate::sizing::fit(&mut fitted, fonts);
        &fitted
    } else {
        d
    };
    let m = metrics(d.width);
    let lanes = |side| d.edges.iter().filter(|e| e.via == Some(side)).count() as f64;
    let reserve = |n: f64| {
        if n > 0.0 {
            (25.0 + 30.0 * (n - 1.0) + 14.0 - m.margin).max(0.0)
        } else {
            0.0
        }
    };
    let (left_reserve, right_reserve) = (
        reserve(lanes(crate::model::Side::Left)),
        reserve(lanes(crate::model::Side::Right)),
    );
    let content_left = m.margin + left_reserve;
    let content_w = d.width - 2.0 * m.margin - left_reserve - right_reserve;
    let mut ctx = Ctx {
        scene: Scene {
            fonts: fonts.clone(),
            width: d.width,
            height: 0.0,
            margin: m.margin,
            content_left,
            content_right: content_left + content_w,
            items: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            keepout: Vec::new(),
        },
        warnings,
        parent: None,
        text_direction: d.text_direction,
        pack: d.auto_layout && !d.width_set && d.preset == crate::model::Preset::Clean,
        machine_gutter: (d.mode == crate::model::Mode::StateMachine)
            .then(|| crate::sizing::machine_gap(d, fonts)),
    };

    // Header.
    let mut y = m.margin;
    let title_w = ctx.width(&d.title, Font::SansBold, m.h1, 0.0);
    let (title_x, title_anchor) = logical_start(
        m.margin,
        d.width - 2.0 * m.margin,
        d.text_direction,
        &d.title,
    );
    ctx.push(Item::Text(TextItem {
        x: title_x,
        y,
        anchor: title_anchor,
        direction: crate::text::Direction::Auto,
        runs: runs(&d.title),
        size: m.h1,
        weight: 680,
        mono: false,
        ink: Ink::Text,
        letter_spacing: 0.0,
    }));
    let mut header_bottom = y;
    if let Some(note) = &d.note {
        y += m.h2 + 8.0;
        let lines = ctx.wrap(note, Font::Sans, m.h2, content_w - title_w.min(0.0));
        let direction = d.text_direction.resolve(note);
        let (note_x, note_anchor) =
            logical_start(m.margin, d.width - 2.0 * m.margin, direction, note);
        for l in lines {
            ctx.push(Item::Text(TextItem {
                x: note_x,
                y,
                anchor: note_anchor,
                direction,
                runs: l,
                size: m.h2,
                weight: 400,
                mono: false,
                ink: Ink::Muted,
                letter_spacing: 0.0,
            }));
            header_bottom = y;
            y += m.h2 + 6.0;
        }
    } else {
        header_bottom = y;
    }

    // Legend: beside the title when it fits, otherwise as the first block.
    let legend = d
        .legend
        .as_ref()
        .map(|l| legend_entries(d, l, &ctx.scene.fonts));
    let mut legend_block = None;
    if let Some((entries, bottom)) = legend {
        let total_w = |ids: &[usize]| {
            ids.iter().map(|&i| entries[i].w).sum::<f64>()
                + 16.0 * ids.len().saturating_sub(1) as f64
        };
        let node_ids: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind.is_some())
            .map(|(i, _)| i)
            .collect();
        let arrow_ids: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind.is_none())
            .map(|(i, _)| i)
            .collect();
        let widest = total_w(&node_ids).max(total_w(&arrow_ids));
        let fits_beside = !bottom && widest <= content_w - title_w - 60.0;
        if fits_beside {
            let mut ly = m.margin - 18.0;
            for ids in [node_ids, arrow_ids] {
                if ids.is_empty() {
                    continue;
                }
                let mut x = if title_anchor == Anchor::End {
                    m.margin
                } else {
                    d.width - m.margin - total_w(&ids)
                };
                for i in ids {
                    draw_legend_entry(&mut ctx, &entries[i], x, ly);
                    x += entries[i].w + 16.0;
                }
                ly += 34.0;
            }
            header_bottom = header_bottom.max(ly - 34.0 + 10.0);
        } else {
            let rows = pack_legend(&entries, content_w);
            let h = rows.len() as f64 * 34.0 - 6.0;
            legend_block = Some((L::Legend { entries, rows, h }, bottom));
        }
    }

    y = header_bottom + 28.0;

    let mut measured: Vec<(L, f64)> = Vec::new();
    if let Some((lb, false)) = &legend_block {
        measured.push((clone_legend(lb), height(lb)));
    }
    for b in &d.blocks {
        let l = measure(&mut ctx, b, content_w);
        let h = height(&l);
        measured.push((l, h));
    }
    if let Some((lb, true)) = &legend_block {
        measured.push((clone_legend(lb), height(lb)));
    }
    if ctx.machine_gutter.is_some() {
        y += 32.0;
    }
    let mut first = true;
    for (l, h) in measured {
        y += gap_before(&l, first);
        first = false;
        draw(&mut ctx, &l, content_left, y, content_w, h);
        y += h;
    }
    ctx.scene.height = (y
        + m.bottom
        + if ctx.machine_gutter.is_some() {
            32.0
        } else {
            0.0
        })
    .ceil();
    ctx.scene
}

fn clone_legend<'a>(l: &L<'a>) -> L<'a> {
    match l {
        L::Legend { entries, rows, h } => L::Legend {
            entries: entries.clone(),
            rows: rows.clone(),
            h: *h,
        },
        _ => unreachable!(),
    }
}

fn gap_before(l: &L, first: bool) -> f64 {
    match l {
        L::Section { section, .. } if !section.band && !first => SECTION_GAP,
        _ if first => 0.0,
        L::Gap(_) => 0.0,
        _ => BLOCK_GAP,
    }
}

impl Ctx<'_> {
    fn node_width(&self, n: &Node, available: f64) -> f64 {
        if self.pack && n.style.shape.compact() && !n.is_composite() {
            n.body_width(available).min(crate::sizing::node_width(
                n,
                &self.scene.fonts,
                self.machine_gutter.unwrap_or(24.0),
            ))
        } else {
            n.body_width(available)
        }
    }

    fn width(&self, s: &str, font: Font, size: f64, spacing: f64) -> f64 {
        width_with_fonts(s, font, size, spacing, &self.scene.fonts)
    }
    fn runs_width(&self, runs: &[Run], font: Font, size: f64) -> f64 {
        runs_width_with_fonts(runs, font, size, &self.scene.fonts)
    }
    fn wrap(&self, s: &str, font: Font, size: f64, max: f64) -> Vec<Vec<Run>> {
        wrap_with_fonts(s, font, size, max, &self.scene.fonts)
    }

    fn prepare_text(&self, item: &mut Item) {
        if let Item::Text(t) = item {
            let plain: String = t.runs.iter().map(|r| r.text.as_str()).collect();
            if t.direction == crate::text::Direction::Auto {
                t.direction = self.text_direction.resolve(&plain);
            }
        }
    }
    fn push(&mut self, mut item: Item) {
        self.prepare_text(&mut item);
        let node = self.parent;
        self.scene.items.push(Placed { item, node });
    }

    fn push_for(&mut self, node: usize, mut item: Item) {
        self.prepare_text(&mut item);
        self.scene.items.push(Placed {
            item,
            node: Some(node),
        });
    }

    fn warn(&mut self, line: Option<usize>, msg: String) {
        self.warnings.push(Warning { line, msg });
    }
}

fn measure<'a>(ctx: &mut Ctx, b: &'a Block, w: f64) -> L<'a> {
    match b {
        Block::Node(n) => measure_node(ctx, n, w),
        Block::Row(r) => measure_row(ctx, r, w),
        Block::Tree {
            direction,
            nodes,
            children,
            root,
        } => measure_tree(ctx, *direction, nodes, children, *root, w),
        Block::Flow { direction, layers } => measure_flow(ctx, *direction, layers, w),
        Block::Section(s) => {
            let children: Vec<(L, f64)> = s
                .children
                .iter()
                .map(|c| {
                    let l = measure(ctx, c, w);
                    let h = height(&l);
                    (l, h)
                })
                .collect();
            let mut h = if s.band { 22.0 } else { 40.0 };
            for (i, (l, ch)) in children.iter().enumerate() {
                h += gap_before(l, i == 0) + ch;
            }
            L::Section {
                section: s,
                children,
                h,
            }
        }
        Block::Text(t) => {
            let lines = wrap_checked(ctx, t, Font::Sans, 12.5, w, None);
            let h = lines.len() as f64 * 20.0;
            L::Text { lines, h }
        }
        Block::Divider { text, tone } => L::Divider {
            text: text.as_deref(),
            tone: *tone,
            h: 28.0,
        },
        Block::Gap(g) => L::Gap(*g),
    }
}

fn measure_tree<'a>(
    ctx: &mut Ctx,
    direction: Direction,
    nodes: &'a [Block],
    children: &[Vec<usize>],
    root: usize,
    w: f64,
) -> L<'a> {
    if ctx.pack {
        return measure_packed_tree(ctx, direction, nodes, children, root, w);
    }
    const GAP: f64 = 48.0;
    let mut depth = vec![0; nodes.len()];
    let mut order = Vec::new();
    let mut stack = vec![root];
    while let Some(i) = stack.pop() {
        order.push(i);
        for &j in children[i].iter().rev() {
            depth[j] = depth[i] + 1;
            stack.push(j);
        }
    }
    let ranks = depth.iter().max().unwrap() + 1;
    let horizontal = direction.horizontal();
    let mut leaves = vec![1usize; nodes.len()];
    for &i in order.iter().rev() {
        if !children[i].is_empty() {
            leaves[i] = children[i].iter().map(|&j| leaves[j]).sum();
        }
    }
    let lane = (w - 24.0 * (leaves[root] - 1) as f64) / leaves[root] as f64;
    let column = (w - GAP * (ranks - 1) as f64) / ranks as f64;
    let widths: Vec<_> = leaves
        .iter()
        .map(|&n| {
            if horizontal {
                column
            } else {
                lane * n as f64 + 24.0 * (n - 1) as f64
            }
        })
        .collect();
    let measured: Vec<_> = nodes
        .iter()
        .zip(&widths)
        .map(|(n, &w)| measure(ctx, n, w))
        .collect();
    let heights: Vec<_> = measured.iter().map(height).collect();
    let mut extent = heights.clone();
    let mut rank_h = vec![0.0f64; ranks];
    for &i in order.iter().rev() {
        rank_h[depth[i]] = rank_h[depth[i]].max(heights[i]);
        if !children[i].is_empty() {
            extent[i] = extent[i].max(
                children[i].iter().map(|&j| extent[j]).sum::<f64>()
                    + GAP * (children[i].len() - 1) as f64,
            );
        }
    }
    let mut offsets = vec![0.0; ranks];
    for i in 1..ranks {
        offsets[i] = offsets[i - 1] + rank_h[i - 1] + GAP;
    }
    let total_h = if horizontal {
        extent[root]
    } else {
        offsets[ranks - 1] + rank_h[ranks - 1]
    };
    let mut cross = vec![0.0; nodes.len()];
    for &i in &order {
        let child_extent = if horizontal {
            children[i].iter().map(|&j| extent[j]).sum::<f64>()
                + GAP * children[i].len().saturating_sub(1) as f64
        } else {
            widths[i]
        };
        let mut cursor = cross[i]
            + if horizontal {
                (extent[i] - child_extent) / 2.0
            } else {
                0.0
            };
        for &j in &children[i] {
            cross[j] = cursor;
            cursor += if horizontal {
                extent[j] + GAP
            } else {
                widths[j] + 24.0
            };
        }
    }
    let cells = measured
        .into_iter()
        .enumerate()
        .map(|(i, l)| {
            let (x, y) = if horizontal {
                let rank = if direction.reversed() {
                    ranks - depth[i] - 1
                } else {
                    depth[i]
                };
                (
                    rank as f64 * (column + GAP),
                    cross[i] + (extent[i] - heights[i]) / 2.0,
                )
            } else {
                let y = offsets[depth[i]] + (rank_h[depth[i]] - heights[i]) / 2.0;
                (
                    cross[i],
                    if direction.reversed() {
                        total_h - y - heights[i]
                    } else {
                        y
                    },
                )
            };
            (l, x, y, widths[i], heights[i])
        })
        .collect();
    L::Flow { cells, h: total_h }
}

/// Allocate cross-axis subtree space from measured node sizes rather than
/// leaf counts. Uneven branches keep their parent over the occupied extent.
fn measure_packed_tree<'a>(
    ctx: &mut Ctx,
    direction: Direction,
    nodes: &'a [Block],
    children: &[Vec<usize>],
    root: usize,
    w: f64,
) -> L<'a> {
    let horizontal = direction.horizontal();
    let mut depth = vec![0; nodes.len()];
    let mut order = Vec::new();
    let mut stack = vec![root];
    while let Some(i) = stack.pop() {
        order.push(i);
        for &j in children[i].iter().rev() {
            depth[j] = depth[i] + 1;
            stack.push(j);
        }
    }
    let ranks = depth.iter().max().unwrap() + 1;
    let widths: Vec<_> = nodes
        .iter()
        .map(|b| crate::sizing::block_width(b, &ctx.scene.fonts, 48.0))
        .collect();
    let measured: Vec<_> = nodes
        .iter()
        .zip(&widths)
        .map(|(b, &w)| measure(ctx, b, w))
        .collect();
    let heights: Vec<_> = measured.iter().map(height).collect();
    let mut extent: Vec<_> = if horizontal {
        heights.clone()
    } else {
        widths.clone()
    };
    let mut rank_size = vec![0.0_f64; ranks];
    let cross_gap = if horizontal { 48.0 } else { 24.0 };
    for &i in order.iter().rev() {
        rank_size[depth[i]] =
            rank_size[depth[i]].max(if horizontal { widths[i] } else { heights[i] });
        extent[i] = extent[i].max(
            children[i].iter().map(|&j| extent[j]).sum::<f64>()
                + cross_gap * children[i].len().saturating_sub(1) as f64,
        );
    }
    let mut offsets = vec![0.0; ranks];
    for i in 1..ranks {
        offsets[i] = offsets[i - 1] + rank_size[i - 1] + 48.0;
    }
    let flow_size = offsets[ranks - 1] + rank_size[ranks - 1];
    let total_h = if horizontal { extent[root] } else { flow_size };
    let total_w = if horizontal { flow_size } else { extent[root] };
    let mut cross = vec![0.0; nodes.len()];
    for &i in &order {
        let occupied = children[i].iter().map(|&j| extent[j]).sum::<f64>()
            + cross_gap * children[i].len().saturating_sub(1) as f64;
        let mut cursor = cross[i] + (extent[i] - occupied) / 2.0;
        for &j in &children[i] {
            cross[j] = cursor;
            cursor += extent[j] + cross_gap;
        }
    }
    let cells = measured
        .into_iter()
        .enumerate()
        .map(|(i, l)| {
            let flow = offsets[depth[i]]
                + (rank_size[depth[i]] - if horizontal { widths[i] } else { heights[i] }) / 2.0;
            let flow = if direction.reversed() {
                flow_size - flow - if horizontal { widths[i] } else { heights[i] }
            } else {
                flow
            };
            let cross =
                cross[i] + (extent[i] - if horizontal { heights[i] } else { widths[i] }) / 2.0;
            let (x, y) = if horizontal {
                (flow, cross)
            } else {
                (cross, flow)
            };
            (l, x + (w - total_w) / 2.0, y, widths[i], heights[i])
        })
        .collect();
    L::Flow { cells, h: total_h }
}

fn measure_flow<'a>(
    ctx: &mut Ctx,
    direction: Direction,
    layers: &'a [Vec<Block>],
    w: f64,
) -> L<'a> {
    let gutter = ctx
        .machine_gutter
        .map(|g| {
            if direction.horizontal() {
                g.min((w / (2.0 * layers.len().max(1) as f64)).max(1.0))
            } else {
                g
            }
        })
        .unwrap_or(24.0);
    let mut cells = Vec::new();
    let mut total_h: f64 = 0.0;
    if direction.horizontal() {
        let count = layers.len().max(1);
        let packed = ctx.pack;
        let preferred: Vec<_> = layers
            .iter()
            .map(|peers| crate::sizing::blocks_width(peers, &ctx.scene.fonts, gutter))
            .collect();
        let sum = preferred.iter().sum::<f64>();
        let avail = (w - gutter * count.saturating_sub(1) as f64).max(1.0);
        let widths: Vec<_> = preferred
            .iter()
            .map(|&p| {
                if packed {
                    p * (avail / sum.max(1.0)).min(1.0)
                } else {
                    avail / count as f64
                }
            })
            .collect();
        let total_w = widths.iter().sum::<f64>() + gutter * count.saturating_sub(1) as f64;
        let mut offsets = vec![0.0; count];
        let mut cursor = (w - total_w) / 2.0;
        for i in if direction.reversed() {
            (0..count).rev().collect::<Vec<_>>()
        } else {
            (0..count).collect()
        } {
            offsets[i] = cursor;
            cursor += widths[i] + gutter;
        }
        for (rank, peers) in layers.iter().enumerate() {
            let cw = widths[rank];
            let mut y = 0.0;
            for peer in peers {
                let l = measure(ctx, peer, cw);
                let h = height(&l);
                cells.push((l, offsets[rank], y, cw, h));
                y += h + ctx.machine_gutter.map(|_| 64.0).unwrap_or(gutter);
            }
            total_h =
                total_h.max((y - ctx.machine_gutter.map(|_| 64.0).unwrap_or(gutter)).max(0.0));
        }
    } else {
        let order: Vec<_> = if direction.reversed() {
            layers.iter().rev().collect()
        } else {
            layers.iter().collect()
        };
        for peers in order {
            for peer in peers {
                let l = measure(ctx, peer, w);
                let h = height(&l);
                cells.push((l, 0.0, total_h, w, h));
                total_h += h + ctx.machine_gutter.map(|_| 64.0).unwrap_or(BLOCK_GAP);
            }
        }
        total_h = (total_h - ctx.machine_gutter.map(|_| 64.0).unwrap_or(BLOCK_GAP)).max(0.0);
    }
    if direction.horizontal()
        && layers
            .iter()
            .flatten()
            .any(|b| matches!(b, Block::Node(n) if n.style.shape.compact()))
    {
        let mut heights = std::collections::BTreeMap::<i64, f64>::new();
        for (_, dx, dy, _, ch) in &cells {
            let bottom = dy + ch;
            heights
                .entry(*dx as i64)
                .and_modify(|h| *h = h.max(bottom))
                .or_insert(bottom);
        }
        for (_, dx, dy, _, _) in &mut cells {
            *dy += (total_h - heights[&(*dx as i64)]) / 2.0;
        }
    }
    L::Flow { cells, h: total_h }
}

fn measure_row<'a>(ctx: &mut Ctx, r: &'a Row, w: f64) -> L<'a> {
    let gutter = r.gutter.unwrap_or(ROW_GUTTER);
    let n = r.cells.len().max(1);
    let weights: Vec<f64> = if r.weights.is_empty() {
        vec![1.0; n]
    } else {
        r.weights.clone()
    };
    let total: f64 = weights.iter().sum();
    let avail = w - gutter * (n as f64 - 1.0);
    let mut cells = Vec::new();
    let mut x = 0.0;
    let mut h: f64 = 0.0;
    for (cell, wt) in r.cells.iter().zip(weights) {
        let cw = avail * wt / total;
        if let Some(b) = cell {
            let l = measure(ctx, b, cw);
            h = h.max(height(&l));
            cells.push(Some((l, x, cw)));
        } else {
            cells.push(None);
        }
        x += cw + gutter;
    }
    L::Row { cells, h }
}

fn measure_node<'a>(ctx: &mut Ctx, n: &'a Node, w: f64) -> L<'a> {
    let w = ctx.node_width(n, w);
    if n.style.shape.marker()
        || (n.style.shape == Shape::Choice && n.title.is_none() && n.lines.is_empty())
    {
        return L::Node {
            node: n,
            lines: Vec::new(),
            children: Vec::new(),
            natural: w,
            content_bottom: 0.0,
        };
    }
    let (pad_x, mut cursor) = match n.style.shape {
        Shape::Card | Shape::Process | Shape::State => (20.0, 10.0),
        Shape::Decision | Shape::Choice => (w / 4.0 + 10.0, 10.0),
        Shape::Terminal => (30.0, 10.0),
        Shape::Initial | Shape::Final => (0.0, 0.0),
        Shape::Api => (16.0, 10.0),
        Shape::Package => (20.0, 56.0 + 20.0),
        Shape::Container => (20.0, 10.0),
    };
    let inner_w = if n.style.shape.diamond() {
        Outline::Diamond(Rect {
            x: 0.0,
            y: 0.0,
            w,
            h: 100.0,
        })
        .text_area(10.0)
        .w
    } else {
        w - 2.0 * pad_x
    };
    let mut lines: Vec<MLine> = Vec::new();
    let mut prev_kind = 0u8;
    let title = n.title.as_deref();
    let mono_head = n.style.mono;
    let line_no = Some(n.line);

    match n.style.shape {
        Shape::Card
        | Shape::Container
        | Shape::Process
        | Shape::Decision
        | Shape::Terminal
        | Shape::State
        | Shape::Choice => {
            if n.style.shape == Shape::Container {
                if let Some(role) = &n.role {
                    lines.push(MLine {
                        lines: vec![runs(role)],
                        size: 12.5,
                        weight: 400,
                        mono: false,
                        ink: Ink::Muted,
                        ascent: 13.0,
                        advance: 21.0,
                        before: 0.0,
                        tag: None,
                    });
                }
                if let Some(t) = title {
                    lines.push(MLine {
                        lines: vec![runs(t)],
                        size: 14.0,
                        weight: 650,
                        mono: true,
                        ink: Ink::Text,
                        ascent: 11.0,
                        advance: 16.0,
                        before: 0.0,
                        tag: n.tag.clone(),
                    });
                }
            } else {
                let mut first_code = false;
                let head_text = match title {
                    Some(t) => Some(t.to_string()),
                    None => n.lines.iter().find_map(|l| {
                        if let Line::Code(c) = l {
                            first_code = true;
                            Some(c.clone())
                        } else {
                            None
                        }
                    }),
                };
                if let Some(head) = head_text {
                    let mono = mono_head || first_code;
                    let font = if mono { Font::Mono } else { Font::SansBold };
                    let tag_w = n
                        .tag
                        .as_ref()
                        .map(|t| ctx.width(&format!("[{t}] "), Font::SansBold, 15.0, 0.0))
                        .unwrap_or(0.0);
                    let wrapped = wrap_checked(ctx, &head, font, 15.0, inner_w - tag_w, line_no);
                    lines.push(MLine {
                        lines: wrapped,
                        size: 15.0,
                        weight: 640,
                        mono,
                        ink: Ink::Text,
                        ascent: 14.0,
                        advance: 19.0,
                        before: 0.0,
                        tag: n.tag.clone(),
                    });
                    prev_kind = 1;
                }
                let mut skipped_first_code = !first_code;
                for l in &n.lines {
                    match l {
                        Line::Code(c) => {
                            if !skipped_first_code {
                                skipped_first_code = true;
                                continue;
                            }
                            let wrapped = wrap_slack(
                                ctx,
                                c,
                                Font::Mono,
                                12.5,
                                inner_w,
                                if n.style.shape.compact() {
                                    0.0
                                } else {
                                    pad_x - 4.0
                                },
                                line_no,
                            );
                            lines.push(MLine {
                                lines: wrapped,
                                size: 12.5,
                                weight: 400,
                                mono: true,
                                ink: Ink::Code,
                                ascent: 14.0,
                                advance: 18.0,
                                before: if prev_kind == 1 { 2.0 } else { 0.0 },
                                tag: None,
                            });
                            prev_kind = 2;
                        }
                        Line::Sub(s) => {
                            let wrapped = wrap_checked(ctx, s, Font::Sans, 12.5, inner_w, line_no);
                            lines.push(MLine {
                                lines: wrapped,
                                size: 12.5,
                                weight: 400,
                                mono: false,
                                ink: Ink::Muted,
                                ascent: 14.0,
                                advance: 18.0,
                                before: 0.0,
                                tag: None,
                            });
                            prev_kind = 3;
                        }
                        Line::Text(t) => {
                            let wrapped = wrap_checked(ctx, t, Font::Sans, 12.5, inner_w, line_no);
                            let before = match prev_kind {
                                0 => 0.0,
                                4 => 8.0,
                                _ => 10.0,
                            };
                            lines.push(MLine {
                                lines: wrapped,
                                size: 12.5,
                                weight: 400,
                                mono: false,
                                ink: Ink::Muted,
                                ascent: 14.0,
                                advance: 18.0,
                                before,
                                tag: None,
                            });
                            prev_kind = 4;
                        }
                    }
                }
            }
        }
        Shape::Initial | Shape::Final => {}
        Shape::Api => {
            if let Some(t) = title {
                let tag_w = n
                    .tag
                    .as_ref()
                    .map(|t| ctx.width(&format!("[{t}] "), Font::SansBold, 12.5, 0.0))
                    .unwrap_or(0.0);
                let wrapped = wrap_checked(ctx, t, Font::SansBold, 12.5, inner_w - tag_w, line_no);
                lines.push(MLine {
                    lines: wrapped,
                    size: 12.5,
                    weight: 650,
                    mono: false,
                    ink: Ink::Muted,
                    ascent: 14.0,
                    advance: 20.0,
                    before: 0.0,
                    tag: n.tag.clone(),
                });
                prev_kind = 1;
            }
            for l in &n.lines {
                match l {
                    Line::Code(c) => {
                        let wrapped =
                            wrap_slack(ctx, c, Font::Mono, 13.0, inner_w, pad_x - 4.0, line_no);
                        lines.push(MLine {
                            lines: wrapped,
                            size: 13.0,
                            weight: 600,
                            mono: true,
                            ink: Ink::Text,
                            ascent: 15.0,
                            advance: 19.0,
                            before: if prev_kind == 1 { 4.0 } else { 0.0 },
                            tag: None,
                        });
                        prev_kind = 2;
                    }
                    Line::Sub(s) | Line::Text(s) => {
                        let wrapped = wrap_checked(ctx, s, Font::Sans, 12.0, inner_w, line_no);
                        lines.push(MLine {
                            lines: wrapped,
                            size: 12.0,
                            weight: 400,
                            mono: false,
                            ink: Ink::Muted,
                            ascent: 14.0,
                            advance: 17.0,
                            before: 4.0,
                            tag: None,
                        });
                        prev_kind = 3;
                    }
                }
            }
        }
        Shape::Package => {
            for l in &n.lines {
                let (text, font, size, mono, ink) = match l {
                    Line::Code(c) => (c, Font::Mono, 12.5, true, Ink::Code),
                    Line::Sub(s) | Line::Text(s) => (s, Font::Sans, 12.5, false, Ink::Muted),
                };
                let wrapped = wrap_checked(ctx, text, font, size, inner_w, line_no);
                lines.push(MLine {
                    lines: wrapped,
                    size,
                    weight: 400,
                    mono,
                    ink,
                    ascent: 14.0,
                    advance: 18.0,
                    before: 0.0,
                    tag: None,
                });
            }
        }
    }

    for ml in &lines {
        cursor += ml.before + ml.advance * ml.lines.len() as f64;
    }
    let content_bottom = cursor;
    let is_container = n.is_container();
    if !lines.is_empty() && !n.children.is_empty() {
        cursor += if n.is_composite() {
            40.0
        } else if is_container {
            6.0
        } else {
            12.0
        };
    }
    if n.style.shape == Shape::Container && lines.is_empty() {
        cursor = 20.0;
    }

    let mut children = Vec::new();
    for (i, c) in n.children.iter().enumerate() {
        let l = measure(ctx, c, inner_w - if n.is_composite() { 64.0 } else { 0.0 });
        let h = height(&l);
        cursor += gap_before(&l, i == 0) + h;
        children.push((l, h));
    }
    let bottom_pad = match n.style.shape {
        Shape::Card
        | Shape::Api
        | Shape::Process
        | Shape::Decision
        | Shape::Terminal
        | Shape::State
        | Shape::Initial
        | Shape::Final
        | Shape::Choice => 10.0,
        Shape::Package | Shape::Container => {
            if n.children.is_empty() {
                14.0
            } else {
                20.0
            }
        }
    };
    let bottom_pad = if n.is_composite() { 52.0 } else { bottom_pad };
    let mut natural = (cursor + bottom_pad).max(if is_container { 40.0 } else { 30.0 });
    if n.style.shape.diamond() {
        natural *= 2.0;
    }
    if n.style.shape == Shape::State {
        natural = natural.max(48.0);
    }
    L::Node {
        node: n,
        lines,
        children,
        natural,
        content_bottom,
    }
}

fn wrap_checked(
    ctx: &mut Ctx,
    text: &str,
    font: Font,
    size: f64,
    max_w: f64,
    line: Option<usize>,
) -> Vec<Vec<Run>> {
    wrap_slack(ctx, text, font, size, max_w, 0.0, line)
}

/// Wraps at `max_w` but only warns when a line also exceeds `max_w + slack`,
/// so a long identifier may run into the padding without a complaint.
fn wrap_slack(
    ctx: &mut Ctx,
    text: &str,
    font: Font,
    size: f64,
    max_w: f64,
    slack: f64,
    line: Option<usize>,
) -> Vec<Vec<Run>> {
    let sans = if font == Font::SansBold {
        Font::SansBold
    } else {
        Font::Sans
    };
    let lines = if font == Font::Mono {
        // Code is never re-flowed inside words; wrap at spaces only.
        ctx.wrap(&format!("`{}`", text.replace('`', "")), sans, size, max_w)
    } else {
        ctx.wrap(text, sans, size, max_w)
    };
    for l in &lines {
        let w = ctx.runs_width(l, sans, size);
        if w > max_w + slack + 0.5 {
            let s: String = l.iter().map(|r| r.text.as_str()).collect();
            ctx.warn(
                line,
                format!(
                    "text overflows its box by {:.0}px: \"{}\"",
                    w - max_w - slack,
                    s
                ),
            );
        }
    }
    lines
}

fn draw(ctx: &mut Ctx, l: &L, x: f64, y: f64, w: f64, h: f64) {
    match l {
        L::Flow { cells, .. } => {
            for (l, dx, dy, cw, ch) in cells {
                draw(ctx, l, x + dx, y + dy, *cw, *ch);
            }
        }
        L::Node {
            node,
            lines,
            children,
            natural,
            content_bottom,
        } => draw_node(
            ctx,
            node,
            lines,
            children,
            *natural,
            *content_bottom,
            x,
            y,
            w,
            h,
        ),
        L::Row { cells, h } => {
            for (cl, cx, cw) in cells.iter().flatten() {
                draw(ctx, cl, x + cx, y, *cw, *h);
            }
        }
        L::Section {
            section, children, ..
        } => {
            let mut cy = y;
            if section.band {
                let label = section.label.to_uppercase();
                let tw = ctx.width(&label, Font::SansBold, 11.5, 0.07);
                let (tx, anchor) = logical_start(x, w, ctx.text_direction, &label);
                ctx.scene.keepout.push(Rect {
                    x: if anchor == Anchor::End { tx - tw } else { tx },
                    y: y + 2.0,
                    w: tw,
                    h: 16.0,
                });
                ctx.push(Item::Text(TextItem {
                    x: tx,
                    y: y + 14.0,
                    anchor,
                    direction: crate::text::Direction::Auto,
                    runs: runs(&label),
                    size: 11.5,
                    weight: 640,
                    mono: false,
                    ink: Ink::Muted,
                    letter_spacing: 0.07,
                }));
                cy += 22.0;
            } else {
                let tw = ctx.width(&section.label, Font::SansBold, 12.5, 0.08);
                let (tx, anchor) = logical_start(x, w, ctx.text_direction, &section.label);
                ctx.scene.keepout.push(Rect {
                    x: if anchor == Anchor::End { tx - tw } else { tx },
                    y: y + 12.0,
                    w: tw,
                    h: 17.0,
                });
                ctx.push(Item::Rule {
                    x1: x,
                    y1: y,
                    x2: x + w,
                    y2: y,
                    tone: None,
                    dashed: false,
                });
                ctx.push(Item::Text(TextItem {
                    x: tx,
                    y: y + 25.0,
                    anchor,
                    direction: crate::text::Direction::Auto,
                    runs: runs(&section.label),
                    size: 12.5,
                    weight: 680,
                    mono: false,
                    ink: Ink::Muted,
                    letter_spacing: 0.08,
                }));
                cy += 40.0;
            }
            for (i, (cl, ch)) in children.iter().enumerate() {
                cy += gap_before(cl, i == 0);
                draw(ctx, cl, x, cy, w, *ch);
                cy += ch;
            }
        }
        L::Text { lines, .. } => {
            let mut ty = y + 14.0;
            let paragraph: String = lines.iter().flatten().map(|r| r.text.as_str()).collect();
            let direction = ctx.text_direction.resolve(&paragraph);
            let (tx, anchor) = logical_start(x, w, direction, &paragraph);
            for line in lines {
                ctx.push(Item::Text(TextItem {
                    x: tx,
                    y: ty,
                    anchor,
                    direction,
                    runs: line.clone(),
                    size: 12.5,
                    weight: 400,
                    mono: false,
                    ink: Ink::Muted,
                    letter_spacing: 0.0,
                }));
                ty += 20.0;
            }
        }
        L::Divider { text, tone, .. } => {
            let my = y + h / 2.0;
            ctx.push(Item::Rule {
                x1: x,
                y1: my,
                x2: x + w,
                y2: my,
                tone: Some(*tone),
                dashed: true,
            });
            if let Some(t) = text {
                let tw = ctx.width(t, Font::Sans, 12.0, 0.0) + 32.0;
                let rect = Rect {
                    x: x + w / 2.0 - tw / 2.0,
                    y: my - 11.0,
                    w: tw,
                    h: 22.0,
                };
                ctx.scene.keepout.push(rect);
                ctx.push(Item::Chip {
                    rect,
                    text: t.to_string(),
                    tone: *tone,
                    rotate: false,
                    bordered: false,
                });
            }
        }
        L::Gap(_) => {}
        L::Legend { entries, rows, .. } => {
            let mut ly = y;
            for row in rows {
                let mut lx = x;
                for &i in row {
                    draw_legend_entry(ctx, &entries[i], lx, ly);
                    lx += entries[i].w + 16.0;
                }
                ly += 34.0;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_node(
    ctx: &mut Ctx,
    n: &Node,
    lines: &[MLine],
    children: &[(L, f64)],
    natural: f64,
    content_bottom: f64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let compact = n.style.shape.compact() && !n.is_composite();
    let nw = ctx.node_width(n, w);
    let (x, w) = (x + (w - nw) / 2.0, nw);
    // Compact shapes keep their measured height in a stretched row and center
    // inside the allocated cell. Legacy cards still stretch with their peers.
    let (y, h) = if compact {
        (y + (h - natural) / 2.0, natural)
    } else {
        (y, h)
    };
    let saved_direction = ctx.text_direction;
    ctx.text_direction = n.text_direction.unwrap_or(saved_direction);
    let rect = Rect { x, y, w, h };
    let outline = match n.style.shape {
        Shape::Decision | Shape::Choice => Outline::Diamond(rect),
        Shape::Terminal => Outline::Rounded {
            rect,
            radius: (h / 2.0).min(24.0),
        },
        Shape::Process | Shape::State => Outline::Rounded { rect, radius: 7.0 },
        Shape::Initial | Shape::Final => Outline::Rounded {
            rect,
            radius: h / 2.0,
        },
        _ => Outline::Rectangle(rect),
    };
    let idx = ctx.scene.nodes.len();
    ctx.scene.nodes.push(NodeRect {
        id: n.id.clone(),
        kind: n.kind.clone(),
        rect,
        outline,
        parent: ctx.parent,
        tone: n.style.tone,
        href: n.href.clone(),
        line: n.line,
    });
    if n.style.shape.marker() {
        ctx.push_for(
            idx,
            Item::StateMarker {
                rect,
                tone: n.style.tone,
                final_state: n.style.shape == Shape::Final,
            },
        );
        ctx.text_direction = saved_direction;
        return;
    }
    let (pad_x, rx, stroke_width) = match n.style.shape {
        Shape::Card | Shape::Process | Shape::State => (20.0, 7.0, 1.25),
        Shape::Decision | Shape::Choice => (w / 4.0 + 10.0, 0.0, 1.25),
        Shape::Terminal => (30.0, (h / 2.0).min(24.0), 1.25),
        Shape::Initial | Shape::Final => (0.0, 0.0, 1.25),
        Shape::Api => (16.0, 6.0, 1.3),
        Shape::Package => (20.0, 10.0, 1.25),
        Shape::Container => (20.0, 8.0, 1.5),
    };
    let white = n.style.shape == Shape::Package;
    ctx.push_for(
        idx,
        if n.style.shape.diamond() {
            Item::Diamond {
                rect,
                tone: n.style.tone,
                hollow: n.style.hollow,
                stroke_width,
            }
        } else {
            Item::Box {
                rect,
                tone: n.style.tone,
                hollow: n.style.hollow || n.is_composite(),
                white,
                rx,
                stroke_width,
            }
        },
    );
    let mut cursor = y;
    match n.style.shape {
        Shape::Package => {
            ctx.push_for(
                idx,
                Item::Strip {
                    rect: Rect { x, y, w, h: 56.0 },
                    rx,
                },
            );
            ctx.push_for(
                idx,
                Item::Rule {
                    x1: x,
                    y1: y + 56.0,
                    x2: x + w,
                    y2: y + 56.0,
                    tone: None,
                    dashed: false,
                },
            );
            if let Some(t) = &n.title {
                let mut rs = Vec::new();
                if let Some(tag) = &n.tag {
                    rs.push(Run::tag(format!("[{tag}] ")));
                }
                rs.push(Run {
                    text: t.clone(),
                    code: n.style.mono,
                    tag: false,
                });
                let (tx, anchor) = logical_start(x + 25.0, w - 50.0, ctx.text_direction, t);
                ctx.push_for(
                    idx,
                    Item::Text(TextItem {
                        x: tx,
                        y: y + 36.0,
                        anchor,
                        direction: crate::text::Direction::Auto,
                        runs: rs,
                        size: 18.0,
                        weight: 680,
                        mono: false,
                        ink: Ink::Text,
                        letter_spacing: 0.0,
                    }),
                );
            }
            cursor += 76.0;
        }
        Shape::Decision | Shape::Choice => cursor = outline.text_area(0.0).y + 10.0,
        Shape::Card
        | Shape::Container
        | Shape::Api
        | Shape::Process
        | Shape::Terminal
        | Shape::State
        | Shape::Initial
        | Shape::Final => {
            cursor += if n.style.shape == Shape::State && !n.is_composite() {
                (h - (content_bottom - 10.0)) / 2.0
            } else {
                10.0
            };
        }
    }
    let inner_w = w - 2.0 * pad_x;
    for ml in lines {
        cursor += ml.before;
        let paragraph: String = ml.lines.iter().flatten().map(|r| r.text.as_str()).collect();
        let direction = ctx.text_direction.resolve(&paragraph);
        for (i, line) in ml.lines.iter().enumerate() {
            let mut rs = line.clone();
            if i == 0
                && let Some(tag) = &ml.tag
            {
                rs.insert(0, Run::tag(format!("[{tag}] ")));
            }
            let rtl = direction == crate::text::Direction::Rtl;
            let (tx, anchor) = match n.style.align {
                Align::Start if rtl => (x + w - pad_x, Anchor::End),
                Align::Start | Align::Left => (x + pad_x, Anchor::Start),
                Align::Right => (x + w - pad_x, Anchor::End),
                Align::Center => (x + w / 2.0, Anchor::Middle),
            };
            ctx.push_for(
                idx,
                Item::Text(TextItem {
                    x: tx,
                    y: cursor + ml.ascent,
                    anchor,
                    direction,
                    runs: rs,
                    size: ml.size,
                    weight: ml.weight,
                    mono: ml.mono,
                    ink: ml.ink,
                    letter_spacing: 0.0,
                }),
            );
            cursor += ml.advance;
        }
    }
    let _ = (natural, content_bottom);
    if n.is_composite() {
        let rule_y = y + content_bottom + 8.0;
        ctx.push_for(
            idx,
            Item::Rule {
                x1: x,
                y1: rule_y,
                x2: x + w,
                y2: rule_y,
                tone: Some(n.style.tone),
                dashed: false,
            },
        );
        ctx.scene.keepout.push(Rect {
            x,
            y,
            w,
            h: content_bottom + 12.0,
        });
    }
    if !children.is_empty() {
        let is_container = n.is_container();
        if !lines.is_empty() {
            cursor += if n.is_composite() {
                40.0
            } else if is_container {
                6.0
            } else {
                12.0
            };
        } else if n.style.shape == Shape::Container {
            cursor = y + 20.0;
        }
        let saved = ctx.parent;
        ctx.parent = Some(idx);
        for (i, (cl, ch)) in children.iter().enumerate() {
            cursor += gap_before(cl, i == 0);
            draw(
                ctx,
                cl,
                x + pad_x + if n.is_composite() { 32.0 } else { 0.0 },
                cursor,
                inner_w - if n.is_composite() { 64.0 } else { 0.0 },
                *ch,
            );
            cursor += ch;
        }
        ctx.parent = saved;
    }
    ctx.text_direction = saved_direction;
}

#[derive(Debug, Clone)]
pub struct LegendEntry {
    pub label: String,
    /// `Some(kind)` draws a node swatch of that kind; `None` draws an arrow sample.
    pub kind: Option<(Tone, bool, bool)>,
    pub arrow: Option<(Tone, bool)>,
    pub w: f64,
}

fn legend_entries(d: &Diagram, l: &Legend, fonts: &Fonts) -> (Vec<LegendEntry>, bool) {
    let mut used_kinds: Vec<String> = Vec::new();
    let mut used_arrows: Vec<(String, Tone, Vec<String>)> = Vec::new();
    fn walk(b: &Block, out: &mut Vec<String>) {
        match b {
            Block::Node(n) => {
                if !out.contains(&n.kind) {
                    out.push(n.kind.clone());
                }
                n.children.iter().for_each(|c| walk(c, out));
            }
            Block::Row(r) => r.cells.iter().flatten().for_each(|c| walk(c, out)),
            Block::Section(s) => s.children.iter().for_each(|c| walk(c, out)),
            Block::Tree { nodes, .. } => nodes.iter().for_each(|c| walk(c, out)),
            Block::Flow { layers, .. } => layers.iter().flatten().for_each(|c| walk(c, out)),
            _ => {}
        }
    }
    d.blocks.iter().for_each(|b| walk(b, &mut used_kinds));
    fn find<'a>(b: &'a Block, id: &str) -> Option<&'a Node> {
        match b {
            Block::Node(n) => {
                if n.id == id {
                    return Some(n);
                }
                n.children.iter().find_map(|c| find(c, id))
            }
            Block::Row(r) => r.cells.iter().flatten().find_map(|c| find(c, id)),
            Block::Section(s) => s.children.iter().find_map(|c| find(c, id)),
            Block::Tree { nodes, .. } => nodes.iter().find_map(|c| find(c, id)),
            Block::Flow { layers, .. } => layers.iter().flatten().find_map(|c| find(c, id)),
            _ => None,
        }
    }
    for e in &d.edges {
        let style = &d.arrows[&e.kind];
        let target = d.blocks.iter().find_map(|b| find(b, &e.to));
        let tone = match (e.tone, style.color) {
            (Some(t), _) => t,
            (None, ArrowColor::Tone(t)) => t,
            (None, ArrowColor::Inherit) => target.map(|n| n.style.tone).unwrap_or(Tone::Gray),
        };
        let target_kind = if style.color == ArrowColor::Inherit {
            target.map(|n| n.kind.clone())
        } else {
            None
        };
        match used_arrows
            .iter_mut()
            .find(|(k, t, _)| *k == e.kind && *t == tone)
        {
            Some((_, _, kinds)) => {
                if let Some(tk) = target_kind
                    && !kinds.contains(&tk)
                {
                    kinds.push(tk);
                }
            }
            None => used_arrows.push((e.kind.clone(), tone, target_kind.into_iter().collect())),
        }
    }
    let kinds: Vec<String> = if l.nodes.is_empty() {
        used_kinds
    } else {
        l.nodes.clone()
    };
    let mut entries = Vec::new();
    for k in kinds {
        let Some(style) = d.kinds.get(&k) else {
            continue;
        };
        let Some(label) = &style.label else { continue };
        let w = width_with_fonts(label, Font::Sans, 12.5, 0.0, fonts) + 28.0;
        entries.push(LegendEntry {
            label: label.clone(),
            kind: Some((style.tone, style.hollow, style.shape == Shape::Package)),
            arrow: None,
            w,
        });
    }
    let arrows: Vec<(String, Tone, Vec<String>)> = if l.arrows.is_empty() {
        used_arrows
    } else {
        used_arrows
            .into_iter()
            .filter(|(k, _, _)| l.arrows.contains(k))
            .collect()
    };
    for (k, tone, target_kinds) in arrows {
        let style = &d.arrows[&k];
        let base = style.label.clone().unwrap_or_else(|| k.clone());
        let label = if style.color == ArrowColor::Inherit {
            // Several kinds may share the tone; name the filled one, since
            // the edge color reads as that node's fill.
            let styles: Vec<&NodeStyle> = target_kinds
                .iter()
                .filter_map(|tk| d.kinds.get(tk))
                .filter(|s| s.label.is_some())
                .collect();
            let target_label = styles
                .iter()
                .find(|s| !s.hollow)
                .or(styles.first())
                .and_then(|s| s.label.clone())
                .or_else(|| {
                    d.kinds
                        .values()
                        .find(|s| s.tone == tone && s.label.is_some())
                        .and_then(|s| s.label.clone())
                })
                .unwrap_or_else(|| tone.name().to_string());
            format!("{base} {target_label}")
        } else {
            base
        };
        let w = width_with_fonts(&label, Font::Sans, 12.5, 0.0, fonts) + 42.0;
        entries.push(LegendEntry {
            label,
            kind: None,
            arrow: Some((tone, style.dashed)),
            w,
        });
    }
    (entries, l.bottom)
}

fn pack_legend(entries: &[LegendEntry], w: f64) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut cw = 0.0;
    let mut cur_is_node: Option<bool> = None;
    for (i, e) in entries.iter().enumerate() {
        let is_node = e.kind.is_some();
        let extra = if cur.is_empty() { e.w } else { e.w + 16.0 };
        if !cur.is_empty() && (cw + extra > w || cur_is_node != Some(is_node)) {
            rows.push(std::mem::take(&mut cur));
            cw = 0.0;
        }
        cw += if cur.is_empty() { e.w } else { e.w + 16.0 };
        cur.push(i);
        cur_is_node = Some(is_node);
    }
    if !cur.is_empty() {
        rows.push(cur);
    }
    rows
}

fn draw_legend_entry(ctx: &mut Ctx, e: &LegendEntry, x: f64, y: f64) {
    if let Some((tone, hollow, white)) = e.kind {
        let rect = Rect {
            x,
            y,
            w: e.w,
            h: 28.0,
        };
        ctx.push(Item::Box {
            rect,
            tone,
            hollow,
            white,
            rx: 6.0,
            stroke_width: 1.3,
        });
        ctx.push(Item::Text(TextItem {
            x: x + e.w / 2.0,
            y: y + 18.5,
            anchor: Anchor::Middle,
            direction: crate::text::Direction::Auto,
            runs: runs(&e.label),
            size: 12.5,
            weight: 400,
            mono: false,
            ink: Ink::Muted,
            letter_spacing: 0.0,
        }));
    } else if let Some((tone, dashed)) = e.arrow {
        ctx.push(Item::Sample {
            x,
            y: y + 14.0,
            tone,
            dashed,
        });
        ctx.push(Item::Text(TextItem {
            x: x + 42.0,
            y: y + 18.5,
            anchor: Anchor::Start,
            direction: crate::text::Direction::Auto,
            runs: runs(&e.label),
            size: 12.5,
            weight: 400,
            mono: false,
            ink: Ink::Muted,
            letter_spacing: 0.0,
        }));
    }
}
