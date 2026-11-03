//! Integration tests: render pipeline outputs (AC-04, AC-05, AC-07, AC-08).
//!
//! The committed golden SVG (`tests/fixtures/minimal/start-task-end.svg`)
//! pins the deterministic serializer output; PNG assertions check dimensions
//! and non-empty pixel content (not pixel equality; ADR-0004).

use std::path::{Path, PathBuf};

use bpmn_raster::{PngOptions, png_dimensions, render_pixmap};
use bpmn_render::{RenderOptions, render_diagram};
use bpmn_svg::to_svg;
use bpmn_xml::from_xml;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn render_svg(name: &str) -> String {
    let bytes = std::fs::read(fixture(name)).expect("fixture");
    let document = from_xml(&bytes).expect("parses");
    let diagram = document.first_diagram().expect("diagram");
    let rendered = render_diagram(&document, diagram, &RenderOptions::default()).expect("renders");
    to_svg(&rendered.scene)
}

#[test]
fn ac04_minimal_symbols_present() {
    let svg = render_svg("minimal/start-task-end.bpmn");
    // Events: thin start circle, thick end circle.
    assert!(svg.contains("id=\"bpmn-shape-StartEvent_1\""));
    assert!(svg.contains("stroke-width=\"3.5\""));
    // Task rounded rect, sequence flow arrows, labels.
    assert!(svg.contains("id=\"bpmn-shape-Task_1\"") && svg.contains("rx=\"8\""));
    assert!(svg.contains("marker-end=\"url(#bpmn-arrow-end)\""));
    assert!(svg.contains(">Approve</text>"));
}

#[test]
fn ac04_gateway_variants() {
    let svg = render_svg("edge-cases/multi-diagram.bpmn");
    assert!(svg.contains("id=\"bpmn-shape-Gateway_P\""));
    // Parallel gateway plus marker.
    assert!(svg.contains("id=\"bpmn-shape-Gateway_P-marker\""));
}

#[test]
fn golden_svg_is_stable() {
    let golden = std::fs::read_to_string(fixture("minimal/start-task-end.svg")).expect("golden");
    assert_eq!(
        render_svg("minimal/start-task-end.bpmn"),
        golden,
        "renderer output diverged from committed golden file"
    );
}

#[test]
fn ac07_png_dimensions_and_content() {
    let bytes = std::fs::read(fixture("minimal/start-task-end.bpmn")).unwrap();
    let document = from_xml(&bytes).unwrap();
    let diagram = document.first_diagram().unwrap();
    let rendered = render_diagram(&document, diagram, &RenderOptions::default()).unwrap();
    let svg = to_svg(&rendered.scene);

    for scale in [1.0, 2.0] {
        let png = bpmn_raster::render_png(
            &svg,
            &PngOptions {
                scale,
                ..PngOptions::default()
            },
        )
        .expect("png");
        let expected = (
            (rendered.scene.viewport.width * scale).round() as u32,
            (rendered.scene.viewport.height * scale).round() as u32,
        );
        assert_eq!(png_dimensions(&png), Some(expected));
        let pixmap = render_pixmap(
            &svg,
            &PngOptions {
                scale,
                ..PngOptions::default()
            },
        )
        .unwrap();
        let white = resvg_reexport_white();
        let non_background = pixmap
            .pixels()
            .iter()
            .filter(|pixel| **pixel != white)
            .count();
        assert!(
            non_background > 200,
            "scale {scale}: expected drawn content, got {non_background} px"
        );
    }
}

fn resvg_reexport_white() -> resvg::tiny_skia::PremultipliedColorU8 {
    resvg::tiny_skia::PremultipliedColorU8::from_rgba(255, 255, 255, 255).unwrap()
}

#[test]
fn ac08_unicode_fixture_renders_with_escapes() {
    let svg = render_svg("edge-cases/unicode.bpmn");
    assert!(svg.contains("&amp;") && svg.contains("Prüfung") && svg.contains("&lt;Test&gt;"));
    assert!(svg.contains("完了 🎉"));
    assert!(
        svg.contains("Café ☕ — «déjà"),
        "unicode event label wrapped"
    );
    assert!(svg.contains("vu»"));
}
