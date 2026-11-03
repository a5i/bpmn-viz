//! Error-path tests (TEST-005 §Errors): stable codes for the failure
//! fixtures, resource limits, and a no-panic property test on generated
//! malformed models.

use bpmn_layout::{LayoutEngine, LayoutLimits, LayoutOptions, NativeBpmnLayout, ScopeId};
use bpmn_model::element::{
    Definitions, FlowElement, FlowNode, NodeKind, Process, RootElement, SequenceFlow, TaskType,
};
use proptest::prelude::*;

mod common;

#[test]
fn invalid_endpoint_returns_structured_error() {
    let document = common::load_fixture("failures/invalid-endpoint.bpmn");
    let error = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("process_invalid"),
            &LayoutOptions::default(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "layout.missing_endpoint");
    assert_eq!(error.element_id(), Some("bad_f1"));
}

#[test]
fn cross_scope_flow_returns_structured_error() {
    let document = common::load_fixture("failures/cross-scope-flow.bpmn");
    let error = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("scope_cross_a"),
            &LayoutOptions::default(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "layout.cross_scope_flow");
    assert_eq!(error.element_id(), Some("cross_f1"));

    // The untouched second scope still lays out correctly.
    let outcome = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("scope_cross_b"),
            &LayoutOptions::default(),
        )
        .expect("other scope lays out");
    assert_eq!(outcome.diagram.plane.shapes.len(), 2);
}

#[test]
fn blocked_feedback_leg_returns_structured_routing_failure() {
    // Known M1 router limitation (TEST-005 §Fixtures, `failures/` #11): a
    // feedback edge whose vertical docking legs are blocked by an off-spine
    // sibling in the same column fails with a structured error naming the
    // flow instead of drawing through shapes. The M2 router must replace
    // this pin with a successful route.
    let document = common::load_fixture("failures/routing-feedback-leg-blocked.bpmn");
    let error = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("process_c13"),
            &LayoutOptions::default(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "layout.routing_failed");
    assert_eq!(error.element_id(), Some("c13_f18"));
}

#[test]
fn nested_subprocess_body_node_is_cross_scope() {
    let xml = r#"
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="p">
    <subProcess id="sub">
      <task id="inner"/>
    </subProcess>
    <startEvent id="s"/>
    <sequenceFlow id="f" sourceRef="s" targetRef="inner"/>
  </process>
</definitions>"#;
    let document = bpmn_xml::from_xml(xml.as_bytes()).expect("parse");
    let error = NativeBpmnLayout
        .layout(&document, &ScopeId::process("p"), &LayoutOptions::default())
        .unwrap_err();
    assert_eq!(error.code(), "layout.cross_scope_flow");
}

#[test]
fn duplicate_node_ids_are_structured_errors() {
    let xml = r#"
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="p_dup">
    <task id="dup"/>
    <task id="dup"/>
    <endEvent id="e"/>
    <sequenceFlow id="f" sourceRef="dup" targetRef="e"/>
  </process>
</definitions>"#;
    let document = bpmn_xml::from_xml(xml.as_bytes()).expect("parse");
    let error = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("p_dup"),
            &LayoutOptions::default(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "layout.duplicate_node");
    assert_eq!(error.element_id(), Some("dup"));
}

#[test]
fn flow_limits_are_enforced() {
    let document = common::load_fixture("core/parallel-split-join.bpmn");
    let options = LayoutOptions {
        resource_limits: LayoutLimits {
            max_nodes: 100,
            max_flows: 3,
        },
        ..LayoutOptions::default()
    };
    let error = NativeBpmnLayout
        .layout(&document, &ScopeId::process("process_parallel"), &options)
        .unwrap_err();
    assert_eq!(error.code(), "layout.limit_exceeded");
}

#[test]
fn empty_scope_is_structured() {
    // A scope with sequence flows but no flow nodes has nothing to place.
    // (A fully empty `<process/>` is dropped at parse time, so the layout
    // engine never sees it.)
    let xml = r#"
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="p_empty">
    <sequenceFlow id="f" sourceRef="a" targetRef="b"/>
  </process>
</definitions>"#;
    let document = bpmn_xml::from_xml(xml.as_bytes()).expect("parse");
    let error = NativeBpmnLayout
        .layout(
            &document,
            &ScopeId::process("p_empty"),
            &LayoutOptions::default(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "layout.empty_scope");
}

// --- property test -------------------------------------------------------

/// Deterministic xorshift PRNG: no external dependency, reproducible corpus.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }

    fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next() % den < num
    }
}

/// Builds a random process of `nodes` tasks and `flows` random sequence flows
/// (dangling refs, self loops and duplicate ids included on purpose). Layout
/// must always answer `Ok` or a structured `Err` — never panic.
fn build_random_process(nodes: usize, flows: usize, seed: u64) -> bpmn_model::BpmnDocument {
    let mut rng = XorShift(seed ^ 0x9E37_79B9_7F4A_7C15);
    let mut process = Process {
        id: Some("p_rand".to_string()),
        ..Process::default()
    };
    let node_ids: Vec<String> = (0..nodes)
        .map(|index| {
            if rng.chance(1, 10) && index > 0 {
                // Deliberate duplicate id.
                format!("n{}", rng.below(index))
            } else {
                format!("n{index}")
            }
        })
        .collect();
    for (index, id) in node_ids.iter().enumerate() {
        process.flow_elements.push(FlowElement::Node(FlowNode {
            id: id.clone(),
            name: None,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            default_flow_ref: None,
            kind: if index % 5 == 0 {
                NodeKind::Task {
                    task_type: TaskType::Abstract,
                }
            } else if index % 5 == 1 {
                NodeKind::StartEvent {
                    definition: bpmn_model::element::EventDefinition::None,
                }
            } else if index % 5 == 2 {
                NodeKind::EndEvent {
                    definition: bpmn_model::element::EventDefinition::None,
                }
            } else {
                NodeKind::Gateway {
                    gateway_type: bpmn_model::element::GatewayType::Exclusive,
                }
            },
        }));
    }
    for index in 0..flows {
        let source = rng.below(nodes);
        let target = rng.below(nodes);
        let source_id = node_ids
            .get(source)
            .cloned()
            .unwrap_or_else(|| "dangling".to_string());
        let target_id = if rng.chance(1, 10) {
            "dangling".to_string()
        } else {
            node_ids
                .get(target)
                .cloned()
                .unwrap_or_else(|| "dangling".to_string())
        };
        process.flow_elements.push(FlowElement::Flow(SequenceFlow {
            id: format!("f{index}"),
            name: None,
            source_ref: source_id,
            target_ref: target_id,
            is_default: false,
            condition: None,
        }));
    }
    let mut definitions = Definitions::default();
    definitions
        .root_elements
        .push(RootElement::Process(process));
    bpmn_model::BpmnDocument {
        definitions,
        parse_diagnostics: Vec::new(),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn no_panic_on_random_models(
        nodes in 1usize..24usize,
        flows in 0usize..40usize,
        seed in any::<u64>(),
    ) {
        let document = build_random_process(nodes, flows, seed);
        let result = NativeBpmnLayout.layout(
            &document,
            &ScopeId::process("p_rand"),
            &LayoutOptions::default(),
        );
        match result {
            Ok(outcome) => {
                // This generator only emits placeable kinds (tasks, events,
                // gateways), and duplicate ids would have failed the layout,
                // so success means every node got exactly one shape.
                prop_assert_eq!(outcome.diagram.plane.shapes.len(), nodes);
            }
            Err(error) => {
                let code = error.code();
                prop_assert!(
                    code.starts_with("layout."),
                    "unexpected error code {code}"
                );
            }
        }
    }
}
