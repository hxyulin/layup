//! Palette, node shapes and the built-in kind presets.
//!
//! Colors are GitHub Primer's light palette so diagrams sit naturally inside
//! rendered Markdown. Each tone has a soft fill, a border and a strong ink
//! used for text and edges of that tone.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tone {
    Gray,
    Blue,
    Green,
    Yellow,
    Purple,
    Orange,
    Red,
}

impl Tone {
    pub fn parse(s: &str) -> Option<Tone> {
        Some(match s {
            "gray" | "grey" | "neutral" => Tone::Gray,
            "blue" => Tone::Blue,
            "green" => Tone::Green,
            "yellow" | "amber" => Tone::Yellow,
            "purple" | "violet" => Tone::Purple,
            "orange" => Tone::Orange,
            "red" => Tone::Red,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Tone::Gray => "gray",
            Tone::Blue => "blue",
            Tone::Green => "green",
            Tone::Yellow => "yellow",
            Tone::Purple => "purple",
            Tone::Orange => "orange",
            Tone::Red => "red",
        }
    }

    pub const ALL: [Tone; 7] = [
        Tone::Gray,
        Tone::Blue,
        Tone::Green,
        Tone::Yellow,
        Tone::Purple,
        Tone::Orange,
        Tone::Red,
    ];

    /// (fill, border, ink) for the light theme.
    pub fn light(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Tone::Gray => ("#f6f8fa", "#8c959f", "#57606a"),
            Tone::Blue => ("#ddf4ff", "#54aeff", "#0969da"),
            Tone::Green => ("#dafbe1", "#4ac26b", "#1a7f37"),
            Tone::Yellow => ("#fff8c5", "#d4a72c", "#9a6700"),
            Tone::Purple => ("#fbefff", "#c297ff", "#8250df"),
            Tone::Orange => ("#fff1e5", "#ffb77c", "#eb6910"),
            Tone::Red => ("#ffebe9", "#ff8182", "#cf222e"),
        }
    }

    /// (fill, border, ink) for the dark theme.
    pub fn dark(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Tone::Gray => ("#21262d", "#6e7681", "#8b949e"),
            Tone::Blue => ("#0c2d6b", "#1f6feb", "#58a6ff"),
            Tone::Green => ("#0f3a1e", "#238636", "#3fb950"),
            Tone::Yellow => ("#3b2300", "#9e6a03", "#d29922"),
            Tone::Purple => ("#2e1a5e", "#8957e5", "#a371f7"),
            Tone::Orange => ("#3d1e00", "#bd561d", "#f0883e"),
            Tone::Red => ("#4a1015", "#da3633", "#f85149"),
        }
    }
}

/// Tones assigned in document order to nodes that omit a tone.
/// Gray is reserved for structure and red for emphasis.
pub const AUTO_CYCLE: [Tone; 5] = [
    Tone::Blue,
    Tone::Green,
    Tone::Yellow,
    Tone::Purple,
    Tone::Orange,
];

/// How a node draws itself and lays out its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Left-aligned head line, optional code and prose lines. Tone fill.
    Card,
    /// Centered title over centered code; the small typed API box.
    Api,
    /// White frame with a gray head strip and a mono name; holds children.
    Package,
    /// Hollow rounded container with a role label and a mono name; holds children.
    Container,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
}

#[derive(Debug, Clone)]
pub struct NodeStyle {
    pub shape: Shape,
    pub tone: Tone,
    /// True when no explicit tone was given: the model assigns the next
    /// tone from `AUTO_CYCLE` in document order.
    pub auto: bool,
    pub hollow: bool,
    pub align: Align,
    /// Head line in the code font.
    pub mono: bool,
    /// Default role label for containers, e.g. "lib crate".
    pub role: Option<String>,
    /// Legend label.
    pub label: Option<String>,
}

pub fn presets() -> BTreeMap<String, NodeStyle> {
    let mut m = BTreeMap::new();
    let card = NodeStyle {
        shape: Shape::Card,
        tone: Tone::Gray,
        auto: true,
        hollow: false,
        align: Align::Left,
        mono: false,
        role: None,
        label: None,
    };
    m.insert("card".into(), card.clone());
    m.insert("node".into(), card.clone());
    m.insert(
        "package".into(),
        NodeStyle {
            shape: Shape::Package,
            tone: Tone::Gray,
            auto: false,
            mono: true,
            ..card.clone()
        },
    );
    m.insert(
        "crate".into(),
        NodeStyle {
            shape: Shape::Container,
            tone: Tone::Blue,
            auto: false,
            hollow: true,
            mono: true,
            role: Some("lib crate".into()),
            label: Some("crate target".into()),
            ..card.clone()
        },
    );
    m.insert(
        "group".into(),
        NodeStyle {
            shape: Shape::Container,
            tone: Tone::Gray,
            auto: false,
            hollow: true,
            ..card.clone()
        },
    );
    let api = NodeStyle {
        shape: Shape::Api,
        align: Align::Center,
        ..card.clone()
    };
    m.insert(
        "trait".into(),
        NodeStyle {
            tone: Tone::Purple,
            auto: false,
            label: Some("trait".into()),
            ..api.clone()
        },
    );
    m.insert(
        "type".into(),
        NodeStyle {
            tone: Tone::Green,
            auto: false,
            label: Some("concrete type".into()),
            ..api.clone()
        },
    );
    m.insert(
        "module".into(),
        NodeStyle {
            tone: Tone::Blue,
            auto: false,
            label: Some("module-like".into()),
            ..api.clone()
        },
    );
    m.insert("api".into(), api);
    m
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowColor {
    Tone(Tone),
    /// Take the color from the target node's tone (used for re-export edges).
    Inherit,
}

#[derive(Debug, Clone)]
pub struct ArrowStyle {
    pub color: ArrowColor,
    pub dashed: bool,
    pub label: Option<String>,
    /// Default chip text drawn on every edge of this kind unless overridden.
    pub chip: Option<String>,
}

pub fn arrow_presets() -> BTreeMap<String, ArrowStyle> {
    let mut m = BTreeMap::new();
    m.insert(
        "default".into(),
        ArrowStyle {
            color: ArrowColor::Tone(Tone::Gray),
            dashed: false,
            label: None,
            chip: None,
        },
    );
    m.insert(
        "impl".into(),
        ArrowStyle {
            color: ArrowColor::Tone(Tone::Orange),
            dashed: false,
            label: Some("implements".into()),
            chip: Some("impl".into()),
        },
    );
    m.insert(
        "extends".into(),
        ArrowStyle {
            color: ArrowColor::Tone(Tone::Purple),
            dashed: false,
            label: Some("extends".into()),
            chip: Some("extends".into()),
        },
    );
    m.insert(
        "uses".into(),
        ArrowStyle {
            color: ArrowColor::Tone(Tone::Blue),
            dashed: false,
            label: Some("uses".into()),
            chip: Some("uses".into()),
        },
    );
    m.insert(
        "exports".into(),
        ArrowStyle {
            color: ArrowColor::Inherit,
            dashed: true,
            label: Some("exports".into()),
            chip: Some("exports".into()),
        },
    );
    m.insert(
        "depends".into(),
        ArrowStyle {
            color: ArrowColor::Tone(Tone::Gray),
            dashed: false,
            label: Some("depends on".into()),
            chip: None,
        },
    );
    m.insert(
        "flow".into(),
        ArrowStyle {
            color: ArrowColor::Tone(Tone::Gray),
            dashed: false,
            label: None,
            chip: None,
        },
    );
    m
}

/// Edge stroke colors: the gray default is the Primer `fg.muted` line color,
/// stronger than the gray tone ink so arrows stay visible on gray cards.
pub fn edge_color(tone: Tone, dark: bool) -> &'static str {
    match (tone, dark) {
        (Tone::Gray, false) => "#6e7781",
        (Tone::Gray, true) => "#8b949e",
        (t, false) => t.light().2,
        (t, true) => t.dark().2,
    }
}
