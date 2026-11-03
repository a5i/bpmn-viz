//! Node.js bindings for the bpmn-viz pipeline (napi-rs 3; ADR-0009).
//!
//! Mirrors the audited DocVera adapter shape: owned handles hold
//! `Arc<Mutex<Option<...>>>`, every heavy operation runs on
//! [`tokio::task::spawn_blocking`], and `close()` releases the model
//! explicitly. The binding exposes the full pipeline including PNG export
//! (`bpmn-raster`) and auto-layout (`bpmn-layout`); it never touches the
//! filesystem — callers pass XML text and receive strings, buffers and JSON.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bpmn_layout::{LayoutEngine, LayoutLimits, LayoutOptions, NativeBpmnLayout, ScopeId};
use bpmn_model::di::{Diagram, Shape};
use bpmn_model::diagnostic::{Diagnostic, Severity};
use bpmn_model::element::{NodeKind, RootElement};
use bpmn_raster::PngOptions;
use bpmn_render::{RenderOptions as CoreRenderOptions, RenderedDiagram};
use bpmn_scene::style::Rgba;
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

/// Envelope version for inspect results (kept in sync with `bpmn inspect`).
const CONTRACT_VERSION: u32 = 1;

fn failure(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

/// Runs a CPU-bound job off the JS thread; the inner error is a formatted
/// diagnostic message (this crate has no anyhow dependency).
async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> napi::Result<T> {
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|error| failure(error.to_string()))?
        .map_err(failure)
}

// ---------------------------------------------------------------------------
// JSON-shaped DTOs (camelCase on the JS side; the hand-written contract is
// owned by ADR-0009 and mirrored by `index.d.ts` and the WASM binding).
// ---------------------------------------------------------------------------

/// One parse/validation/render warning or error.
#[napi(object)]
pub struct DiagnosticDto {
    /// Stable machine-readable code.
    pub code: String,
    /// `error` or `warning`.
    pub severity: String,
    /// Human-readable explanation.
    pub message: String,
    /// Semantic or DI element id, when known.
    pub element_id: Option<String>,
    /// One-based source line, when known.
    pub line: Option<u32>,
    /// One-based source column, when known.
    pub column: Option<u32>,
}

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

/// Document summary, same contract as `bpmn inspect --format json`.
#[napi(object)]
pub struct InspectResult {
    /// Envelope version.
    pub contract_version: u32,
    /// `definitions/@id`.
    pub definitions_id: Option<String>,
    /// Number of diagrams in source order.
    pub diagram_count: u32,
    /// Number of processes.
    pub processes: u32,
    /// Number of collaborations.
    pub collaborations: u32,
    /// Number of pools.
    pub participants: u32,
    /// Number of lanes.
    pub lanes: u32,
    /// Flow nodes grouped by kind.
    pub nodes_by_kind: BTreeMap<String, u32>,
    /// Sequence flows.
    pub sequence_flows: u32,
    /// Message flows.
    pub message_flows: u32,
    /// Text annotations (artifact count).
    pub text_annotations: u32,
    /// Parse and validation diagnostics.
    pub diagnostics: Vec<DiagnosticDto>,
}

/// Validation outcome (same diagnostics as `bpmn validate`).
#[napi(object)]
pub struct ValidationReportDto {
    /// Number of error-severity diagnostics.
    pub error_count: u32,
    /// Number of warning-severity diagnostics.
    pub warning_count: u32,
    /// Ordered diagnostics.
    pub diagnostics: Vec<DiagnosticDto>,
}

/// SVG render options; diagram defaults to the first diagram.
#[napi(object)]
pub struct RenderOptionsJs {
    /// Diagram id to render (wins over `diagramIndex`).
    pub diagram: Option<String>,
    /// Zero-based diagram index to render.
    pub diagram_index: Option<u32>,
    /// Font family list for labels.
    pub font_family: Option<String>,
    /// Base label font size in user units.
    pub font_size: Option<f64>,
    /// Viewport padding in user units.
    pub padding: Option<f64>,
}

/// PNG render options; extends [`RenderOptionsJs`] with raster settings.
#[napi(object)]
pub struct PngOptionsJs {
    /// Diagram id to render (wins over `diagramIndex`).
    pub diagram: Option<String>,
    /// Zero-based diagram index to render.
    pub diagram_index: Option<u32>,
    /// Font family list for labels.
    pub font_family: Option<String>,
    /// Base label font size in user units.
    pub font_size: Option<f64>,
    /// Viewport padding in user units.
    pub padding: Option<f64>,
    /// Positive finite scale factor applied to the SVG size.
    pub scale: Option<f64>,
    /// Background color as CSS hex (`#rgb`, `#rrggbb`, `#rrggbbaa`).
    pub background: Option<String>,
    /// Maximum output width in pixels.
    pub max_width: Option<u32>,
    /// Maximum output height in pixels.
    pub max_height: Option<u32>,
    /// Maximum total pixels (width × height).
    pub max_pixels: Option<f64>,
}

/// Auto-layout options (`bpmn-layout`, ADR-0006/0007; left-to-right, M1).
#[napi(object)]
pub struct LayoutOptionsJs {
    /// Process id to lay out; defaults to the first process with an id.
    pub process: Option<String>,
    /// Horizontal distance between rank columns.
    pub rank_gap: Option<f64>,
    /// Vertical distance between nodes in a band row.
    pub node_gap: Option<f64>,
    /// Clearance routed segments keep from shape interiors.
    pub routing_clearance: Option<f64>,
    /// Maximum number of flow nodes in the scope.
    pub max_nodes: Option<u32>,
    /// Maximum number of sequence flows in the scope.
    pub max_flows: Option<u32>,
}

/// SVG render result; `width`/`height` are the viewport user units.
#[napi(object)]
pub struct SvgResult {
    /// Deterministic SVG document.
    pub svg: String,
    /// Viewport width in user units.
    pub width: f64,
    /// Viewport height in user units.
    pub height: f64,
    /// Non-fatal render/import diagnostics.
    pub warnings: Vec<DiagnosticDto>,
}

/// PNG render result.
#[napi(object)]
pub struct PngResult {
    /// Encoded PNG bytes.
    pub png: Buffer,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Non-fatal render/import/rasterization diagnostics.
    pub warnings: Vec<DiagnosticDto>,
}

/// DI bounds DTO.
#[napi(object)]
pub struct BoundsDto {
    /// X coordinate of the top-left corner.
    pub x: f64,
    /// Y coordinate of the top-left corner.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// DI waypoint DTO.
#[napi(object)]
pub struct PointDto {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
}

/// DI shape DTO (generated or imported).
#[napi(object)]
pub struct ShapeDto {
    /// DI shape id, when present.
    pub id: Option<String>,
    /// Semantic element id this shape provides geometry for.
    pub bpmn_element: String,
    /// Shape bounds.
    pub bounds: BoundsDto,
    /// `isHorizontal`, relevant for pools and lanes.
    pub is_horizontal: Option<bool>,
    /// `isExpanded`, relevant for subprocesses.
    pub is_expanded: Option<bool>,
}

/// DI edge DTO (generated or imported).
#[napi(object)]
pub struct EdgeDto {
    /// DI edge id, when present.
    pub id: Option<String>,
    /// Semantic element id this edge provides geometry for.
    pub bpmn_element: String,
    /// Ordered waypoints.
    pub waypoints: Vec<PointDto>,
}

/// DI diagram DTO.
#[napi(object)]
pub struct DiagramDto {
    /// Diagram id, when present.
    pub id: Option<String>,
    /// Display name, when present.
    pub name: Option<String>,
    /// Plane shapes in document order.
    pub shapes: Vec<ShapeDto>,
    /// Plane edges in document order.
    pub edges: Vec<EdgeDto>,
}

/// Auto-layout outcome: generated DI plus ordered warnings (ADR-0007).
#[napi(object)]
pub struct LayoutResult {
    /// Generated diagram.
    pub diagram: DiagramDto,
    /// Non-fatal layout notes.
    pub warnings: Vec<DiagnosticDto>,
}

// ---------------------------------------------------------------------------
// Core helpers shared by the handle methods and the module-level one-shots.
// ---------------------------------------------------------------------------

fn parse_document(xml: &str) -> Result<bpmn_model::BpmnDocument, String> {
    bpmn_xml::from_xml(xml.as_bytes()).map_err(|error| error.to_string())
}

/// Resolves the selected diagram: `diagram` id first, then `diagram_index`,
/// else the first diagram in source order.
fn select_diagram<'a>(
    document: &'a bpmn_model::BpmnDocument,
    diagram: Option<&str>,
    diagram_index: Option<u32>,
) -> Result<&'a Diagram, String> {
    if let Some(id) = diagram {
        return document
            .diagram_by_id(id)
            .ok_or_else(|| format!("no diagram with id `{id}`"));
    }
    if let Some(index) = diagram_index {
        return document.diagram_by_index(index as usize).ok_or_else(|| {
            format!(
                "diagram index {index} out of range (document has {} diagrams)",
                document.diagram_count()
            )
        });
    }
    document
        .first_diagram()
        .ok_or_else(|| "document contains no diagram to render".to_string())
}

fn core_render_options(options: &RenderOptionsJs) -> CoreRenderOptions {
    CoreRenderOptions {
        theme: bpmn_render::Theme::default(),
        font_family: options
            .font_family
            .clone()
            .unwrap_or_else(|| "sans-serif".to_string()),
        font_size: options.font_size.unwrap_or(12.0),
        padding: options.padding.unwrap_or(10.0),
    }
}

fn render_core(
    document: &bpmn_model::BpmnDocument,
    diagram: &Diagram,
    options: &RenderOptionsJs,
) -> Result<RenderedDiagram, String> {
    bpmn_render::render_diagram(document, diagram, &core_render_options(options))
        .map_err(|error| error.to_string())
}

/// Parses CSS hex colors (`#rgb`, `#rrggbb`, `#rrggbbaa`); alpha defaults to
/// opaque, matching `PngOptions::default()`.
fn parse_color(value: &str) -> Result<Rgba, String> {
    let hex = value
        .strip_prefix('#')
        .ok_or_else(|| format!("background must be CSS hex like `#rrggbb`, got `{value}`"))?;
    let parse_channel = |text: &str| {
        u8::from_str_radix(text, 16)
            .map_err(|_| format!("background has an invalid hex channel: `{value}`"))
    };
    match hex.len() {
        3 => {
            let bytes: Vec<char> = hex.chars().collect();
            Ok(Rgba::with_alpha(
                parse_channel(&bytes[0].to_string())?,
                parse_channel(&bytes[1].to_string())?,
                parse_channel(&bytes[2].to_string())?,
                255,
            ))
        }
        6 => Ok(Rgba::with_alpha(
            parse_channel(&hex[0..2])?,
            parse_channel(&hex[2..4])?,
            parse_channel(&hex[4..6])?,
            255,
        )),
        8 => Ok(Rgba::with_alpha(
            parse_channel(&hex[0..2])?,
            parse_channel(&hex[2..4])?,
            parse_channel(&hex[4..6])?,
            parse_channel(&hex[6..8])?,
        )),
        _ => Err(format!(
            "background must be #rgb, #rrggbb or #rrggbbaa, got `{value}`"
        )),
    }
}

fn png_options(options: &PngOptionsJs) -> Result<PngOptions, String> {
    let defaults = PngOptions::default();
    Ok(PngOptions {
        scale: options.scale.unwrap_or(defaults.scale),
        background: match &options.background {
            Some(value) => parse_color(value)?,
            None => defaults.background,
        },
        max_width: options.max_width.unwrap_or(defaults.max_width),
        max_height: options.max_height.unwrap_or(defaults.max_height),
        max_pixels: options.max_pixels.unwrap_or(defaults.max_pixels as f64) as u64,
    })
}

fn png_options_of_render(options: &PngOptionsJs) -> RenderOptionsJs {
    RenderOptionsJs {
        diagram: options.diagram.clone(),
        diagram_index: options.diagram_index,
        font_family: options.font_family.clone(),
        font_size: options.font_size,
        padding: options.padding,
    }
}

fn layout_options(options: &LayoutOptionsJs) -> LayoutOptions {
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

/// Resolves the layout scope: explicit process id, else the first process
/// carrying an id (the DI plane binds to a process id, ADR-0007).
fn resolve_scope(
    document: &bpmn_model::BpmnDocument,
    process: Option<&str>,
) -> Result<ScopeId, String> {
    match process {
        Some(id) => Ok(ScopeId::process(id)),
        None => document
            .processes()
            .find_map(|process| process.id.as_deref())
            .map(ScopeId::process)
            .ok_or_else(|| "document contains no process with an id to lay out".to_string()),
    }
}

fn layout_core(
    document: &bpmn_model::BpmnDocument,
    options: &LayoutOptionsJs,
) -> Result<bpmn_layout::LayoutOutcome, String> {
    let scope = resolve_scope(document, options.process.as_deref())?;
    NativeBpmnLayout
        .layout(document, &scope, &layout_options(options))
        .map_err(|error| error.to_string())
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

fn inspect_core(
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

fn validate_core(report: &bpmn_validate::ValidationReport) -> ValidationReportDto {
    ValidationReportDto {
        error_count: report.error_count() as u32,
        warning_count: report.warning_count() as u32,
        diagnostics: report.diagnostics.iter().map(diagnostic_to_dto).collect(),
    }
}

fn render_svg_bytes(
    document: &bpmn_model::BpmnDocument,
    options: &RenderOptionsJs,
) -> Result<SvgResult, String> {
    let diagram = select_diagram(document, options.diagram.as_deref(), options.diagram_index)?;
    let rendered = render_core(document, diagram, options)?;
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

fn render_png_bytes(
    document: &bpmn_model::BpmnDocument,
    options: &PngOptionsJs,
) -> Result<PngResult, String> {
    let raster = png_options(options)?;
    let svg = render_svg_bytes(document, &png_options_of_render(options))?;
    let png = bpmn_raster::render_png_with_diagnostics(&svg.svg, &raster)
        .map_err(|error| error.to_string())?;
    let (width, height) = bpmn_raster::png_dimensions(&png.bytes)
        .ok_or_else(|| "PNG encoding produced no dimensions".to_string())?;
    let mut warnings = svg.warnings;
    warnings.extend(png.diagnostics.iter().map(diagnostic_to_dto));
    Ok(PngResult {
        png: png.bytes.into(),
        width,
        height,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Module-level one-shots: the JS equivalent of `bpmn render`.
// ---------------------------------------------------------------------------

/// Parses and renders one diagram to SVG in a single call.
#[napi]
pub async fn render_svg(xml: String, options: Option<RenderOptionsJs>) -> napi::Result<SvgResult> {
    let options = options.unwrap_or(RenderOptionsJs {
        diagram: None,
        diagram_index: None,
        font_family: None,
        font_size: None,
        padding: None,
    });
    blocking(move || {
        let document = parse_document(&xml)?;
        render_svg_bytes(&document, &options)
    })
    .await
}

/// Parses and renders one diagram to PNG in a single call (native fonts).
#[napi]
pub async fn render_png(xml: String, options: Option<PngOptionsJs>) -> napi::Result<PngResult> {
    let options = options.unwrap_or(PngOptionsJs {
        diagram: None,
        diagram_index: None,
        font_family: None,
        font_size: None,
        padding: None,
        scale: None,
        background: None,
        max_width: None,
        max_height: None,
        max_pixels: None,
    });
    blocking(move || {
        let document = parse_document(&xml)?;
        render_png_bytes(&document, &options)
    })
    .await
}

// ---------------------------------------------------------------------------
// Reusable document handle.
// ---------------------------------------------------------------------------

/// Owned reusable BPMN document. Every operation re-locks the model; call
/// [`NativeDocument::close`] to release it deterministically.
#[napi(js_name = "BpmnDocument")]
pub struct NativeDocument {
    inner: Arc<Mutex<Option<bpmn_model::BpmnDocument>>>,
}

impl NativeDocument {
    async fn run<T: Send + 'static>(
        &self,
        job: impl FnOnce(&bpmn_model::BpmnDocument) -> Result<T, String> + Send + 'static,
    ) -> napi::Result<T> {
        let inner = self.inner.clone();
        blocking(move || {
            let guard = inner
                .lock()
                .map_err(|_| "document lock poisoned".to_string())?;
            job(guard
                .as_ref()
                .ok_or_else(|| "document is closed".to_string())?)
        })
        .await
    }
}

/// Parses and owns one BPMN document.
#[napi]
impl NativeDocument {
    /// Parses BPMN 2.0 XML text; throws with the parse diagnostic on failure.
    #[napi(factory)]
    pub async fn from_xml(xml: String) -> napi::Result<Self> {
        let document = blocking(move || parse_document(&xml)).await?;
        Ok(Self {
            inner: Arc::new(Mutex::new(Some(document))),
        })
    }

    /// Document summary, same contract as `bpmn inspect --format json`.
    #[napi]
    pub async fn inspect(&self) -> napi::Result<InspectResult> {
        self.run(|document| {
            let report = bpmn_validate::validate(document);
            Ok(inspect_core(document, &report))
        })
        .await
    }

    /// Validation report (errors, warnings, ordered diagnostics).
    #[napi]
    pub async fn validate(&self) -> napi::Result<ValidationReportDto> {
        self.run(|document| Ok(validate_core(&bpmn_validate::validate(document))))
            .await
    }

    /// Number of diagrams in source order.
    #[napi]
    pub fn diagram_count(&self) -> napi::Result<u32> {
        self.locked(|document| Ok(document.diagram_count() as u32))
    }

    /// All diagram ids in source order (diagrams without an id are skipped).
    #[napi]
    pub fn diagram_ids(&self) -> napi::Result<Vec<String>> {
        self.locked(|document| {
            Ok(document
                .definitions
                .diagrams
                .iter()
                .filter_map(|diagram| diagram.id.clone())
                .collect())
        })
    }

    /// Renders the selected (default: first) diagram to deterministic SVG.
    #[napi]
    pub async fn render_svg(&self, options: Option<RenderOptionsJs>) -> napi::Result<SvgResult> {
        let options = options.unwrap_or(RenderOptionsJs {
            diagram: None,
            diagram_index: None,
            font_family: None,
            font_size: None,
            padding: None,
        });
        self.run(move |document| render_svg_bytes(document, &options))
            .await
    }

    /// Renders the selected (default: first) diagram to PNG (native fonts).
    #[napi]
    pub async fn render_png(&self, options: Option<PngOptionsJs>) -> napi::Result<PngResult> {
        let options = options.unwrap_or(PngOptionsJs {
            diagram: None,
            diagram_index: None,
            font_family: None,
            font_size: None,
            padding: None,
            scale: None,
            background: None,
            max_width: None,
            max_height: None,
            max_pixels: None,
        });
        self.run(move |document| render_png_bytes(document, &options))
            .await
    }

    /// Runs auto-layout and returns the generated DI as JSON (ADR-0007).
    #[napi]
    pub async fn layout(&self, options: Option<LayoutOptionsJs>) -> napi::Result<LayoutResult> {
        let options = options.unwrap_or(LayoutOptionsJs {
            process: None,
            rank_gap: None,
            node_gap: None,
            routing_clearance: None,
            max_nodes: None,
            max_flows: None,
        });
        self.run(move |document| {
            let outcome = layout_core(document, &options)?;
            Ok(LayoutResult {
                diagram: diagram_to_dto(&outcome.diagram),
                warnings: outcome.warnings.iter().map(layout_warning_to_dto).collect(),
            })
        })
        .await
    }

    /// Lays out the selected process, then renders the generated DI to SVG.
    ///
    /// This is the no-DI path: models without `bpmndi` geometry render
    /// through generated bounds/waypoints instead of failing with a missing
    /// geometry error.
    #[napi]
    pub async fn render_layout_svg(
        &self,
        options: Option<LayoutOptionsJs>,
    ) -> napi::Result<SvgResult> {
        let options = options.unwrap_or(LayoutOptionsJs {
            process: None,
            rank_gap: None,
            node_gap: None,
            routing_clearance: None,
            max_nodes: None,
            max_flows: None,
        });
        self.run(move |document| {
            let outcome = layout_core(document, &options)?;
            let render = bpmn_render::render_diagram(
                document,
                &outcome.diagram,
                &CoreRenderOptions::default(),
            )
            .map_err(|error| error.to_string())?;
            let viewport = render.scene.viewport;
            let mut warnings: Vec<DiagnosticDto> = document
                .parse_diagnostics
                .iter()
                .map(diagnostic_to_dto)
                .collect();
            warnings.extend(outcome.warnings.iter().map(layout_warning_to_dto));
            warnings.extend(render.diagnostics.iter().map(diagnostic_to_dto));
            Ok(SvgResult {
                svg: bpmn_svg::to_svg(&render.scene),
                width: viewport.width,
                height: viewport.height,
                warnings,
            })
        })
        .await
    }

    /// Releases the model; later operations throw "document is closed".
    #[napi]
    pub fn close(&self) -> napi::Result<()> {
        self.inner
            .lock()
            .map_err(|_| failure("document lock poisoned"))?
            .take();
        Ok(())
    }
}

impl NativeDocument {
    /// Synchronous accessor for trivial metadata reads.
    fn locked<T>(
        &self,
        job: impl FnOnce(&bpmn_model::BpmnDocument) -> napi::Result<T>,
    ) -> napi::Result<T> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| failure("document lock poisoned"))?;
        job(guard
            .as_ref()
            .ok_or_else(|| failure("document is closed"))?)
    }
}
