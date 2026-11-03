//! Option and scope validation (TASK-018 §5.1).

use crate::api::{LayoutError, LayoutOptions, Orientation, ScopeId};
use bpmn_model::element::Process;

/// Rejects option values this engine version does not support, before any
/// geometry is computed. Gaps must be finite and nonnegative; they are
/// rounded to whole coordinates once, when applied.
pub(crate) fn validate_options(options: &LayoutOptions) -> Result<(), LayoutError> {
    if !options.deterministic {
        return Err(LayoutError::UnsupportedOption {
            option: "deterministic",
            reason: "this engine is deterministic only; tie-breaking always follows BPMN declaration order",
        });
    }
    if options.orientation != Orientation::LeftToRight {
        return Err(LayoutError::UnsupportedOption {
            option: "orientation",
            reason: "M1 supports LeftToRight only",
        });
    }
    for (name, value) in [
        ("rank_gap", options.rank_gap),
        ("node_gap", options.node_gap),
        ("routing_clearance", options.routing_clearance),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(LayoutError::UnsupportedOption {
                option: name,
                reason: "must be finite and nonnegative",
            });
        }
    }
    Ok(())
}

/// Resolves the requested [`ScopeId`] to the process with that id.
pub(crate) fn resolve_scope<'a>(
    model: &'a bpmn_model::BpmnDocument,
    scope: &ScopeId,
) -> Result<&'a Process, LayoutError> {
    match scope {
        ScopeId::Process(id) => model
            .processes()
            .find(|process| process.id.as_deref() == Some(id.as_str()))
            .ok_or_else(|| LayoutError::UnknownScope { scope: id.clone() }),
    }
}

/// Rounds an already validated option value to whole coordinates. This is the
/// single rounding policy of the engine (ADR-0006): internal geometry is
/// integer-based afterwards.
pub(crate) fn round_option(value: f64) -> i64 {
    value.round() as i64
}
