//! Command implementations.

pub mod inspect;
pub mod render;
pub mod validate;

use std::path::Path;

use bpmn_model::BpmnDocument;
use bpmn_validate::ValidationReport;
use bpmn_xml::ParseError;

use crate::cli::CliError;

/// Parses a BPMN file, mapping fatal parse failures to exit code 3.
pub(crate) fn load_document(input: &Path) -> Result<BpmnDocument, CliError> {
    let bytes = crate::cli::read_input(input)?;
    bpmn_xml::from_xml(&bytes)
        .map_err(|error: ParseError| CliError::InvalidBpmn(vec![error.diagnostic]))
}

/// Parses and validates; errors map to exit code 3 with all diagnostics.
pub(crate) fn load_and_validate(
    input: &Path,
) -> Result<(BpmnDocument, ValidationReport), CliError> {
    let document = load_document(input)?;
    let mut diagnostics = document.parse_diagnostics.clone();
    let report = bpmn_validate::validate(&document);
    if report.has_errors() {
        diagnostics.extend(report.diagnostics.iter().cloned());
        return Err(CliError::InvalidBpmn(diagnostics));
    }
    Ok((document, report))
}
