#!/usr/bin/env node
/**
 * Dev-only reference harness (EXP-004, ADR-0006).
 *
 * Runs the PINNED bpmn-io/bpmn-auto-layout engine (v2.0.0-alpha.2, MIT) on the
 * project-owned layout fixtures and emits canonical JSON
 * `{fixture, reference, shapes, edges, warnings}` used by the Rust
 * differential tests (`crates/bpmn-layout/tests/reference_parity.rs`).
 *
 * This tool never runs from the Rust library or from `cargo test`; it exists
 * so the committed reference JSON under `tests/fixtures/reference/` can be
 * regenerated and audited. The upstream repository and node_modules are not
 * committed.
 *
 * Usage:
 *   cd tools/reference-layout && npm install        # once; gitignored
 *   node run.mjs ../../crates/bpmn-layout/tests/fixtures/core/*.bpmn \
 *        --out ../../crates/bpmn-layout/tests/fixtures/reference
 */

import { promises as fs } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { BpmnModdle } from 'bpmn-moddle';
import { layoutProcess } from 'bpmn-auto-layout';

const PIN = {
  repository: 'https://github.com/bpmn-io/bpmn-auto-layout',
  version: '2.0.0-alpha.2',
  commit: '75a7f30958c7050474d48db09afd4cc7d352ccad',
};

function parseArguments(argv) {
  const input = [];
  let out = null;
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === '--out') {
      out = argv[i + 1];
      i += 1;
    } else {
      input.push(argv[i]);
    }
  }
  return { input, out };
}

async function main() {
  const { input, out } = parseArguments(process.argv.slice(2));
  if (input.length === 0) {
    process.stderr.write('usage: node run.mjs <fixture.bpmn...> [--out DIR]\n');
    process.exit(2);
  }

  const moddle = new BpmnModdle();

  for (const file of input) {
    const xml = await fs.readFile(file, 'utf8');
    const { xml: laidOut, warnings } = await layoutProcess(xml);
    const { rootElement } = await moddle.fromXML(laidOut);

    const shapes = [];
    const edges = [];

    for (const diagram of rootElement.get('diagrams') || []) {
      for (const element of diagram.plane.get('planeElement') || []) {
        if (element.$type === 'bpmndi:BPMNShape') {
          const { x, y, width, height } = element.bounds;
          shapes.push({
            di: element.id,
            bpmnElement: element.bpmnElement.id,
            x: Math.round(x),
            y: Math.round(y),
            width: Math.round(width),
            height: Math.round(height),
          });
        } else if (element.$type === 'bpmndi:BPMNEdge') {
          edges.push({
            di: element.id,
            bpmnElement: element.bpmnElement.id,
            waypoints: (element.waypoint || []).map(({ x, y }) => [
              Math.round(x),
              Math.round(y),
            ]),
          });
        }
      }
    }

    shapes.sort((a, b) => a.bpmnElement.localeCompare(b.bpmnElement));
    edges.sort((a, b) => a.bpmnElement.localeCompare(b.bpmnElement));

    const record = {
      fixture: path.basename(file),
      reference: PIN,
      shapes,
      edges,
      warnings: (warnings || []).map(({ code, elementId, message }) => ({
        code,
        elementId: elementId || null,
        message,
      })),
    };

    const json = `${JSON.stringify(record, null, 2)}\n`;
    if (out) {
      const target = path.join(out, `${path.basename(file, path.extname(file))}.json`);
      await fs.writeFile(target, json, 'utf8');
      process.stdout.write(`${path.basename(file)} -> ${target} (${shapes.length} shapes, ${edges.length} edges)\n`);
    } else {
      process.stdout.write(json);
    }
  }
}

main().catch((error) => {
  process.stderr.write(`${error}\n`);
  process.exit(1);
});
