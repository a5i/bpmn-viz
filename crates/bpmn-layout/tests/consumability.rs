//! Renderer consumability (TEST-005 §Assertions, ADR-0007 §Validation):
//! generated DI must render to deterministic SVG and a nonempty PNG through
//! the unchanged `bpmn-render`/`bpmn-svg`/`bpmn-raster` pipeline, and the
//! Preserve contract must stay intact (a no-DI document still fails loudly
//! even though the layout crate is linked in).

use bpmn_layout::{LayoutEngine, LayoutOptions, NativeBpmnLayout, ScopeId};
use bpmn_raster::{PngOptions, render_png};
use bpmn_render::{RenderOptions, render_diagram, render_first_diagram};
use bpmn_svg::to_svg;

mod common;

fn png_options() -> PngOptions {
    PngOptions {
        scale: 2.0,
        ..PngOptions::default()
    }
}

#[test]
fn generated_di_renders_to_deterministic_svg_and_png() {
    for (stem, process_id) in common::accepted_fixtures() {
        let document = common::load_accepted(stem);
        let outcome = common::layout_accepted(stem, process_id);

        let rendered = render_diagram(&document, &outcome.diagram, &RenderOptions::default())
            .unwrap_or_else(|error| panic!("{stem}: render failed: {error}"));

        let svg_first = to_svg(&rendered.scene);
        let svg_second = to_svg(&rendered.scene);
        assert!(
            svg_first.contains("<svg") && svg_first.contains("</svg>"),
            "{stem}: SVG output is not a valid document"
        );
        assert_eq!(
            svg_first, svg_second,
            "{stem}: SVG serialization is not deterministic"
        );

        let png = render_png(&svg_first, &png_options())
            .unwrap_or_else(|error| panic!("{stem}: rasterization failed: {error}"));
        assert!(
            png.len() > 100 && png.starts_with(&[0x89, b'P', b'N', b'G']),
            "{stem}: PNG output missing or not a PNG"
        );
    }
}

#[test]
fn every_accepted_node_has_rendered_geometry() {
    for (stem, process_id) in common::accepted_fixtures() {
        let document = common::load_accepted(stem);
        let outcome = common::layout_accepted(stem, process_id);
        let rendered = render_diagram(&document, &outcome.diagram, &RenderOptions::default())
            .unwrap_or_else(|error| panic!("{stem}: render failed: {error}"));
        let svg = to_svg(&rendered.scene);
        for shape in &outcome.diagram.plane.shapes {
            assert!(
                svg.contains(&shape.bpmn_element),
                "{stem}: node {} missing from rendered SVG",
                shape.bpmn_element
            );
        }
    }
}

#[test]
fn preserve_mode_still_fails_loudly_without_layout() {
    // A no-DI document rendered through the untouched renderer must still
    // fail (ADR-0003); the presence of bpmn-layout in the workspace changes
    // nothing about the render path.
    let document = common::load_fixture("core/start-task-end.bpmn");
    assert!(document.definitions.diagrams.is_empty());
    let error = render_first_diagram(&document, &RenderOptions::default()).unwrap_err();
    assert!(
        matches!(
            error,
            bpmn_render::RenderError::EmptyDiagram
                | bpmn_render::RenderError::MissingGeometry { .. }
        ),
        "unexpected render error: {error}"
    );
}

#[test]
fn layout_call_leaves_authored_di_untouched() {
    // Generate mode: authored DI (even if present) is ignored for placement
    // and the document is not modified (ADR-0007).
    let document = common::load_fixture("core/start-task-end.bpmn");
    let before = format!("{document:?}");
    let _ = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("process_order"),
            &LayoutOptions::default(),
        )
        .expect("layout");
    assert_eq!(
        before,
        format!("{document:?}"),
        "document mutated by layout"
    );
    assert!(document.definitions.diagrams.is_empty());
}
