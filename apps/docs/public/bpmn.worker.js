// Playground worker: instantiates the committed wasm-pack `web` artifact
// (crates/bpmn-wasm/pkg-web, ADR-0009) and exposes the parse → validate →
// render pipeline synchronously on this worker thread. No input leaves the
// browser; the UI terminates the worker after its own timeout.
import init, { BpmnDocument } from "./runtime/bpmn_wasm.js";

let ready = null;
function ensureReady() {
  if (!ready) ready = init();
  return ready;
}

self.onmessage = async ({ data }) => {
  const started = performance.now();
  let document;
  try {
    await ensureReady();
    document = new BpmnDocument(data.xml);
    try {
      const validate = document.validate();
      const inspect = document.inspect();
      const options = data.options ?? undefined;
      let render = null;
      let layout = null;
      if (data.mode === "layout") {
        render = document.renderLayoutSvg(options);
        layout = document.layout(options);
      } else {
        render = document.renderSvg(options);
      }
      self.postMessage({
        ok: true,
        result: { validate, inspect, render, layout, renderMode: data.mode },
        duration: performance.now() - started,
      });
    } finally {
      document.free();
    }
  } catch (error) {
    self.postMessage({ ok: false, error: error instanceof Error ? error.message : String(error) });
  }
};
