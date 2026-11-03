---
id: TASK-003
type: task
title: BPMN XML namespaces
status: done
relations:
  parent: EPIC-001
  implements: [SPEC-001]

---
# BPMN XML namespaces

## Goal

Import BPMN XML by namespace URI, never by literal prefix (AC-01), with bounded and entity-safe parsing.

## Implementation

Implemented `bpmn-xml` on `quick-xml` `NsReader`: `read_resolved_event` + `resolver()` for element/attribute namespace resolution, `xsi:type` resolution (including prefixes bound to BPMN in scope), default-namespace documents, UTF-8/BOM handling, 64 MiB and depth-512 limits, and rejection of undefined entities. Modeled boundary events, ioSpecification data inputs/outputs, data shapes and association flows for id resolution.

## Verification

14 unit tests in `crates/bpmn-xml/src/parser.rs` cover arbitrary prefixes, `xsi:type` overrides, event definitions, subprocess bodies, collaborations, malformed input, entity rejection, size/depth limits and Unicode escapes. Integration test `arbitrary_prefixes_parse_and_render` covers the CLI path.

## Done

- `cargo test -p bpmn-xml` green.
- Fixture `tests/fixtures/edge-cases/custom-prefixes.bpmn` renders with default-flow tick and conditional diamond.
