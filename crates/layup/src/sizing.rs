//! Font-aware preferred widths. Explicit canvas widths and manual presets
//! retain authored geometry; automatic canvases grow to fit each region.
use crate::{
    model::{Block, Diagram, Line, Mode, Node, Preset},
    style::Shape,
    text::{Font, Fonts, runs, runs_width_with_fonts},
};

fn text_width(s: &str, font: Font, size: f64, fonts: &Fonts) -> f64 {
    s.lines()
        .map(|line| runs_width_with_fonts(&runs(line), font, size, fonts))
        .fold(0.0, f64::max)
}

pub(crate) fn node_width(n: &Node, fonts: &Fonts, gap: f64) -> f64 {
    if n.style.shape.marker()
        || (n.style.shape == Shape::Choice && n.title.is_none() && n.lines.is_empty())
    {
        return n.body_width(0.0);
    }
    let (head_font, head_size) = match n.style.shape {
        Shape::Package => (Font::SansBold, 18.0),
        Shape::Container => (Font::Mono, 14.0),
        Shape::Api => (Font::SansBold, 12.5),
        _ => (
            if n.style.mono {
                Font::Mono
            } else {
                Font::SansBold
            },
            15.0,
        ),
    };
    let mut text = n
        .title
        .as_deref()
        .map_or(0.0, |t| text_width(t, head_font, head_size, fonts));
    if let Some(tag) = &n.tag {
        text += text_width(&format!("[{tag}] "), Font::SansBold, head_size, fonts);
    }
    if let Some(role) = &n.role {
        text = text.max(text_width(role, Font::Sans, 12.5, fonts));
    }
    for line in &n.lines {
        let (s, font) = match line {
            Line::Code(s) => (s, Font::Mono),
            Line::Sub(s) | Line::Text(s) => (s, Font::Sans),
        };
        text = text.max(text_width(s, font, 12.5, fonts));
    }
    let preferred = if n.style.shape.diamond() {
        (2.0 * (text + 20.0)).max(120.0)
    } else {
        (text
            + if n.style.shape == Shape::Terminal {
                60.0
            } else {
                40.0
            })
        .max(88.0)
    };
    if n.is_container() {
        preferred.max(
            blocks_width(&n.children, fonts, gap)
                + 40.0
                + if n.is_composite() { 64.0 } else { 0.0 },
        )
    } else if n.style.shape.compact() {
        n.body_width(preferred)
    } else {
        preferred.clamp(220.0, 420.0)
    }
}

pub(crate) fn blocks_width(blocks: &[Block], fonts: &Fonts, gap: f64) -> f64 {
    blocks
        .iter()
        .map(|b| block_width(b, fonts, gap))
        .fold(0.0, f64::max)
}

pub(crate) fn block_width(b: &Block, fonts: &Fonts, gap: f64) -> f64 {
    match b {
        Block::Node(n) => node_width(n, fonts, gap),
        Block::Row(r) => {
            let total = if r.weights.is_empty() {
                r.cells.len() as f64
            } else {
                r.weights.iter().sum()
            };
            r.cells
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    c.as_ref().map_or(88.0, |b| block_width(b, fonts, gap)) * total
                        / r.weights.get(i).copied().unwrap_or(1.0)
                })
                .fold(0.0, f64::max)
                + r.gutter.unwrap_or(12.0) * r.cells.len().saturating_sub(1) as f64
        }
        Block::Section(s) => blocks_width(&s.children, fonts, gap).max(text_width(
            &s.label,
            Font::SansBold,
            12.5,
            fonts,
        )),
        Block::Tree {
            direction,
            nodes,
            children,
            root,
        } => {
            let widths: Vec<_> = nodes.iter().map(|n| block_width(n, fonts, gap)).collect();
            let mut stack = vec![(*root, 0)];
            let mut order = Vec::new();
            let mut rank_widths = Vec::<f64>::new();
            while let Some((i, depth)) = stack.pop() {
                if rank_widths.len() <= depth {
                    rank_widths.push(0.0);
                }
                rank_widths[depth] = rank_widths[depth].max(widths[i]);
                order.push(i);
                stack.extend(children[i].iter().map(|&j| (j, depth + 1)));
            }
            if direction.horizontal() {
                rank_widths.iter().sum::<f64>() + 48.0 * rank_widths.len().saturating_sub(1) as f64
            } else {
                let mut extents = widths;
                for &i in order.iter().rev() {
                    extents[i] = extents[i].max(
                        children[i].iter().map(|&j| extents[j]).sum::<f64>()
                            + 24.0 * children[i].len().saturating_sub(1) as f64,
                    );
                }
                extents[*root]
            }
        }
        Block::Flow { direction, layers } => {
            if direction.horizontal() {
                layers
                    .iter()
                    .map(|l| blocks_width(l, fonts, gap))
                    .sum::<f64>()
                    + gap * layers.len().saturating_sub(1) as f64
            } else {
                layers
                    .iter()
                    .map(|l| blocks_width(l, fonts, gap))
                    .fold(0.0, f64::max)
            }
        }
        Block::Text(_) => 280.0,
        Block::Divider { text, .. } => text
            .as_deref()
            .map_or(0.0, |t| text_width(t, Font::Sans, 12.0, fonts) + 32.0),
        Block::Gap(_) => 0.0,
    }
}

pub(crate) fn machine_gap(d: &Diagram, fonts: &Fonts) -> f64 {
    d.edges
        .iter()
        .filter_map(|e| e.label.as_deref())
        .map(|t| text_width(t, Font::SansBold, 11.5, fonts) + 30.0)
        .fold(64.0, f64::max)
        .min(160.0)
}

pub(crate) fn fit(d: &mut Diagram, fonts: &Fonts) {
    if d.width_set || !d.auto_layout || d.preset != Preset::Clean {
        return;
    }
    let gap = if d.mode == Mode::StateMachine {
        machine_gap(d, fonts)
    } else {
        24.0
    };
    let body = blocks_width(&d.blocks, fonts, gap);
    let caption = d
        .edges
        .iter()
        .filter_map(|e| e.label.as_deref())
        .map(|t| text_width(t, Font::SansBold, 11.5, fonts) + 80.0)
        .fold(0.0, f64::max);
    let header = text_width(&d.title, Font::SansBold, 24.0, fonts);
    let lanes = d.edges.iter().filter(|e| e.via.is_some()).count() as f64 * 30.0;
    d.width = (body.max(caption).max(header) + 100.0 + lanes)
        .max(900.0)
        .ceil();
}
