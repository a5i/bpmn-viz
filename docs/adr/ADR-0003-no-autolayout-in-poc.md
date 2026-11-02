---
id: ADR-0003
type: adr
title: No automatic layout in PoC
status: proposed
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
---
# No automatic layout in PoC

## Context

The BPMN renderer prototype requires a bounded and auditable engineering decision.

## Decision

Require BPMN DI for rendering; no inferred placement in the prototype.

## Alternatives Considered

- Pending evaluation in implementation spike.

## Consequences

- Affects initial scope, implementation and testing contracts.

## Validation

- Confirm with prototype acceptance tests and revisit before accepting ADR.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
