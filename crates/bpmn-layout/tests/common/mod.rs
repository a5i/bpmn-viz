//! Shared helpers for the `bpmn-layout` integration tests.
//!
//! Each integration test binary compiles this module separately, so helpers
//! that a given binary does not use are expected and allowed.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use bpmn_layout::{LayoutEngine, LayoutOptions, LayoutOutcome, NativeBpmnLayout, ScopeId};
use bpmn_model::BpmnDocument;
use bpmn_xml::from_xml;

/// The layout fixtures directory of this crate.
#[allow(dead_code)]
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The eight accepted core fixtures with their process ids (TEST-005).
pub const CORE_FIXTURES: &[(&str, &str)] = &[
    ("start-task-end", "process_order"),
    ("xor-split-join", "process_xor"),
    ("parallel-split-join", "process_parallel"),
    ("loop-back-edge", "process_loop"),
    ("multiple-starts", "process_multi_start"),
    ("disconnected", "process_disconnected"),
    ("reverse-declaration-order", "process_reverse"),
    ("long-edge", "process_long_edge"),
];

/// The twenty accepted complex stress fixtures with their process ids
/// (TEST-005 §Fixtures). They run through the invariant, determinism,
/// warning-contract and consumability suites; frozen coordinate snapshots and
/// reference parity stay on the core set.
#[allow(dead_code)]
pub const COMPLEX_FIXTURES: &[(&str, &str)] = &[
    ("c01-order-fulfillment", "process_c01"),
    ("c02-insurance-claim", "process_c02"),
    ("c03-loan-approval", "process_c03"),
    ("c04-travel-booking", "process_c04"),
    ("c05-invoice-processing", "process_c05"),
    ("c06-software-release", "process_c06"),
    ("c07-hiring-pipeline", "process_c07"),
    ("c08-manufacturing-qc", "process_c08"),
    ("c09-content-moderation", "process_c09"),
    ("c10-supply-chain-delay", "process_c10"),
    ("c11-patient-intake", "process_c11"),
    ("c12-asymmetric-branches", "process_c12"),
    ("c13-double-diamond", "process_c13"),
    ("c14-long-pipeline", "process_c14"),
    ("c15-scrambled-declaration", "process_c15"),
    ("c16-disconnected-zoo", "process_c16"),
    ("c17-crossing-flows", "process_c17"),
    ("c18-wide-fan-in", "process_c18"),
    ("c19-escalation-maze", "process_c19"),
    ("c20-subprocess-cascade", "process_c20"),
];

/// Every accepted fixture (core + complex) in stable order.
#[allow(dead_code)]
pub fn accepted_fixtures() -> impl Iterator<Item = (&'static str, &'static str)> {
    CORE_FIXTURES
        .iter()
        .copied()
        .chain(COMPLEX_FIXTURES.iter().copied())
}

/// Loads an accepted fixture by stem, resolving the `core/` or `complex/`
/// class directory from the fixture table.
#[allow(dead_code)]
pub fn load_accepted(stem: &str) -> BpmnDocument {
    let in_complex = COMPLEX_FIXTURES
        .iter()
        .any(|(candidate, _)| *candidate == stem);
    let class = if in_complex { "complex" } else { "core" };
    load_fixture(&format!("{class}/{stem}.bpmn"))
}

/// Lays out any accepted fixture (core or complex) with default options.
#[allow(dead_code)]
pub fn layout_accepted(stem: &str, process_id: &str) -> LayoutOutcome {
    let document = load_accepted(stem);
    NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process(process_id),
            &LayoutOptions::default(),
        )
        .unwrap_or_else(|error| panic!("layout {stem}: {error}"))
}

/// Loads a fixture XML from the fixtures directory.
#[allow(dead_code)]
pub fn load_fixture(relative: &str) -> BpmnDocument {
    let bytes = std::fs::read(fixtures_dir().join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"));
    from_xml(&bytes).unwrap_or_else(|error| panic!("parse {relative}: {error}"))
}

/// Lays out a core fixture with default options.
#[allow(dead_code)]
pub fn layout_core(stem: &str, process_id: &str) -> LayoutOutcome {
    let document = load_fixture(&format!("core/{stem}.bpmn"));
    NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process(process_id),
            &LayoutOptions::default(),
        )
        .unwrap_or_else(|error| panic!("layout {stem}: {error}"))
}

/// Builds the canonical JSON representation used by snapshots and differential
/// tests: shapes and edges sorted by semantic id, integer coordinates,
/// warnings reduced to stable codes.
#[allow(dead_code)]
pub fn canonical(outcome: &LayoutOutcome) -> serde_json::Value {
    let mut shapes: Vec<serde_json::Value> = outcome
        .diagram
        .plane
        .shapes
        .iter()
        .map(|shape| {
            serde_json::json!({
                "bpmnElement": shape.bpmn_element,
                "x": shape.bounds.x,
                "y": shape.bounds.y,
                "width": shape.bounds.width,
                "height": shape.bounds.height,
            })
        })
        .collect();
    shapes.sort_by_key(|shape| shape["bpmnElement"].as_str().map(str::to_string));

    let mut edges: Vec<serde_json::Value> = outcome
        .diagram
        .plane
        .edges
        .iter()
        .map(|edge| {
            serde_json::json!({
                "bpmnElement": edge.bpmn_element,
                "waypoints": edge
                    .waypoints
                    .iter()
                    .map(|point| [point.x, point.y])
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    edges.sort_by_key(|edge| edge["bpmnElement"].as_str().map(str::to_string));

    let warnings: Vec<String> = outcome
        .warnings
        .iter()
        .map(|warning| warning.code.as_str().to_string())
        .collect();

    serde_json::json!({
        "shapes": shapes,
        "edges": edges,
        "warnings": warnings,
    })
}
