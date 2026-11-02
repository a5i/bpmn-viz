---
id: ADR-0002
type: adr
title: Separate semantic, DI and scene models
status: proposed
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
---
# Separate semantic, DI and scene models

## Context

The BPMN renderer prototype requires a bounded and auditable engineering decision.

## Decision

Preserve independent semantic, diagram-geometry and output-scene layers.

## Alternatives Considered

- Pending evaluation in implementation spike.

## Consequences

- Affects initial scope, implementation and testing contracts.

## Validation

- Confirm with prototype acceptance tests and revisit before accepting ADR.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
