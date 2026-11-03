//! Renderer-agnostic 2D scene primitives (SPEC-001 section 4, ADR-0002).
//!
//! A [`Scene`] is a deterministic, ordered list of drawable nodes plus
//! viewport and reusable defs (markers). It knows nothing about BPMN: the
//! `bpmn-render` crate builds scenes, the `bpmn-svg` crate serializes them,
//! and `bpmn-raster` rasterizes serialized SVG.

pub mod node;
pub mod path;
pub mod style;
pub mod text;

pub use node::{Node, Scene};
pub use style::{FontWeight, Marker, Orient, Paint, Rgba, Style, TextStyle, Viewport};
pub use text::TextAnchor;

/// Marker definition ids shared by renderers.
pub mod ids {
    /// Arrowhead marker appended to sequence flow edges.
    pub const ARROW_END: &str = "bpmn-arrow-end";
    /// Small diamond used for conditional sequence flows.
    pub const CONDITIONAL_MARKER: &str = "bpmn-conditional";
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Affine;

    #[test]
    fn scene_holds_ordered_nodes() {
        let mut scene = Scene::new(Viewport::new(0.0, 0.0, 100.0, 100.0));
        scene.push(Node::rect("r1", 0.0, 0.0, 10.0, 10.0, Style::default()));
        scene.push(Node::rect("r2", 5.0, 5.0, 10.0, 10.0, Style::default()));
        assert_eq!(scene.nodes.len(), 2);
        assert_eq!(scene.nodes[0].id().unwrap(), "r1");
        assert_eq!(scene.nodes[1].id().unwrap(), "r2");
        scene.wrap_next(Some("g".into()), Affine::translate((1.0, 2.0)));
        scene.push(Node::rect("r3", 5.0, 5.0, 10.0, 10.0, Style::default()));
        assert_eq!(scene.nodes.len(), 3);
        assert!(matches!(scene.nodes[2], Node::Group { .. }));
    }
}
