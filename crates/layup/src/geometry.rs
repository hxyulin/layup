//! Shared node outlines. Routing uses bounding rectangles to find conservative
//! channels, then attaches endpoints to the actual outline.
use crate::{layout::Rect, model::Side};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outline {
    Rectangle(Rect),
    Rounded { rect: Rect, radius: f64 },
    Diamond(Rect),
}

impl Outline {
    pub fn bounds(self) -> Rect {
        match self {
            Self::Rectangle(r) | Self::Diamond(r) | Self::Rounded { rect: r, .. } => r,
        }
    }

    /// A centered rectangle whose corners stay inside the outline. Padding is
    /// measured inside this area, so wrapped labels cannot touch sloping sides.
    pub fn text_area(self, padding: f64) -> Rect {
        let r = self.bounds();
        match self {
            Self::Diamond(_) => Rect {
                x: r.x + r.w / 4.0,
                y: r.y + r.h / 4.0,
                w: r.w / 2.0,
                h: r.h / 2.0,
            }
            .grow(-padding),
            Self::Rounded { radius, .. } => {
                let radius = radius.min(r.w / 2.0).min(r.h / 2.0);
                r.grow(-radius * (1.0 - std::f64::consts::FRAC_1_SQRT_2) - padding)
            }
            Self::Rectangle(_) => r.grow(-padding),
        }
    }

    /// Intersection of an axis-aligned ray with the requested half of the
    /// boundary. Clamp spread ports so they never leave the outline.
    pub fn port(self, side: Side, along: f64) -> (f64, f64) {
        let r = self.bounds();
        let vertical = matches!(side, Side::Top | Side::Bottom);
        let (center, half) = if vertical {
            (r.cx(), r.w / 2.0)
        } else {
            (r.cy(), r.h / 2.0)
        };
        let along = along.clamp(center - half, center + half);
        let offset = (along - center).abs();
        let inset = match self {
            Self::Diamond(_) => offset / half * if vertical { r.h / 2.0 } else { r.w / 2.0 },
            Self::Rounded { radius, .. } => {
                let radius = radius.min(r.w / 2.0).min(r.h / 2.0);
                let d = (offset - (half - radius)).max(0.0);
                radius - (radius * radius - d * d).max(0.0).sqrt()
            }
            Self::Rectangle(_) => 0.0,
        };
        match side {
            Side::Top => (along, r.y + inset),
            Side::Bottom => (along, r.bottom() - inset),
            Side::Left => (r.x + inset, along),
            Side::Right => (r.right() - inset, along),
        }
    }

    pub fn contains(self, p: (f64, f64)) -> bool {
        let r = self.bounds();
        if !r.contains(p.0, p.1) {
            return false;
        }
        let left = self.port(Side::Left, p.1).0;
        let right = self.port(Side::Right, p.1).0;
        p.0 > left && p.0 < right
    }

    /// Whether an orthogonal segment crosses the interior, allowing boundary
    /// contact. Bounding rectangles remain a cheap broad phase in the router.
    pub fn segment_hits(self, p: (f64, f64), q: (f64, f64)) -> bool {
        let r = self.bounds();
        if (p.0 - q.0).abs() < 0.001 {
            if p.0 <= r.x + 0.5 || p.0 >= r.right() - 0.5 {
                return false;
            }
            let top = self.port(Side::Top, p.0).1;
            let bottom = self.port(Side::Bottom, p.0).1;
            p.1.max(q.1) > top + 0.5 && p.1.min(q.1) < bottom - 0.5
        } else {
            if p.1 <= r.y + 0.5 || p.1 >= r.bottom() - 0.5 {
                return false;
            }
            let left = self.port(Side::Left, p.1).0;
            let right = self.port(Side::Right, p.1).0;
            p.0.max(q.0) > left + 0.5 && p.0.min(q.0) < right - 0.5
        }
    }

    pub fn vertices(self) -> [(f64, f64); 4] {
        let r = self.bounds();
        [
            (r.cx(), r.y),
            (r.right(), r.cy()),
            (r.cx(), r.bottom()),
            (r.x, r.cy()),
        ]
    }

    pub fn intersects_rect(self, r: Rect) -> bool {
        let corners = [
            (r.x, r.y),
            (r.right(), r.y),
            (r.right(), r.bottom()),
            (r.x, r.bottom()),
        ];
        corners.iter().any(|&p| self.contains(p))
            || r.contains(self.bounds().cx(), self.bounds().cy())
            || (0..4).any(|i| self.segment_hits(corners[i], corners[(i + 1) % 4]))
    }
}
