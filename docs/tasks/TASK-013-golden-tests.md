---
id: TASK-013
type: task
title: Golden tests
status: done
relations:
  parent: EPIC-003
  implements: [SPEC-001]

---
# Golden tests

## Goal

Pin deterministic output and layered regression tests per ADR-0004.

## Implementation

Committed the reviewed golden SVG for the minimal fixture; layered integration tests assert structural SVG properties, PNG dimensions/content at 1x/2x, Unicode escapes and negative-coordinate viewports.

## Verification

`golden_svg_is_stable` pins byte-exact output (caught one real serializer drift during development); 4 render integration tests + 17 CLI tests green.

## Done

- `cargo test --workspace` green (18 suites).
- Golden file reviewed against rendered output.
