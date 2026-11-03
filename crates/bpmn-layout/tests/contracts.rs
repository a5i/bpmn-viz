//! Contract tests (TEST-005 §Assertions): option validation, scope
//! resolution, M1 warning coverage, output association.

use bpmn_layout::{
    LayoutEngine, LayoutError, LayoutLimits, LayoutOptions, LayoutWarningCode, NativeBpmnLayout,
    ScopeId,
};
use bpmn_xml::from_xml;

mod common;

const SUBPROCESS_XML: &str = r#"
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="p_sub">
    <startEvent id="s1"/>
    <subProcess id="sub1" name="Nested">
      <startEvent id="s2"/>
      <task id="t2"/>
      <endEvent id="e2"/>
      <sequenceFlow id="bf1" sourceRef="s2" targetRef="t2"/>
      <sequenceFlow id="bf2" sourceRef="t2" targetRef="e2"/>
    </subProcess>
    <endEvent id="e1"/>
    <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1"/>
    <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1"/>
  </process>
</definitions>"#;

const BOUNDARY_XML: &str = r#"
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="p_boundary">
    <startEvent id="s1"/>
    <task id="host"/>
    <endEvent id="e1"/>
    <boundaryEvent id="err" attachedToRef="host"/>
    <task id="handler"/>
    <sequenceFlow id="f1" sourceRef="s1" targetRef="host"/>
    <sequenceFlow id="f2" sourceRef="host" targetRef="e1"/>
    <sequenceFlow id="f3" sourceRef="err" targetRef="handler"/>
  </process>
</definitions>"#;

const UNSUPPORTED_XML: &str = r#"
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="p_unsupported">
    <startEvent id="s1"/>
    <complexGateway id="cx"/>
    <adHocSubProcess id="adhoc"/>
    <endEvent id="e1"/>
    <sequenceFlow id="f1" sourceRef="s1" targetRef="e1"/>
  </process>
</definitions>"#;

const LANES_XML: &str = r#"
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="p_lanes">
    <laneSet id="ls">
      <lane id="lane_1"><flowNodeRef>s1</flowNodeRef></lane>
    </laneSet>
    <startEvent id="s1"/>
    <endEvent id="e1"/>
    <sequenceFlow id="f1" sourceRef="s1" targetRef="e1"/>
  </process>
</definitions>"#;

fn layout(
    xml: &str,
    scope: &str,
    options: &LayoutOptions,
) -> Result<bpmn_layout::LayoutOutcome, LayoutError> {
    let document = from_xml(xml.as_bytes()).expect("parse");
    NativeBpmnLayout.layout(&document, &ScopeId::process(scope), options)
}

#[test]
fn unknown_scope_returns_stable_code() {
    let document = common::load_fixture("core/start-task-end.bpmn");
    let error = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("missing_process"),
            &LayoutOptions::default(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "layout.unknown_scope");
    assert_eq!(error.element_id(), Some("missing_process"));
}

#[test]
fn unsupported_options_are_rejected() {
    let non_deterministic = LayoutOptions {
        deterministic: false,
        ..LayoutOptions::default()
    };
    let error = layout(SUBPROCESS_XML, "p_sub", &non_deterministic).unwrap_err();
    assert_eq!(error.code(), "layout.unsupported_option");

    // Orientation: M1 has exactly one supported variant; the option is
    // validated for forward compatibility (a future RightToLeft variant must
    // keep failing loudly until implemented).
}
#[test]
fn invalid_gap_values_are_rejected() {
    for value in [f64::NAN, f64::INFINITY, -1.0] {
        let options = LayoutOptions {
            rank_gap: value,
            ..LayoutOptions::default()
        };
        assert_eq!(
            layout(SUBPROCESS_XML, "p_sub", &options)
                .unwrap_err()
                .code(),
            "layout.unsupported_option",
            "rank_gap = {value}"
        );
    }
}

#[test]
fn subprocess_body_is_skipped_with_warning_but_flows_work() {
    let outcome = layout(SUBPROCESS_XML, "p_sub", &LayoutOptions::default()).expect("layout");
    assert_eq!(outcome.diagram.plane.shapes.len(), 3); // s1, sub1, e1
    assert_eq!(outcome.diagram.plane.edges.len(), 2);
    assert!(outcome.warnings.iter().any(|warning| warning.code
        == LayoutWarningCode::SubprocessBodySkipped
        && warning.element_id.as_deref() == Some("sub1")));
    // Plane is bound to the selected process.
    assert_eq!(outcome.diagram.plane.bpmn_element.as_deref(), Some("p_sub"));
}

#[test]
fn boundary_event_is_skipped_with_warning_and_exception_flow_not_routed() {
    let outcome = layout(BOUNDARY_XML, "p_boundary", &LayoutOptions::default()).expect("layout");
    // Placed: s1, host, e1, handler (the boundary event itself is skipped).
    assert_eq!(outcome.diagram.plane.shapes.len(), 4);
    // Routed: f1, f2 (f3 touches the skipped boundary event).
    assert_eq!(outcome.diagram.plane.edges.len(), 2);
    assert!(
        outcome
            .warnings
            .iter()
            .any(|warning| warning.code == LayoutWarningCode::BoundaryEventSkipped)
    );
    assert!(outcome.warnings.iter().any(|warning| warning.code
        == LayoutWarningCode::UnsupportedFlowSkipped
        && warning.element_id.as_deref() == Some("f3")));
    assert!(
        outcome
            .diagram
            .plane
            .edges
            .iter()
            .all(|edge| edge.bpmn_element != "f3")
    );
}

#[test]
fn unsupported_nodes_are_skipped_with_warnings() {
    let document = from_xml(UNSUPPORTED_XML.as_bytes()).expect("parse");
    let outcome = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("p_unsupported"),
            &LayoutOptions::default(),
        )
        .expect("layout");
    let placed: Vec<&str> = outcome
        .diagram
        .plane
        .shapes
        .iter()
        .map(|shape| shape.bpmn_element.as_str())
        .collect();
    assert_eq!(placed, ["s1", "e1"]);
    // Complex gateway is a modeled flow node outside the M1 matrix.
    assert!(outcome.warnings.iter().any(|warning| warning.code
        == LayoutWarningCode::UnsupportedNodeSkipped
        && warning.element_id.as_deref() == Some("cx")));
    // adHocSubProcess is not a modeled flow element at all: the parser ignores
    // it (M3 policy), so layout neither places nor warns about it.
    assert!(document.find_node("adhoc").is_none());
    assert!(
        outcome
            .diagram
            .plane
            .shapes
            .iter()
            .all(|shape| shape.bpmn_element != "adhoc")
    );
}

#[test]
fn lane_membership_is_reported_ignored() {
    let outcome = layout(LANES_XML, "p_lanes", &LayoutOptions::default()).expect("layout");
    assert!(outcome.warnings.iter().any(|warning| warning.code
        == LayoutWarningCode::LaneMembershipIgnored
        && warning.element_id.as_deref() == Some("ls")));
}

#[test]
fn limit_exceeded_is_structured() {
    let options = LayoutOptions {
        resource_limits: LayoutLimits {
            max_nodes: 2,
            max_flows: 10,
        },
        ..LayoutOptions::default()
    };
    let error = layout(SUBPROCESS_XML, "p_sub", &options).unwrap_err();
    assert_eq!(error.code(), "layout.limit_exceeded");
    assert_eq!(error.element_id(), None);
}

#[test]
fn self_loop_is_supported() {
    let xml = r#"
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="p_self">
    <task id="only"/>
    <sequenceFlow id="f_self" sourceRef="only" targetRef="only"/>
  </process>
</definitions>"#;
    let outcome = layout(xml, "p_self", &LayoutOptions::default()).expect("layout");
    assert_eq!(outcome.diagram.plane.shapes.len(), 1);
    assert_eq!(outcome.diagram.plane.edges.len(), 1);
    assert!(outcome.warnings.is_empty());
}

#[test]
fn warnings_are_empty_for_clean_scopes() {
    for (stem, process_id) in common::CORE_FIXTURES {
        let outcome = common::layout_core(stem, process_id);
        assert!(
            outcome.warnings.is_empty(),
            "{stem} unexpectedly warned: {:?}",
            outcome.warnings
        );
    }
}
