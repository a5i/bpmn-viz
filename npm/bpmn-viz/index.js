'use strict';
const { resolve } = require('node:path');
const { existsSync } = require('node:fs');
const { platformName } = require('./platforms.cjs');
function loadNative() {
  const binary = resolve(__dirname, `bpmn_viz.${platformName()}.node`);
  if (existsSync(binary)) return require(binary);
  const local = resolve(__dirname, 'bpmn_node.node');
  if (existsSync(local)) return require(local);
  throw new Error(`Missing bpmn-viz native addon: ${binary}. In a source checkout, run npm run build.`);
}
const native = loadNative();

/**
 * Parse, validate, auto-layout and render BPMN 2.0 models.
 *
 * All heavy work runs off the JS thread; call `close()` to release a
 * document deterministically. See ADR-0009 for the contract.
 */
module.exports = {
  BpmnDocument: native.BpmnDocument,
  renderSvg: native.renderSvg,
  renderPng: native.renderPng,
};
