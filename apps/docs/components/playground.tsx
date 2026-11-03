"use client";

import dynamic from "next/dynamic";
import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTheme } from "next-themes";
import { xml } from "@codemirror/lang-xml";
import { AlertCircle, ArrowUpRight, Braces, CheckCircle2, FileCode2, FileText, Image as ImageIcon, LoaderCircle, Play, RotateCcw, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { playgroundExamples, type PlaygroundExample } from "@/lib/example";
import type { LayoutEdge, LayoutShape, RenderMode, RunResult } from "@/lib/types";

const CodeMirror = dynamic(() => import("@uiw/react-codemirror"), { ssr: false, loading: () => <div className="editor-loading">Loading editor…</div> });
type Result = { result: RunResult; duration: number };
type OutputTab = "preview" | "diagnostics" | "layout";

const AUTO_RENDER_DELAY = 600;

export function Playground({ compact = false }: { compact?: boolean }) {
  const { resolvedTheme } = useTheme();
  const [exampleId, setExampleId] = useState(playgroundExamples[0].id);
  const example = playgroundExamples.find(item => item.id === exampleId)!;
  const [document, setDocument] = useState(playgroundExamples[0].document);
  const [mode, setMode] = useState<RenderMode>(playgroundExamples[0].mode);
  const [busy, setBusy] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [result, setResult] = useState<Result | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [outputTab, setOutputTab] = useState<OutputTab>("preview");
  const [runNonce, setRunNonce] = useState(0);
  const workerRef = useRef<Worker | null>(null);
  const runIdRef = useRef(0);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Rendering is automatic: every run supersedes the previous one, so edits
  // never queue behind in-flight work. The worker persists across runs (the
  // WASM module stays initialized); a hung run terminates and rebuilds it.
  const startRun = useCallback(() => {
    if (debounceRef.current) { clearTimeout(debounceRef.current); debounceRef.current = null; }
    const id = ++runIdRef.current;
    let worker = workerRef.current;
    if (!worker) {
      worker = new Worker("/bpmn.worker.js", { type: "module" });
      workerRef.current = worker;
    }
    setBusy(true); setError(null);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => {
      if (runIdRef.current !== id) return;
      workerRef.current?.terminate(); workerRef.current = null;
      setError("Execution stopped after 20 seconds. Simplify the model and try again.");
      setResult(null); setBusy(false);
    }, 20_000);
    worker.onmessage = ({ data }) => {
      if (runIdRef.current !== id) return;
      if (timeoutRef.current) { clearTimeout(timeoutRef.current); timeoutRef.current = null; }
      if (data.ok) { setResult({ result: data.result, duration: data.duration }); setDirty(false); setOutputTab(data.result.renderMode === "layout" || data.result.validate.errorCount === 0 ? "preview" : "diagnostics"); }
      else { setError(data.error); setResult(null); }
      setBusy(false);
    };
    worker.onerror = event => {
      if (runIdRef.current !== id) return;
      if (timeoutRef.current) { clearTimeout(timeoutRef.current); timeoutRef.current = null; }
      workerRef.current?.terminate(); workerRef.current = null;
      setError(event.message || "Could not start the WASM engine. Check your connection and retry.");
      setResult(null); setBusy(false);
    };
    worker.postMessage({ xml: document, mode });
  }, [document, mode]);

  const finishRun = useCallback(() => {
    if (timeoutRef.current) { clearTimeout(timeoutRef.current); timeoutRef.current = null; }
    workerRef.current?.terminate(); workerRef.current = null;
    setBusy(false);
  }, []);

  // Render by default: the diagram appears on load (immediately), then live
  // re-renders (debounced) on every edit. runNonce forces a render after
  // example switches and resets even when the text is unchanged.
  useEffect(() => () => finishRun(), [finishRun]);
  const skipAutoRef = useRef(true);
  useEffect(() => {
    const immediate = skipAutoRef.current;
    skipAutoRef.current = false;
    debounceRef.current = setTimeout(() => startRun(), immediate ? 0 : AUTO_RENDER_DELAY);
    return () => { if (debounceRef.current) { clearTimeout(debounceRef.current); debounceRef.current = null; } };
  }, [startRun, runNonce]);

  const dirtyNow = dirty && !!result;
  const pending = busy && !result;
  const errors = result?.result.validate.errorCount ?? 0;
  const warnings = result?.result.validate.warningCount ?? 0;

  function loadExample(next: PlaygroundExample) {
    setExampleId(next.id);
    setDocument(next.document);
    setMode(next.mode);
    setResult(null); setError(null); setDirty(false);
    setOutputTab("preview");
    setRunNonce(value => value + 1);
  }
  function reset() { loadExample(example); }
  function chooseExample(id: string) {
    const next = playgroundExamples.find(item => item.id === id);
    if (!next || next.id === exampleId) return;
    if ((document !== example.document || mode !== example.mode) && !window.confirm("Switch examples and discard your edits?")) return;
    loadExample(next);
  }
  function chooseMode(value: string) {
    const next = value as RenderMode;
    if (next === mode) return;
    setMode(next); setDirty(true);
  }
  function breakExample() {
    setDocument(current => example.breakDocument(current));
    setDirty(true);
  }

  const diagrams = result?.result.inspect.diagramCount ?? 0;
  return <div className={`playground ${compact ? "playground-compact" : ""}`}>
    <div className="workspace-bar"><span><span className="status-dot" /> {compact ? "LIVE EXAMPLE" : "LOCAL WORKSPACE"}</span><span className="workspace-runtime">Rust → WASM <span>·</span> In your browser</span>{compact && <Link href="/playground/">Open playground <ArrowUpRight size={13} /></Link>}</div>
    {!compact && <div className="example-picker">
      <div className="example-picker-heading">
        <label>EXAMPLE<select aria-label="Choose example" value={exampleId} onChange={event => chooseExample(event.target.value)}>{playgroundExamples.map(item => <option key={item.id} value={item.id}>{item.title}</option>)}</select></label>
        <label>RENDER MODE<select aria-label="Render mode" value={mode} onChange={event => chooseMode(event.target.value)}><option value="di">Provided DI</option><option value="layout">Auto-layout</option></select></label>
        <span className="example-file-count">1 model · 0 dependencies</span>
      </div>
      <p>{example.description}</p>
      <p className="example-challenge"><strong>Try this:</strong> {example.challenge}</p>
    </div>}
    <Tabs value="document" className="flex-col gap-0">
      <div className="editor-toolbar">
        <TabsList variant="line" aria-label="Workspace files">
          <TabsTrigger value="document"><FileText />{example.fileName}</TabsTrigger>
        </TabsList>
        <div className="editor-actions">
          <Button variant="ghost" size="icon-sm" aria-label="Reset example" onClick={reset}><RotateCcw /></Button>
          <Button onClick={startRun} size="sm">{busy ? <LoaderCircle className="animate-spin" /> : <Play size={12} />} {busy ? "Running…" : mode === "layout" ? "Lay out & render" : "Render now"}</Button>
        </div>
      </div>
      <TabsContent value="document" className="editor-area">
        <CodeMirror value={document} height={compact ? "300px" : "390px"} theme={resolvedTheme === "dark" ? "dark" : "light"} extensions={[xml()]} onChange={value => { setDocument(value); setDirty(true); }} basicSetup={{ foldGutter: false, highlightActiveLine: true, autocompletion: false }} aria-label="Edit BPMN XML" />
      </TabsContent>
    </Tabs>
    <Tabs value={outputTab} onValueChange={value => setOutputTab(value as OutputTab)} className="flex-col gap-0">
      <div className="output-bar">
        <TabsList variant="line" aria-label="Output panels">
          <TabsTrigger value="preview"><ImageIcon />Preview</TabsTrigger>
          <TabsTrigger value="diagnostics"><ShieldCheck />Diagnostics {result && <span className="count">{result.result.validate.diagnostics.length}</span>}</TabsTrigger>
          <TabsTrigger value="layout"><Braces />Generated DI</TabsTrigger>
        </TabsList>
        <span className="output-meta" data-dirty={dirtyNow}>{pending ? "Rendering…" : busy ? "Re-rendering…" : result ? `${diagrams} ${diagrams === 1 ? "diagram" : "diagrams"} · ${Math.round(result.duration)} ms incl. startup` : "Live preview — renders as you edit"}</span>
      </div>
      {(["preview", "diagnostics", "layout"] as const).map(panel => <TabsContent value={panel} key={panel}>
      {outputTab === panel && (
    <div className="output-body" role="status" aria-live="polite" aria-busy={busy}>
      {error ? <div className="execution-error"><AlertCircle size={18} /><div><strong>Could not render this model</strong><pre>{error}</pre></div></div> : pending ? <div className="output-empty"><LoaderCircle className="animate-spin" size={20} /><div><strong>Preparing the engine…</strong><p>The WASM runtime initializes on first load; the diagram renders automatically.</p></div></div> : !result ? <div className="output-empty"><FileCode2 size={20} /><div><strong>Your BPMN. Real pixels.</strong><p>Edit the model — the preview re-renders automatically.</p></div></div> : panel === "preview" ? <>
        {result.result.render && <><div className={`diagram-canvas ${compact ? "diagram-canvas-compact" : ""}`} role="img" aria-label="Rendered BPMN diagram" dangerouslySetInnerHTML={{ __html: result.result.render.svg }} /><div className="diagram-meta"><span>{result.result.render.width} × {result.result.render.height} user units</span><span>{result.result.renderMode === "layout" ? "auto-layout" : "provided DI"}</span>{result.result.render.warnings.length > 0 && <span>{result.result.render.warnings.length} render {result.result.render.warnings.length === 1 ? "warning" : "warnings"}</span>}</div></>}
      </> : panel === "layout" ? <>
        {result.result.layout ? <LayoutTable shapes={result.result.layout.diagram.shapes} edges={result.result.layout.diagram.edges} /> : <p className="muted">Switch the render mode to Auto-layout to generate Diagram Interchange for this model.</p>}
      </> : <>
        {!result.result.validate.diagnostics.length ? <div className="success-result"><CheckCircle2 size={21} /><div><strong>All checks passed</strong><p>References, geometry and DI coverage validate cleanly.</p></div><span className="result-badge">VALID</span></div> : <><div className="diagnostic-summary">{errors} errors · {warnings} warnings</div>{result.result.validate.diagnostics.map((diagnostic, index) => <div className="diagnostic" key={index}><AlertCircle size={16} /><div><code>{example.fileName}{diagnostic.line != null ? `:${diagnostic.line}${diagnostic.column != null ? `:${diagnostic.column}` : ""}` : ""}</code><p>{diagnostic.message}</p><small>{diagnostic.code} · {diagnostic.severity}</small></div></div>)}</>}
      </>}
    </div>
      )}
      </TabsContent>)}
    </Tabs>
    <div className="workspace-bottom"><span><ShieldCheck size={13} /> Files stay in this browser. No server execution. Renders automatically as you edit.</span><Button variant="ghost" size="xs" onClick={breakExample}>Introduce an error ↗</Button></div>
  </div>;
}

function LayoutTable({ shapes, edges }: { shapes: LayoutShape[]; edges: LayoutEdge[] }) {
  return <div className="query-table-wrap"><table className="query-table"><thead><tr><th>kind</th><th>element</th><th>geometry</th></tr></thead><tbody>
    {shapes.map((shape, index) => <tr key={`s${index}`}><td>shape</td><td>{shape.bpmnElement}</td><td>{Math.round(shape.bounds.x)}, {Math.round(shape.bounds.y)} · {Math.round(shape.bounds.width)} × {Math.round(shape.bounds.height)}</td></tr>)}
    {edges.map((edge, index) => <tr key={`e${index}`}><td>edge</td><td>{edge.bpmnElement}</td><td>{edge.waypoints.map(point => `${Math.round(point.x)},${Math.round(point.y)}`).join(" → ")}</td></tr>)}
  </tbody></table></div>;
}
