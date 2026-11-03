//! Integer rectangles, points and segment/obstacle helpers used by the layout
//! pipeline.
//!
//! The engine deliberately computes in integer coordinates: sizes, gaps and
//! band pitches are whole numbers, so every derived coordinate is exact and
//! reproducible without float comparisons (ADR-0006 determinism policy). The
//! only floating point appears once, when the final geometry is written into
//! the DI model.

/// An axis-aligned integer rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IRect {
    /// X coordinate of the top-left corner.
    pub x: i64,
    /// Y coordinate of the top-left corner.
    pub y: i64,
    /// Width; always positive.
    pub w: i64,
    /// Height; always positive.
    pub h: i64,
}

impl IRect {
    /// Creates a new rectangle.
    pub(crate) const fn new(x: i64, y: i64, w: i64, h: i64) -> Self {
        Self { x, y, w, h }
    }

    /// Right edge (exclusive coordinate of the boundary line).
    pub(crate) const fn right(&self) -> i64 {
        self.x + self.w
    }

    /// Bottom edge (exclusive coordinate of the boundary line).
    pub(crate) const fn bottom(&self) -> i64 {
        self.y + self.h
    }

    /// Horizontal center (rounded towards negative infinity; all canonical
    /// widths are even, so this is exact in practice).
    pub(crate) const fn center_x(&self) -> i64 {
        self.x + self.w / 2
    }

    /// Vertical center.
    pub(crate) const fn center_y(&self) -> i64 {
        self.y + self.h / 2
    }

    /// True when both rectangles strictly overlap (touching edges do not
    /// count).
    pub(crate) fn overlaps(&self, other: &IRect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// The rectangle shrunk by `d` on every side. A segment running exactly
    /// along the original boundary therefore misses the inset rectangle.
    pub(crate) fn inset(&self, d: i64) -> IRect {
        IRect::new(self.x + d, self.y + d, self.w - 2 * d, self.h - 2 * d)
    }

    /// True when the point lies on the rectangle boundary (the shape border
    /// where waypoints dock).
    pub(crate) fn on_boundary(&self, x: i64, y: i64) -> bool {
        let inside_x = x >= self.x && x <= self.right();
        let inside_y = y >= self.y && y <= self.bottom();
        (inside_x && (y == self.y || y == self.bottom()))
            || (inside_y && (x == self.x || x == self.right()))
    }
}

/// True when the horizontal segment `(x1..x2, y)` penetrates the interior of
/// `rect`. Boundary touching is not a penetration.
pub(crate) fn horizontal_segment_hits(x1: i64, x2: i64, y: i64, rect: &IRect) -> bool {
    y > rect.y && y < rect.bottom() && x1.max(x2) > rect.x && x1.min(x2) < rect.right()
}

/// True when the vertical segment `(x, y1..y2)` penetrates the interior of
/// `rect`. Boundary touching is not a penetration.
pub(crate) fn vertical_segment_hits(x: i64, y1: i64, y2: i64, rect: &IRect) -> bool {
    x > rect.x && x < rect.right() && y1.max(y2) > rect.y && y1.min(y2) < rect.bottom()
}

/// Removes zero-length segments and merges collinear consecutive segments of
/// an orthogonal waypoint list, keeping at least two waypoints.
pub(crate) fn simplify_waypoints(points: Vec<(i64, i64)>) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = Vec::with_capacity(points.len());
    for (x, y) in points {
        if let Some(&(lx, ly)) = out.last() {
            if lx == x && ly == y {
                continue; // zero-length segment
            }
            // Collinear with the previous kept segment and its predecessor?
            if out.len() >= 2 {
                let (px, py) = out[out.len() - 2];
                let collinear_x = px == lx && lx == x;
                let collinear_y = py == ly && ly == y;
                if collinear_x || collinear_y {
                    *out.last_mut().expect("len >= 2") = (x, y);
                    continue;
                }
            }
        }
        out.push((x, y));
    }
    if out.len() == 1 {
        out.push(out[0]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_requires_strict_intersection() {
        let a = IRect::new(0, 0, 10, 10);
        let b = IRect::new(10, 0, 10, 10); // touching edge
        let c = IRect::new(5, 5, 10, 10); // real overlap
        assert!(!a.overlaps(&b));
        assert!(a.overlaps(&c));
    }

    #[test]
    fn inset_boundary_touch_is_no_hit() {
        let rect = IRect::new(0, 0, 100, 80);
        let obstacle = rect.inset(1);
        assert!(!horizontal_segment_hits(-10, 200, 0, &obstacle));
        assert!(!horizontal_segment_hits(-10, 200, 80, &obstacle));
        assert!(horizontal_segment_hits(-10, 200, 40, &obstacle));
        assert!(!vertical_segment_hits(0, -10, 200, &obstacle));
        assert!(vertical_segment_hits(50, -10, 200, &obstacle));
    }

    #[test]
    fn simplify_merges_collinear_and_drops_zero_length() {
        let pts = vec![(0, 0), (0, 0), (10, 0), (20, 0), (20, 10), (20, 10)];
        assert_eq!(simplify_waypoints(pts), vec![(0, 0), (20, 0), (20, 10)]);
        assert_eq!(simplify_waypoints(vec![(5, 5)]), vec![(5, 5), (5, 5)]);
    }
}
