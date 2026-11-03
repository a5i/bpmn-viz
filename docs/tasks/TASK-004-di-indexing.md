---
id: TASK-004
type: task
title: DI indexing
status: done
relations:
  parent: EPIC-001
  implements: [SPEC-001]

---
# DI indexing

## Goal

Model BPMN DI (diagrams, planes, shapes, edges, bounds, waypoints, labels) with finiteness validation and stable ordering.

## Implementation

Implemented `bpmn-model::di` types with `Bounds::is_valid` (finite, positive width/height), waypoint finiteness, DI label bounds, `isHorizontal`/`isExpanded` attributes, and source-order preservation of diagrams, shapes and edges for stable selection (AC-06).

## Verification

Unit tests in `bpmn-model` and `bpmn-validate` cover NaN/infinite bounds, waypoint counts, union/pad geometry and diagram selection by index/id (`ac06_diagrams_are_enumerable_and_selectable`).

## Done

- `cargo test -p bpmn-model` green.
- `tests/fixtures/edge-cases/multi-diagram.bpmn` selects consistently by index and id.
