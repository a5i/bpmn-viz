//! XML namespace constants and namespace-aware helpers.
//!
//! Identification never relies on literal prefixes (`bpmn:`, `bpmndi:`,
//! `dc:`, `di:`); every element and attribute is resolved to its namespace
//! URI (SPEC-001 section 4, AC-01).

/// BPMN 2.0 semantics namespace (`bpmn:` in most documents).
pub const BPMN_MODEL: &str = "http://www.omg.org/spec/BPMN/20100524/MODEL";

/// BPMN Diagram Interchange namespace (`bpmndi:` in most documents).
pub const BPMN_DI: &str = "http://www.omg.org/spec/BPMN/20100524/DI";

/// Diagram Common namespace (`dc:` in most documents).
pub const DC: &str = "http://www.omg.org/spec/DD/20100524/DC";

/// Diagram Interchange namespace (`di:` in most documents).
pub const DI: &str = "http://www.omg.org/spec/DD/20100524/DI";

/// XML Schema instance namespace (`xsi:` in most documents).
pub const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_are_the_official_20100524_values() {
        assert!(BPMN_MODEL.ends_with("/BPMN/20100524/MODEL"));
        assert!(BPMN_DI.ends_with("/BPMN/20100524/DI"));
        assert!(DC.ends_with("/DD/20100524/DC"));
        assert!(DI.ends_with("/DD/20100524/DI"));
    }
}
