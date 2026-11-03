---
id: EPIC-001
type: epic
title: Parser and DI
status: done
relations:
  implements: [SPEC-001]
  children: [TASK-003, TASK-004, TASK-005]
---
# Parser and DI

## Outcome

A namespace-aware importer and validator that turns BPMN XML into a typed semantic + DI model with structured diagnostics, covering AC-01, AC-02, AC-03 and AC-06 of SPEC-001.

## Scope

Crates `bpmn-model`, `bpmn-xml`, `bpmn-validate`; synthetic fixtures for minimal, invalid and edge cases; support for boundary events, data shapes and associations at id/DI resolution level (see [TASK-005](../tasks/TASK-005-semantic-resolver.md)).

## Acceptance Criteria

- AC-01: arbitrary namespace prefixes and `xsi:type` handling proven by `tests/fixtures/edge-cases/custom-prefixes.bpmn` and unit tests.
- AC-02: duplicate ids, dangling references and non-finite bounds detected with stable codes.
- AC-03: DI shape/edge ids resolve to semantic elements; geometry preserved verbatim.
- AC-06: multi-diagram documents enumerable and selectable by index or id.

## Tasks

- [x] [TASK-003](../tasks/TASK-003-xml-namespaces.md) — BPMN XML namespaces (done)
- [x] [TASK-004](../tasks/TASK-004-di-indexing.md) — DI indexing (done)
- [x] [TASK-005](../tasks/TASK-005-semantic-resolver.md) — semantic resolver and diagnostics (done)
