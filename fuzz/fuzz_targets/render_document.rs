//! Fuzzes the full parse→render→serialize pipeline with arbitrary bytes.
//! Run: cargo +nightly fuzz run render_document -- -max_len=65536

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(document) = bpmn_xml::from_xml(data) else {
        return;
    };
    let Ok(diagram) = document.first_diagram().ok_or(()) else {
        return;
    };
    if let Ok(rendered) =
        bpmn_render::render_diagram(&document, diagram, &bpmn_render::RenderOptions::default())
    {
        let svg = bpmn_svg::to_svg(&rendered.scene);
        let _ = bpmn_raster::render_png(&svg, &bpmn_raster::PngOptions::default());
    }
});
