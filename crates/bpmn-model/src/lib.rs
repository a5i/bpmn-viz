//! Semantic BPMN 2.0 model, Diagram Interchange (DI) types and diagnostics.
//!
//! The model is deliberately free of rendering concerns: geometry lives in
//! [`di`], semantics in [`element`], and shared diagnostics in [`diagnostic`].
//! See `docs/architecture/data-model.md` and SPEC-001 section 4.

pub mod di;
pub mod diagnostic;
pub mod element;

pub use di::{Bounds, BpmnLabel, Diagram, Edge, Plane, Point, Shape};
pub use diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceLocation};
pub use element::{
    Artifact, Collaboration, Definitions, EventDefinition, FlowElement, FlowNode, GatewayType,
    Lane, LaneSet, MessageFlow, NodeKind, Participant, Process, RootElement, SequenceFlow,
    TaskType, TextAnnotation,
};

/// A fully parsed BPMN document: semantics plus Diagram Interchange.
///
/// A fully parsed BPMN document: semantics plus Diagram Interchange.
///
/// Fatal import problems are reported as errors by the `bpmn-xml` crate;
/// non-fatal notes (ignored vendor extensions and similar) are collected
/// in [`BpmnDocument::parse_diagnostics`].
#[derive(Debug, Clone, Default)]
pub struct BpmnDocument {
    /// Document-level `definitions` contents.
    pub definitions: Definitions,
    /// Non-fatal diagnostics recorded during import.
    pub parse_diagnostics: Vec<Diagnostic>,
}

impl BpmnDocument {
    /// Diagram by zero-based index in source order.
    pub fn diagram_by_index(&self, index: usize) -> Option<&Diagram> {
        self.definitions.diagrams.get(index)
    }

    /// Diagram by its DI id.
    pub fn diagram_by_id(&self, id: &str) -> Option<&Diagram> {
        self.definitions
            .diagrams
            .iter()
            .find(|d| d.id.as_deref() == Some(id))
    }

    /// First diagram in source order (documented CLI default).
    pub fn first_diagram(&self) -> Option<&Diagram> {
        self.definitions.diagrams.first()
    }

    /// Number of diagrams in source order.
    pub fn diagram_count(&self) -> usize {
        self.definitions.diagrams.len()
    }

    /// All processes, including those nested in the model root.
    pub fn processes(&self) -> impl Iterator<Item = &Process> {
        self.definitions
            .root_elements
            .iter()
            .filter_map(|root| match root {
                RootElement::Process(process) => Some(process),
                RootElement::Collaboration(_) => None,
            })
    }

    /// All collaborations.
    pub fn collaborations(&self) -> impl Iterator<Item = &Collaboration> {
        self.definitions
            .root_elements
            .iter()
            .filter_map(|root| match root {
                RootElement::Collaboration(collaboration) => Some(collaboration),
                RootElement::Process(_) => None,
            })
    }

    /// Look up a flow node by id anywhere in the document (recursing into
    /// subprocess bodies).
    pub fn find_node(&self, id: &str) -> Option<&FlowNode> {
        self.processes().find_map(|p| p.find_node(id))
    }

    /// Look up a sequence flow by id anywhere in the document.
    pub fn find_sequence_flow(&self, id: &str) -> Option<&SequenceFlow> {
        self.processes().find_map(|p| p.find_sequence_flow(id))
    }

    /// Look up a participant by id.
    pub fn find_participant(&self, id: &str) -> Option<&Participant> {
        self.collaborations()
            .find_map(|c| c.participants.iter().find(|p| p.id == id))
    }

    /// Look up a lane by id in any lane set.
    pub fn find_lane(&self, id: &str) -> Option<&Lane> {
        self.processes().find_map(|p| p.find_lane(id))
    }

    /// Look up an artifact (for example a text annotation) by id.
    pub fn find_artifact(&self, id: &str) -> Option<&Artifact> {
        self.processes().find_map(|p| p.find_artifact(id))
    }

    /// Look up a message flow by id.
    pub fn find_message_flow(&self, id: &str) -> Option<&MessageFlow> {
        self.collaborations()
            .find_map(|c| c.message_flows.iter().find(|f| f.id == id))
    }
}
