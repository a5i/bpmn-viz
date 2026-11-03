// Node ESM entry over the wasm-pack `nodejs` (CJS) target.
import wasm from './node/bpmn_wasm.js';
const { BpmnDocument } = wasm;
export { wasm as default, BpmnDocument };
