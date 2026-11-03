(async () => {
  const { smoke } = await import('./shared.mjs');
  const { BpmnDocument } = require('../pkg-node/bpmn_wasm.js');
  smoke({ BpmnDocument });

  // Node-only: byte parity with the committed CLI fixture (the same gate the
  // native addon runs against `bpmn render`).
  const fs = require('node:fs');
  const path = require('node:path');
  const fixtureDir = path.resolve(__dirname, '../../../tests/fixtures/minimal');
  const document = new BpmnDocument(fs.readFileSync(path.join(fixtureDir, 'start-task-end.bpmn'), 'utf8'));
  const rendered = document.renderSvg();
  const expected = fs.readFileSync(path.join(fixtureDir, 'start-task-end.svg'), 'utf8');
  if (rendered.svg !== expected) throw new Error('WASM SVG differs from the committed CLI fixture');
  const inspected = document.inspect();
  if (inspected.definitionsId !== 'Definitions_Minimal') throw new Error('Unexpected definitions id');
  console.log('Node WASM smoke: PASS (fixture parity included)');
})().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
