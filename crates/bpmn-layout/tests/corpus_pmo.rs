//! PMo external corpus smoke (TEST-005 §Stress coverage extension, ADR-0008,
//! TASK-019). The pinned PMo Dataset (Zenodo 10.5281/zenodo.15857589, v1.0.0,
//! CC-BY-4.0) provides 55 real process models as ground-truth BPMN with DI
//! (`external/pmo/bpmn/`) and the same models without the diagram definition
//! (`external/pmo/bpmn_process/`).
//!
//! The suite self-skips unless `scripts/download-fixtures.sh` has materialized
//! the corpus (local/scheduled runs only; per-PR CI stays synthetic-only per
//! ADR-0005). Corpus files are untrusted third-party input: nothing here may
//! panic, and every failure must surface as a typed error with a stable code.

use std::path::{Path, PathBuf};

use bpmn_layout::{LayoutEngine, LayoutOptions, NativeBpmnLayout, ScopeId};
use bpmn_model::BpmnDocument;
use bpmn_model::di::Bounds;
use bpmn_render::{RenderError, RenderOptions, render_first_diagram};
use bpmn_xml::from_xml;

mod common;

/// The corpus is versioned by the lock manifest; a drift here means the
/// upstream dataset changed and the manifest must be re-pinned first.
const EXPECTED_MODELS: usize = 55;

fn corpus_dir() -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../external/pmo");
    dir.is_dir().then_some(dir)
}

fn corpus_files(dir: &Path, sub: &str) -> Vec<(String, PathBuf)> {
    let mut files: Vec<_> = std::fs::read_dir(dir.join(sub))
        .unwrap_or_else(|error| panic!("read corpus {sub}: {error}"))
        .map(|entry| entry.expect("corpus dir entry"))
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "bpmn"))
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            )
        })
        .collect();
    files.sort();
    files
}

fn load(path: &Path, name: &str) -> BpmnDocument {
    let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("read {name}: {error}"));
    from_xml(&bytes).unwrap_or_else(|error| panic!("{name}: corpus file failed to parse: {error}"))
}

fn first_process_id(document: &BpmnDocument) -> String {
    document
        .processes()
        .find_map(|process| process.id.clone())
        .unwrap_or_default()
}

fn assert_invariants(name: &str, document: &BpmnDocument, layout: &bpmn_layout::LayoutOutcome) {
    let plane = &layout.diagram.plane;

    // Shapes: valid, positive, integer coordinates (TEST-005 §Geometry).
    for shape in &plane.shapes {
        let bounds = shape.bounds;
        assert!(bounds.is_valid(), "{name}: invalid bounds for {shape:?}");
        for value in [bounds.x, bounds.y, bounds.width, bounds.height] {
            assert!(
                value.fract() == 0.0,
                "{name}: non-integer coordinate {value}"
            );
        }
    }

    // No two shapes overlap.
    for (index, a) in plane.shapes.iter().enumerate() {
        for b in &plane.shapes[index + 1..] {
            assert!(
                !overlap(&a.bounds, &b.bounds),
                "{name}: shapes {} and {} overlap",
                a.bpmn_element,
                b.bpmn_element
            );
        }
    }

    // Edges: ordered axis-aligned waypoints docking on the flow endpoints.
    for edge in &plane.edges {
        assert!(
            edge.waypoints.len() >= 2,
            "{name}: edge {} has fewer than two waypoints",
            edge.bpmn_element
        );
        for point in &edge.waypoints {
            assert!(point.is_finite(), "{name}: non-finite waypoint");
        }
        for pair in edge.waypoints.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(
                a.x == b.x || a.y == b.y,
                "{name}: diagonal segment on edge {}",
                edge.bpmn_element
            );
        }
        let flow = document
            .find_sequence_flow(&edge.bpmn_element)
            .unwrap_or_else(|| panic!("{name}: flow {} missing", edge.bpmn_element));
        let source = plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == flow.source_ref)
            .unwrap_or_else(|| panic!("{name}: source shape {} missing", flow.source_ref));
        let target = plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == flow.target_ref)
            .unwrap_or_else(|| panic!("{name}: target shape {} missing", flow.target_ref));
        let first = edge.waypoints[0];
        let last = edge.waypoints[edge.waypoints.len() - 1];
        assert!(
            on_boundary(&source.bounds, first.x, first.y),
            "{name}: edge {} does not dock on source {}",
            edge.bpmn_element,
            flow.source_ref
        );
        assert!(
            on_boundary(&target.bounds, last.x, last.y),
            "{name}: edge {} does not dock on target {}",
            edge.bpmn_element,
            flow.target_ref
        );
    }
}

fn overlap(a: &Bounds, b: &Bounds) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn on_boundary(bounds: &Bounds, x: f64, y: f64) -> bool {
    let in_x = x >= bounds.x && x <= bounds.x + bounds.width;
    let in_y = y >= bounds.y && y <= bounds.y + bounds.height;
    (in_x && (y == bounds.y || y == bounds.y + bounds.height))
        || (in_y && (x == bounds.x || x == bounds.x + bounds.width))
}

#[test]
fn pmo_no_di_corpus_layouts_are_total_deterministic_and_invariant_clean() {
    let Some(dir) = corpus_dir() else {
        println!("corpus not downloaded; skipping (per-PR CI stays synthetic-only)");
        return;
    };
    let files = corpus_files(&dir, "bpmn_process");
    assert_eq!(
        files.len(),
        EXPECTED_MODELS,
        "PMo no-DI corpus size drifted from the pinned manifest"
    );

    let mut laid_out = 0usize;
    let mut failed: Vec<(String, String)> = Vec::new();
    for (name, path) in &files {
        let document = load(path, name);
        assert!(
            document.definitions.diagrams.is_empty(),
            "{name}: no-DI variant unexpectedly carries diagrams"
        );
        let scope = ScopeId::process(first_process_id(&document));
        let outcome = match NativeBpmnLayout.layout(&document, &scope, &LayoutOptions::default()) {
            Ok(outcome) => outcome,
            Err(error) => {
                failed.push((name.clone(), error.code().to_string()));
                continue;
            }
        };
        assert_invariants(name, &document, &outcome);

        // Determinism: a second run must produce byte-identical canonical geometry.
        let second = NativeBpmnLayout
            .layout(&document, &scope, &LayoutOptions::default())
            .unwrap_or_else(|error| panic!("{name}: non-deterministic success/failure: {error}"));
        assert_eq!(
            common::canonical(&outcome),
            common::canonical(&second),
            "{name}: repeated layout differs"
        );
        laid_out += 1;
    }

    println!(
        "pmo bpmn_process: {laid_out}/{} laid out; typed failures: {:?}",
        files.len(),
        failed
    );
    assert!(
        laid_out >= 1,
        "no PMo model laid out; corpus unusable for layout smoke"
    );
}

#[test]
fn pmo_with_di_corpus_renders_through_untouched_preserve_path() {
    let Some(dir) = corpus_dir() else {
        println!("corpus not downloaded; skipping (per-PR CI stays synthetic-only)");
        return;
    };
    let files = corpus_files(&dir, "bpmn");
    assert_eq!(
        files.len(),
        EXPECTED_MODELS,
        "PMo with-DI corpus size drifted from the pinned manifest"
    );

    let mut rendered = 0usize;
    let mut failed: Vec<(String, String)> = Vec::new();
    for (name, path) in &files {
        let document = load(path, name);
        match render_first_diagram(&document, &RenderOptions::default()) {
            Ok(rendered_diagram) => {
                let svg = bpmn_svg::to_svg(&rendered_diagram.scene);
                assert!(
                    svg.contains("<svg") && svg.contains("</svg>"),
                    "{name}: Preserve render produced invalid SVG"
                );
                rendered += 1;
            }
            Err(error @ RenderError::MissingGeometry { .. })
            | Err(error @ RenderError::EmptyDiagram) => {
                failed.push((name.clone(), error.to_string()));
            }
        }
    }

    println!(
        "pmo bpmn: {rendered}/{} rendered via Preserve; typed failures: {}",
        files.len(),
        failed.len()
    );
    assert!(
        rendered >= 1,
        "no PMo with-DI model rendered; corpus unusable for Preserve smoke"
    );
}
