//! Semantic BPMN 2.0 elements (no geometry; see [`crate::di`]).
//!
//! The subset follows SPEC-001 section 3 P0: events, tasks and common task
//! kinds, exclusive/parallel/inclusive gateways, sequence flows, pools,
//! lanes, subprocesses, and text annotations. Unknown vendor extensions are
//! tolerated during import and never modeled here.

/// Document-level `bpmn:definitions` contents.
#[derive(Debug, Clone, Default)]
pub struct Definitions {
    /// `definitions/@id`.
    pub id: Option<String>,
    /// `definitions/@targetNamespace`.
    pub target_namespace: Option<String>,
    /// Root elements in source order (processes and collaborations).
    pub root_elements: Vec<RootElement>,
    /// Diagrams in source order; selection index follows this order.
    pub diagrams: Vec<crate::di::Diagram>,
}

/// A root element under `definitions`.
#[derive(Debug, Clone)]
pub enum RootElement {
    /// A `bpmn:process`.
    Process(Process),
    /// A `bpmn:collaboration`.
    Collaboration(Collaboration),
}

/// A `bpmn:process` with its ordered flow elements.
#[derive(Debug, Clone, Default)]
pub struct Process {
    /// Process id.
    pub id: Option<String>,
    /// Process name.
    pub name: Option<String>,
    /// `isExecutable` attribute.
    pub is_executable: Option<bool>,
    /// Flow elements (nodes and sequence flows) in source order.
    pub flow_elements: Vec<FlowElement>,
    /// Lane sets in source order.
    pub lane_sets: Vec<LaneSet>,
    /// Artifacts such as text annotations.
    pub artifacts: Vec<Artifact>,
}

impl Process {
    /// Recursively find a flow node by id, including subprocess bodies.
    pub fn find_node(&self, id: &str) -> Option<&FlowNode> {
        self.flow_elements.iter().find_map(|element| match element {
            FlowElement::Node(node) if node.id == id => Some(node),
            FlowElement::Node(FlowNode {
                kind: NodeKind::SubProcess { body, .. },
                ..
            }) => find_node_in_body(body, id),
            _ => None,
        })
    }

    /// Recursively find a sequence flow by id, including subprocess bodies.
    pub fn find_sequence_flow(&self, id: &str) -> Option<&SequenceFlow> {
        self.flow_elements.iter().find_map(|element| match element {
            FlowElement::Flow(flow) if flow.id == id => Some(flow),
            FlowElement::Node(FlowNode {
                kind: NodeKind::SubProcess { body, .. },
                ..
            }) => find_flow_in_body(body, id),
            _ => None,
        })
    }

    /// Find a lane by id in any lane set.
    pub fn find_lane(&self, id: &str) -> Option<&Lane> {
        self.lane_sets
            .iter()
            .flat_map(|set| set.lanes.iter())
            .find(|lane| lane.id == id)
    }

    /// Find an artifact by id.
    pub fn find_artifact(&self, id: &str) -> Option<&Artifact> {
        self.artifacts.iter().find(|artifact| artifact.id() == id)
    }

    /// Iterate all flow nodes recursively (depth first, source order).
    pub fn walk_nodes(&self) -> Vec<&FlowNode> {
        let mut nodes = Vec::new();
        collect_nodes(&self.flow_elements, &mut nodes);
        nodes
    }

    /// Iterate all sequence flows recursively.
    pub fn walk_flows(&self) -> Vec<&SequenceFlow> {
        let mut flows = Vec::new();
        collect_flows(&self.flow_elements, &mut flows);
        flows
    }
}

fn find_node_in_body<'a>(body: &'a [FlowElement], id: &str) -> Option<&'a FlowNode> {
    body.iter().find_map(|element| match element {
        FlowElement::Node(node) if node.id == id => Some(node),
        FlowElement::Node(FlowNode {
            kind: NodeKind::SubProcess { body: inner, .. },
            ..
        }) => find_node_in_body(inner, id),
        _ => None,
    })
}

fn find_flow_in_body<'a>(body: &'a [FlowElement], id: &str) -> Option<&'a SequenceFlow> {
    body.iter().find_map(|element| match element {
        FlowElement::Flow(flow) if flow.id == id => Some(flow),
        FlowElement::Node(FlowNode {
            kind: NodeKind::SubProcess { body: inner, .. },
            ..
        }) => find_flow_in_body(inner, id),
        _ => None,
    })
}

fn collect_nodes<'a>(body: &'a [FlowElement], out: &mut Vec<&'a FlowNode>) {
    for element in body {
        match element {
            FlowElement::Node(node) => {
                out.push(node);
                if let NodeKind::SubProcess { body: inner, .. } = &node.kind {
                    collect_nodes(inner, out);
                }
            }
            FlowElement::Flow(_) => {}
        }
    }
}

fn collect_flows<'a>(body: &'a [FlowElement], out: &mut Vec<&'a SequenceFlow>) {
    for element in body {
        match element {
            FlowElement::Flow(flow) => out.push(flow),
            FlowElement::Node(node) => {
                if let NodeKind::SubProcess { body: inner, .. } = &node.kind {
                    collect_flows(inner, out);
                }
            }
        }
    }
}

/// One entry of `process/flowElements`.
#[derive(Debug, Clone)]
pub enum FlowElement {
    /// A flow node (event, activity, gateway).
    Node(FlowNode),
    /// A sequence flow between nodes.
    Flow(SequenceFlow),
}

/// A semantic flow node. Geometry lives in DI shapes referenced by `id`.
#[derive(Debug, Clone)]
pub struct FlowNode {
    /// Node id (BPMN ids are required for rendered elements).
    pub id: String,
    /// Display name.
    pub name: Option<String>,
    /// `incoming` sequence flow references as declared.
    pub incoming: Vec<String>,
    /// `outgoing` sequence flow references as declared.
    pub outgoing: Vec<String>,
    /// The `default` attribute: id of the default outgoing sequence flow.
    pub default_flow_ref: Option<String>,
    /// The symbol kind determining the glyph.
    pub kind: NodeKind,
}

/// The BPMN symbol family of a flow node.
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// Start event; `definition` selects the marker glyph.
    StartEvent { definition: EventDefinition },
    /// End event.
    EndEvent { definition: EventDefinition },
    /// Intermediate catch event.
    IntermediateCatchEvent { definition: EventDefinition },
    /// Intermediate throw event.
    IntermediateThrowEvent { definition: EventDefinition },
    /// Task or one of the common task kinds.
    Task { task_type: TaskType },
    /// Gateway.
    Gateway { gateway_type: GatewayType },
    /// Embedded subprocess; `body` holds nested flow elements.
    SubProcess {
        /// Semantic `triggeredByEvent` attribute.
        triggered_by_event: bool,
        /// Nested flow elements in source order.
        body: Vec<FlowElement>,
    },
    /// A recognized BPMN flow node outside the P0 glyph subset (for example
    /// `callActivity`). It resolves references normally and renders with a
    /// conspicuous fallback glyph plus a warning (SPEC-001 section 2).
    Other {
        /// Local name of the semantic element, e.g. `callActivity`.
        local_name: String,
    },
}

/// Marker element found inside an event, if any.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EventDefinition {
    /// No marker element (plain event).
    #[default]
    None,
    /// Message.
    Message,
    /// Timer.
    Timer,
    /// Error.
    Error,
    /// Escalation.
    Escalation,
    /// Terminate.
    Terminate,
    /// Compensation.
    Compensation,
    /// Conditional.
    Conditional,
    /// Link.
    Link,
    /// Signal.
    Signal,
    /// Cancel.
    Cancel,
    /// Multiple / parallel markers; drawn as fallback with a warning.
    Multiple,
}

/// Common task kinds (element name or `xsi:type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    /// Abstract `task`.
    Abstract,
    /// `userTask`.
    User,
    /// `serviceTask`.
    Service,
    /// `manualTask`.
    Manual,
    /// `scriptTask`.
    Script,
    /// `sendTask`.
    Send,
    /// `receiveTask`.
    Receive,
    /// `businessRuleTask`.
    BusinessRule,
}

/// Gateway kinds; P0 renders exclusive, parallel and inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayType {
    /// `exclusiveGateway` (X marker).
    Exclusive,
    /// `parallelGateway` (plus marker).
    Parallel,
    /// `inclusiveGateway` (circle marker).
    Inclusive,
    /// `eventBasedGateway`; rendered as fallback with a warning in P0.
    EventBased,
    /// `complexGateway`; rendered as fallback with a warning in P0.
    Complex,
}

/// A `bpmn:sequenceFlow`.
#[derive(Debug, Clone)]
pub struct SequenceFlow {
    /// Flow id.
    pub id: String,
    /// Display name.
    pub name: Option<String>,
    /// `sourceRef` node id.
    pub source_ref: String,
    /// `targetRef` node id.
    pub target_ref: String,
    /// True when this flow is the `default` of its source node.
    pub is_default: bool,
    /// Condition expression text, when present.
    pub condition: Option<String>,
}

/// A `bpmn:laneSet`.
#[derive(Debug, Clone, Default)]
pub struct LaneSet {
    /// Lane set id.
    pub id: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Lanes in source order.
    pub lanes: Vec<Lane>,
}

/// A `bpmn:lane`.
#[derive(Debug, Clone, Default)]
pub struct Lane {
    /// Lane id.
    pub id: String,
    /// Display name.
    pub name: Option<String>,
    /// `flowNodeRef` node ids.
    pub flow_node_refs: Vec<String>,
}

/// A `bpmn:participant` (a pool).
#[derive(Debug, Clone, Default)]
pub struct Participant {
    /// Participant id.
    pub id: String,
    /// Display name.
    pub name: Option<String>,
    /// Referenced process id.
    pub process_ref: Option<String>,
}

/// A `bpmn:messageFlow`.
#[derive(Debug, Clone)]
pub struct MessageFlow {
    /// Flow id.
    pub id: String,
    /// Display name.
    pub name: Option<String>,
    /// Source participant or node id.
    pub source_ref: String,
    /// Target participant or node id.
    pub target_ref: String,
}

/// A `bpmn:collaboration` with pools and message flows.
#[derive(Debug, Clone, Default)]
pub struct Collaboration {
    /// Collaboration id.
    pub id: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Participants in source order.
    pub participants: Vec<Participant>,
    /// Message flows in source order.
    pub message_flows: Vec<MessageFlow>,
}

/// Process artifacts (P0: text annotations).
#[derive(Debug, Clone)]
pub enum Artifact {
    /// A `bpmn:textAnnotation`.
    TextAnnotation(TextAnnotation),
}

impl Artifact {
    /// Artifact id.
    pub fn id(&self) -> &str {
        match self {
            Artifact::TextAnnotation(annotation) => &annotation.id,
        }
    }
}

/// A `bpmn:textAnnotation` with its text content.
#[derive(Debug, Clone, Default)]
pub struct TextAnnotation {
    /// Annotation id.
    pub id: String,
    /// The text content.
    pub text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BpmnDocument;
    use crate::di::{Diagram, Plane};

    fn task(id: &str, name: Option<&str>) -> FlowElement {
        FlowElement::Node(FlowNode {
            id: id.to_string(),
            name: name.map(str::to_string),
            incoming: Vec::new(),
            outgoing: Vec::new(),
            default_flow_ref: None,
            kind: NodeKind::Task {
                task_type: TaskType::Abstract,
            },
        })
    }

    #[test]
    fn finds_nested_nodes_and_flows() {
        let flow = FlowElement::Flow(SequenceFlow {
            id: "Flow_inner".to_string(),
            name: None,
            source_ref: "A".to_string(),
            target_ref: "B".to_string(),
            is_default: false,
            condition: None,
        });
        let process = Process {
            flow_elements: vec![
                task("Outer", None),
                FlowElement::Node(FlowNode {
                    id: "Sub".to_string(),
                    name: None,
                    incoming: Vec::new(),
                    outgoing: Vec::new(),
                    default_flow_ref: None,
                    kind: NodeKind::SubProcess {
                        triggered_by_event: false,
                        body: vec![task("Inner", None), flow],
                    },
                }),
            ],
            ..Process::default()
        };
        assert!(process.find_node("Inner").is_some());
        assert!(process.find_sequence_flow("Flow_inner").is_some());
        assert!(process.find_node("Missing").is_none());
        assert_eq!(process.walk_nodes().len(), 3);
    }

    #[test]
    fn document_diagram_lookup() {
        let document = BpmnDocument {
            definitions: Definitions {
                diagrams: vec![Diagram {
                    id: Some("D1".to_string()),
                    name: None,
                    plane: Plane::default(),
                }],
                ..Definitions::default()
            },
            parse_diagnostics: Vec::new(),
        };
        assert!(document.diagram_by_id("D1").is_some());
        assert!(document.diagram_by_index(1).is_none());
        assert_eq!(document.diagram_count(), 1);
    }
}
