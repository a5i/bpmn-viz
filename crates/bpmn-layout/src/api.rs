//! Public layout API: engine trait, scope selection, options, outcome and the
//! typed error/warning taxonomy (ADR-0006, ADR-0007).
//!
//! The API is independent of XML parsing, SVG rendering and rasterization:
//! it consumes [`bpmn_model::BpmnDocument`] semantics and produces
//! [`bpmn_model::di::Diagram`] geometry.

use bpmn_model::di::Diagram;

use crate::{di, graph, placement, routing, semantics, validation};

/// Selects the semantic scope a layout is computed for.
///
/// M1 supports single process scopes; the selected process must exist and
/// flows must not reference nodes outside of it (structured
/// [`LayoutError::CrossScopeFlow`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeId {
    /// The process with the given id.
    Process(String),
}

impl ScopeId {
    /// Selects a process scope by id.
    pub fn process(id: impl Into<String>) -> Self {
        ScopeId::Process(id.into())
    }
}

/// Layout direction. M1 supports left-to-right only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Left-to-right narrative flow (reference default).
    #[default]
    LeftToRight,
}

/// Resource limits protecting against pathological inputs. Requests beyond a
/// limit fail with [`LayoutError::LimitExceeded`] instead of exhausting
/// memory or time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutLimits {
    /// Maximum number of flow nodes in the selected scope.
    pub max_nodes: usize,
    /// Maximum number of sequence flows in the selected scope.
    pub max_flows: usize,
}

impl Default for LayoutLimits {
    fn default() -> Self {
        Self {
            max_nodes: 10_000,
            max_flows: 10_000,
        }
    }
}

/// Tunables of the layered algorithm. Gaps are defaults, not immutable BPMN
/// requirements; they must be finite and nonnegative and are rounded to whole
/// coordinates once, when applied.
///
/// Defaults mirror the pinned reference constants: rank gap 100, node gap 80,
/// routing clearance 20.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutOptions {
    /// Layout direction; M1 supports [`Orientation::LeftToRight`] only.
    pub orientation: Orientation,
    /// Horizontal distance between rank columns.
    pub rank_gap: f64,
    /// Vertical distance between nodes in a band row (the semantic band pitch
    /// adds a fixed band height on top of this gap).
    pub node_gap: f64,
    /// Clearance that routed segments keep from shape interiors.
    pub routing_clearance: f64,
    /// M1 is fully deterministic; every tie-break follows BPMN declaration
    /// order regardless of this flag, and `false` is rejected with
    /// [`LayoutError::UnsupportedOption`] so future non-deterministic
    /// strategies cannot silently change output stability.
    pub deterministic: bool,
    /// Resource limits.
    pub resource_limits: LayoutLimits,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            orientation: Orientation::LeftToRight,
            rank_gap: 100.0,
            node_gap: 80.0,
            routing_clearance: 20.0,
            deterministic: true,
            resource_limits: LayoutLimits::default(),
        }
    }
}

/// Stable, machine-readable warning categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutWarningCode {
    /// A flow node outside the M1 supported matrix was skipped.
    UnsupportedNodeSkipped,
    /// A sequence flow touching a skipped node was skipped.
    UnsupportedFlowSkipped,
    /// An embedded subprocess was laid out as a collapsed activity and its
    /// body was not recursed into (M2 feature).
    SubprocessBodySkipped,
    /// A boundary event was skipped together with its exception flows (M2
    /// feature).
    BoundaryEventSkipped,
    /// Lane membership was ignored (lane rendering is an M2 feature).
    LaneMembershipIgnored,
    /// A process artifact (annotation, data shape) was skipped (M3 feature).
    ArtifactSkipped,
}

impl LayoutWarningCode {
    /// Stable code string used in diagnostics output.
    pub fn as_str(&self) -> &'static str {
        match self {
            LayoutWarningCode::UnsupportedNodeSkipped => "layout.unsupported_node_skipped",
            LayoutWarningCode::UnsupportedFlowSkipped => "layout.unsupported_flow_skipped",
            LayoutWarningCode::SubprocessBodySkipped => "layout.subprocess_body_skipped",
            LayoutWarningCode::BoundaryEventSkipped => "layout.boundary_event_skipped",
            LayoutWarningCode::LaneMembershipIgnored => "layout.lane_membership_ignored",
            LayoutWarningCode::ArtifactSkipped => "layout.artifact_skipped",
        }
    }
}

/// A non-fatal layout note with a stable code, the affected semantic element
/// and human-readable context. Warnings are ordered deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutWarning {
    /// Warning category.
    pub code: LayoutWarningCode,
    /// Semantic element id the warning is about, when applicable.
    pub element_id: Option<String>,
    /// Human-readable context.
    pub message: String,
}

impl LayoutWarning {
    /// Creates a warning about one element.
    pub fn new(
        code: LayoutWarningCode,
        element_id: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            element_id,
            message: message.into(),
        }
    }
}

/// Fatal layout failures with stable machine-readable codes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    /// The requested scope does not exist in the document.
    #[error("unknown layout scope `{scope}`")]
    UnknownScope {
        /// Requested scope id.
        scope: String,
    },
    /// The selected scope contains no flow nodes at all.
    #[error("layout scope `{scope}` has no flow nodes")]
    EmptyScope {
        /// Selected scope id.
        scope: String,
    },
    /// Two flow nodes in the scope share an id.
    #[error("duplicate flow node id `{id}` in layout scope")]
    DuplicateNode {
        /// Duplicated id.
        id: String,
    },
    /// A sequence flow references an id that exists nowhere in the document.
    #[error("sequence flow `{flow}` references missing endpoint `{endpoint}`")]
    MissingEndpoint {
        /// Flow id.
        flow: String,
        /// Unresolvable endpoint id.
        endpoint: String,
    },
    /// A sequence flow leaves the selected scope (for example into a
    /// subprocess body or another process). Scopes are never silently mixed.
    #[error("sequence flow `{flow}` leaves the selected layout scope")]
    CrossScopeFlow {
        /// Flow id.
        flow: String,
    },
    /// No collision-free orthogonal route exists for a sequence flow under
    /// the given options.
    #[error("sequence flow `{flow}` could not be routed without crossing unrelated shapes")]
    RoutingFailed {
        /// Flow id.
        flow: String,
    },
    /// A defensive check failed: after removing DFS back edges a cycle
    /// remained, which would be an engine bug rather than bad input.
    #[error("cycle could not be broken in scope `{scope}`; this is an engine defect")]
    CycleNotBroken {
        /// Selected scope id.
        scope: String,
    },
    /// The DI verifier rejected generated geometry; this is an engine defect
    /// guard, never a property of valid input.
    #[error("generated DI failed verification: {reason}")]
    VerificationFailed {
        /// Description of the violated invariant.
        reason: String,
    },
    /// The scope exceeds `LayoutOptions::resource_limits`.
    #[error("layout resource limit exceeded: {limit}")]
    LimitExceeded {
        /// Name of the exceeded limit.
        limit: &'static str,
    },
    /// The requested option value or combination is not supported by this
    /// engine version.
    #[error("unsupported layout option `{option}`: {reason}")]
    UnsupportedOption {
        /// Option name.
        option: &'static str,
        /// Why it is rejected.
        reason: &'static str,
    },
}

impl LayoutError {
    /// Stable machine-readable code for diagnostics output.
    pub fn code(&self) -> &'static str {
        match self {
            LayoutError::UnknownScope { .. } => "layout.unknown_scope",
            LayoutError::EmptyScope { .. } => "layout.empty_scope",
            LayoutError::DuplicateNode { .. } => "layout.duplicate_node",
            LayoutError::MissingEndpoint { .. } => "layout.missing_endpoint",
            LayoutError::CrossScopeFlow { .. } => "layout.cross_scope_flow",
            LayoutError::RoutingFailed { .. } => "layout.routing_failed",
            LayoutError::CycleNotBroken { .. } => "layout.cycle_not_broken",
            LayoutError::VerificationFailed { .. } => "layout.verification_failed",
            LayoutError::LimitExceeded { .. } => "layout.limit_exceeded",
            LayoutError::UnsupportedOption { .. } => "layout.unsupported_option",
        }
    }

    /// Semantic element id the error is about, when applicable.
    pub fn element_id(&self) -> Option<&str> {
        match self {
            LayoutError::UnknownScope { scope }
            | LayoutError::EmptyScope { scope }
            | LayoutError::CycleNotBroken { scope } => Some(scope),
            LayoutError::DuplicateNode { id }
            | LayoutError::MissingEndpoint { flow: id, .. }
            | LayoutError::CrossScopeFlow { flow: id }
            | LayoutError::RoutingFailed { flow: id } => Some(id),
            _ => None,
        }
    }
}

/// The result of a successful layout: generated DI plus ordered warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutOutcome {
    /// Generated diagram. The plane is bound to the selected process; shape
    /// and edge ids are `<bpmnElement>_di`, and `bpmn_element` always carries
    /// the semantic element id (ADR-0007).
    pub diagram: Diagram,
    /// Non-fatal notes in deterministic order.
    pub warnings: Vec<LayoutWarning>,
}

/// The layout engine contract (TASK-018 §4). Implementations must be
/// deterministic for identical input and options, must not panic on any
/// [`BpmnDocument`], and must not mutate the model.
pub trait LayoutEngine {
    /// Lays out `scope` of `model` under `options`.
    fn layout(
        &self,
        model: &bpmn_model::BpmnDocument,
        scope: &ScopeId,
        options: &LayoutOptions,
    ) -> Result<LayoutOutcome, LayoutError>;
}

/// The native, deterministic BPMN-aware layered layout engine (ADR-0006).
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeBpmnLayout;

impl LayoutEngine for NativeBpmnLayout {
    fn layout(
        &self,
        model: &bpmn_model::BpmnDocument,
        scope: &ScopeId,
        options: &LayoutOptions,
    ) -> Result<LayoutOutcome, LayoutError> {
        validation::validate_options(options)?;
        let process = validation::resolve_scope(model, scope)?;
        let mut warnings = Vec::new();
        let mut graph = graph::ScopeGraph::extract(model, process, options, &mut warnings)?;
        let analysis = semantics::analyze(&mut graph)?;
        let placement = placement::place(&graph, &analysis, options);
        let routes = routing::route(&graph, &placement, options)?;
        let diagram = di::generate(scope, &graph, &placement, &routes);
        di::verify(&diagram, &graph, scope)?;
        Ok(LayoutOutcome { diagram, warnings })
    }
}
