//! Path construction helpers built on `kurbo`.

pub use kurbo::{Affine, BezPath, Circle, Point, Rect, RoundedRect};

/// Open polyline through `points` (needs at least two points).
pub fn polyline(points: &[(f64, f64)]) -> BezPath {
    let mut path = BezPath::new();
    let mut iter = points.iter();
    if let Some((x, y)) = iter.next() {
        path.move_to((*x, *y));
    }
    for (x, y) in iter {
        path.line_to((*x, *y));
    }
    path
}

/// Closed diamond (gateway glyph) centered at (`cx`, `cy`).
pub fn diamond(cx: f64, cy: f64, half_width: f64, half_height: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to((cx, cy - half_height));
    path.line_to((cx + half_width, cy));
    path.line_to((cx, cy + half_height));
    path.line_to((cx - half_width, cy));
    path.close_path();
    path
}

/// Closed polygon through `points`.
pub fn polygon(points: &[(f64, f64)]) -> BezPath {
    let mut path = polyline(points);
    path.close_path();
    path
}

/// Axis-aligned open bracket `[` shape used by text annotations:
/// vertical line at `x` from `y` to `y + height` with two tabs of `tab` size.
pub fn annotation_bracket(x: f64, y: f64, height: f64, tab: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to((x + tab, y));
    path.line_to((x, y));
    path.line_to((x, y + height));
    path.line_to((x + tab, y + height));
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyline_elements() {
        let path = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        let elements = path.elements();
        assert_eq!(elements.len(), 3);
    }

    #[test]
    fn diamond_is_closed() {
        let path = diamond(10.0, 10.0, 10.0, 10.0);
        assert!(matches!(
            path.elements().last(),
            Some(kurbo::PathEl::ClosePath)
        ));
    }
}
