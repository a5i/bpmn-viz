//! Structural and geometry validation for BPMN documents.
//!
//! Entry point: [`validate`]. Produces a [`ValidationReport`] of structured
//! diagnostics covering duplicate ids, dangling references, invalid
//! geometry, and missing Diagram Interchange (ADR-0003: no auto-layout —
//! a missing shape is an error, never a fallback).

pub mod geometry;
pub mod reference;

use bpmn_model::diagnostic::{Diagnostic, Severity};
use bpmn_model::BpmnDocument;

/// Result of validating one document.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// All diagnostics sorted by (severity, code, element id, message).
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    /// True when at least one diagnostic is an error.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Number of error-severity diagnostics.
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }

    /// Number of warning-severity diagnostics.
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// Diagnostics filtered to errors only.
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|d| d.is_error())
    }

    /// Diagnostics filtered to warnings only.
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
    }

    fn sort(&mut self) {
        self.diagnostics.sort_by(|a, b| {
            (a.severity.as_str(), a.code.as_str(), &a.element_id, &a.message).cmp(&(
                b.severity.as_str(),
                b.code.as_str(),
                &b.element_id,
                &b.message,
            ))
        });
    }
}

/// Validates references and geometry of `document`.
pub fn validate(document: &BpmnDocument) -> ValidationReport {
    let mut diagnostics = reference::check_references(document);
    diagnostics.extend(geometry::check_geometry(document));
    let mut report = ValidationReport { diagnostics };
    report.sort();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpmn_xml::from_xml;

    fn validate_xml(xml: &str) -> ValidationReport {
        let document = from_xml(xml.as_bytes()).expect("fixture must parse");
        validate(&document)
    }

    #[test]
    fn detects_duplicate_ids() {
        let report = validate_xml(
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="P"><task id="X"/><task id="X"/></process>
</definitions>"#,
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "reference.duplicate_id" && d.element_id.as_deref() == Some("X")));
    }

    #[test]
    fn detects_dangling_sequence_flow_refs() {
        let report = validate_xml(
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="P">
    <task id="A"/>
    <sequenceFlow id="F" sourceRef="A" targetRef="Ghost"/>
  </process>
</definitions>"#,
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "reference.dangling"
                && d.element_id.as_deref() == Some("F")));
    }

    #[test]
    fn detects_dangling_participant_process_ref() {
        let report = validate_xml(
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <collaboration id="C"><participant id="Pool" processRef="Nope"/></collaboration>
</definitions>"#,
        );
        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "reference.dangling"
                && d.element_id.as_deref() == Some("Pool")));
    }

    #[test]
    fn detects_missing_diagram_interchange() {
        let report = validate_xml(
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="P"><task id="A"/></process>
</definitions>"#,
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "di.missing_interchange"));
    }

    #[test]
    fn detects_node_without_shape() {
        let report = validate_xml(
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                 xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
                 xmlns:dc="http://www.omg.org/spec/DD/20100524/DC">
  <process id="P"><task id="A"/><task id="B"/></process>
  <bpmndi:BPMNDiagram id="D"><bpmndi:BPMNPlane bpmnElement="P">
    <bpmndi:BPMNShape id="sA" bpmnElement="A"><dc:Bounds x="0" y="0" width="100" height="80"/></bpmndi:BPMNShape>
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#,
        );
        assert!(report.diagnostics.iter().any(|d| d.code.as_str() == "di.missing_shape"
            && d.element_id.as_deref() == Some("B")));
        assert!(!report.diagnostics.iter().any(|d| d.element_id.as_deref() == Some("A")));
    }

    #[test]
    fn detects_nan_bounds_and_short_edges() {
        let report = validate_xml(
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                 xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
                 xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
                 xmlns:di="http://www.omg.org/spec/DD/20100524/DI">
  <process id="P"><task id="A"/><sequenceFlow id="F" sourceRef="A" targetRef="A"/></process>
  <bpmndi:BPMNDiagram id="D"><bpmndi:BPMNPlane bpmnElement="P">
    <bpmndi:BPMNShape id="sA" bpmnElement="A"><dc:Bounds x="NaN" y="0" width="100" height="80"/></bpmndi:BPMNShape>
    <bpmndi:BPMNEdge id="eF" bpmnElement="F"><di:waypoint x="1" y="1"/></bpmndi:BPMNEdge>
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#,
        );
        assert!(report.diagnostics.iter().any(|d| d.code.as_str() == "geometry.invalid"));
        assert!(report.diagnostics.iter().any(|d| d.code.as_str() == "di.missing_waypoints"));
    }

    #[test]
    fn detects_dangling_di_references() {
        let report = validate_xml(
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                 xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
                 xmlns:dc="http://www.omg.org/spec/DD/20100524/DC">
  <process id="P"><task id="A"/></process>
  <bpmndi:BPMNDiagram id="D"><bpmndi:BPMNPlane bpmnElement="P">
    <bpmndi:BPMNShape id="sGhost" bpmnElement="Ghost"><dc:Bounds x="0" y="0" width="10" height="10"/></bpmndi:BPMNShape>
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#,
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "di.dangling_element_ref"
                && d.element_id.as_deref() == Some("Ghost")));
    }

    #[test]
    fn collapsed_subprocess_children_are_exempt_from_shape_requirement() {
        let report = validate_xml(
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                 xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
                 xmlns:dc="http://www.omg.org/spec/DD/20100524/DC">
  <process id="P">
    <subProcess id="Sub"><task id="Inner"/></subProcess>
  </process>
  <bpmndi:BPMNDiagram id="D"><bpmndi:BPMNPlane bpmnElement="P">
    <bpmndi:BPMNShape id="sSub" bpmnElement="Sub" isExpanded="false"><dc:Bounds x="0" y="0" width="50" height="50"/></bpmndi:BPMNShape>
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#,
        );
        assert!(!report.diagnostics.iter().any(|d| d.element_id.as_deref() == Some("Inner")));
    }
}
