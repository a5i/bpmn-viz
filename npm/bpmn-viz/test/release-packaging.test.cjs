'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { mkdtempSync, mkdirSync, writeFileSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve, dirname } = require('node:path');
const { validateRelease } = require('../scripts/validate-release.cjs');
const { binaryNames } = require('../platforms.cjs');
const manifest = require('../package.json');
const wasmFiles = require('../scripts/wasm-files.cjs');

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'bpmn-viz-release-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  writeFileSync(join(root, 'package.json'), JSON.stringify(manifest));
  for (const name of [...binaryNames, ...wasmFiles, 'index.js', 'index.d.ts', 'platforms.cjs', 'LICENSE']) {
    mkdirSync(dirname(join(root, name)), { recursive: true });
    writeFileSync(join(root, name), 'fixture');
  }
  return root;
}

test('one release package contains all four platform binaries and no dependencies', t => {
  assert.equal(manifest.optionalDependencies, undefined);
  validateRelease(fixture(t));
});

test('release rejects missing/empty/unexpected binaries', async t => {
  for (const mutation of ['missing', 'empty', 'unexpected']) {
    await t.test(mutation, t => {
      const root = fixture(t);
      const expected = mutation === 'empty' ? /Missing or empty release file/ : /platform binaries/;
      if (mutation === 'missing') rmSync(join(root, binaryNames[0]));
      if (mutation === 'empty') writeFileSync(join(root, binaryNames[0]), '');
      if (mutation === 'unexpected') writeFileSync(join(root, 'bpmn_viz.darwin-x64.node'), 'obsolete Intel build');
      assert.throws(() => validateRelease(root), expected);
    });
  }
});

test('release rejects empty runtime payload files', async t => {
  await t.test('empty wasm payload', t => {
    const root = fixture(t);
    writeFileSync(join(root, wasmFiles[0]), '');
    assert.throws(() => validateRelease(root), /Missing or empty release file/);
  });
  await t.test('empty TypeScript contract', t => {
    const root = fixture(t);
    writeFileSync(join(root, 'index.d.ts'), '');
    assert.throws(() => validateRelease(root), /Missing or empty release file: index\.d\.ts/);
  });
});

test('wasm subpath resolves without loading the native entry', () => {
  // Export map contract: require('@a5i/bpmn-viz/wasm') must not touch the
  // .node binary; validated against the package.json directly (the packed
  // artifact is checked again by validate-release + test:wasm).
  const resolved = resolve(__dirname, '../wasm/node.cjs');
  assert.match(manifest.exports['./wasm'].node.require, /^\.\/wasm\/node\.cjs$/);
  assert.match(resolved, /wasm\/node\.cjs$/);
});
