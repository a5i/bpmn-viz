//! Integration tests: parse-level behaviors across crates (AC-01, AC-02,
//! AC-03, AC-06).

use std::path::{Path, PathBuf};

use bpmn_validate::validate;
use bpmn_xml::from_xml;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn parse_fixture(name: &str) -> bpmn_model::BpmnDocument {
    let bytes = std::fs::read(fixture(name)).expect("fixture exists");
    from_xml(&bytes).expect("fixture parses")
}

#[test]
fn ac01_minimal_ids_and_names() {
    let document = parse_fixture("minimal/start-task-end.bpmn");
    let process = document.processes().next().unwrap();
    assert_eq!(process.id.as_deref(), Some("Process_Minimal"));
    assert_eq!(
        document.find_node("Task_1").unwrap().name.as_deref(),
        Some("Approve request")
    );
}

#[test]
fn ac02_invalid_fixtures_produce_expected_codes() {
    let duplicate = parse_fixture("invalid/duplicate-id.bpmn");
    let report = validate(&duplicate);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "reference.duplicate_id"
                && d.element_id.as_deref() == Some("Task_Dup"))
    );

    let dangling = parse_fixture("invalid/dangling-reference.bpmn");
    let report = validate(&dangling);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "reference.dangling"
                && d.element_id.as_deref() == Some("Flow_Broken"))
    );

    let nan = parse_fixture("invalid/nan-bounds.bpmn");
    let report = validate(&nan);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "geometry.invalid")
    );
}

#[test]
fn ac03_di_resolves_to_semantic_elements() {
    let document = parse_fixture("minimal/start-task-end.bpmn");
    let diagram = document.first_diagram().unwrap();
    for shape in &diagram.plane.shapes {
        assert!(
            document.find_node(&shape.bpmn_element).is_some(),
            "shape {} must resolve",
            shape.bpmn_element
        );
    }
    // Geometry is preserved verbatim.
    let task_shape = &diagram.plane.shapes[1];
    assert_eq!(
        task_shape.bounds,
        bpmn_model::di::Bounds::new(260.0, 78.0, 100.0, 80.0)
    );
}

#[test]
fn ac06_diagrams_are_enumerable_and_selectable() {
    let document = parse_fixture("edge-cases/multi-diagram.bpmn");
    assert_eq!(document.diagram_count(), 2);
    assert!(document.diagram_by_id("Diagram_Second").is_some());
    assert!(document.diagram_by_index(1).is_some());
    let second = document.diagram_by_index(1).unwrap();
    assert_eq!(second.plane.shapes.len(), 3);
}
