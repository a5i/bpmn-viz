//! Determinism tests (TEST-005 §Determinism): identical input and options
//! produce identical canonical geometry and ordered diagnostics; layout never
//! mutates the semantic model.

use bpmn_layout::{LayoutEngine, LayoutOptions, NativeBpmnLayout, ScopeId};

mod common;

#[test]
fn repeated_layouts_are_byte_identical() {
    for (stem, process_id) in common::accepted_fixtures() {
        let document = common::load_accepted(stem);
        let mut canonicals = Vec::new();
        for _ in 0..3 {
            let outcome = NativeBpmnLayout
                .layout(
                    &document,
                    &ScopeId::process(process_id),
                    &LayoutOptions::default(),
                )
                .expect("layout succeeds");
            canonicals
                .push(serde_json::to_string(&common::canonical(&outcome)).expect("serialize"));
        }
        let first = &canonicals[0];
        for (run, canonical) in canonicals.iter().enumerate() {
            assert_eq!(first, canonical, "{stem}: run {run} diverged from run 0");
        }
    }
}

#[test]
fn layout_does_not_mutate_the_semantic_model() {
    for (stem, process_id) in common::accepted_fixtures() {
        let document = common::load_accepted(stem);
        let before = format!("{document:?}");
        let _ = NativeBpmnLayout
            .layout(
                &document,
                &ScopeId::process(process_id),
                &LayoutOptions::default(),
            )
            .expect("layout succeeds");
        let after = format!("{document:?}");
        assert_eq!(before, after, "{stem}: semantic model changed");
    }
}

#[test]
fn layout_output_is_independent_of_engine_instance() {
    // NativeBpmnLayout is a zero-state struct; distinct instances must agree.
    let document = common::load_fixture("core/xor-split-join.bpmn");
    let a = bpmn_layout::NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("process_xor"),
            &LayoutOptions::default(),
        )
        .expect("layout");
    let b = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("process_xor"),
            &LayoutOptions::default(),
        )
        .expect("layout");
    assert_eq!(
        serde_json::to_string(&common::canonical(&a)).unwrap(),
        serde_json::to_string(&common::canonical(&b)).unwrap()
    );
}
