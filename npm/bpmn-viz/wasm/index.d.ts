export type {
  Diagnostic,
  InspectResult,
  ValidationReport,
  RenderOptions,
  SvgResult,
  LayoutBounds,
  LayoutPoint,
  LayoutShape,
  LayoutEdge,
  DiagramJson,
  LayoutOptions,
  LayoutResult,
} from '../index';

/** Initialization output of the wasm-bindgen `web` target. */
export interface InitOutput {
  memory: WebAssembly.Memory;
}

/**
 * Reusable parsed BPMN document (synchronous WASM binding, SVG-only).
 * PNG rasterization is native-only; rasterize `renderSvg()` output on a
 * canvas in the browser (ADR-0009).
 */
export class BpmnDocument {
  constructor(xml: string);
  /** Number of diagrams in source order. */
  get diagramCount(): number;
  /** All diagram ids in source order (diagrams without an id are skipped). */
  diagramIds(): string[];
  /** Document summary, same contract as `bpmn inspect --format json`. */
  inspect(): InspectResult;
  /** Validation report (errors, warnings, ordered diagnostics). */
  validate(): ValidationReport;
  /** Renders the selected (default: first) diagram to deterministic SVG. */
  renderSvg(options?: RenderOptions): SvgResult;
  /** Runs auto-layout and returns the generated DI as JSON. */
  layout(options?: LayoutOptions): LayoutResult;
  /** Lays out the selected process, then renders the generated DI to SVG. */
  renderLayoutSvg(options?: LayoutOptions): SvgResult;
  /** Releases WASM resources. */
  free(): void;
}

/** Loads and initializes the WASM module (browser/bundler `web` target). */
export default function init(
  moduleOrPath?: new (url?: string | URL) => RequestInfo | URL,
): Promise<InitOutput>;
