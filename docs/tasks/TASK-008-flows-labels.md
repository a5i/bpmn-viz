---
id: TASK-008
type: task
title: Flows and labels
status: done
relations:
  parent: EPIC-002
  implements: [SPEC-001]

---
# Flows and labels

## Goal

Render sequence flows with DI waypoints and decorations, and lay out labels deterministically.

## Implementation

Implemented `bpmn-render::flows` (waypoint polylines, arrowhead marker, default-flow slash tick, conditional diamond, dashed message/association edges) and `bpmn-render::labels` (DI label bounds precedence, inside-center labels for tasks/subprocesses, below-shape labels for events/gateways, rotated pool/lane names, midpoint flow labels, annotation text).

## Verification

Unit tests cover waypoint edge cases, decoration markers and label positions; integration tests assert `marker-start/end` and label escapes in rendered SVG.

## Done

- `cargo test -p bpmn-render` green.
- Fixture `custom-prefixes.bpmn` renders default tick + conditional diamond (asserted).
