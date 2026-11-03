//! Dev-only preview helper for corpus galleries (TASK-019): renders one BPMN
//! file to SVG + PNG either through the native layout (mode `layout`, from the
//! semantic model only) or through the authored DI (mode `authored`, the
//! Preserve path, bypassing CLI validation gates). Not part of the library
//! API; not exercised by CI.
//!
//! Usage:
//! `cargo run -p bpmn-layout --example render_preview -- <layout|authored> <input.bpmn> <output-stem>`

use std::path::PathBuf;

use bpmn_layout::{LayoutEngine, LayoutOptions, NativeBpmnLayout, ScopeId};
use bpmn_raster::{PngOptions, render_png};
use bpmn_render::{RenderOptions, render_diagram, render_first_diagram};
use bpmn_svg::to_svg;
use bpmn_xml::from_xml;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let mode = args
        .next()
        .expect("usage: <layout|authored> <input.bpmn> <output-stem>")
        .to_string_lossy()
        .into_owned();
    let input = PathBuf::from(
        args.next()
            .expect("usage: <layout|authored> <input.bpmn> <output-stem>"),
    );
    let output_stem = PathBuf::from(
        args.next()
            .expect("usage: <layout|authored> <input.bpmn> <output-stem>"),
    );

    let bytes = std::fs::read(&input).unwrap_or_else(|error| panic!("read {input:?}: {error}"));
    let document = from_xml(&bytes).unwrap_or_else(|error| panic!("parse {input:?}: {error}"));

    let rendered = match mode.as_str() {
        "authored" => render_first_diagram(&document, &RenderOptions::default())
            .unwrap_or_else(|error| panic!("{input:?}: authored render failed: {error}")),
        "layout" => {
            let scope = ScopeId::process(
                document
                    .processes()
                    .find_map(|process| process.id.clone())
                    .unwrap_or_else(|| panic!("{input:?}: no process id")),
            );
            let outcome = NativeBpmnLayout
                .layout(&document, &scope, &LayoutOptions::default())
                .unwrap_or_else(|error| panic!("{input:?}: layout failed: {error}"));
            render_diagram(&document, &outcome.diagram, &RenderOptions::default())
                .unwrap_or_else(|error| panic!("{input:?}: render failed: {error}"))
        }
        other => panic!("unknown mode {other:?}; use layout|authored"),
    };

    let svg = to_svg(&rendered.scene);
    let svg_path = PathBuf::from(format!("{}.svg", output_stem.display()));
    std::fs::write(&svg_path, &svg).unwrap_or_else(|error| panic!("write {svg_path:?}: {error}"));

    let png = match render_png(
        &svg,
        &PngOptions {
            scale: 2.0,
            ..PngOptions::default()
        },
    ) {
        Ok(png) => png,
        // Large layouts can exceed the raster pixel budget at 2x; fall back to 1x.
        Err(_) => render_png(
            &svg,
            &PngOptions {
                scale: 1.0,
                ..PngOptions::default()
            },
        )
        .unwrap_or_else(|error| panic!("rasterize: {error}")),
    };
    let png_path = PathBuf::from(format!("{}.png", output_stem.display()));
    std::fs::write(&png_path, png).unwrap_or_else(|error| panic!("write {png_path:?}: {error}"));
    println!("rendered {input:?} -> {}", output_stem.display());
}
