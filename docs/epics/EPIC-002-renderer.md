---
id: EPIC-002
type: epic
title: Renderer
status: done
relations:
  implements: [SPEC-001]
  children: [TASK-006, TASK-007, TASK-008, TASK-009, TASK-010, TASK-011]
---
# Renderer

## Outcome

Scene construction, deterministic SVG serialization, resvg PNG export and the `bpmn` CLI, covering AC-04, AC-05, AC-07, AC-09 and AC-10 of SPEC-001.

## Scope

Crates `bpmn-scene`, `bpmn-render`, `bpmn-svg`, `bpmn-raster`, `bpmn-cli`; glyphs for the SPEC-001 section 3 P0 symbol set plus boundary events; pools/lanes with rotated labels; expanded/collapsed subprocesses; sequence flows with default/conditional decorations; dashed message and association edges; fallback glyphs with `symbol.unsupported` warnings.

## Acceptance Criteria

- AC-04: events, tasks, gateways, arrows and labels render as valid, stable SVG.
- AC-05: collaboration with pool/lanes and expanded subprocess fixtures render.
- AC-07: PNG exports at 1×/2× have expected dimensions and non-background content.
- AC-09: missing DI fails with diagnostics; no hidden layout fallback.
- AC-10: CLI contract per SPEC-001 section 8 (exit codes 0/2/3/4/5).

## Tasks

- [x] [TASK-006](../tasks/TASK-006-scene-primitives.md) — scene primitives (done)
- [x] [TASK-007](../tasks/TASK-007-node-glyphs.md) — node glyphs (done)
- [x] [TASK-008](../tasks/TASK-008-flows-labels.md) — flows/labels (done)
- [x] [TASK-009](../tasks/TASK-009-svg-serializer.md) — SVG serializer (done)
- [x] [TASK-010](../tasks/TASK-010-png-export.md) — PNG export (done)
- [x] [TASK-011](../tasks/TASK-011-cli.md) — CLI (done)
