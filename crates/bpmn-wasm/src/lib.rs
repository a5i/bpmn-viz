//! WASM adapter for the bpmn-viz pipeline (ADR-0009): callers supply all
//! input as BPMN XML text; no filesystem API and no PNG rasterization —
//! `resvg`/`fontdb` stay out of the WASM graph, browsers rasterize the
//! returned SVG on canvas. JSON contracts mirror the Node.js binding and the
//! CLI (`inspect`/`validate` envelopes, deterministic SVG bytes).

#![deny(unsafe_code)]

use std::collections::BTreeMap;

use bpmn_layout::{LayoutEngine, LayoutLimits, LayoutOptions, NativeBpmnLayout, ScopeId};
use bpmn_model::di::{Diagram, Shape};
use bpmn_model::diagnostic::{Diagnostic, Severity};
use bpmn_model::element::{NodeKind, RootElement};
use bpmn_render::RenderOptions;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Envelope version for inspect results (kept in sync with `bpmn inspect`).
const CONTRACT_VERSION: u32 = 1;

fn error(e: impl std::fmt::Display) -> JsValue {
    // Construct a real JS Error, rather than throwing a string.
    wasm_bindgen::JsError::new(&e.to_string()).into()
}

fn to_js(value: &impl Serialize) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(error)
}

fn from_js<T: serde::de::DeserializeOwned + Default>(value: JsValue) -> Result<T, JsValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(T::default());
    }
    serde_wasm_bindgen::from_value(value).map_err(error)
}

// ---------------------------------------------------------------------------
// JSON contracts (camelCase; mirrored by bpmn-node and `index.d.ts`).
// ---------------------------------------------------------------------------

fn diagnostic_to_dto(diagnostic: &Diagnostic) -> DiagnosticDto {
    DiagnosticDto {
        code: diagnostic.code.as_str().to_string(),
        severity: diagnostic.severity.as_str().to_string(),
        message: diagnostic.message.clone(),
        element_id: diagnostic.element_id.clone(),
        line: diagnostic.location.map(|l| l.line as u32),
        column: diagnostic.location.map(|l| l.column as u32),
    }
}

fn layout_warning_to_dto(warning: &bpmn_layout::LayoutWarning) -> DiagnosticDto {
    DiagnosticDto {
        code: warning.code.as_str().to_string(),
        severity: Severity::Warning.as_str().to_string(),
        message: warning.message.clone(),
        element_id: warning.element_id.clone(),
        line: None,
        column: None,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticDto {
    code: String,
    severity: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    element_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectResult {
    contract_version: u32,
    definitions_id: Option<String>,
    diagram_count: u32,
    processes: u32,
    collaborations: u32,
    participants: u32,
    lanes: u32,
    nodes_by_kind: BTreeMap<String, u32>,
    sequence_flows: u32,
    message_flows: u32,
    text_annotations: u32,
    diagnostics: Vec<DiagnosticDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidationReportDto {
    error_count: u32,
    warning_count: u32,
    diagnostics: Vec<DiagnosticDto>,
}

#[derive(Default, Deserialize)]
// Note: serde-wasm-bindgen cannot enforce deny_unknown_fields; option
// strictness is the TypeScript contract's job.
#[serde(default, rename_all = "camelCase")]
struct RenderOptionsJson {
    diagram: Option<String>,
    diagram_index: Option<u32>,
    font_family: Option<String>,
    font_size: Option<f64>,
    padding: Option<f64>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LayoutOptionsJson {
    process: Option<String>,
    rank_gap: Option<f64>,
    node_gap: Option<f64>,
    routing_clearance: Option<f64>,
    max_nodes: Option<f64>,
    max_flows: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SvgResult {
    svg: String,
    width: f64,
    height: f64,
    warnings: Vec<DiagnosticDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundsDto {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PointDto {
    x: f64,
    y: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ShapeDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    bpmn_element: String,
    bounds: BoundsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_horizontal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_expanded: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EdgeDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    bpmn_element: String,
    waypoints: Vec<PointDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagramDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    shapes: Vec<ShapeDto>,
    edges: Vec<EdgeDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LayoutResult {
    diagram: DiagramDto,
    warnings: Vec<DiagnosticDto>,
}

// ---------------------------------------------------------------------------
// Core helpers (mirrored by bpmn-node; kept per-adapter on purpose).
// ---------------------------------------------------------------------------

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

fn select_diagram<'a>(
    document: &'a bpmn_model::BpmnDocument,
    options: &RenderOptionsJson,
) -> Result<&'a Diagram, JsValue> {
    if let Some(id) = &options.diagram {
        return document
            .diagram_by_id(id)
            .ok_or_else(|| error(format!("no diagram with id `{id}`")));
    }
    if let Some(index) = options.diagram_index {
        return document.diagram_by_index(index as usize).ok_or_else(|| {
            error(format!(
                "diagram index {index} out of range (document has {} diagrams)",
                document.diagram_count()
            ))
        });
    }
    document
        .first_diagram()
        .ok_or_else(|| error("document contains no diagram to render"))
}

fn render_options(options: &RenderOptionsJson) -> RenderOptions {
    RenderOptions {
        font_family: options
            .font_family
            .clone()
            .unwrap_or_else(|| "sans-serif".to_string()),
        font_size: options.font_size.unwrap_or(12.0),
        padding: options.padding.unwrap_or(10.0),
        ..RenderOptions::default()
    }
}

fn render_svg(
    document: &bpmn_model::BpmnDocument,
    diagram: &Diagram,
    options: &RenderOptionsJson,
) -> Result<SvgResult, JsValue> {
    let rendered =
        bpmn_render::render_diagram(document, diagram, &render_options(options)).map_err(error)?;
    let viewport = rendered.scene.viewport;
    Ok(SvgResult {
        svg: bpmn_svg::to_svg(&rendered.scene),
        width: viewport.width,
        height: viewport.height,
        warnings: document
            .parse_diagnostics
            .iter()
            .chain(rendered.diagnostics.iter())
            .map(diagnostic_to_dto)
            .collect(),
    })
}

fn layout_options(options: &LayoutOptionsJson) -> LayoutOptions {
    let defaults = LayoutOptions::default();
    LayoutOptions {
        orientation: defaults.orientation,
        rank_gap: options.rank_gap.unwrap_or(defaults.rank_gap),
        node_gap: options.node_gap.unwrap_or(defaults.node_gap),
        routing_clearance: options
            .routing_clearance
            .unwrap_or(defaults.routing_clearance),
        deterministic: true,
        resource_limits: LayoutLimits {
            max_nodes: options
                .max_nodes
                .map(|value| value as usize)
                .unwrap_or(defaults.resource_limits.max_nodes),
            max_flows: options
                .max_flows
                .map(|value| value as usize)
                .unwrap_or(defaults.resource_limits.max_flows),
        },
    }
}

fn resolve_scope(
    document: &bpmn_model::BpmnDocument,
    process: Option<&str>,
) -> Result<ScopeId, JsValue> {
    match process {
        Some(id) => Ok(ScopeId::process(id)),
        None => document
            .processes()
            .find_map(|process| process.id.as_deref())
            .map(ScopeId::process)
            .ok_or_else(|| error("document contains no process with an id to lay out")),
    }
}

fn layout(
    document: &bpmn_model::BpmnDocument,
    options: &LayoutOptionsJson,
) -> Result<bpmn_layout::LayoutOutcome, JsValue> {
    let scope = resolve_scope(document, options.process.as_deref())?;
    NativeBpmnLayout
        .layout(document, &scope, &layout_options(options))
        .map_err(error)
}

fn diagram_to_dto(diagram: &Diagram) -> DiagramDto {
    DiagramDto {
        id: diagram.id.clone(),
        name: diagram.name.clone(),
        shapes: diagram
            .plane
            .shapes
            .iter()
            .map(|shape: &Shape| ShapeDto {
                id: shape.id.clone(),
                bpmn_element: shape.bpmn_element.clone(),
                bounds: BoundsDto {
                    x: shape.bounds.x,
                    y: shape.bounds.y,
                    width: shape.bounds.width,
                    height: shape.bounds.height,
                },
                is_horizontal: shape.is_horizontal,
                is_expanded: shape.is_expanded,
            })
            .collect(),
        edges: diagram
            .plane
            .edges
            .iter()
            .map(|edge| EdgeDto {
                id: edge.id.clone(),
                bpmn_element: edge.bpmn_element.clone(),
                waypoints: edge
                    .waypoints
                    .iter()
                    .map(|point| PointDto {
                        x: point.x,
                        y: point.y,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn inspect(
    document: &bpmn_model::BpmnDocument,
    report: &bpmn_validate::ValidationReport,
) -> InspectResult {
    let mut nodes_by_kind: BTreeMap<String, u32> = BTreeMap::new();
    let mut sequence_flows = 0u32;
    let mut participants = 0u32;
    let mut lanes = 0u32;
    let mut text_annotations = 0u32;
    for root in &document.definitions.root_elements {
        match root {
            RootElement::Process(process) => {
                for node in process.walk_nodes() {
                    *nodes_by_kind
                        .entry(kind_name(&node.kind).to_string())
                        .or_insert(0) += 1;
                }
                sequence_flows += process.walk_flows().len() as u32;
                for lane_set in &process.lane_sets {
                    lanes += lane_set.lanes.len() as u32;
                }
                text_annotations += process.artifacts.len() as u32;
            }
            RootElement::Collaboration(collaboration) => {
                participants += collaboration.participants.len() as u32;
            }
        }
    }
    let message_flows: u32 = document
        .collaborations()
        .map(|collaboration| collaboration.message_flows.len() as u32)
        .sum();
    let mut diagnostics: Vec<DiagnosticDto> = document
        .parse_diagnostics
        .iter()
        .map(diagnostic_to_dto)
        .collect();
    diagnostics.extend(report.diagnostics.iter().map(diagnostic_to_dto));
    InspectResult {
        contract_version: CONTRACT_VERSION,
        definitions_id: document.definitions.id.clone(),
        diagram_count: document.diagram_count() as u32,
        processes: document.processes().count() as u32,
        collaborations: document.collaborations().count() as u32,
        participants,
        lanes,
        nodes_by_kind,
        sequence_flows,
        message_flows,
        text_annotations,
        diagnostics,
    }
}

// ---------------------------------------------------------------------------
// Exported class.
// ---------------------------------------------------------------------------

/// Reusable parsed BPMN document: validate, inspect, render SVG, auto-layout.
#[wasm_bindgen(js_name = BpmnDocument)]
pub struct WasmDocument {
    inner: bpmn_model::BpmnDocument,
}

#[wasm_bindgen(js_class = BpmnDocument)]
impl WasmDocument {
    /// Parses BPMN 2.0 XML text; throws a JS `Error` carrying the parse
    /// diagnostic on malformed input.
    #[wasm_bindgen(constructor)]
    pub fn new(xml: &str) -> Result<WasmDocument, JsValue> {
        let inner = bpmn_xml::from_xml(xml.as_bytes()).map_err(error)?;
        Ok(Self { inner })
    }

    /// Number of diagrams in source order.
    #[wasm_bindgen(getter, js_name = diagramCount)]
    pub fn diagram_count(&self) -> u32 {
        self.inner.diagram_count() as u32
    }

    /// All diagram ids in source order (diagrams without an id are skipped).
    #[wasm_bindgen(js_name = diagramIds)]
    pub fn diagram_ids(&self) -> Vec<JsValue> {
        self.inner
            .definitions
            .diagrams
            .iter()
            .filter_map(|diagram| diagram.id.clone())
            .map(JsValue::from)
            .collect()
    }

    /// Document summary, same contract as `bpmn inspect --format json`.
    pub fn inspect(&self) -> Result<JsValue, JsValue> {
        let report = bpmn_validate::validate(&self.inner);
        to_js(&inspect(&self.inner, &report))
    }

    /// Validation report (errors, warnings, ordered diagnostics).
    pub fn validate(&self) -> Result<JsValue, JsValue> {
        let report = bpmn_validate::validate(&self.inner);
        to_js(&ValidationReportDto {
            error_count: report.error_count() as u32,
            warning_count: report.warning_count() as u32,
            diagnostics: report.diagnostics.iter().map(diagnostic_to_dto).collect(),
        })
    }

    /// Renders the selected (default: first) diagram to deterministic SVG.
    #[wasm_bindgen(js_name = renderSvg)]
    pub fn render_svg(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options: RenderOptionsJson = from_js(options)?;
        let diagram = select_diagram(&self.inner, &options)?;
        to_js(&render_svg(&self.inner, diagram, &options)?)
    }

    /// Runs auto-layout and returns the generated DI as JSON (ADR-0007).
    pub fn layout(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options: LayoutOptionsJson = from_js(options)?;
        let outcome = layout(&self.inner, &options)?;
        to_js(&LayoutResult {
            diagram: diagram_to_dto(&outcome.diagram),
            warnings: outcome.warnings.iter().map(layout_warning_to_dto).collect(),
        })
    }

    /// Lays out the selected process, then renders the generated DI to SVG
    /// (the no-DI path: models without `bpmndi` geometry stay renderable).
    #[wasm_bindgen(js_name = renderLayoutSvg)]
    pub fn render_layout_svg(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options: LayoutOptionsJson = from_js(options)?;
        let outcome = layout(&self.inner, &options)?;
        let rendered =
            bpmn_render::render_diagram(&self.inner, &outcome.diagram, &RenderOptions::default())
                .map_err(error)?;
        let viewport = rendered.scene.viewport;
        let mut warnings: Vec<DiagnosticDto> = self
            .inner
            .parse_diagnostics
            .iter()
            .map(diagnostic_to_dto)
            .collect();
        warnings.extend(outcome.warnings.iter().map(layout_warning_to_dto));
        warnings.extend(rendered.diagnostics.iter().map(diagnostic_to_dto));
        to_js(&SvgResult {
            svg: bpmn_svg::to_svg(&rendered.scene),
            width: viewport.width,
            height: viewport.height,
            warnings,
        })
    }
}
