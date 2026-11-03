//! BPMN glyph and scene construction (ADR-0002: BPMN → scene is separate
//! from scene → SVG).
//!
//! Entry point: [`render_diagram`]. Geometry comes exclusively from DI
//! (ADR-0003); unsupported symbols render with a conspicuous fallback glyph
//! and a `symbol.unsupported` warning (SPEC-001 section 2).

pub mod flows;
pub mod glyphs;
pub mod labels;
pub mod renderer;

use bpmn_model::diagnostic::Diagnostic;
use bpmn_model::{BpmnDocument, di};
use bpmn_scene::Scene;
use bpmn_scene::style::{Rgba, Viewport};
use thiserror::Error;

/// Default visual theme, roughly aligned with common BPMN tooling.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Stroke color of shapes.
    pub shape_stroke: Rgba,
    /// Fill color of shapes.
    pub shape_fill: Rgba,
    /// Stroke color of sequence flows.
    pub flow_stroke: Rgba,
    /// Text color.
    pub text: Rgba,
    /// Fill of lanes and pool bands.
    pub container_fill: Rgba,
    /// Fill of the unsupported-symbol fallback glyph.
    pub fallback_fill: Rgba,
    /// Stroke of the fallback glyph.
    pub fallback_stroke: Rgba,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            shape_stroke: Rgba::rgb(26, 26, 26),
            shape_fill: Rgba::WHITE,
            flow_stroke: Rgba::rgb(26, 26, 26),
            text: Rgba::BLACK,
            container_fill: Rgba::WHITE,
            fallback_fill: Rgba::rgb(255, 243, 205),
            fallback_stroke: Rgba::rgb(181, 137, 0),
        }
    }
}

/// Rendering configuration.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Visual theme.
    pub theme: Theme,
    /// Font family list for labels.
    pub font_family: String,
    /// Base label font size in user units.
    pub font_size: f64,
    /// Viewport padding in user units.
    pub padding: f64,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            font_family: "sans-serif".to_string(),
            font_size: 12.0,
            padding: 10.0,
        }
    }
}

/// Render failure; missing DI is an error, never a fallback (AC-09).
#[derive(Debug, Clone, Error)]
pub enum RenderError {
    /// A required shape or edge is missing although it must be rendered.
    #[error("missing diagram interchange for element `{element_id}`")]
    MissingGeometry {
        /// Semantic element id that has no DI.
        element_id: String,
    },
    /// The diagram plane references nothing renderable or geometry is empty.
    #[error("diagram has no renderable geometry")]
    EmptyDiagram,
}

/// A rendered scene plus non-fatal diagnostics (for example unsupported
/// symbols drawn with fallback glyphs).
#[derive(Debug, Clone)]
pub struct RenderedDiagram {
    /// The constructed scene.
    pub scene: Scene,
    /// Warnings emitted while rendering.
    pub diagnostics: Vec<Diagnostic>,
}

/// Renders one DI diagram of `document` into a scene.
pub fn render_diagram(
    document: &BpmnDocument,
    diagram: &di::Diagram,
    options: &RenderOptions,
) -> Result<RenderedDiagram, RenderError> {
    renderer::render(document, diagram, options)
}

/// Convenience: renders the first diagram, if any.
pub fn render_first_diagram(
    document: &BpmnDocument,
    options: &RenderOptions,
) -> Result<RenderedDiagram, RenderError> {
    let diagram = document.first_diagram().ok_or(RenderError::EmptyDiagram)?;
    render_diagram(document, diagram, options)
}

/// Unions all geometry of a diagram into the raw viewport (before padding).
pub fn diagram_viewport(diagram: &di::Diagram) -> Viewport {
    renderer::union_viewport(diagram)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpmn_xml::from_xml;

    const MINIMAL: &str = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
        xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
        xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
        xmlns:di="http://www.omg.org/spec/DD/20100524/DI">
  <process id="P">
    <startEvent id="S" name="start"/>
    <endEvent id="E"/>
    <sequenceFlow id="F" sourceRef="S" targetRef="E"/>
  </process>
  <bpmndi:BPMNDiagram id="D"><bpmndi:BPMNPlane id="Pl" bpmnElement="P">
    <bpmndi:BPMNShape id="sS" bpmnElement="S"><dc:Bounds x="160" y="80" width="36" height="36"/></bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="sE" bpmnElement="E"><dc:Bounds x="360" y="80" width="36" height="36"/></bpmndi:BPMNShape>
    <bpmndi:BPMNEdge id="eF" bpmnElement="F">
      <di:waypoint x="196" y="98"/><di:waypoint x="360" y="98"/>
    </bpmndi:BPMNEdge>
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#;

    #[test]
    fn renders_minimal_scene() {
        let document = from_xml(MINIMAL.as_bytes()).unwrap();
        let diagram = document.first_diagram().unwrap();
        let rendered = render_diagram(&document, diagram, &RenderOptions::default()).unwrap();
        assert!(!rendered.scene.nodes.is_empty());
        assert!(rendered.diagnostics.is_empty());
        // Viewport covers shapes plus padding.
        assert!(rendered.scene.viewport.width >= 236.0);
        assert!(rendered.scene.viewport.height >= 56.0);
    }

    #[test]
    fn first_diagram_helper_works() {
        let document = from_xml(MINIMAL.as_bytes()).unwrap();
        assert!(render_first_diagram(&document, &RenderOptions::default()).is_ok());
        assert!(matches!(
            render_first_diagram(&BpmnDocument::default(), &RenderOptions::default()),
            Err(RenderError::EmptyDiagram)
        ));
    }
    const EXTERNAL_LABELS: &str = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
        xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
        xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
        xmlns:di="http://www.omg.org/spec/DD/20100524/DI">
  <process id="P">
    <startEvent id="S" name="Start Event"/>
    <endEvent id="E"/>
    <sequenceFlow id="F" sourceRef="S" targetRef="E" name="to end"/>
  </process>
  <bpmndi:BPMNDiagram id="D"><bpmndi:BPMNPlane id="Pl" bpmnElement="P">
    <bpmndi:BPMNShape id="sS" bpmnElement="S"><dc:Bounds x="100" y="50" width="36" height="36"/>
      <bpmndi:BPMNLabel><dc:Bounds x="40" y="90" width="70" height="12"/></bpmndi:BPMNLabel>
    </bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="sE" bpmnElement="E"><dc:Bounds x="300" y="50" width="36" height="36"/></bpmndi:BPMNShape>
    <bpmndi:BPMNEdge id="eF" bpmnElement="F">
      <di:waypoint x="136" y="68"/><di:waypoint x="300" y="68"/>
      <bpmndi:BPMNLabel><dc:Bounds x="150" y="30" width="40" height="12"/></bpmndi:BPMNLabel>
    </bpmndi:BPMNEdge>
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#;

    #[test]
    fn viewport_includes_out_of_bounds_di_labels() {
        // MIWG A.1.0 regression (TASK-017): exporters may place BPMNLabel
        // bounds outside every shape; the viewport must cover them or the
        // SVG/PNG clips the text.
        let document = from_xml(EXTERNAL_LABELS.as_bytes()).unwrap();
        let diagram = document.first_diagram().unwrap();
        let viewport = diagram_viewport(diagram);
        // Shape-only union is x 100..336, y 50..86; labels reach x=40,
        // y=30 (edge label) and y=102 (shape label bottom).
        assert!(viewport.x <= 40.0, "min x {}, expected <= 40", viewport.x);
        assert!(viewport.y <= 30.0, "min y {}, expected <= 30", viewport.y);
        assert!(
            viewport.x + viewport.width >= 110.0,
            "max x {}, expected >= 110",
            viewport.x + viewport.width
        );
        assert!(
            viewport.y + viewport.height >= 102.0,
            "max y {}, expected >= 102",
            viewport.y + viewport.height
        );
    }

    #[test]
    fn viewport_ignores_invalid_label_bounds() {
        let document = from_xml(
            EXTERNAL_LABELS
                .replace(
                    "<dc:Bounds x=\"40\" y=\"90\" width=\"70\" height=\"12\"/>",
                    "<dc:Bounds x=\"40\" y=\"90\" width=\"0\" height=\"12\"/>",
                )
                .as_bytes(),
        )
        .unwrap();
        let diagram = document.first_diagram().unwrap();
        let viewport = diagram_viewport(diagram);
        // The invalid shape label must not extend the viewport to x=40; the
        // valid edge label (x 150..190, y 30..42) still does.
        assert!(viewport.x > 40.0, "min x {}, expected > 40", viewport.x);
        assert!(viewport.y <= 30.0, "min y {}, expected <= 30", viewport.y);
    }
}
