//! Layout performance corpus (TEST-005 §Exit Criteria, EXP-004 §Results).
//!
//! Synthetic chain models (start → N tasks → end) at 10 / 100 / 1,000 flow
//! nodes, measured with a fixed iteration protocol on the host machine.
//! Numbers are recorded in `docs/experiments/EXP-004-reference-layout.md` as
//! a baseline; no user-facing threshold is derived from them.
//!
//! Run: `cargo bench -p bpmn-layout` (release profile, `harness = false`).

use std::time::Instant;

use bpmn_layout::{LayoutEngine, LayoutOptions, NativeBpmnLayout, ScopeId};
use bpmn_xml::from_xml;

/// Builds a chain BPMN document with `tasks` tasks between start and end.
fn chain_xml(tasks: usize) -> String {
    let mut xml = String::from(
        r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"><process id="bench">"#,
    );
    xml.push_str(r#"<startEvent id="n_start"/>"#);
    for index in 0..tasks {
        xml.push_str(&format!(r#"<task id="n_t{index}" name="Step {index}"/>"#));
    }
    xml.push_str(r#"<endEvent id="n_end"/>"#);
    let mut previous = "n_start".to_string();
    for index in 0..tasks {
        xml.push_str(&format!(
            r#"<sequenceFlow id="f{index}" sourceRef="{previous}" targetRef="n_t{index}"/>"#,
        ));
        previous = format!("n_t{index}");
    }
    xml.push_str(&format!(
        r#"<sequenceFlow id="f_end" sourceRef="{previous}" targetRef="n_end"/>"#,
    ));
    xml.push_str("</process></definitions>");
    xml
}

fn measure(nodes: usize, tasks: usize, iterations: usize) {
    let xml = chain_xml(tasks);
    let document = from_xml(xml.as_bytes()).expect("bench model parses");
    // Warmup (page caches, allocator).
    for _ in 0..5 {
        let _ = NativeBpmnLayout
            .layout(
                &document,
                &ScopeId::process("bench"),
                &LayoutOptions::default(),
            )
            .expect("layout succeeds");
    }
    let mut samples: Vec<f64> = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let outcome = NativeBpmnLayout
            .layout(
                &document,
                &ScopeId::process("bench"),
                &LayoutOptions::default(),
            )
            .expect("layout succeeds");
        let elapsed = start.elapsed().as_secs_f64() * 1_000.0;
        assert_eq!(outcome.diagram.plane.shapes.len(), nodes);
        samples.push(elapsed);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).expect("finite durations"));
    let median = samples[samples.len() / 2];
    let p95 = samples[(samples.len() as f64 * 0.95) as usize];
    let worst = samples[samples.len() - 1];
    println!(
        "nodes={nodes:>5} median={median:.3}ms p95={p95:.3}ms worst={worst:.3}ms iterations={iterations}"
    );
}

fn main() {
    println!("bpmn-layout corpus (release profile, this machine; EXP-004 baseline)");
    measure(12, 10, 2_000);
    measure(102, 100, 500);
    measure(1_002, 1_000, 100);
}
