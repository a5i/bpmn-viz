//! BPMN Diagram Interchange (DI) types.
//!
//! DI supplies all geometry; the renderer preserves provided coordinates
//! rather than recalculating them (SPEC-001 section 2, ADR-0003). Numbers
//! are IEEE doubles validated for finiteness by [`Bounds::is_valid`].

/// Axis-aligned rectangle `dc:Bounds`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// X coordinate of the top-left corner.
    pub x: f64,
    /// Y coordinate of the top-left corner.
    pub y: f64,
    /// Width; must be positive to be usable.
    pub width: f64,
    /// Height; must be positive to be usable.
    pub height: f64,
}

impl Bounds {
    /// Creates bounds without validation.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// True when all coordinates are finite and width/height are positive
    /// and finite.
    pub fn is_valid(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width > 0.0
            && self.height > 0.0
    }

    /// Center point, if all coordinates are finite.
    pub fn center(&self) -> Option<(f64, f64)> {
        if !self.is_valid() {
            return None;
        }
        Some((self.x + self.width / 2.0, self.y + self.height / 2.0))
    }

    /// Smallest rectangle containing both `self` and `other`.
    pub fn union(&self, other: &Bounds) -> Bounds {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = (self.x + self.width).max(other.x + other.width);
        let max_y = (self.y + self.height).max(other.y + other.height);
        Bounds::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Rectangle grown by `pad` on every side.
    pub fn padded(&self, pad: f64) -> Bounds {
        Bounds::new(
            self.x - pad,
            self.y - pad,
            self.width + 2.0 * pad,
            self.height + 2.0 * pad,
        )
    }
}

/// A DI point (`di:waypoint` or label position).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
}

impl Point {
    /// Creates a point without validation.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// True when both coordinates are finite.
    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// Optional label geometry attached to shapes and edges.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BpmnLabel {
    /// Optional label bounds; validated when present.
    pub bounds: Option<Bounds>,
}

/// A `bpmndi:BPMNShape`: geometry for one semantic element.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    /// DI shape id (`BPMNShape/@id`).
    pub id: Option<String>,
    /// Semantic element id this shape provides geometry for.
    pub bpmn_element: String,
    /// Shape bounds.
    pub bounds: Bounds,
    /// Optional label bounds.
    pub label: Option<BpmnLabel>,
    /// `isHorizontal` attribute, relevant for pools and lanes.
    pub is_horizontal: Option<bool>,
    /// `isExpanded` attribute, relevant for subprocesses.
    pub is_expanded: Option<bool>,
}

/// A `bpmndi:BPMNEdge`: waypoint list for one semantic flow.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// DI edge id (`BPMNEdge/@id`).
    pub id: Option<String>,
    /// Semantic element id this edge provides geometry for.
    pub bpmn_element: String,
    /// Ordered waypoints.
    pub waypoints: Vec<Point>,
    /// Optional label bounds.
    pub label: Option<BpmnLabel>,
}

/// A `bpmndi:BPMNPlane`: the shape and edge set of one diagram.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plane {
    /// Plane id.
    pub id: Option<String>,
    /// Semantic element the plane shows (a collaboration or process id).
    pub bpmn_element: Option<String>,
    /// Shapes in document order.
    pub shapes: Vec<Shape>,
    /// Edges in document order.
    pub edges: Vec<Edge>,
}

/// A `bpmndi:BPMNDiagram`: one selectable diagram of the document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diagram {
    /// Diagram id; selection by id uses this.
    pub id: Option<String>,
    /// Display name if present.
    pub name: Option<String>,
    /// The plane holding all geometry.
    pub plane: Plane,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_validation_rejects_nan_and_nonpositive() {
        assert!(Bounds::new(0.0, 0.0, 10.0, 20.0).is_valid());
        assert!(!Bounds::new(f64::NAN, 0.0, 10.0, 20.0).is_valid());
        assert!(!Bounds::new(0.0, f64::INFINITY, 10.0, 20.0).is_valid());
        assert!(!Bounds::new(0.0, 0.0, 0.0, 20.0).is_valid());
        assert!(!Bounds::new(0.0, 0.0, -1.0, 20.0).is_valid());
    }

    #[test]
    fn union_and_pad_are_stable() {
        let a = Bounds::new(0.0, 0.0, 10.0, 10.0);
        let b = Bounds::new(20.0, 30.0, 5.0, 5.0);
        let u = a.union(&b);
        assert_eq!(u, Bounds::new(0.0, 0.0, 25.0, 35.0));
        assert_eq!(u.padded(10.0), Bounds::new(-10.0, -10.0, 45.0, 55.0));
    }

    #[test]
    fn point_finiteness() {
        assert!(Point::new(1.5, -2.0).is_finite());
        assert!(!Point::new(f64::NAN, 0.0).is_finite());
    }
}
