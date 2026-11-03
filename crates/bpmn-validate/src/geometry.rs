//! Geometry checks: bounds validity, waypoint validity and DI coverage.

use std::collections::HashSet;

use bpmn_model::BpmnDocument;
use bpmn_model::diagnostic::{Diagnostic, DiagnosticCode};
use bpmn_model::element::{FlowElement, NodeKind};

/// Runs all geometry checks; returns unsorted diagnostics.
pub fn check_geometry(document: &BpmnDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if document.definitions.diagrams.is_empty() {
        // One actionable document-level error instead of per-element noise
        // (AC-09: no hidden fallback).
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::MissingDiagramInterchange,
            "document contains no bpmndi:BPMNDiagram; rendering requires provided geometry",
        ));
        return diagnostics;
    }

    let shapes_by_element = document
        .definitions
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.plane.shapes.iter())
        .map(|shape| (shape.bpmn_element.as_str(), shape))
        .collect::<Vec<_>>();

    // Bounds and label validity (AC-02).
    for diagram in &document.definitions.diagrams {
        for shape in &diagram.plane.shapes {
            if !shape.bounds.is_valid() {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidGeometry,
                        format!(
                            "BPMNShape bounds must be finite with positive width and height (got x={}, y={}, width={}, height={})",
                            shape.bounds.x, shape.bounds.y, shape.bounds.width, shape.bounds.height
                        ),
                    )
                    .with_element_id(shape.bpmn_element.clone()),
                );
            }
            if let Some(label_bounds) = shape.label.and_then(|label| label.bounds)
                && !label_bounds.is_valid()
            {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidGeometry,
                        "BPMNShape label bounds must be finite with positive size",
                    )
                    .with_element_id(shape.bpmn_element.clone()),
                );
            }
        }
        for edge in &diagram.plane.edges {
            if edge.waypoints.len() < 2 {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::MissingWaypoints,
                        format!(
                            "BPMNEdge has {} waypoint(s); at least two are required",
                            edge.waypoints.len()
                        ),
                    )
                    .with_element_id(edge.bpmn_element.clone()),
                );
            }
            for waypoint in &edge.waypoints {
                if !waypoint.is_finite() {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidGeometry,
                            "BPMNEdge waypoints must be finite numbers",
                        )
                        .with_element_id(edge.bpmn_element.clone()),
                    );
                    break;
                }
            }
            if let Some(label_bounds) = edge.label.and_then(|label| label.bounds)
                && !label_bounds.is_valid()
            {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidGeometry,
                        "BPMNEdge label bounds must be finite with positive size",
                    )
                    .with_element_id(edge.bpmn_element.clone()),
                );
            }
        }
    }

    // Coverage: every rendered element needs geometry somewhere (AC-09).
    let shaped: HashSet<&str> = shapes_by_element.iter().map(|(id, _)| *id).collect();
    let collapsed: HashSet<&str> = shapes_by_element
        .iter()
        .filter(|(_, shape)| shape.is_expanded == Some(false))
        .map(|(id, _)| *id)
        .collect();

    let mut missing_shapes: Vec<(String, &'static str)> = Vec::new();
    let mut missing_edges: Vec<String> = Vec::new();

    let has_edge = |id: &str| {
        document.definitions.diagrams.iter().any(|diagram| {
            diagram
                .plane
                .edges
                .iter()
                .any(|edge| edge.bpmn_element == id)
        })
    };

    for root in &document.definitions.root_elements {
        match root {
            bpmn_model::element::RootElement::Process(process) => {
                check_coverage_body(
                    &process.flow_elements,
                    &collapsed,
                    &shaped,
                    &has_edge,
                    &mut missing_shapes,
                    &mut missing_edges,
                );
                for lane_set in &process.lane_sets {
                    for lane in &lane_set.lanes {
                        if !shaped.contains(lane.id.as_str()) {
                            missing_shapes.push((lane.id.clone(), "lane"));
                        }
                    }
                }
                for artifact in &process.artifacts {
                    if !shaped.contains(artifact.id()) {
                        missing_shapes.push((artifact.id().to_string(), "text annotation"));
                    }
                }
            }
            bpmn_model::element::RootElement::Collaboration(collaboration) => {
                for participant in &collaboration.participants {
                    if !shaped.contains(participant.id.as_str()) {
                        missing_shapes.push((participant.id.clone(), "participant"));
                    }
                }
            }
        }
    }

    for (id, kind) in &missing_shapes {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::MissingShape,
                format!(
                    "no bpmndi:BPMNShape exists for {kind} `{id}`; auto-layout is not supported (ADR-0003)"
                ),
            )
            .with_element_id(id.clone()),
        );
    }
    for id in &missing_edges {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::MissingShape,
                format!(
                    "no bpmndi:BPMNEdge exists for sequence flow `{id}`; auto-layout is not supported (ADR-0003)"
                ),
            )
            .with_element_id(id.clone()),
        );
    }

    diagnostics
}

fn check_coverage_body(
    elements: &[FlowElement],
    collapsed: &HashSet<&str>,
    shaped: &HashSet<&str>,
    has_edge: &impl Fn(&str) -> bool,
    missing_shapes: &mut Vec<(String, &'static str)>,
    missing_edges: &mut Vec<String>,
) {
    for element in elements {
        match element {
            FlowElement::Node(node) => {
                if !shaped.contains(node.id.as_str()) {
                    missing_shapes.push((node.id.clone(), "flow node"));
                }
                if let NodeKind::SubProcess { body, .. } = &node.kind
                    && !collapsed.contains(node.id.as_str())
                {
                    check_coverage_body(
                        body,
                        collapsed,
                        shaped,
                        has_edge,
                        missing_shapes,
                        missing_edges,
                    );
                }
            }
            FlowElement::Flow(flow) => {
                if !has_edge(&flow.id) {
                    missing_edges.push(flow.id.clone());
                }
            }
        }
    }
}
