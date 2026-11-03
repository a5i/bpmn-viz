'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { readFileSync } = require('node:fs');
const { join, resolve } = require('node:path');
const { BpmnDocument } = require('..');

const repo = resolve(__dirname, '../../..');
const bin = process.env.BPMN_BIN || join(repo, 'target/debug', process.platform === 'win32' ? 'bpmn.exe' : 'bpmn');
const fixtureDir = join(repo, 'tests/fixtures/minimal');
const xml = readFileSync(join(fixtureDir, 'start-task-end.bpmn'), 'utf8');

function cli(...args) {
  return execFileSync(bin, args, { encoding: 'utf8' });
}

test('native addon matches the CLI for inspect, validate and SVG bytes', async () => {
  const document = await BpmnDocument.fromXml(xml);
  try {
    // SVG bytes must be identical to `bpmn render` stdout output.
    const cliSvg = cli('render', join(fixtureDir, 'start-task-end.bpmn'), '--format', 'svg', '--output', '-');
    assert.equal((await document.renderSvg()).svg, cliSvg, 'SVG parity with the CLI');

    // Inspect envelope: the CLI adds `command` and `input`; everything else
    // must match the addon result exactly.
    const cliInspect = camelize(JSON.parse(cli('inspect', join(fixtureDir, 'start-task-end.bpmn'), '--format', 'json')));
    delete cliInspect.command;
    delete cliInspect.input;
    const addonInspect = await document.inspect();
    assert.deepEqual(addonInspect, cliInspect, 'inspect parity with the CLI');
  } finally {
    await document.close();
  }
});

function camelize(value) {
  if (Array.isArray(value)) return value.map(camelize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [
        key.replace(/_([a-z])/g, (_, c) => c.toUpperCase()),
        camelize(item),
      ]),
    );
  }
  return value;
}
