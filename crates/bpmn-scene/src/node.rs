//! Scene graph nodes and the scene container.

use crate::style::{Marker, Style, TextStyle, Viewport};
use crate::text::TextAnchor;
use kurbo::{Affine, BezPath};

/// A drawable scene node. Rendering order is exactly the insertion order
/// into [`Scene::nodes`] (z-order contract: backgrounds, flows, nodes,
/// markers, labels).
#[derive(Debug, Clone)]
pub enum Node {
    /// Grouping container with optional transform.
    Group {
        /// Stable element id.
        id: Option<String>,
        /// Optional affine transform.
        transform: Option<Affine>,
        /// Child nodes in paint order.
        children: Vec<Node>,
    },
    /// Arbitrary path.
    Path {
        /// Stable element id.
        id: Option<String>,
        /// Path data.
        data: BezPath,
        /// Paint style.
        style: Style,
    },
    /// Axis-aligned rectangle with optional corner radius.
    Rect {
        /// Stable element id.
        id: Option<String>,
        /// Min x.
        x: f64,
        /// Min y.
        y: f64,
        /// Width.
        width: f64,
        /// Height.
        height: f64,
        /// Corner radius (0 for sharp corners).
        corner_radius: f64,
        /// Paint style.
        style: Style,
    },
    /// Ellipse (circles included).
    Ellipse {
        /// Stable element id.
        id: Option<String>,
        /// Center x.
        cx: f64,
        /// Center y.
        cy: f64,
        /// Radius x.
        rx: f64,
        /// Radius y.
        ry: f64,
        /// Paint style.
        style: Style,
    },
    /// Single-line text. Multi-line labels emit one node per line.
    Text {
        /// Stable element id.
        id: Option<String>,
        /// Baseline x.
        x: f64,
        /// Baseline y.
        y: f64,
        /// Content (escaped at serialization time).
        content: String,
        /// Text style.
        style: TextStyle,
        /// Horizontal anchor.
        anchor: TextAnchor,
    },
}

impl Node {
    /// The stable element id, if any.
    pub fn id(&self) -> Option<&str> {
        match self {
            Node::Group { id, .. }
            | Node::Path { id, .. }
            | Node::Rect { id, .. }
            | Node::Ellipse { id, .. }
            | Node::Text { id, .. } => id.as_deref(),
        }
    }

    /// Rectangle constructor.
    pub fn rect(
        id: impl Into<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        style: Style,
    ) -> Self {
        Node::Rect {
            id: Some(id.into()),
            x,
            y,
            width,
            height,
            corner_radius: 0.0,
            style,
        }
    }

    /// Rounded rectangle constructor.
    pub fn rounded_rect(
        id: impl Into<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        corner_radius: f64,
        style: Style,
    ) -> Self {
        Node::Rect {
            id: Some(id.into()),
            x,
            y,
            width,
            height,
            corner_radius,
            style,
        }
    }

    /// Circle constructor.
    pub fn circle(id: impl Into<String>, cx: f64, cy: f64, r: f64, style: Style) -> Self {
        Node::Ellipse {
            id: Some(id.into()),
            cx,
            cy,
            rx: r,
            ry: r,
            style,
        }
    }

    /// Path constructor.
    pub fn path(id: impl Into<String>, data: BezPath, style: Style) -> Self {
        Node::Path {
            id: Some(id.into()),
            data,
            style,
        }
    }

    /// Text constructor.
    pub fn text(
        id: impl Into<String>,
        x: f64,
        y: f64,
        content: impl Into<String>,
        style: TextStyle,
        anchor: TextAnchor,
    ) -> Self {
        Node::Text {
            id: Some(id.into()),
            x,
            y,
            content: content.into(),
            style,
            anchor,
        }
    }
}

/// A complete drawable scene.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    /// Coordinate box the scene is drawn into.
    pub viewport: Viewport,
    /// Reusable marker definitions.
    pub defs: Vec<Marker>,
    /// Nodes in paint order.
    pub nodes: Vec<Node>,
    /// Wrap the next pushed node in a group with this transform.
    pending_transform: Option<(Option<String>, Affine)>,
}

impl Scene {
    /// Creates an empty scene for `viewport`.
    pub fn new(viewport: Viewport) -> Self {
        Self {
            viewport,
            defs: Vec::new(),
            nodes: Vec::new(),
            pending_transform: None,
        }
    }

    /// Registers a marker definition.
    pub fn define_marker(&mut self, marker: Marker) {
        self.defs.push(marker);
    }

    /// Arranges for the next pushed node to be wrapped in a group with
    /// `transform` (used for rotated lane and pool labels).
    pub fn wrap_next(&mut self, id: Option<String>, transform: Affine) {
        self.pending_transform = Some((id, transform));
    }

    /// Appends a node, honoring any pending wrap.
    pub fn push(&mut self, node: Node) {
        match self.pending_transform.take() {
            Some((id, transform)) => self.nodes.push(Node::Group {
                id,
                transform: Some(transform),
                children: vec![node],
            }),
            None => self.nodes.push(node),
        }
    }

    /// Appends a child node under (creating if needed) a group with `id`,
    /// keeping background/foreground ordering explicit and stable.
    pub fn push_into_group(&mut self, group_id: &str, node: Node) {
        if let Some(existing) = self.nodes.iter_mut().find(
            |candidate| matches!(candidate, Node::Group { id: Some(id), .. } if id == group_id),
        ) {
            if let Node::Group { children, .. } = existing {
                children.push(node);
            }
            return;
        }
        self.nodes.push(Node::Group {
            id: Some(group_id.to_string()),
            transform: None,
            children: vec![node],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Rgba;

    #[test]
    fn push_into_group_is_stable() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 10.0, 10.0));
        let style = Style {
            fill: Some(crate::style::Paint::Color(Rgba::WHITE)),
            ..Style::default()
        };
        scene.push_into_group(
            "pool",
            Node::rect("band", 0.0, 0.0, 5.0, 5.0, style.clone()),
        );
        scene.push_into_group("pool", Node::rect("band2", 1.0, 1.0, 5.0, 5.0, style));
        assert_eq!(scene.nodes.len(), 1);
        match &scene.nodes[0] {
            Node::Group { children, .. } => assert_eq!(children.len(), 2),
            other => panic!("expected group, got {other:?}"),
        }
    }

    #[test]
    fn wrap_next_groups_single_node() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 10.0, 10.0));
        scene.wrap_next(Some("rot".into()), Affine::rotate(1.0));
        scene.push(Node::text(
            "t",
            0.0,
            0.0,
            "x",
            TextStyle::default(),
            TextAnchor::Start,
        ));
        assert_eq!(scene.nodes.len(), 1);
        assert!(matches!(
            &scene.nodes[0],
            Node::Group {
                transform: Some(_),
                ..
            }
        ));
    }
}
