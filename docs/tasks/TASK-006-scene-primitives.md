---
id: TASK-006
type: task
title: Scene primitives
status: done
relations:
  parent: EPIC-002
  implements: [SPEC-001]

---
# Scene primitives

## Goal

Provide a renderer-agnostic scene model with deterministic paint order (ADR-0002).

## Implementation

Implemented `bpmn-scene`: `Scene` with viewport/defs/nodes, `Node` variants (group/path/rect/ellipse/text), styles, markers, text anchor, approximate wrapping, and `kurbo`-based path helpers (polyline, diamond, polygon, annotation bracket).

## Verification

9 unit tests in `bpmn-scene` cover ordering, grouped pushes, transform wrapping and wrap behavior.

## Done

- `cargo test -p bpmn-scene` green.
- Scene types carry no BPMN concepts (ADR-0002 boundary respected).
