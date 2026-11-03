// Performance baseline harness (REPORT-001). Not a criterion bench: prints
// measured timings for synthetic documents of 100 / 1,000 / 10,000 elements.
//
// Run: cargo bench -p bpmn-cli --bench render

use std::fmt::Write as _;
use std::time::Instant;

use bpmn_model::BpmnDocument;
use bpmn_render::{RenderOptions, render_diagram};
use bpmn_svg::to_svg;
use bpmn_xml::from_xml;

/// Generates a synthetic linear process with `nodes` tasks and DI.
fn synthetic(nodes: usize) -> Vec<u8> {
    let mut semantic = String::new();
    let mut diagram = String::new();
    let per_row = 20;
    let width = 100;
    let height = 80;
    for index in 0..nodes {
        let column = index % per_row;
        let row = index / per_row;
        let x = 40 + column * (width + 40);
        let y = 40 + row * (height + 60);
        let _ = write!(
            semantic,
            r#"<task id="T{index}" name="Task number {index}"/><sequenceFlow id="F{index}" sourceRef="{}" targetRef="T{index}"/>"#,
            if index == 0 {
                "Start".to_string()
            } else {
                format!("T{}", index - 1)
            }
        );
        let _ = write!(
            diagram,
            r#"<bpmndi:BPMNShape id="T{index}_di" bpmnElement="T{index}"><dc:Bounds x="{x}" y="{y}" width="{width}" height="{height}"/></bpmndi:BPMNShape>"#
        );
        if index > 0 {
            let px = x - 40;
            let py = y + height / 2;
            let _ = write!(
                diagram,
                r#"<bpmndi:BPMNEdge id="F{index}_di" bpmnElement="F{index}"><di:waypoint x="{px}" y="{py}"/><di:waypoint x="{x}" y="{py}"/></bpmndi:BPMNEdge>"#
            );
        }
    }
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
             xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
             xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
             id="Defs_Bench" targetNamespace="http://example.test/bench">
  <process id="P" isExecutable="false">
    <startEvent id="Start"><outgoing>F0</outgoing></startEvent>
    <endEvent id="End"><incoming>FTail</incoming></endEvent>
    <sequenceFlow id="FTail" sourceRef="T{last}" targetRef="End"/>
    {semantic}
  </process>
  <bpmndi:BPMNDiagram id="D"><bpmndi:BPMNPlane id="Pl" bpmnElement="P">
    <bpmndi:BPMNShape id="Start_di" bpmnElement="Start"><dc:Bounds x="0" y="40" width="36" height="36"/></bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="End_di" bpmnElement="End"><dc:Bounds x="{end_x}" y="40" width="36" height="36"/></bpmndi:BPMNShape>
    <bpmndi:BPMNEdge id="FTail_di" bpmnElement="FTail"><di:waypoint x="{tail_x}" y="80"/><di:waypoint x="{end_x}" y="58"/></bpmndi:BPMNEdge>
    {diagram}
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#,
        last = nodes - 1,
        end_x = 40 + nodes * (width + 40),
        tail_x = 40 + (nodes - 1) * (width + 40) + width,
    );
    document.into_bytes()
}

fn measure(name: &str, bytes: &[u8], raster: bool) {
    let parse_start = Instant::now();
    let document: BpmnDocument = from_xml(bytes).expect("synthetic document parses");
    let parse = parse_start.elapsed();

    let render_start = Instant::now();
    let diagram = document.first_diagram().expect("diagram");
    let rendered = render_diagram(&document, diagram, &RenderOptions::default()).expect("renders");
    let render = render_start.elapsed();

    let svg_start = Instant::now();
    let svg = to_svg(&rendered.scene);
    let serialize = svg_start.elapsed();

    let mut raster_line = String::from("-");
    if raster {
        let png_start = Instant::now();
        let png = bpmn_raster::render_png(
            &svg,
            &bpmn_raster::PngOptions {
                scale: 1.0,
                ..bpmn_raster::PngOptions::default()
            },
        );
        match png {
            Ok(bytes) => raster_line = format!("{:?} ({} bytes)", png_start.elapsed(), bytes.len()),
            Err(error) => raster_line = format!("skipped: {error}"),
        }
    }

    println!(
        "{name}: elements={} parse={:?} render={:?} svg={:?} ({} bytes) png={raster_line}",
        document
            .processes()
            .map(|process| process.walk_nodes().len())
            .sum::<usize>(),
        parse,
        render,
        serialize,
        svg.len(),
    );
}

fn main() {
    println!("BPMN renderer performance baseline (bench profile; REPORT-001)");
    measure("S 100", &synthetic(100), true);
    measure("S 1_000", &synthetic(1_000), true);
    measure("S 10_000", &synthetic(10_000), false);
}
