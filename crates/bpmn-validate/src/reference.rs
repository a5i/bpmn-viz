//! Semantic and DI reference checks: duplicate ids and dangling references.

use std::collections::{HashMap, HashSet};

use bpmn_model::diagnostic::{Diagnostic, DiagnosticCode};
use bpmn_model::element::{FlowElement, NodeKind};
use bpmn_model::BpmnDocument;

/// Semantic ids known to the document plus their owning scope.
pub struct IdRegistry {
    /// id → number of declarations.
    pub counts: HashMap<String, usize>,
    /// id → owning scope id (process or collaboration id).
    pub scope: HashMap<String, String>,
    /// Flow node ids.
    pub nodes: HashSet<String>,
    /// Sequence flow ids.
    pub flows: HashSet<String>,
    /// Message flow ids.
    pub message_flows: HashSet<String>,
    /// Participant ids.
    pub participants: HashSet<String>,
    /// Lane ids.
    pub lanes: HashSet<String>,
    /// Artifact ids.
    pub artifacts: HashSet<String>,
    /// Process ids.
    pub processes: HashSet<String>,
}

impl IdRegistry {
    /// True when `id` is any known semantic element.
    pub fn contains(&self, id: &str) -> bool {
        self.nodes.contains(id)
            || self.flows.contains(id)
            || self.message_flows.contains(id)
            || self.participants.contains(id)
            || self.lanes.contains(id)
            || self.artifacts.contains(id)
            || self.processes.contains(id)
    }
}

/// Builds the id registry used by all reference checks.
pub fn build_registry(document: &BpmnDocument) -> IdRegistry {
    let mut registry = IdRegistry {
        counts: HashMap::new(),
        scope: HashMap::new(),
        nodes: HashSet::new(),
        flows: HashSet::new(),
        message_flows: HashSet::new(),
        participants: HashSet::new(),
        lanes: HashSet::new(),
        artifacts: HashSet::new(),
        processes: HashSet::new(),
    };
    let mut note = |registry: &mut IdRegistry, id: &str, scope: &str| {
        *registry.counts.entry(id.to_string()).or_insert(0) += 1;
        registry.scope.entry(id.to_string()).or_insert_with(|| scope.to_string());
    };
    for root in &document.definitions.root_elements {
        match root {
            bpmn_model::element::RootElement::Process(process) => {
                let scope = process
                    .id
                    .clone()
                    .unwrap_or_else(|| format!("<process {}>", registry.processes.len()));
                if let Some(id) = &process.id {
                    note(&mut registry, id, &scope);
                    registry.processes.insert(id.clone());
                }
                collect_flow_elements(&process.flow_elements, &scope, &mut registry, &mut note);
                for lane_set in &process.lane_sets {
                    if let Some(id) = &lane_set.id {
                        note(&mut registry, id, &scope);
                    }
                    for lane in &lane_set.lanes {
                        note(&mut registry, &lane.id, &scope);
                        registry.lanes.insert(lane.id.clone());
                    }
                }
                for artifact in &process.artifacts {
                    note(&mut registry, artifact.id(), &scope);
                    registry.artifacts.insert(artifact.id().to_string());
                }
            }
            bpmn_model::element::RootElement::Collaboration(collaboration) => {
                let scope = collaboration.id.clone().unwrap_or_else(|| "<collaboration>".into());
                if let Some(id) = &collaboration.id {
                    note(&mut registry, id, &scope);
                }
                for participant in &collaboration.participants {
                    note(&mut registry, &participant.id, &scope);
                    registry.participants.insert(participant.id.clone());
                }
                for flow in &collaboration.message_flows {
                    note(&mut registry, &flow.id, &scope);
                    registry.message_flows.insert(flow.id.clone());
                }
            }
        }
    }
    for diagram in &document.definitions.diagrams {
        if let Some(id) = &diagram.id {
            *registry.counts.entry(id.clone()).or_insert(0) += 1;
        }
        if let Some(id) = &diagram.plane.id {
            *registry.counts.entry(id.clone()).or_insert(0) += 1;
        }
        for shape in &diagram.plane.shapes {
            if let Some(id) = &shape.id {
                *registry.counts.entry(id.clone()).or_insert(0) += 1;
            }
        }
        for edge in &diagram.plane.edges {
            if let Some(id) = &edge.id {
                *registry.counts.entry(id.clone()).or_insert(0) += 1;
            }
        }
    }
    registry
}

fn collect_flow_elements(
    elements: &[FlowElement],
    scope: &str,
    registry: &mut IdRegistry,
    note: &mut impl FnMut(&mut IdRegistry, &str, &str),
) {
    for element in elements {
        match element {
            FlowElement::Node(node) => {
                note(registry, &node.id, scope);
                registry.nodes.insert(node.id.clone());
                if let NodeKind::SubProcess { body, .. } = &node.kind {
                    collect_flow_elements(body, scope, registry, note);
                }
            }
            FlowElement::Flow(flow) => {
                note(registry, &flow.id, scope);
                registry.flows.insert(flow.id.clone());
            }
        }
    }
}

/// Runs all reference checks; returns unsorted diagnostics.
pub fn check_references(document: &BpmnDocument) -> Vec<Diagnostic> {
    let registry = build_registry(document);
    let mut diagnostics = Vec::new();

    // Duplicate ids (AC-02).
    let mut duplicated: Vec<&String> = registry
        .counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(id, _)| id)
        .collect();
    duplicated.sort();
    for id in duplicated {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::DuplicateId,
                format!("id `{id}` is declared {} times; ids must be unique", registry.counts[id]),
            )
            .with_element_id(id.clone()),
        );
    }

    for root in &document.definitions.root_elements {
        match root {
            bpmn_model::element::RootElement::Process(process) => {
                let scope = process.id.clone().unwrap_or_default();
                check_flows_in_body(&process.flow_elements, &scope, &registry, &mut diagnostics);
                for lane_set in &process.lane_sets {
                    for lane in &lane_set.lanes {
                        for node_ref in &lane.flow_node_refs {
                            check_ref(
                                &mut diagnostics,
                                &registry,
                                &node_ref.clone(),
                                &format!("lane `{}` flowNodeRef", lane.id),
                            );
                        }
                    }
                }
            }
            bpmn_model::element::RootElement::Collaboration(collaboration) => {
                for participant in &collaboration.participants {
                    if let Some(process_ref) = &participant.process_ref {
                        if !registry.processes.contains(process_ref) {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::DanglingReference,
                                    format!(
                                        "participant references process `{process_ref}` which does not exist"
                                    ),
                                )
                                .with_element_id(participant.id.clone()),
                            );
                        }
                    }
                }
                for flow in &collaboration.message_flows {
                    for (role, reference) in [("sourceRef", &flow.source_ref), ("targetRef", &flow.target_ref)] {
                        if !registry.participants.contains(reference) && !registry.nodes.contains(reference) {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::DanglingReference,
                                    format!(
                                        "message flow {role} `{reference}` does not exist"
                                    ),
                                )
                                .with_element_id(flow.id.clone()),
                            );
                        }
                    }
                }
            }
        }
    }

    // DI references (AC-03).
    for diagram in &document.definitions.diagrams {
        if let Some(plane_element) = &diagram.plane.bpmn_element {
            if !registry.processes.contains(plane_element)
                && document.collaborations().all(|c| c.id.as_deref() != Some(plane_element))
            {
                diagnostics.push(Diagnostic::warning(
                    DiagnosticCode::DanglingDiReference,
                    format!("BPMNPlane references `{plane_element}` which does not exist"),
                ));
            }
        }
        for shape in &diagram.plane.shapes {
            if !registry.contains(&shape.bpmn_element) {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::DanglingDiReference,
                        format!(
                            "BPMNShape references element `{}` which does not exist",
                            shape.bpmn_element
                        ),
                    )
                    .with_element_id(shape.bpmn_element.clone()),
                );
            }
        }
        for edge in &diagram.plane.edges {
            if !registry.flows.contains(&edge.bpmn_element)
                && !registry.message_flows.contains(&edge.bpmn_element)
            {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::DanglingDiReference,
                        format!(
                            "BPMNEdge references flow `{}` which does not exist",
                            edge.bpmn_element
                        ),
                    )
                    .with_element_id(edge.bpmn_element.clone()),
                );
            }
        }
    }

    diagnostics
}

fn check_flows_in_body(
    elements: &[FlowElement],
    scope: &str,
    registry: &IdRegistry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for element in elements {
        match element {
            FlowElement::Node(node) => {
                if let NodeKind::SubProcess { body, .. } = &node.kind {
                    check_flows_in_body(body, scope, registry, diagnostics);
                }
            }
            FlowElement::Flow(flow) => {
                for (role, reference) in [("sourceRef", &flow.source_ref), ("targetRef", &flow.target_ref)] {
                    if reference.is_empty() {
                        continue; // parse already reported this
                    }
                    if !registry.nodes.contains(reference) {
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::DanglingReference,
                                format!("sequence flow {role} `{reference}` does not exist"),
                            )
                            .with_element_id(flow.id.clone()),
                        );
                    } else if !scope.is_empty() && registry.scope.get(reference).map(String::as_str) != Some(scope)
                    {
                        diagnostics.push(
                            Diagnostic::warning(
                                DiagnosticCode::ForeignScopeReference,
                                format!(
                                    "sequence flow {role} `{reference}` resolves outside the enclosing process"
                                ),
                            )
                            .with_element_id(flow.id.clone()),
                        );
                    }
                }
            }
        }
    }
}

fn check_ref(
    diagnostics: &mut Vec<Diagnostic>,
    registry: &IdRegistry,
    id: &str,
    context: &str,
) {
    if !registry.nodes.contains(id) {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::DanglingReference,
                format!("{context} references `{id}` which does not exist"),
            )
            .with_element_id(id.to_string()),
        );
    }
}
