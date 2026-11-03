//! `bpmn validate` implementation: structured diagnostics in text or JSON.

use serde::Serialize;

use crate::cli::{CliError, ExitCode};
use crate::{OutputFormatKind, ValidateCommand};

/// Version bump for result envelope changes.
const CONTRACT_VERSION: u32 = 1;

/// JSON validation result.
#[derive(Serialize)]
pub struct ValidateResult {
    /// Envelope version.
    pub contract_version: u32,
    /// Command name.
    pub command: &'static str,
    /// Input path as given.
    pub input: String,
    /// Number of error diagnostics.
    pub error_count: usize,
    /// Number of warning diagnostics.
    pub warning_count: usize,
    /// All diagnostics sorted deterministically.
    pub diagnostics: Vec<crate::cli::DiagnosticDto>,
}

pub(crate) fn run(
    command: &ValidateCommand,
    format: OutputFormatKind,
) -> Result<ExitCode, CliError> {
    let document = crate::commands::load_document(&command.input)?;
    let report = bpmn_validate::validate(&document);
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
    let error_count = diagnostics.iter().filter(|d| d.severity == "error").count();
    let warning_count = diagnostics
        .iter()
        .filter(|d| d.severity == "warning")
        .count();

    let result = ValidateResult {
        contract_version: CONTRACT_VERSION,
        command: "validate",
        input: command.input.display().to_string(),
        error_count,
        warning_count,
        diagnostics,
    };

    match format {
        OutputFormatKind::Text => {
            for diagnostic in &result.diagnostics {
                println!(
                    "{}[{}]: {}{}",
                    diagnostic.severity,
                    diagnostic.code,
                    diagnostic.message,
                    diagnostic
                        .element_id
                        .as_ref()
                        .map(|id| format!(" (element {id})"))
                        .unwrap_or_default(),
                );
            }
            println!(
                "{}: {} error(s), {} warning(s)",
                command.input.display(),
                result.error_count,
                result.warning_count
            );
        }
        OutputFormatKind::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).expect("serialize")
            );
        }
    }
    // Invalid documents still print their full report; the caller decides by
    // exit code. --strict promotes warnings to failures.
    let failing = result.error_count > 0 || (command.strict && result.warning_count > 0);
    if failing {
        return Ok(ExitCode::InvalidBpmn);
    }
    Ok(ExitCode::Success)
}
