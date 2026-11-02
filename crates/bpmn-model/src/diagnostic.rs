//! Stable diagnostic codes, severities and source locations.
//!
//! Diagnostics are the common currency of every pipeline stage (SPEC-001
//! section 4): parsing errors, conformance warnings, missing-geometry
//! failures and unsupported-symbol warnings are distinguishable by
//! [`DiagnosticCode`] and [`Severity`]. Codes are stable strings; never
//! renumber a published code.

use std::fmt;

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The requested operation cannot continue or produced a wrong model.
    Error,
    /// The model is tolerated but the output may be incomplete or unusual.
    Warning,
}

impl Severity {
    /// Stable lowercase spelling used in JSON output.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Machine-readable diagnostic codes with stable string spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    /// XML is not well-formed.
    XmlSyntax,
    /// XML uses an encoding the parser rejects.
    XmlEncoding,
    /// A namespace prefix could not be resolved.
    XmlUnresolvedPrefix,
    /// Nesting depth exceeded the configured safety limit.
    XmlDepthLimit,
    /// Input exceeded the configured size limit.
    XmlSizeLimit,
    /// The root element is not `definitions` in the BPMN namespace.
    DefinitionsMissingRoot,
    /// The same id is declared more than once.
    DuplicateId,
    /// A reference points to an id that does not exist.
    DanglingReference,
    /// A reference resolves to an element outside its expected scope.
    ForeignScopeReference,
    /// Bounds or waypoints are not finite or not usable.
    InvalidGeometry,
    /// A rendered element has no DI shape (ADR-0003: never auto-layout).
    MissingShape,
    /// An edge lacks usable waypoints.
    MissingWaypoints,
    /// DI references a semantic element that does not exist.
    DanglingDiReference,
    /// No `BPMNDiagram` exists at all.
    MissingDiagramInterchange,
    /// The requested diagram does not exist.
    DiagramNotFound,
    /// A semantic symbol has no dedicated glyph and is drawn with a fallback.
    UnsupportedSymbol,
    /// An element outside the supported subset was ignored during import.
    IgnoredElement,
}

impl DiagnosticCode {
    /// Stable dotted code spelling (for example `reference.dangling`).
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticCode::XmlSyntax => "xml.syntax",
            DiagnosticCode::XmlEncoding => "xml.encoding",
            DiagnosticCode::XmlUnresolvedPrefix => "xml.unresolved_prefix",
            DiagnosticCode::XmlDepthLimit => "xml.depth_limit",
            DiagnosticCode::XmlSizeLimit => "xml.size_limit",
            DiagnosticCode::DefinitionsMissingRoot => "definitions.missing_root",
            DiagnosticCode::DuplicateId => "reference.duplicate_id",
            DiagnosticCode::DanglingReference => "reference.dangling",
            DiagnosticCode::ForeignScopeReference => "reference.foreign_scope",
            DiagnosticCode::InvalidGeometry => "geometry.invalid",
            DiagnosticCode::MissingShape => "di.missing_shape",
            DiagnosticCode::MissingWaypoints => "di.missing_waypoints",
            DiagnosticCode::DanglingDiReference => "di.dangling_element_ref",
            DiagnosticCode::MissingDiagramInterchange => "di.missing_interchange",
            DiagnosticCode::DiagramNotFound => "diagram.not_found",
            DiagnosticCode::UnsupportedSymbol => "symbol.unsupported",
            DiagnosticCode::IgnoredElement => "import.ignored_element",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One-based position in the source document, when known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    /// One-based line number.
    pub line: usize,
    /// One-based column (Unicode scalar) position.
    pub column: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A structured diagnostic attached to a pipeline stage result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable machine-readable code.
    pub code: DiagnosticCode,
    /// Severity classification.
    pub severity: Severity,
    /// Human-readable explanation.
    pub message: String,
    /// Semantic or DI element id the diagnostic is about, when known.
    pub element_id: Option<String>,
    /// Source position in the original XML, when known.
    pub location: Option<SourceLocation>,
}

impl Diagnostic {
    /// Convenience constructor for errors.
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            element_id: None,
            location: None,
        }
    }

    /// Convenience constructor for warnings.
    pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            element_id: None,
            location: None,
        }
    }

    /// Attach the affected element id.
    pub fn with_element_id(mut self, id: impl Into<String>) -> Self {
        self.element_id = Some(id.into());
        self
    }

    /// Attach the source location.
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// True when the diagnostic is an error.
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]: {}", self.severity, self.code, self.message)?;
        if let Some(id) = &self.element_id {
            write!(f, " (element {id})")?;
        }
        if let Some(location) = &self.location {
            write!(f, " at {location}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_context() {
        let diagnostic = Diagnostic::warning(DiagnosticCode::UnsupportedSymbol, "fallback glyph")
            .with_element_id("Gateway_1")
            .with_location(SourceLocation { line: 7, column: 5 });
        let rendered = diagnostic.to_string();
        assert!(rendered.contains("warning[symbol.unsupported]"));
        assert!(rendered.contains("Gateway_1"));
        assert!(rendered.contains("7:5"));
    }

    #[test]
    fn codes_are_stable() {
        assert_eq!(
            DiagnosticCode::DanglingReference.as_str(),
            "reference.dangling"
        );
        assert_eq!(DiagnosticCode::MissingShape.as_str(), "di.missing_shape");
    }
}
