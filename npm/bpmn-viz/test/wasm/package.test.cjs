'use strict';
// Tests the assembled @a5i/bpmn-viz/wasm runtime (run `npm run build:wasm`
// first). The native addon must not be loaded here.
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join, resolve } = require('node:path');

const pkgRoot = resolve(__dirname, '../..');
const repo = resolve(pkgRoot, '../..');
let wasm;
try {
  wasm = require(join(pkgRoot, 'wasm/node.cjs'));
} catch (error) {
  console.error('wasm runtime not assembled yet; run `npm run build:wasm` first');
  throw error;
}

const fixtureDir = join(repo, 'tests/fixtures/minimal');
const xml = readFileSync(join(fixtureDir, 'start-task-end.bpmn'), 'utf8');
const expectedSvg = readFileSync(join(fixtureDir, 'start-task-end.svg'), 'utf8');

test('packaged wasm runtime renders the CLI fixture byte-for-byte', () => {
  assert.equal(typeof wasm.BpmnDocument, 'function', 'BpmnDocument must be exported');
  const document = new wasm.BpmnDocument(xml);
  try {
    assert.equal(document.diagramCount, 1);
    const rendered = document.renderSvg();
    assert.equal(rendered.svg, expectedSvg, 'WASM SVG parity with the committed CLI fixture');
    assert.equal(document.inspect().contractVersion, 1);
  } finally {
    document.free();
  }
});
