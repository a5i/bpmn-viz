//! Shared CLI plumbing: exit codes, diagnostics printing and atomic writes.
//!
//! Exit codes (SPEC-001 section 8): 0 success, 2 invalid CLI usage,
//! 3 invalid BPMN/DI, 4 IO failure, 5 render failure.

use bpmn_model::diagnostic::Diagnostic;
use std::fs;
use std::io;
use std::path::Path;

/// Process exit codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Success.
    Success = 0,
    /// Invalid CLI usage.
    Usage = 2,
    /// Invalid BPMN or Diagram Interchange.
    InvalidBpmn = 3,
    /// Filesystem failure.
    Io = 4,
    /// Rendering failure.
    Render = 5,
}

/// High-level CLI failure carrying its exit code.
#[derive(Debug)]
pub enum CliError {
    /// Usage or argument conflict.
    Usage(String),
    /// Invalid BPMN/DI (parse or validation failure).
    InvalidBpmn(Vec<Diagnostic>),
    /// Filesystem failure.
    Io(String),
    /// Rendering failure.
    Render(String),
}

impl CliError {
    /// The process exit code for this failure.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            CliError::Usage(_) => ExitCode::Usage,
            CliError::InvalidBpmn(_) => ExitCode::InvalidBpmn,
            CliError::Io(_) => ExitCode::Io,
            CliError::Render(_) => ExitCode::Render,
        }
    }

    /// Human-readable summary line for stderr.
    pub fn summary(&self) -> String {
        match self {
            CliError::Usage(message) => message.clone(),
            CliError::InvalidBpmn(diagnostics) => format!(
                "invalid BPMN input ({} diagnostics); details below",
                diagnostics.len()
            ),
            CliError::Io(message) => message.clone(),
            CliError::Render(message) => message.clone(),
        }
    }
}

/// Prints diagnostics as text lines to stderr.
pub fn print_diagnostics_text(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        eprintln!("{diagnostic}");
    }
}

/// JSON diagnostic DTO (stable field names, SPEC-001 section 8).
#[derive(serde::Serialize)]
pub struct DiagnosticDto {
    /// Stable code string.
    pub code: String,
    /// `error` or `warning`.
    pub severity: String,
    /// Human-readable message.
    pub message: String,
    /// Element id when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
    /// Source location when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationDto>,
}

/// JSON source location DTO.
#[derive(serde::Serialize)]
pub struct LocationDto {
    /// One-based line.
    pub line: usize,
    /// One-based column.
    pub column: usize,
}

impl From<&Diagnostic> for DiagnosticDto {
    fn from(diagnostic: &Diagnostic) -> Self {
        Self {
            code: diagnostic.code.as_str().to_string(),
            severity: diagnostic.severity.as_str().to_string(),
            message: diagnostic.message.clone(),
            element_id: diagnostic.element_id.clone(),
            location: diagnostic.location.map(|location| LocationDto {
                line: location.line,
                column: location.column,
            }),
        }
    }
}

/// Reads a file, mapping errors to [`CliError::Io`].
pub fn read_input(path: &Path) -> Result<Vec<u8>, CliError> {
    fs::read(path).map_err(|error| CliError::Io(format!("cannot read {}: {error}", path.display())))
}

/// Writes bytes atomically: temp file in the target directory, then rename.
/// On platforms where rename-over-existing fails, the destination is
/// removed first. `path == "-"` means stdout and is handled by callers.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    if !directory.as_os_str().is_empty() && !directory.exists() {
        return Err(CliError::Io(format!(
            "output directory does not exist: {}",
            directory.display()
        )));
    }
    let file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    let mut temp = directory.to_path_buf();
    temp.push(format!(
        ".{}.tmp-{}",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    fs::write(&temp, bytes)
        .map_err(|error| CliError::Io(format!("cannot write {}: {error}", temp.display())))?;
    if path.exists() {
        // Windows rename cannot replace an existing file.
        if let Err(error) = fs::remove_file(path) {
            let _ = fs::remove_file(&temp);
            return Err(CliError::Io(format!(
                "cannot replace {}: {error}",
                path.display()
            )));
        }
    }
    match fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(CliError::Io(format!(
                "cannot finalize {}: {error}",
                path.display()
            )))
        }
    }
}

/// Writes to stdout, mapping IO errors.
pub fn write_stdout(bytes: &[u8]) -> Result<(), CliError> {
    use io::Write;
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(bytes)
        .and_then(|()| lock.flush())
        .map_err(|error| CliError::Io(format!("cannot write to stdout: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpmn_model::diagnostic::{Diagnostic, DiagnosticCode};

    #[test]
    fn exit_codes_match_spec() {
        assert_eq!(CliError::Usage("x".into()).exit_code() as u32, 2);
        assert_eq!(
            CliError::InvalidBpmn(vec![Diagnostic::error(DiagnosticCode::DuplicateId, "x")])
                .exit_code() as u32,
            3
        );
        assert_eq!(CliError::Io("x".into()).exit_code() as u32, 4);
        assert_eq!(CliError::Render("x".into()).exit_code() as u32, 5);
    }

    #[test]
    fn atomic_write_roundtrip() {
        let directory = std::env::temp_dir().join(format!("bpmn-cli-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let target = directory.join("out.svg");
        write_atomic(&target, b"one").unwrap();
        write_atomic(&target, b"two").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"two");
        // No temp leftovers.
        let leftovers: Vec<_> = fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn diagnostic_dto_serializes_stable_fields() {
        let diagnostic = Diagnostic::error(DiagnosticCode::DuplicateId, "dup")
            .with_element_id("X")
            .with_location(bpmn_model::diagnostic::SourceLocation { line: 3, column: 5 });
        let value = serde_json::to_value(DiagnosticDto::from(&diagnostic)).unwrap();
        assert_eq!(value["code"], "reference.duplicate_id");
        assert_eq!(value["severity"], "error");
        assert_eq!(value["element_id"], "X");
        assert_eq!(value["location"]["line"], 3);
    }
}
