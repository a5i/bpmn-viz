---
id: ADR-0002
type: adr
title: Separate semantic, DI and scene models
status: accepted
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
  informed_by: [EXP-001]
---
# Separate semantic, DI and scene models

## Context

BPMN semantics and Diagram Interchange (DI) are different concerns (SPEC-001 section 2), and a renderer that conflates them becomes hard to test, hard to extend to new output formats, and prone to inventing geometry.

## Decision

Keep three independent models with explicit boundaries: `bpmn-model` (semantic elements + DI types + diagnostics), `bpmn-scene` (renderer-agnostic primitives), and stage crates between them (`bpmn-xml` import, `bpmn-validate` checks, `bpmn-render` scene construction, `bpmn-svg`/`bpmn-raster` output). The scene graph carries no BPMN ids except optional stable metadata (`id` attributes).

## Alternatives Considered

- **Single AST rendered directly to SVG:** fewer crates, but validation, alternate outputs and testing of geometry rules all collapse into one pass. Rejected.
- **Adopt ebi_bpmn's model for semantics:** behavioral focus, no DI, source-order lossy (EXP-001). Rejected.

## Consequences

- Each stage can be tested in isolation (90+ unit and integration tests target stages, not the whole binary).
- New output formats (PDF, HTML) would only add a scene consumer.
- More types and more files; mitigated by small, focused crates.

## Validation

- The render pipeline passes fixture and MIWG integration tests; the golden SVG test (`tests/fixtures/minimal/start-task-end.svg`) pins scene→SVG output while parser tests pin semantics independently.
- Element IDs remain stable in SVG output (`id="bpmn-shape-<id>"`, `id="bpmn-edge-<id>"`), asserted by integration tests.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
- [EXP-001](../experiments/EXP-001-ebi-bpmn-evaluation.md)
