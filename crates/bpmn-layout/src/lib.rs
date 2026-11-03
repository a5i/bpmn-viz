//! Deterministic BPMN-aware automatic layout producing typed BPMN DI.
//!
//! [`NativeBpmnLayout`] consumes a semantic [`BpmnDocument`] and a selected
//! [`ScopeId`], and generates a [`bpmn_model::di::Diagram`] with shape bounds
//! and orthogonal edge waypoints plus ordered, typed warnings. The semantic
//! model is never mutated (ADR-0007). The algorithm is a BPMN-specific layered
//! layout (left-to-right ranks, semantic vertical bands, orthogonal routing)
//! mirroring the behavior contract of the pinned `bpmn-io/bpmn-auto-layout`
//! 2.0.0-alpha.2 reference; see `docs/experiments/EXP-004-reference-layout.md`
//! and ADR-0006.
//!
//! # Example
//!
//! ```
//! use bpmn_layout::{LayoutEngine, LayoutOptions, NativeBpmnLayout, ScopeId};
//!
//! const XML: &str = r#"
//! <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
//!   <process id="P">
//!     <startEvent id="s"/>
//!     <task id="t" name="Do it"/>
//!     <endEvent id="e"/>
//!     <sequenceFlow id="f1" sourceRef="s" targetRef="t"/>
//!     <sequenceFlow id="f2" sourceRef="t" targetRef="e"/>
//!   </process>
//! </definitions>"#;
//!
//! let document = bpmn_xml::from_xml(XML.as_bytes()).unwrap();
//! let outcome = NativeBpmnLayout::default()
//!     .layout(&document, &ScopeId::process("P"), &LayoutOptions::default())
//!     .unwrap();
//! assert_eq!(outcome.diagram.plane.shapes.len(), 3);
//! assert_eq!(outcome.diagram.plane.edges.len(), 2);
//! assert!(outcome.warnings.is_empty());
//! ```

mod api;
mod di;
mod geometry;
mod graph;
mod placement;
mod routing;
mod semantics;
mod validation;

pub use api::{
    LayoutEngine, LayoutError, LayoutLimits, LayoutOptions, LayoutOutcome, LayoutWarning,
    LayoutWarningCode, NativeBpmnLayout, Orientation, ScopeId,
};
