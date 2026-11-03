---
id: TASK-009
type: task
title: SVG serializer
status: done
relations:
  parent: EPIC-002
  implements: [SPEC-001]

---
# SVG serializer

## Goal

Serialize scenes to byte-deterministic SVG (AC-04).

## Implementation

Implemented the owned `bpmn-svg` writer: fixed attribute order, trimmed numbers (3 decimals), XML escaping for text and attributes, marker defs, stable element ids, no timestamps. Evaluated and rejected the `svg` crate during this task (recorded in ADR-0001).

## Verification

5 unit tests cover number trimming, escaping, determinism, groups/transforms and path data; the golden test `golden_svg_is_stable` pins the full minimal output byte-for-byte.

## Done

- `cargo test -p bpmn-svg` green.
- Golden file committed at `tests/fixtures/minimal/start-task-end.svg`.
