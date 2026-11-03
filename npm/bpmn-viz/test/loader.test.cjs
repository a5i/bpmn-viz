'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { platformName, binaryNames } = require('../platforms.cjs');
const pkg = require('../index.js');

test('platform table maps supported runners and rejects the rest', () => {
  assert.equal(platformName('linux', 'x64'), 'linux-x64-gnu');
  assert.equal(platformName('linux', 'arm64'), 'linux-arm64-gnu');
  assert.equal(platformName('darwin', 'arm64'), 'darwin-arm64');
  assert.equal(platformName('win32', 'x64'), 'win32-x64-msvc');
  assert.throws(() => platformName('darwin', 'x64'), /Unsupported bpmn-viz platform: darwin-x64/,
    'retired Intel macOS must be rejected');
  assert.throws(() => platformName('sunos', 'x64'), /Unsupported bpmn-viz platform/);
  assert.equal(binaryNames.length, 4);
  assert.deepEqual([...binaryNames].sort(), [
    'bpmn_viz.darwin-arm64.node',
    'bpmn_viz.linux-arm64-gnu.node',
    'bpmn_viz.linux-x64-gnu.node',
    'bpmn_viz.win32-x64-msvc.node',
  ]);
});

test('entry exposes the native document class and one-shot renders', () => {
  assert.equal(typeof pkg.BpmnDocument, 'function');
  assert.equal(typeof pkg.BpmnDocument.fromXml, 'function');
  assert.equal(typeof pkg.renderSvg, 'function');
  assert.equal(typeof pkg.renderPng, 'function');
});

test('loader prefers platform binaries, falls back to the dev build, explains the remedy', () => {
  const source = readFileSync(join(__dirname, '..', 'index.js'), 'utf8');
  const platformFirst = source.indexOf('bpmn_viz.${platformName()}.node');
  const localFallback = source.indexOf("bpmn_node.node'");
  assert.ok(platformFirst >= 0, 'platform-specific binary name expected');
  assert.ok(localFallback > platformFirst, 'local dev build is the second resolution step');
  assert.match(source, /npm run build/, 'error must explain the source-checkout remedy');
});
