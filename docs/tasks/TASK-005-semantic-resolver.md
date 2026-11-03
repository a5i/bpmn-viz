---
id: TASK-005
type: task
title: Semantic resolver and diagnostics
status: done
relations:
  parent: EPIC-001
  implements: [SPEC-001]

---
# Semantic resolver and diagnostics

## Goal

Validate references and geometry with stable diagnostic codes and severities (AC-02, AC-09).

## Implementation

Implemented `bpmn-validate`: id registry over processes/subprocess bodies, duplicate id detection, dangling sequence-flow/participant/message-flow/lane checks, foreign-scope warnings, DI shape/edge reference resolution (including association-flavored unsupported elements), boundary `attachedToRef` checks, invalid geometry, and DI coverage requiring shapes for flow nodes, containers and DI-referenced data shapes — with collapsed-subprocess exemption.

## Verification

8 unit tests in `bpmn-validate` (duplicate ids, dangling refs, missing DI, missing shapes, NaN bounds, collapsed exemption) plus CLI integration tests with the `invalid/*` fixtures.

## Done

- `cargo test -p bpmn-validate` green.
- `bpmn validate` exit 3 with stable codes for all invalid fixtures.
