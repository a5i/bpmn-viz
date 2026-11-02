---
id: ADR-0004
type: adr
title: Use structural and golden visual tests
status: proposed
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
---
# Use structural and golden visual tests

## Context

The BPMN renderer prototype requires a bounded and auditable engineering decision.

## Decision

Combine structural assertions, geometry checks and reviewed raster goldens.

## Alternatives Considered

- Pending evaluation in implementation spike.

## Consequences

- Affects initial scope, implementation and testing contracts.

## Validation

- Confirm with prototype acceptance tests and revisit before accepting ADR.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
