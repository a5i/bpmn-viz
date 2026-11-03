---
id: ADR-0003
type: adr
title: DI-required rendering and no automatic layout in the prototype
status: accepted
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
---
# DI-required rendering and no automatic layout in the prototype

## Context

BPMN DI supplies all geometry. Automatic layout of models without DI is a research-scale problem (layering, edge routing, label placement) that would dominate the prototype's budget and produce low-quality diagrams for real models.

## Decision

The renderer consumes provided DI geometry verbatim. Models without `bpmndi:BPMNDiagram`, elements without shapes, or edges without waypoints yield structured diagnostics (`di.missing_interchange`, `di.missing_shape`, `di.missing_waypoints`) and exit code 3 — never a hidden layout fallback. Data shapes that the DI references must have geometry; unreferenced data-shape definitions are inert.

## Alternatives Considered

- **Best-effort grid layout as fallback:** hides authoring problems, produces misleading diagrams, and violates the "conspicuous failure" requirement (AC-09). Rejected.
- **Third-party layout crate (elk/rust-sugiyama):** new dependency class, non-determinism risk, out of prototype scope. Deferred to a future ADR if no-DI support becomes a requirement.

## Consequences

- MIWG-style files with complete DI render correctly; broken or missing DI fails loudly and early (`bpmn validate` gives the same answer as `bpmn render`).
- Supported-element coverage had to be wide enough for real files: boundary events, data shapes and association edges are modeled for id/DI resolution so that real MIWG files do not fail on P1 elements (see REPORT-001).

## Validation

- `tests/fixtures/invalid/missing-di.bpmn` produces `di.missing_interchange` and render exits 3 (integration test `missing_diagram_interchange_is_error`).
- All seven selected MIWG files (A.1.0, A.2.0, A.3.0, A.4.0, A.4.1, C.4.0, C.8.0) validate with 0 errors against pinned revisions in `external/` (REPORT-001).

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
- [REPORT-001](../reports/REPORT-001-conformance-baseline.md)
