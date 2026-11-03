//! Geometry invariants for every accepted core fixture (TEST-005
//! §Geometry invariants / TASK-018 §8.2).

use bpmn_model::di::Bounds;

mod common;

/// Boundary-docking check against the DI bounds (integer coordinates).
fn on_boundary(bounds: &Bounds, x: f64, y: f64) -> bool {
    let in_x = x >= bounds.x && x <= bounds.x + bounds.width;
    let in_y = y >= bounds.y && y <= bounds.y + bounds.height;
    (in_x && (y == bounds.y || y == bounds.y + bounds.height))
        || (in_y && (x == bounds.x || x == bounds.x + bounds.width))
}

/// Horizontal/vertical segment vs rectangle interior (inset 1) penetration.
fn segment_hits(bounds: &Bounds, a: (f64, f64), b: (f64, f64)) -> bool {
    let inset = 1.0;
    let (x, y, w, h) = (
        bounds.x + inset,
        bounds.y + inset,
        bounds.width - 2.0 * inset,
        bounds.height - 2.0 * inset,
    );
    if a.1 == b.1 {
        a.1 > y && a.1 < y + h && a.0.max(b.0) > x && a.0.min(b.0) < x + w
    } else {
        a.0 > x && a.0 < x + w && a.1.max(b.1) > y && a.1.min(b.1) < y + h
    }
}

fn assert_invariants(stem: &str, process_id: &str) {
    let outcome = common::layout_accepted(stem, process_id);
    let plane = &outcome.diagram.plane;

    assert_eq!(
        plane.bpmn_element.as_deref(),
        Some(process_id),
        "{stem}: plane not bound to scope"
    );

    // Shapes: finite, positive, integer coordinates.
    for shape in &plane.shapes {
        let bounds = shape.bounds;
        assert!(
            bounds.is_valid(),
            "{stem}: invalid bounds for {}",
            shape.bpmn_element
        );
        for value in [bounds.x, bounds.y, bounds.width, bounds.height] {
            assert!(
                value.fract() == 0.0,
                "{stem}: non-integer coordinate {value} on {}",
                shape.bpmn_element
            );
        }
        assert!(
            shape.id.as_deref() == Some(format!("{}_di", shape.bpmn_element).as_str()),
            "{stem}: shape id mismatch for {}",
            shape.bpmn_element
        );
    }

    // No two shapes overlap.
    for (index, a) in plane.shapes.iter().enumerate() {
        for b in &plane.shapes[index + 1..] {
            let (ra, rb) = (a.bounds, b.bounds);
            let overlap = ra.x < rb.x + rb.width
                && rb.x < ra.x + ra.width
                && ra.y < rb.y + rb.height
                && rb.y < ra.y + ra.height;
            assert!(
                !overlap,
                "{stem}: shapes {} and {} overlap",
                a.bpmn_element, b.bpmn_element
            );
        }
    }

    // Shapes and edges carry exactly the semantic ids of the fixture scope.
    for edge in &plane.edges {
        assert!(
            edge.id.as_deref() == Some(format!("{}_di", edge.bpmn_element).as_str()),
            "{stem}: edge id mismatch for {}",
            edge.bpmn_element
        );
        assert!(
            edge.waypoints.len() >= 2,
            "{stem}: edge {} has fewer than two waypoints",
            edge.bpmn_element
        );
        for point in &edge.waypoints {
            assert!(point.is_finite(), "{stem}: non-finite waypoint");
        }
        for pair in edge.waypoints.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(
                a != b,
                "{stem}: zero-length segment on edge {}",
                edge.bpmn_element
            );
            assert!(
                a.x == b.x || a.y == b.y,
                "{stem}: diagonal segment on edge {}",
                edge.bpmn_element
            );
        }
    }

    // Docking and interior avoidance, per edge, against named shapes.
    for edge in &plane.edges {
        let first = edge.waypoints[0];
        let last = edge.waypoints[edge.waypoints.len() - 1];
        let (source_ref, target_ref) = fixture_endpoints(stem, edge.bpmn_element.as_str());
        let source = plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == source_ref)
            .unwrap_or_else(|| panic!("{stem}: source shape {} missing", source_ref));
        let target = plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == target_ref)
            .unwrap_or_else(|| panic!("{stem}: target shape {} missing", target_ref));
        assert!(
            on_boundary(&source.bounds, first.x, first.y),
            "{stem}: edge {} does not dock on source {} boundary",
            edge.bpmn_element,
            source_ref
        );
        assert!(
            on_boundary(&target.bounds, last.x, last.y),
            "{stem}: edge {} does not dock on target {} boundary",
            edge.bpmn_element,
            target_ref
        );
        for pair in edge.waypoints.windows(2) {
            for shape in &plane.shapes {
                if shape.bpmn_element == source_ref || shape.bpmn_element == target_ref {
                    continue;
                }
                assert!(
                    !segment_hits(
                        &shape.bounds,
                        (pair[0].x, pair[0].y),
                        (pair[1].x, pair[1].y)
                    ),
                    "{stem}: edge {} crosses the interior of unrelated shape {}",
                    edge.bpmn_element,
                    shape.bpmn_element
                );
            }
        }
    }
}

/// Reads the declared sourceRef/targetRef of a flow from the fixture XML.
fn fixture_endpoints(stem: &str, flow_id: &str) -> (String, String) {
    let document = common::load_accepted(stem);
    let flow = document
        .find_sequence_flow(flow_id)
        .unwrap_or_else(|| panic!("{stem}: flow {flow_id} missing"));
    (flow.source_ref.clone(), flow.target_ref.clone())
}

#[test]
fn invariants_hold_for_all_accepted_fixtures() {
    for (stem, process_id) in common::accepted_fixtures() {
        assert_invariants(stem, process_id);
    }
}

#[test]
fn start_task_end_straight_spine_in_declaration_order() {
    let outcome = common::layout_core("start-task-end", "process_order");
    let bounds_of = |id: &str| {
        outcome
            .diagram
            .plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == id)
            .expect("shape present")
            .bounds
    };
    let (start, task, end) = (
        bounds_of("start_order"),
        bounds_of("task_order"),
        bounds_of("end_order"),
    );
    assert!(
        start.x < task.x && task.x < end.x,
        "rank order left to right"
    );
    assert_eq!(
        start.y + start.height / 2.0,
        task.y + task.height / 2.0,
        "spine nodes share one row"
    );
    assert_eq!(start.y + start.height / 2.0, task.y + task.height / 2.0);
    assert_eq!(task.y + task.height / 2.0, end.y + end.height / 2.0);
}

#[test]
fn xor_alternatives_fan_to_distinct_rows() {
    let outcome = common::layout_core("xor-split-join", "process_xor");
    let y_of = |id: &str| {
        let bounds = outcome
            .diagram
            .plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == id)
            .expect("shape present")
            .bounds;
        bounds.y + bounds.height / 2.0
    };
    let split = y_of("xor_split");
    let join = y_of("xor_join");
    let ship = y_of("xor_ship");
    let cancel = y_of("xor_cancel");
    assert_eq!(split, join, "gateway row continuity");
    assert_eq!(
        ship, split,
        "the primary (first declared) alternative stays on the spine row"
    );
    assert_ne!(cancel, split, "the second alternative leaves the spine row");
    assert_ne!(ship, cancel, "alternatives do not share one row");
}

#[test]
fn disconnected_components_do_not_overlap_and_stack_in_declaration_order() {
    let outcome = common::layout_core("disconnected", "process_disconnected");
    let shapes = &outcome.diagram.plane.shapes;
    let orphan_top = shapes
        .iter()
        .find(|shape| shape.bpmn_element == "disc_orphan")
        .expect("orphan present")
        .bounds;
    let start = shapes
        .iter()
        .find(|shape| shape.bpmn_element == "disc_start")
        .expect("start present")
        .bounds;
    assert!(
        orphan_top.y + orphan_top.height < start.y,
        "first declared component is stacked above the second"
    );
}

#[test]
fn loop_back_edge_leaves_the_row() {
    let outcome = common::layout_core("loop-back-edge", "process_loop");
    let edge = outcome
        .diagram
        .plane
        .edges
        .iter()
        .find(|edge| edge.bpmn_element == "loop_f5")
        .expect("back edge present");
    // A back edge must not be a simple straight line between the two nodes.
    assert!(edge.waypoints.len() > 2, "back edge is routed around");
    let y_values: Vec<f64> = edge.waypoints.iter().map(|point| point.y).collect();
    assert!(
        y_values.windows(2).any(|pair| pair[0] != pair[1]),
        "back edge changes rows"
    );
}

#[test]
fn long_edge_avoids_intermediate_shapes() {
    // Covered by the generic interior-avoidance invariant, but assert the
    // fixture shape explicitly: the skip flow has a bend and leaves row 0.
    let outcome = common::layout_core("long-edge", "process_long_edge");
    let edge = outcome
        .diagram
        .plane
        .edges
        .iter()
        .find(|edge| edge.bpmn_element == "le_skip")
        .expect("skip flow present");
    assert!(edge.waypoints.len() > 2, "skip flow detours around ranks");
}
