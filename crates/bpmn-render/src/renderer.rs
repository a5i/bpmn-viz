//! Render orchestration: indexing, z-order and viewport computation.

use std::collections::{HashMap, HashSet};

use crate::flows;
use crate::glyphs;
use crate::labels;
use crate::{RenderError, RenderOptions, RenderedDiagram};
use bpmn_model::BpmnDocument;
use bpmn_model::di::Diagram;
use bpmn_model::diagnostic::{Diagnostic, DiagnosticCode};
use bpmn_model::element::{FlowElement, NodeKind};
use bpmn_scene::style::{Marker, Orient, Paint, Viewport};
use bpmn_scene::{Scene, ids};
use kurbo::BezPath;

/// Semantic and DI indexes for one render pass.
pub(crate) struct RenderIndexes<'a> {
    pub nodes: HashMap<&'a str, &'a bpmn_model::element::FlowNode>,
    pub flows: HashMap<&'a str, &'a bpmn_model::element::SequenceFlow>,
    pub participants: HashMap<&'a str, &'a bpmn_model::element::Participant>,
    pub lanes: HashMap<&'a str, &'a bpmn_model::element::Lane>,
    pub message_flows: HashMap<&'a str, &'a bpmn_model::element::MessageFlow>,
    pub annotations: HashMap<&'a str, &'a bpmn_model::element::TextAnnotation>,
    /// Ids inside collapsed subprocess bodies (their shapes are skipped).
    pub skipped: HashSet<&'a str>,
}

pub(crate) fn build_indexes<'a>(
    document: &'a BpmnDocument,
    diagram: &'a Diagram,
) -> RenderIndexes<'a> {
    let mut nodes = HashMap::new();
    let mut flows = HashMap::new();
    let mut annotations = HashMap::new();
    for process in document.processes() {
        index_body(
            &process.flow_elements,
            &mut nodes,
            &mut flows,
            &mut annotations,
        );
        for artifact in &process.artifacts {
            if let bpmn_model::element::Artifact::TextAnnotation(annotation) = artifact {
                annotations.insert(annotation.id.as_str(), annotation);
            }
        }
    }
    let mut participants = HashMap::new();
    let mut lanes = HashMap::new();
    let mut message_flows = HashMap::new();
    for collaboration in document.collaborations() {
        for participant in &collaboration.participants {
            participants.insert(participant.id.as_str(), participant);
        }
        for flow in &collaboration.message_flows {
            message_flows.insert(flow.id.as_str(), flow);
        }
    }
    for process in document.processes() {
        for lane_set in &process.lane_sets {
            for lane in &lane_set.lanes {
                lanes.insert(lane.id.as_str(), lane);
            }
        }
    }
    let collapsed: HashSet<&str> = diagram
        .plane
        .shapes
        .iter()
        .filter(|shape| shape.is_expanded == Some(false))
        .map(|shape| shape.bpmn_element.as_str())
        .collect();
    let mut skipped = HashSet::new();
    for id in &collapsed {
        if let Some(node) = nodes.get(id)
            && let NodeKind::SubProcess { body, .. } = &node.kind
        {
            collect_body_ids(body, &mut skipped);
        }
    }
    RenderIndexes {
        nodes,
        flows,
        participants,
        lanes,
        annotations,
        message_flows,
        skipped,
    }
}

fn index_body<'a>(
    body: &'a [FlowElement],
    nodes: &mut HashMap<&'a str, &'a bpmn_model::element::FlowNode>,
    flows: &mut HashMap<&'a str, &'a bpmn_model::element::SequenceFlow>,
    _annotations: &mut HashMap<&'a str, &'a bpmn_model::element::TextAnnotation>,
) {
    for element in body {
        match element {
            FlowElement::Node(node) => {
                nodes.insert(node.id.as_str(), node);
                if let NodeKind::SubProcess { body, .. } = &node.kind {
                    index_body(body, nodes, flows, _annotations);
                }
            }
            FlowElement::Flow(flow) => {
                flows.insert(flow.id.as_str(), flow);
            }
        }
    }
}

fn collect_body_ids<'a>(body: &'a [FlowElement], out: &mut HashSet<&'a str>) {
    for element in body {
        match element {
            FlowElement::Node(node) => {
                out.insert(node.id.as_str());
                if let NodeKind::SubProcess { body, .. } = &node.kind {
                    collect_body_ids(body, out);
                }
            }
            FlowElement::Flow(flow) => {
                out.insert(flow.id.as_str());
            }
        }
    }
}

/// Unions all plane geometry into the raw viewport (before padding).
pub fn union_viewport(diagram: &Diagram) -> Viewport {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for shape in &diagram.plane.shapes {
        min_x = min_x.min(shape.bounds.x);
        min_y = min_y.min(shape.bounds.y);
        max_x = max_x.max(shape.bounds.x + shape.bounds.width);
        max_y = max_y.max(shape.bounds.y + shape.bounds.height);
    }
    for edge in &diagram.plane.edges {
        for point in &edge.waypoints {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
    }
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return Viewport::new(0.0, 0.0, 1.0, 1.0);
    }
    let width = (max_x - min_x).max(50.0);
    let height = (max_y - min_y).max(50.0);
    Viewport::new(min_x, min_y, width, height)
}

pub(crate) fn render(
    document: &BpmnDocument,
    diagram: &Diagram,
    options: &RenderOptions,
) -> Result<RenderedDiagram, RenderError> {
    let indexes = build_indexes(document, diagram);
    let mut diagnostics = Vec::new();

    // Z-order contract: pool/lanes → flows → nodes → node markers → labels.
    let mut scene = Scene::new(union_viewport(diagram).padded_viewport(options.padding));
    scene.define_marker(arrow_marker(options));
    scene.define_marker(conditional_marker(options));

    // 1. Containers (participants and lanes).
    for shape in &diagram.plane.shapes {
        if indexes.skipped.contains(shape.bpmn_element.as_str()) {
            continue;
        }
        if indexes
            .participants
            .contains_key(shape.bpmn_element.as_str())
        {
            glyphs::draw_participant(&mut scene, shape, options);
        } else if indexes.lanes.contains_key(shape.bpmn_element.as_str()) {
            glyphs::draw_lane(&mut scene, shape, options);
        }
    }

    // 2. Flows.
    for edge in &diagram.plane.edges {
        if indexes.skipped.contains(edge.bpmn_element.as_str()) {
            continue;
        }
        if let Some(flow) = indexes.flows.get(edge.bpmn_element.as_str()) {
            flows::draw_flow(&mut scene, flow, edge, options);
            labels::draw_flow_label(&mut scene, flow, edge, options);
        } else if let Some(message_flow) = indexes.message_flows.get(edge.bpmn_element.as_str()) {
            flows::draw_message_flow(&mut scene, message_flow, edge, options);
            labels::draw_message_flow_label(&mut scene, message_flow, edge, options);
        }
    }

    // 3./4./5. Nodes with glyphs, markers and labels.
    for shape in &diagram.plane.shapes {
        if indexes.skipped.contains(shape.bpmn_element.as_str()) {
            continue;
        }
        let id = shape.bpmn_element.as_str();
        if indexes.participants.contains_key(id) {
            labels::draw_participant_label(
                &mut scene,
                indexes.participants.get(id).and_then(|p| p.name.as_deref()),
                shape,
                options,
            );
            continue;
        }
        if indexes.lanes.contains_key(id) {
            labels::draw_lane_label(
                &mut scene,
                indexes.lanes.get(id).and_then(|l| l.name.as_deref()),
                shape,
                options,
            );
            continue;
        }
        if let Some(node) = indexes.nodes.get(id) {
            glyphs::draw_node(&mut scene, node, shape, options, &mut diagnostics);
            labels::draw_node_label(&mut scene, node, shape, options);
            continue;
        }
        if let Some(annotation) = indexes.annotations.get(id) {
            glyphs::draw_annotation(&mut scene, annotation, shape, options);
            continue;
        }
        diagnostics.push(
            Diagnostic::warning(
                DiagnosticCode::DanglingDiReference,
                format!(
                    "shape references `{id}` which is not a rendered semantic element; drawing placeholder"
                ),
            )
            .with_element_id(id.to_string()),
        );
        glyphs::draw_placeholder(&mut scene, shape, options);
    }

    Ok(RenderedDiagram { scene, diagnostics })
}

fn arrow_marker(options: &RenderOptions) -> Marker {
    let fill = Paint::Color(options.theme.flow_stroke);
    // Triangle pointing right, tip at (10, 5).
    let mut data = BezPath::new();
    data.move_to((0.0, 0.0));
    data.line_to((10.0, 5.0));
    data.line_to((0.0, 10.0));
    data.close_path();
    Marker {
        id: ids::ARROW_END.to_string(),
        view_box_size: 10.0,
        ref_x: 9.0,
        ref_y: 5.0,
        marker_width: 10.0,
        marker_height: 10.0,
        orient: Orient::Auto,
        data,
        fill,
    }
}

fn conditional_marker(options: &RenderOptions) -> Marker {
    let stroke = options.theme.flow_stroke;
    // Small outline diamond used at flow start for conditional flows.
    let mut data = BezPath::new();
    data.move_to((0.0, 5.0));
    data.line_to((5.0, 0.0));
    data.line_to((10.0, 5.0));
    data.line_to((5.0, 10.0));
    data.close_path();
    Marker {
        id: ids::CONDITIONAL_MARKER.to_string(),
        view_box_size: 10.0,
        ref_x: 0.5,
        ref_y: 5.0,
        marker_width: 10.0,
        marker_height: 10.0,
        orient: Orient::Auto,
        data,
        fill: Paint::Color(stroke),
    }
}

/// Viewport with padding applied (never negative origin drift).
trait Padded {
    fn padded_viewport(&self, pad: f64) -> Viewport;
}

impl Padded for Viewport {
    fn padded_viewport(&self, pad: f64) -> Viewport {
        Viewport::new(
            self.x - pad,
            self.y - pad,
            self.width + 2.0 * pad,
            self.height + 2.0 * pad,
        )
    }
}
