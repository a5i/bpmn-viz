// Structural mirror of the JSON contracts exposed by the bindings
// (npm/bpmn-viz/index.d.ts, crates/bpmn-wasm DTOs; kept in sync there).
// Only the fields the playground consumes are listed.

export type Severity = "error" | "warning";

export interface Diagnostic {
  code: string;
  severity: Severity;
  message: string;
  elementId?: string;
  line?: number;
  column?: number;
}

export interface ValidationReport {
  errorCount: number;
  warningCount: number;
  diagnostics: Diagnostic[];
}

export interface RenderOptions {
  /** Diagram id to render (wins over diagramIndex). */
  diagram?: string;
  /** Zero-based diagram index to render. */
  diagramIndex?: number;
  fontFamily?: string;
  fontSize?: number;
  padding?: number;
}

export interface SvgResult {
  svg: string;
  width: number;
  height: number;
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
  process?: string;
  rankGap?: number;
  nodeGap?: number;
  routingClearance?: number;
  maxNodes?: number;
  maxFlows?: number;
}

export interface LayoutResult {
  /** Generated diagram (ADR-0007 id scheme). */
  diagram: DiagramJson;
  /** Non-fatal layout notes. */
  warnings: Diagnostic[];
}
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

/** One playground run: everything the UI needs to draw all output tabs. */
export interface RunResult {
  validate: ValidationReport;
  inspect: InspectResult;
  render: SvgResult | null;
  layout: LayoutResult | null;
  renderMode: RenderMode;
}
export type RenderMode = "di" | "layout";

/** postMessage envelope from public/bpmn.worker.js. */
export type WorkerResponse =
  | { ok: true; result: RunResult; duration: number }
  | { ok: false; error: string };
