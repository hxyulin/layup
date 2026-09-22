//! SVG emitter. Output is self-contained: fonts are embedded, colors
//! live in CSS custom properties on the root so a host page (or the
//! `auto` theme's media query) can restyle it, and every node and edge
//! carries `data-` attributes for interactive hosts.

use std::fmt::Write as _;

use crate::Compiled;
use crate::Theme;
use crate::layout::{Anchor, Ink, Item, Placed, Rect, Scene, TextItem};
use crate::style::{Tone, edge_color};
use crate::text::Run;

pub fn render(c: &Compiled, theme: Theme) -> String {
    let scene = &c.scene;
    let mut s = String::new();
    let class = match theme {
        Theme::Light => "layup",
        Theme::Dark => "layup dark",
        Theme::Auto => "layup auto",
    };
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" class="{class}" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-labelledby="layup-title layup-desc">"#,
        w = num(scene.width),
        h = num(scene.height)
    );
    let _ = writeln!(
        s,
        "  <title id=\"layup-title\">{}</title>",
        esc(&c.diagram.title)
    );
    let desc = c
        .diagram
        .desc
        .clone()
        .or_else(|| c.diagram.note.clone())
        .unwrap_or_default();
    let _ = writeln!(s, "  <desc id=\"layup-desc\">{}</desc>", esc(&desc));
    let _ = writeln!(
        s,
        "  <metadata>{}</metadata>",
        esc(&include_str!("../fonts/OFL.txt")
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n"))
    );
    s.push_str("  <style>\n");
    s.push_str(&stylesheet());
    s.push_str("  </style>\n");
    s.push_str("  <defs>\n");
    for t in Tone::ALL {
        let _ = writeln!(
            s,
            r#"    <marker id="m-{n}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" class="mk mk-{n}"/></marker>"#,
            n = t.name()
        );
    }
    s.push_str("  </defs>\n");
    let _ = writeln!(
        s,
        r#"  <rect class="bg" width="{}" height="{}"/>"#,
        num(scene.width),
        num(scene.height)
    );
    let _ = writeln!(
        s,
        r#"  <rect class="frame" x="0.5" y="0.5" width="{}" height="{}"/>"#,
        num(scene.width - 1.0),
        num(scene.height - 1.0)
    );

    s.push_str("  <g class=\"content\">\n");
    let mut open: Option<usize> = None;
    for p in &scene.items {
        if p.node != open {
            if open.is_some() {
                s.push_str("    </g>\n");
            }
            if let Some(i) = p.node {
                let n = &scene.nodes[i];
                let _ = writeln!(
                    s,
                    r#"    <g class="node k-{}" data-id="{}">"#,
                    esc(&n.kind),
                    esc(&n.id)
                );
            }
            open = p.node;
        }
        item(&mut s, p, scene);
    }
    if open.is_some() {
        s.push_str("    </g>\n");
    }
    s.push_str("  </g>\n");

    s.push_str("  <g class=\"edges\">\n");
    for e in &scene.edges {
        let _ = writeln!(
            s,
            r#"    <g class="edge k-{}" data-from="{}" data-to="{}">"#,
            esc(&e.kind),
            esc(&e.from),
            esc(&e.to)
        );
        let mut d = String::new();
        for (i, (x, y)) in e.points.iter().enumerate() {
            let _ = write!(
                d,
                "{}{} {}",
                if i == 0 { "M" } else { " L" },
                num(*x),
                num(*y)
            );
        }
        let mut attrs = format!(r#"class="ln ln-{}""#, e.tone.name());
        if e.dashed {
            attrs.push_str(r#" stroke-dasharray="7 5""#);
        }
        if e.head_end {
            let _ = write!(attrs, r#" marker-end="url(#m-{})""#, e.tone.name());
        }
        if e.head_start {
            let _ = write!(attrs, r#" marker-start="url(#m-{})""#, e.tone.name());
        }
        let _ = writeln!(s, r#"      <path d="{d}" {attrs}/>"#);
        if let Some(chip) = &e.chip {
            item(
                &mut s,
                &Placed {
                    item: chip.clone(),
                    node: None,
                },
                scene,
            );
        }
        s.push_str("    </g>\n");
    }
    s.push_str("  </g>\n");
    s.push_str("</svg>\n");
    s
}

fn item(s: &mut String, p: &Placed, _scene: &Scene) {
    match &p.item {
        Item::Box {
            rect,
            tone,
            hollow,
            white,
            rx,
            stroke_width,
        } => {
            let fill = if *hollow {
                "hollow".to_string()
            } else if *white {
                "white".to_string()
            } else {
                format!("bg-{}", tone.name())
            };
            let _ = writeln!(
                s,
                r#"      <rect class="box {fill} bd-{}" {} rx="{}" stroke-width="{}"/>"#,
                tone.name(),
                rect_attrs(rect),
                num(*rx),
                num(*stroke_width)
            );
        }
        Item::Strip { rect, rx } => {
            let r = num(*rx);
            let _ = writeln!(
                s,
                r#"      <path class="strip" d="M{x} {y1} L{x} {yr} A{r} {r} 0 0 1 {xr} {y} L{xr2} {y} A{r} {r} 0 0 1 {x2} {yr} L{x2} {y1} Z"/>"#,
                x = num(rect.x),
                y = num(rect.y),
                y1 = num(rect.bottom()),
                yr = num(rect.y + rx),
                xr = num(rect.x + rx),
                xr2 = num(rect.right() - rx),
                x2 = num(rect.right()),
            );
        }
        Item::Rule {
            x1,
            y1,
            x2,
            y2,
            tone,
            dashed,
        } => {
            let class = match tone {
                Some(t) => format!("rule ln-{}", t.name()),
                None => "rule".into(),
            };
            let dash = if *dashed {
                r#" stroke-dasharray="6 5""#
            } else {
                ""
            };
            let _ = writeln!(
                s,
                r#"      <line class="{class}" x1="{}" y1="{}" x2="{}" y2="{}"{dash}/>"#,
                num(*x1),
                num(*y1),
                num(*x2),
                num(*y2)
            );
        }
        Item::Text(t) => text(s, t),
        Item::Chip {
            rect,
            text: label,
            tone,
            rotate,
            bordered,
        } => {
            let cx = rect.cx();
            let cy = rect.cy();
            if *rotate {
                let _ = writeln!(
                    s,
                    r#"      <g transform="rotate(-90 {} {})">"#,
                    num(cx),
                    num(cy)
                );
            }
            let (class, tclass) = if *bordered {
                ("chip", "t chip-t")
            } else {
                ("chip plain", "t chip-t plain")
            };
            let _ = writeln!(
                s,
                r#"      <rect class="{class}" {} rx="4"/>"#,
                rect_attrs(rect)
            );
            let _ = writeln!(
                s,
                r#"      <text class="{tclass} ink-{}" x="{}" y="{}" text-anchor="middle">{}</text>"#,
                tone.name(),
                num(cx),
                num(cy + 4.0),
                esc(label)
            );
            if *rotate {
                s.push_str("      </g>\n");
            }
        }
        Item::Sample { x, y, tone, dashed } => {
            let dash = if *dashed {
                r#" stroke-dasharray="7 5""#
            } else {
                ""
            };
            let _ = writeln!(
                s,
                r#"      <path class="ln ln-{n}" d="M{} {} H{}"{dash} marker-end="url(#m-{n})"/>"#,
                num(*x),
                num(*y),
                num(x + 32.0),
                n = tone.name()
            );
        }
    }
}

fn text(s: &mut String, t: &TextItem) {
    let anchor = match t.anchor {
        Anchor::Start => "",
        Anchor::Middle => r#" text-anchor="middle""#,
        Anchor::End => r#" text-anchor="end""#,
    };
    let ink = match t.ink {
        Ink::Text => "ink".to_string(),
        Ink::Muted => "muted".to_string(),
        Ink::Code => "codeink".to_string(),
        Ink::Tone(tone) => format!("ink-{}", tone.name()),
    };
    let font = if t.mono { "m" } else { "t" };
    let spacing = if t.letter_spacing > 0.0 {
        format!(r#" letter-spacing="{}""#, num(t.letter_spacing * t.size))
    } else {
        String::new()
    };
    let _ = write!(
        s,
        r#"      <text class="{font} {ink}" x="{}" y="{}" font-size="{}" font-weight="{}"{anchor}{spacing}>"#,
        num(t.x),
        num(t.y),
        num(t.size),
        t.weight
    );
    for r in &t.runs {
        run(s, r, t.mono);
    }
    s.push_str("</text>\n");
}

fn run(s: &mut String, r: &Run, base_mono: bool) {
    let body = esc(&r.text.replace(' ', "\u{a0}"));
    if r.tag {
        let _ = write!(s, r#"<tspan class="tag">{body}</tspan>"#);
    } else if r.code && !base_mono {
        let _ = write!(s, r#"<tspan class="m">{body}</tspan>"#);
    } else if !r.code && base_mono {
        let _ = write!(s, r#"<tspan class="t">{body}</tspan>"#);
    } else {
        s.push_str(&body);
    }
}

fn rect_attrs(r: &Rect) -> String {
    format!(
        r#"x="{}" y="{}" width="{}" height="{}""#,
        num(r.x),
        num(r.y),
        num(r.w),
        num(r.h)
    )
}

pub fn num(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r == r.trunc() {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c => o.push(c),
        }
    }
    o
}

fn vars(dark: bool) -> String {
    let mut v = String::new();
    if dark {
        v.push_str("--bg:#0d1117;--frame:#30363d;--ink:#e6edf3;--muted:#8b949e;--codeink:#79c0ff;--rule:#21262d;--strip:#161b22;--white:#0d1117;--chipbd:#30363d;");
    } else {
        v.push_str("--bg:#fdfdfd;--frame:#d8dee4;--ink:#1f2328;--muted:#57606a;--codeink:#0550ae;--rule:#e6e9ed;--strip:#f6f8fa;--white:#ffffff;--chipbd:#d0d7de;");
    }
    for t in Tone::ALL {
        let (bg, bd, ink) = if dark { t.dark() } else { t.light() };
        let _ = write!(
            v,
            "--{n}-bg:{bg};--{n}-bd:{bd};--{n}-ink:{ink};--{n}-edge:{};",
            edge_color(t, dark),
            n = t.name()
        );
    }
    v
}

pub fn stylesheet() -> String {
    let mut css = crate::text::stylesheet().to_owned();
    let _ = writeln!(css, "    .layup{{{}}}", vars(false));
    let _ = writeln!(css, "    .layup.dark{{{}}}", vars(true));
    let _ = writeln!(
        css,
        "    @media (prefers-color-scheme: dark){{.layup.auto{{{}}}}}",
        vars(true)
    );
    css.push_str(
        r#"    .t{font-family:'Layup Sans',sans-serif}
    .m{font-family:'Layup Mono',monospace;font-weight:400}
    .t,.m{font-kerning:none;font-variant-ligatures:none;font-synthesis:none}
    .ink{fill:var(--ink)}.muted{fill:var(--muted)}.codeink{fill:var(--codeink)}
    .tag{fill:var(--blue-edge);font-weight:650}
    .bg{fill:var(--bg)}.frame{fill:none;stroke:var(--frame)}
    .rule{stroke:var(--rule);stroke-width:1;fill:none}
    .strip{fill:var(--strip)}
    .box{stroke-width:1.25}.hollow{fill:none}.white{fill:var(--white)}
    .ln{fill:none;stroke-width:1.8}
    .chip{fill:var(--bg);stroke:var(--chipbd);stroke-width:.8}.chip.plain{stroke:none}
    .chip-t{font-size:11.5px;font-weight:650}.chip-t.plain{font-size:12px;font-weight:400}
"#,
    );
    for t in Tone::ALL {
        let n = t.name();
        let _ = writeln!(
            css,
            "    .bg-{n}{{fill:var(--{n}-bg)}}.bd-{n}{{stroke:var(--{n}-bd)}}.ink-{n}{{fill:var(--{n}-edge)}}.ln-{n}{{stroke:var(--{n}-edge)}}.mk-{n}{{fill:var(--{n}-edge)}}"
        );
    }
    css
}
