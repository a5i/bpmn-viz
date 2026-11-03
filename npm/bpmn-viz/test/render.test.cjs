'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join, resolve } = require('node:path');
const { BpmnDocument, renderSvg } = require('..');

const repo = resolve(__dirname, '../../..');
const fixtureDir = join(repo, 'tests/fixtures/minimal');
const xml = readFileSync(join(fixtureDir, 'start-task-end.bpmn'), 'utf8');
const expectedSvg = readFileSync(join(fixtureDir, 'start-task-end.svg'), 'utf8');

test('renderSvg matches the committed CLI fixture byte-for-byte', async () => {
  await assert.rejects(renderSvg('not xml'), /XML|parse|invalid|error/i, 'parse failure rejects');
  const document = await BpmnDocument.fromXml(xml);
  try {
    const result = await document.renderSvg();
    assert.equal(result.svg, expectedSvg);
    assert.equal(result.width, 324);
    assert.equal(result.height, 100);
    assert.deepEqual(result.warnings, []);
    assert.deepEqual(await document.diagramIds(), ['BPMNDiagram_1']);
    assert.equal(await document.diagramCount(), 1);
  } finally {
    await document.close();
  }
});

test('renderPng honors scale and background, refuses invalid colors', async () => {
  const document = await BpmnDocument.fromXml(xml);
  try {
    const result = await document.renderPng({ scale: 2, background: '#123456' });
    assert.equal(result.width, 648);
    assert.equal(result.height, 200);
    assert.equal(result.png.subarray(1, 4).toString('ascii'), 'PNG');
    assert.ok(result.png.length > 0);
    await assert.rejects(document.renderPng({ background: 'red' }), /CSS hex/);
    await assert.rejects(document.renderPng({ scale: 0 }), /scale/);
  } finally {
    await document.close();
  }
});

test('layout renders no-DI models and reports generated DI', async () => {
  const noDi = '<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" id="defs"><process id="P"><startEvent id="s"/><task id="t" name="Do it"/><endEvent id="e"/><sequenceFlow id="f1" sourceRef="s" targetRef="t"/><sequenceFlow id="f2" sourceRef="t" targetRef="e"/></process></definitions>';
  const document = await BpmnDocument.fromXml(noDi);
  try {
    await assert.rejects(document.renderSvg(), /no diagram/,
      'missing DI must reject, never fall back (AC-09)');
    const report = await document.validate();
    assert.equal(report.errorCount, 1);
    assert.equal(report.diagnostics[0].code, 'di.missing_interchange');
    const laid = await document.layout();
    assert.equal(laid.diagram.shapes.length, 3);
    assert.equal(laid.diagram.edges.length, 2);
    assert.deepEqual(laid.warnings, []);
    const rendered = await document.renderLayoutSvg();
    assert.ok(rendered.svg.startsWith('<?xml'));
    assert.ok(rendered.svg.includes('<svg'));
    // Deterministic: identical options, identical output.
    assert.equal(JSON.stringify(await document.layout()), JSON.stringify(laid));
  } finally {
    await document.close();
  }
});

test('diagram selection by id and index, unknown selections reject', async () => {
  const document = await BpmnDocument.fromXml(xml);
  try {
    const byId = await document.renderSvg({ diagram: 'BPMNDiagram_1' });
    const byIndex = await document.renderSvg({ diagramIndex: 0 });
    assert.equal(byId.svg, byIndex.svg);
    await assert.rejects(document.renderSvg({ diagram: 'missing' }), /no diagram with id `missing`/);
    await assert.rejects(document.renderSvg({ diagramIndex: 5 }), /out of range/);
  } finally {
    await document.close();
  }
});

test('closed documents reject further work', async () => {
  const document = await BpmnDocument.fromXml(xml);
  await document.close();
  await assert.rejects(document.inspect(), /document is closed/);
  await document.close(); // idempotent
});
