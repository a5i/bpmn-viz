//! Complex stress fixtures (TEST-005 §Fixtures, `complex/` class): warning
//! contracts and behavior-level assertions on realistic topologies. Generic
//! geometry invariants, determinism and consumability run for this class too;
//! frozen coordinate snapshots and reference parity intentionally stay on the
//! small core set (TEST-005 §Fixtures).

mod common;

/// Every complex fixture must lay out with exactly its documented warnings:
/// no silent skips. Only the subprocess cascade warns, because M1 collapses
/// subprocess bodies (M2 recurses into them).
#[test]
fn warning_contract_is_per_fixture() {
    for (stem, process_id) in common::COMPLEX_FIXTURES.iter().copied() {
        let outcome = common::layout_accepted(stem, process_id);
        let mut actual: Vec<(Option<String>, &str)> = outcome
            .warnings
            .iter()
            .map(|warning| (warning.element_id.clone(), warning.code.as_str()))
            .collect();
        actual.sort();
        let expected: Vec<(Option<String>, &str)> = if stem == "c20-subprocess-cascade" {
            ["c20_build", "c20_deploy", "c20_discovery", "c20_test"]
                .iter()
                .map(|id| (Some(id.to_string()), "layout.subprocess_body_skipped"))
                .collect()
        } else {
            Vec::new()
        };
        assert_eq!(actual, expected, "{stem}: unexpected layout warnings");
    }
}

/// The accepted double-diamond topology (with the secondary path to an end)
/// lays out with full DI coverage; its loop-only sibling is pinned separately
/// in `failures/routing-feedback-leg-blocked.bpmn`.
#[test]
fn double_diamond_with_secondary_end_path_lays_out() {
    let outcome = common::layout_accepted("c13-double-diamond", "process_c13");
    assert_eq!(outcome.diagram.plane.shapes.len(), 17);
    assert_eq!(outcome.diagram.plane.edges.len(), 22);
    assert!(outcome.warnings.is_empty());
}

/// The eight intake pre-processing tasks of the wide fan-in share one rank
/// and must land on visibly separated rows.
#[test]
fn wide_fan_in_channels_land_on_distinct_rows() {
    let outcome = common::layout_accepted("c18-wide-fan-in", "process_c18");
    let center_y = |id: &str| {
        let bounds = outcome
            .diagram
            .plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == id)
            .unwrap_or_else(|| panic!("shape {id} missing"))
            .bounds;
        bounds.y + bounds.height / 2.0
    };
    let mut rows: Vec<f64> = [
        "c18_t_web",
        "c18_t_email",
        "c18_t_phone",
        "c18_t_chat",
        "c18_t_social",
        "c18_t_desk",
        "c18_t_partner",
        "c18_t_api",
    ]
    .iter()
    .map(|id| center_y(id))
    .collect();
    rows.sort_by(|a, b| a.partial_cmp(b).expect("finite coordinates"));
    for pair in rows.windows(2) {
        assert!(
            pair[1] - pair[0] >= 80.0,
            "channel rows are not separated: {} .. {}",
            pair[0],
            pair[1]
        );
    }
}

/// The ten-step pipeline keeps its spine left-to-right monotonic.
#[test]
fn long_pipeline_keeps_spine_order_monotonic() {
    let outcome = common::layout_accepted("c14-long-pipeline", "process_c14");
    let center_x = |id: &str| {
        let bounds = outcome
            .diagram
            .plane
            .shapes
            .iter()
            .find(|shape| shape.bpmn_element == id)
            .unwrap_or_else(|| panic!("shape {id} missing"))
            .bounds;
        bounds.x + bounds.width / 2.0
    };
    let spine = [
        "c14_receive",
        "c14_verify",
        "c14_inventory",
        "c14_reserve",
        "c14_picking",
        "c14_pick",
        "c14_pack",
        "c14_shipment",
        "c14_carrier",
    ];
    for pair in spine.windows(2) {
        assert!(
            center_x(pair[0]) < center_x(pair[1]),
            "spine order violated between {} and {}",
            pair[0],
            pair[1]
        );
    }
}
