//! DI generation and invariant verification (TASK-018 §5.5).
//!
//! `generate` converts the internal placement and routes into the typed
//! `bpmn_model::di` model: one `BPMNShape` per placed node and one
//! `BPMNEdge` per placed flow, with ids `<bpmnElement>_di` and integer
//! coordinates (the single rounding/normalization point of the engine).
//! `verify` re-checks the TEST-005 geometry invariants against the produced
//! diagram alone; a violation is an engine defect (`VerificationFailed`).

use bpmn_model::di::{Bounds, Diagram, Edge, Plane, Point, Shape};

use crate::api::{LayoutError, ScopeId};
use crate::geometry::IRect;
use crate::graph::ScopeGraph;
use crate::placement::Placement;
use crate::routing::Routes;

/// Generates the typed DI diagram for the placed scope.
pub(crate) fn generate(
    scope: &ScopeId,
    graph: &ScopeGraph,
    placement: &Placement,
    routes: &Routes,
) -> Diagram {
    let scope_id = match scope {
        ScopeId::Process(id) => id.clone(),
    };
    let shapes: Vec<Shape> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let rect = placement.rects[index];
            Shape {
                id: Some(format!("{}_di", node.id)),
                bpmn_element: node.id.clone(),
                bounds: Bounds::new(rect.x as f64, rect.y as f64, rect.w as f64, rect.h as f64),
                label: None,
                is_horizontal: None,
                is_expanded: None,
            }
        })
        .collect();
    let edges: Vec<Edge> = graph
        .flows
        .iter()
        .enumerate()
        .map(|(index, flow)| Edge {
            id: Some(format!("{}_di", flow.id)),
            bpmn_element: flow.id.clone(),
            waypoints: routes.waypoints[index]
                .iter()
                .map(|(x, y)| Point::new(*x as f64, *y as f64))
                .collect(),
            label: None,
        })
        .collect();
    Diagram {
        id: Some(format!("BPMNDiagram_{scope_id}")),
        name: None,
        plane: Plane {
            id: Some(format!("BPMNPlane_{scope_id}")),
            bpmn_element: Some(scope_id),
            shapes,
            edges,
        },
    }
}

/// Verifies the TEST-005 geometry invariants against the generated diagram.
/// Any violation is an engine defect, never a property of valid input.
pub(crate) fn verify(
    diagram: &Diagram,
    graph: &ScopeGraph,
    scope: &ScopeId,
) -> Result<(), LayoutError> {
    let fail = |reason: String| LayoutError::VerificationFailed { reason };
    let plane = &diagram.plane;

    // Coverage: exactly one shape per placed node, one edge per placed flow.
    if plane.shapes.len() != graph.nodes.len() {
        return Err(fail(format!(
            "expected {} shapes, found {}",
            graph.nodes.len(),
            plane.shapes.len()
        )));
    }
    if plane.edges.len() != graph.flows.len() {
        return Err(fail(format!(
            "expected {} edges, found {}",
            graph.flows.len(),
            plane.edges.len()
        )));
    }
    let scope_id = match scope {
        ScopeId::Process(id) => id.as_str(),
    };
    if plane.bpmn_element.as_deref() != Some(scope_id) {
        return Err(fail(
            "plane is not bound to the selected process".to_string(),
        ));
    }
    for (shape, node) in plane.shapes.iter().zip(&graph.nodes) {
        if shape.bpmn_element != node.id {
            return Err(fail(format!("shape order diverged at `{}`", node.id)));
        }
        let bounds = shape.bounds;
        if !bounds.is_valid() {
            return Err(fail(format!("invalid bounds for `{}`", node.id)));
        }
    }
    for (edge, flow) in plane.edges.iter().zip(&graph.flows) {
        if edge.bpmn_element != flow.id {
            return Err(fail(format!("edge order diverged at `{}`", flow.id)));
        }
        if edge.waypoints.len() < 2 {
            return Err(fail(format!(
                "edge `{}` has fewer than two waypoints",
                flow.id
            )));
        }
        for point in &edge.waypoints {
            if !point.is_finite() {
                return Err(fail(format!("non-finite waypoint on edge `{}`", flow.id)));
            }
        }
        // Zero-length segments and axis alignment between consecutive points.
        for pair in edge.waypoints.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a == b {
                return Err(fail(format!("zero-length segment on edge `{}`", flow.id)));
            }
            if a.x != b.x && a.y != b.y {
                return Err(fail(format!(
                    "edge `{}` contains a diagonal segment",
                    flow.id
                )));
            }
        }
        // Docking: first waypoint on the source boundary, last on the target.
        let source_shape = plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == graph.nodes[flow.source].id)
            .ok_or_else(|| fail(format!("missing source shape for edge `{}`", flow.id)))?;
        let target_shape = plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == graph.nodes[flow.target].id)
            .ok_or_else(|| fail(format!("missing target shape for edge `{}`", flow.id)))?;
        let source_rect = to_irect(&source_shape.bounds);
        let target_rect = to_irect(&target_shape.bounds);
        let first = edge.waypoints[0];
        let last = *edge.waypoints.last().expect("len >= 2 checked");
        if !source_rect.on_boundary(first.x as i64, first.y as i64) {
            return Err(fail(format!(
                "edge `{}` does not dock on its source boundary",
                flow.id
            )));
        }
        if !target_rect.on_boundary(last.x as i64, last.y as i64) {
            return Err(fail(format!(
                "edge `{}` does not dock on its target boundary",
                flow.id
            )));
        }
        // Interior avoidance: no segment enters an unrelated shape (inset by
        // 1 so boundary runs are legal).
        for segment in edge.waypoints.windows(2) {
            let (a, b) = (segment[0], segment[1]);
            for shape in &plane.shapes {
                if shape.bpmn_element == source_shape.bpmn_element
                    || shape.bpmn_element == target_shape.bpmn_element
                {
                    continue;
                }
                let rect = to_irect(&shape.bounds).inset(1);
                let hit = if a.y == b.y {
                    crate::geometry::horizontal_segment_hits(
                        a.x as i64, b.x as i64, a.y as i64, &rect,
                    )
                } else {
                    crate::geometry::vertical_segment_hits(
                        a.x as i64, a.y as i64, b.y as i64, &rect,
                    )
                };
                if hit {
                    return Err(fail(format!(
                        "edge `{}` enters the interior of `{}`",
                        flow.id, shape.bpmn_element
                    )));
                }
            }
        }
    }

    // No two shapes overlap.
    for (index, a) in plane.shapes.iter().enumerate() {
        for b in &plane.shapes[index + 1..] {
            let (ra, rb) = (to_irect(&a.bounds), to_irect(&b.bounds));
            if ra.overlaps(&rb) {
                return Err(fail(format!(
                    "shapes `{}` and `{}` overlap",
                    a.bpmn_element, b.bpmn_element
                )));
            }
        }
    }
    Ok(())
}

/// Converts validated DI bounds into the internal integer rectangle.
fn to_irect(bounds: &Bounds) -> IRect {
    IRect::new(
        bounds.x as i64,
        bounds.y as i64,
        bounds.width as i64,
        bounds.height as i64,
    )
}
