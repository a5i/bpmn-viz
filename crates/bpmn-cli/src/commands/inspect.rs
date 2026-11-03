//! `bpmn inspect` implementation: document summary in text or JSON.

use std::collections::BTreeMap;

use bpmn_model::element::{NodeKind, RootElement};
use serde::Serialize;

use crate::cli::{CliError, ExitCode};
use crate::{InspectCommand, OutputFormatKind};

/// Version bump for result envelope changes.
const CONTRACT_VERSION: u32 = 1;

/// JSON summary result.
#[derive(Serialize)]
pub struct InspectResult {
    /// Envelope version.
    pub contract_version: u32,
    /// Command name.
    pub command: &'static str,
    /// Input path as given.
    pub input: String,
    /// `definitions/@id`.
    pub definitions_id: Option<String>,
    /// Number of diagrams in source order.
    pub diagram_count: usize,
    /// Process count.
    pub processes: usize,
    /// Collaboration count.
    pub collaborations: usize,
    /// Participants (pools).
    pub participants: usize,
    /// Lanes.
    pub lanes: usize,
    /// Flow nodes grouped by kind.
    pub nodes_by_kind: BTreeMap<String, usize>,
    /// Sequence flows.
    pub sequence_flows: usize,
    /// Message flows.
    pub message_flows: usize,
    /// Text annotations.
    pub text_annotations: usize,
    /// Non-fatal import diagnostics.
    pub diagnostics: Vec<crate::cli::DiagnosticDto>,
}

pub(crate) fn run(
    command: &InspectCommand,
    format: OutputFormatKind,
) -> Result<ExitCode, CliError> {
    let document = crate::commands::load_document(&command.input)?;
    let report = bpmn_validate::validate(&document);
    let result = build_result(&command.input.display().to_string(), &document, &report);

    match format {
        OutputFormatKind::Text => print_text(&result, &report),
        OutputFormatKind::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).expect("serialize")
            );
        }
    }
    Ok(ExitCode::Success)
}

fn build_result(
    input: &str,
    document: &bpmn_model::BpmnDocument,
    report: &bpmn_validate::ValidationReport,
) -> InspectResult {
    let mut nodes_by_kind = BTreeMap::new();
    let mut sequence_flows = 0;
    let mut participants = 0;
    let mut lanes = 0;
    let mut text_annotations = 0;
    for root in &document.definitions.root_elements {
        match root {
            RootElement::Process(process) => {
                for node in process.walk_nodes() {
                    *nodes_by_kind
                        .entry(kind_name(&node.kind).to_string())
                        .or_insert(0) += 1;
                }
                sequence_flows += process.walk_flows().len();
                for lane_set in &process.lane_sets {
                    lanes += lane_set.lanes.len();
                }
                text_annotations += process.artifacts.len();
            }
            RootElement::Collaboration(collaboration) => {
                participants += collaboration.participants.len();
                // Message flows counted separately below.
            }
        }
    }
    let message_flows: usize = document
        .collaborations()
        .map(|c| c.message_flows.len())
        .sum();
    let mut diagnostics: Vec<crate::cli::DiagnosticDto> = document
        .parse_diagnostics
        .iter()
        .map(crate::cli::DiagnosticDto::from)
        .collect();
    diagnostics.extend(
        report
            .diagnostics
            .iter()
            .map(crate::cli::DiagnosticDto::from),
    );

    InspectResult {
        contract_version: CONTRACT_VERSION,
        command: "inspect",
        input: input.to_string(),
        definitions_id: document.definitions.id.clone(),
        diagram_count: document.diagram_count(),
        processes: document.processes().count(),
        collaborations: document.collaborations().count(),
        participants,
        lanes,
        nodes_by_kind,
        sequence_flows,
        message_flows,
        text_annotations,
        diagnostics,
    }
}

fn kind_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::StartEvent { .. } => "startEvent",
        NodeKind::EndEvent { .. } => "endEvent",
        NodeKind::IntermediateCatchEvent { .. } => "intermediateCatchEvent",
        NodeKind::IntermediateThrowEvent { .. } => "intermediateThrowEvent",
        NodeKind::BoundaryEvent { .. } => "boundaryEvent",
        NodeKind::Task { .. } => "task",
        NodeKind::Gateway { .. } => "gateway",
        NodeKind::SubProcess { .. } => "subProcess",
        NodeKind::Other { .. } => "other",
    }
}

fn print_text(result: &InspectResult, report: &bpmn_validate::ValidationReport) {
    println!(
        "definitions: {}",
        result.definitions_id.as_deref().unwrap_or("(no id)")
    );
    println!("diagrams:    {}", result.diagram_count);
    println!("processes:   {}", result.processes);
    println!("collaborations: {}", result.collaborations);
    println!("participants: {}", result.participants);
    println!("lanes:       {}", result.lanes);
    println!(
        "flows:       {} sequence, {} message",
        result.sequence_flows, result.message_flows
    );
    println!("annotations: {}", result.text_annotations);
    if result.nodes_by_kind.is_empty() {
        println!("nodes:       (none)");
    } else {
        for (kind, count) in &result.nodes_by_kind {
            println!("nodes:       {kind} x{count}");
        }
    }
    println!(
        "validation:  {} error(s), {} warning(s)",
        report.error_count(),
        report.warning_count()
    );
}

/// Formats a number for the render summary (integer-like, trimmed).
pub(crate) fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_trims_integers() {
        assert_eq!(format_number(256.0), "256");
        assert_eq!(format_number(256.5), "256.50");
    }
}
