export interface Diagnostic {
  /** Stable machine-readable code. */
  code: string;
  /** `error` or `warning`. */
  severity: 'error' | 'warning';
  /** Human-readable explanation. */
  message: string;
  /** Semantic or DI element id, when known. */
  elementId?: string;
  /** One-based source line, when known. */
  line?: number;
  /** One-based source column, when known. */
  column?: number;
}

/** Same contract as `bpmn inspect --format json` (minus `command`/`input`). */
export interface InspectResult {
  contractVersion: 1;
  definitionsId?: string;
  diagramCount: number;
  processes: number;
  collaborations: number;
  participants: number;
  lanes: number;
  nodesByKind: Record<string, number>;
  sequenceFlows: number;
  messageFlows: number;
  textAnnotations: number;
  diagnostics: Diagnostic[];
}

export interface ValidationReport {
  errorCount: number;
  warningCount: number;
  diagnostics: Diagnostic[];
}

export interface RenderOptions {
  /** Diagram id to render (wins over `diagramIndex`). */
  diagram?: string;
  /** Zero-based diagram index to render. */
  diagramIndex?: number;
  /** Font family list for labels (default `sans-serif`). */
  fontFamily?: string;
  /** Base label font size in user units (default 12). */
  fontSize?: number;
  /** Viewport padding in user units (default 10). */
  padding?: number;
}

export interface PngOptions extends RenderOptions {
  /** Positive finite scale factor applied to the SVG size (default 1). */
  scale?: number;
  /** CSS hex background: `#rgb`, `#rrggbb` or `#rrggbbaa` (default opaque white). */
  background?: string;
  /** Maximum output width in pixels. */
  maxWidth?: number;
  /** Maximum output height in pixels. */
  maxHeight?: number;
  /** Maximum total pixels (width × height). */
  maxPixels?: number;
}

export interface SvgResult {
  /** Deterministic SVG document (same bytes as `bpmn render --format svg`). */
  svg: string;
  /** Viewport width in user units. */
  width: number;
  /** Viewport height in user units. */
  height: number;
  /** Non-fatal render/import diagnostics. */
  warnings: Diagnostic[];
}

export interface PngResult {
  /** Encoded PNG bytes. */
  png: Buffer;
  /** Output width in pixels. */
  width: number;
  /** Output height in pixels. */
  height: number;
  /** Non-fatal render/import/rasterization diagnostics. */
  warnings: Diagnostic[];
}

export interface LayoutBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface LayoutPoint {
  x: number;
  y: number;
}

export interface LayoutShape {
  id?: string;
  /** Semantic element id this shape provides geometry for. */
  bpmnElement: string;
  bounds: LayoutBounds;
  isHorizontal?: boolean;
  isExpanded?: boolean;
}

export interface LayoutEdge {
  id?: string;
  /** Semantic element id this edge provides geometry for. */
  bpmnElement: string;
  waypoints: LayoutPoint[];
}

export interface DiagramJson {
  id?: string;
  name?: string;
  shapes: LayoutShape[];
  edges: LayoutEdge[];
}

export interface LayoutOptions {
  /** Process id to lay out; defaults to the first process with an id. */
  process?: string;
  /** Horizontal distance between rank columns (default 100). */
  rankGap?: number;
  /** Vertical distance between nodes in a band row (default 80). */
  nodeGap?: number;
  /** Clearance routed segments keep from shape interiors (default 20). */
  routingClearance?: number;
  /** Maximum number of flow nodes in the scope (default 10000). */
  maxNodes?: number;
  /** Maximum number of sequence flows in the scope (default 10000). */
  maxFlows?: number;
}

export interface LayoutResult {
  /** Generated diagram (ADR-0007 id scheme). */
  diagram: DiagramJson;
  /** Non-fatal layout notes. */
  warnings: Diagnostic[];
}

/**
 * Owned reusable BPMN document. Every heavy operation runs off the JS
 * thread; call `close()` to release the model deterministically.
 */
export class BpmnDocument {
  /** Parses BPMN 2.0 XML text; rejects with the parse diagnostic on failure. */
  static fromXml(xml: string): Promise<BpmnDocument>;
  /** Document summary, same contract as `bpmn inspect --format json`. */
  inspect(): Promise<InspectResult>;
  /** Validation report (errors, warnings, ordered diagnostics). */
  validate(): Promise<ValidationReport>;
  /** Number of diagrams in source order. */
  diagramCount(): number;
  /** All diagram ids in source order (diagrams without an id are skipped). */
  diagramIds(): string[];
  /** Renders the selected (default: first) diagram to deterministic SVG. */
  renderSvg(options?: RenderOptions): Promise<SvgResult>;
  /** Renders the selected (default: first) diagram to PNG (native fonts). */
  renderPng(options?: PngOptions): Promise<PngResult>;
  /** Runs auto-layout and returns the generated DI as JSON. */
  layout(options?: LayoutOptions): Promise<LayoutResult>;
  /**
   * Lays out the selected process, then renders the generated DI to SVG.
   * The no-DI path: models without `bpmndi` geometry stay renderable.
   */
  renderLayoutSvg(options?: LayoutOptions): Promise<SvgResult>;
  /** Releases the model; later operations reject with "document is closed". */
  close(): void;
}

/** One-shot parse + render (the JS equivalent of `bpmn render --format svg`). */
export function renderSvg(xml: string, options?: RenderOptions): Promise<SvgResult>;
/** One-shot parse + render (the JS equivalent of `bpmn render --format png`). */
export function renderPng(xml: string, options?: PngOptions): Promise<PngResult>;
