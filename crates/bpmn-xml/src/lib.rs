//! Namespace-aware BPMN 2.0 XML import.
//!
//! Entry points: [`from_xml`] and [`from_xml_with_options`]. Parsing is
//! namespace-URI based (never prefix based), bounded, and free of entity
//! expansion; see `parser` for details and `namespaces` for the recognized
//! URIs.

pub mod namespaces;
pub mod parser;

pub use namespaces::{BPMN_DI, BPMN_MODEL, DC, DI, XSI};
pub use parser::{from_xml, from_xml_with_options, ParseError, ParseOptions};

#[cfg(test)]
mod tests {
    #[test]
    fn public_api_is_reachable() {
        let bytes = br#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"/>"#;
        let document = crate::from_xml(bytes).expect("empty definitions parse");
        assert_eq!(document.diagram_count(), 0);
    }
}
