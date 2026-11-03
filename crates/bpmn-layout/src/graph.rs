//! Scope-local graph: extraction, referential validation and stable indexed
//! ids (TASK-018 §5.1).
//!
//! Only ordinary sequence-flow participants of the selected process enter the
//! graph. Boundary events, unsupported nodes, artifacts and lane membership
//! are separated out with warnings; flows touching them are skipped rather
//! than misrepresented. All ordering derives from BPMN declaration order.

use std::collections::HashMap;

use bpmn_model::BpmnDocument;
use bpmn_model::element::{FlowElement, NodeKind, Process};

use crate::api::{LayoutError, LayoutOptions, LayoutWarning, LayoutWarningCode};

/// A supported flow node of the scope with stable graph indexes.
#[derive(Debug, Clone)]
pub(crate) struct NodeRec {
    /// Semantic id.
    pub id: String,
    /// Declaration index within the process (stable ordering basis).
    pub decl: usize,
    /// Canonical width.
    pub width: i64,
    /// Canonical height.
    pub height: i64,
    /// Flow indexes ending at this node (declaration order).
    pub incoming: Vec<usize>,
    /// Flow indexes starting at this node (declaration order).
    pub outgoing: Vec<usize>,
    /// Weakly connected component id (assigned by [`crate::semantics`]).
    pub component: usize,
    /// Rank (longest path over forward edges; assigned by
    /// [`crate::semantics`]).
    pub rank: usize,
    /// Semantic band (0 = spine; assigned by [`crate::semantics`]).
    pub band: i64,
    /// True for end events.
    pub is_end: bool,
    /// True for start events.
    pub is_start_event: bool,
}

/// A supported sequence flow of the scope.
#[derive(Debug, Clone)]
pub(crate) struct FlowRec {
    /// Semantic id.
    pub id: String,
    /// Source node index.
    pub source: usize,
    /// Target node index.
    pub target: usize,
    /// The `default` attribute of the source node selects this flow.
    pub is_default: bool,
    /// True when a deterministic DFS classifies this edge as closing a cycle.
    pub back_edge: bool,
    /// True when this flow lies on the component's primary spine.
    pub spine: bool,
}

/// The scope-local graph. Node and flow vectors are in declaration order;
/// indexes are stable across the pipeline.
#[derive(Debug, Clone)]
pub(crate) struct ScopeGraph {
    /// Semantic id of the selected process.
    pub scope_id: String,
    /// Supported flow nodes in declaration order.
    pub nodes: Vec<NodeRec>,
    /// Supported sequence flows in declaration order.
    pub flows: Vec<FlowRec>,
}

impl ScopeGraph {
    /// Extracts the graph from one process scope, emitting warnings for
    /// unsupported constructs and returning structured errors for invalid
    /// references. Never panics on malformed input.
    pub(crate) fn extract(
        model: &BpmnDocument,
        process: &Process,
        options: &LayoutOptions,
        warnings: &mut Vec<LayoutWarning>,
    ) -> Result<Self, LayoutError> {
        let scope_id = process.id.clone().unwrap_or_default();

        // Resource limits count every top-level flow element of the scope,
        // including ones that will be skipped with a warning.
        let mut declared_nodes = 0usize;
        let mut declared_flows = 0usize;
        for element in &process.flow_elements {
            match element {
                FlowElement::Node(_) => declared_nodes += 1,
                FlowElement::Flow(_) => declared_flows += 1,
            }
        }
        if declared_nodes > options.resource_limits.max_nodes {
            return Err(LayoutError::LimitExceeded { limit: "max_nodes" });
        }
        if declared_flows > options.resource_limits.max_flows {
            return Err(LayoutError::LimitExceeded { limit: "max_flows" });
        }

        // First pass: nodes. The id map is used for resolution only; output
        // ordering never iterates it (ADR-0006 determinism policy).
        let mut nodes: Vec<NodeRec> = Vec::new();
        let mut node_index: HashMap<&str, usize> = HashMap::new();
        // In-scope ids that resolve but are not laid out (boundary events,
        // unsupported kinds): flows touching them are skipped, not errors.
        let mut skipped_ids: Vec<&str> = Vec::new();
        for element in &process.flow_elements {
            let FlowElement::Node(node) = element else {
                continue;
            };
            if node_index.contains_key(node.id.as_str()) {
                return Err(LayoutError::DuplicateNode {
                    id: node.id.clone(),
                });
            }
            match classify(node) {
                Classification::Placed(width, height) => {
                    if let NodeKind::SubProcess { body, .. } = &node.kind
                        && !body.is_empty()
                    {
                        warnings.push(LayoutWarning::new(
                            LayoutWarningCode::SubprocessBodySkipped,
                            Some(node.id.clone()),
                            "embedded subprocess laid out as a collapsed activity; recursive child layout arrives in M2",
                        ));
                    }
                    let index = nodes.len();
                    node_index.insert(node.id.as_str(), index);
                    nodes.push(NodeRec {
                        id: node.id.clone(),
                        decl: index,
                        width,
                        height,
                        incoming: Vec::new(),
                        outgoing: Vec::new(),
                        component: 0,
                        rank: 0,
                        band: 0,
                        is_end: matches!(node.kind, NodeKind::EndEvent { .. }),
                        is_start_event: matches!(node.kind, NodeKind::StartEvent { .. }),
                    });
                }
                Classification::Skipped(message) => {
                    warnings.push(LayoutWarning::new(
                        skipped_warning_code(node),
                        Some(node.id.clone()),
                        message,
                    ));
                    skipped_ids.push(node.id.as_str());
                }
            }
        }

        if nodes.is_empty() {
            return Err(LayoutError::EmptyScope { scope: scope_id });
        }

        // Artifacts and lanes are M2/M3 features; record them honestly.
        for artifact in &process.artifacts {
            warnings.push(LayoutWarning::new(
                LayoutWarningCode::ArtifactSkipped,
                Some(artifact.id().to_string()),
                "artifacts are not placed by M1 layout",
            ));
        }
        for lane_set in &process.lane_sets {
            warnings.push(LayoutWarning::new(
                LayoutWarningCode::LaneMembershipIgnored,
                Some(lane_set.id.clone().unwrap_or_else(|| scope_id.clone())),
                "lane membership is ignored by M1 layout",
            ));
        }

        // Second pass: sequence flows.
        let mut flows: Vec<FlowRec> = Vec::new();
        for element in &process.flow_elements {
            let FlowElement::Flow(flow) = element else {
                continue;
            };
            let mut endpoints = Vec::with_capacity(2);
            for endpoint_ref in [flow.source_ref.as_str(), flow.target_ref.as_str()] {
                match resolve_endpoint(
                    endpoint_ref,
                    model,
                    &node_index,
                    &skipped_ids,
                    flow.id.as_str(),
                )? {
                    Some(index) => endpoints.push(index),
                    None => {
                        // Flow touches a skipped node: skip the flow itself.
                        endpoints.clear();
                        break;
                    }
                }
            }
            if endpoints.len() != 2 {
                warnings.push(LayoutWarning::new(
                    LayoutWarningCode::UnsupportedFlowSkipped,
                    Some(flow.id.clone()),
                    "flow touches a node skipped by M1 layout (boundary event or unsupported element)",
                ));
                continue;
            }
            let index = flows.len();
            flows.push(FlowRec {
                id: flow.id.clone(),
                source: endpoints[0],
                target: endpoints[1],
                is_default: flow.is_default,
                back_edge: false,
                spine: false,
            });
            nodes[endpoints[0]].outgoing.push(index);
            nodes[endpoints[1]].incoming.push(index);
        }

        Ok(Self {
            scope_id,
            nodes,
            flows,
        })
    }
}

/// Resolves one flow endpoint. Returns:
/// - `Ok(Some(index))` when the endpoint is a placed node,
/// - `Ok(None)` when it resolves to an in-scope skipped node (the flow is
///   skipped with a warning),
/// - `Err` for missing or cross-scope references.
fn resolve_endpoint(
    id: &str,
    model: &BpmnDocument,
    node_index: &HashMap<&str, usize>,
    skipped_ids: &[&str],
    flow_id: &str,
) -> Result<Option<usize>, LayoutError> {
    if let Some(&index) = node_index.get(id) {
        return Ok(Some(index));
    }
    if skipped_ids.contains(&id) {
        return Ok(None);
    }
    if model.find_node(id).is_some() {
        // Resolvable, but outside the selected top-level scope (subprocess
        // body or another process). Scopes are never silently mixed.
        return Err(LayoutError::CrossScopeFlow {
            flow: flow_id.to_string(),
        });
    }
    Err(LayoutError::MissingEndpoint {
        flow: flow_id.to_string(),
        endpoint: id.to_string(),
    })
}

/// What happens to a declared flow node in M1.
enum Classification {
    /// Placed with canonical width and height.
    Placed(i64, i64),
    /// Skipped with a warning message.
    Skipped(&'static str),
}

fn classify(node: &bpmn_model::element::FlowNode) -> Classification {
    match &node.kind {
        NodeKind::StartEvent { .. }
        | NodeKind::EndEvent { .. }
        | NodeKind::IntermediateCatchEvent { .. }
        | NodeKind::IntermediateThrowEvent { .. } => Classification::Placed(36, 36),
        NodeKind::BoundaryEvent { .. } => {
            Classification::Skipped("boundary events are not placed by M1 layout (M2 feature)")
        }
        NodeKind::Task { .. } => Classification::Placed(100, 80),
        NodeKind::SubProcess { .. } => Classification::Placed(100, 80),
        NodeKind::Gateway { gateway_type } => match gateway_type {
            bpmn_model::element::GatewayType::Complex => Classification::Skipped(
                "complex gateways are outside the M1 supported element matrix",
            ),
            _ => Classification::Placed(50, 50),
        },
        NodeKind::Other { local_name } => {
            if local_name == "callActivity" {
                Classification::Placed(100, 80)
            } else {
                Classification::Skipped("element kind is outside the M1 supported element matrix")
            }
        }
    }
}

fn skipped_warning_code(node: &bpmn_model::element::FlowNode) -> LayoutWarningCode {
    if matches!(node.kind, NodeKind::BoundaryEvent { .. }) {
        LayoutWarningCode::BoundaryEventSkipped
    } else {
        LayoutWarningCode::UnsupportedNodeSkipped
    }
}
