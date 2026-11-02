---
id: ADR-0005
type: adr
title: Pin third-party fixtures
status: proposed
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
---
# Pin third-party fixtures

## Context

The BPMN renderer prototype requires a bounded and auditable engineering decision.

## Decision

Download external licensed datasets into ignored directory; pin revisions and attribution in manifest.

## Alternatives Considered

- Pending evaluation in implementation spike.

## Consequences

- Affects initial scope, implementation and testing contracts.

## Validation

- Confirm with prototype acceptance tests and revisit before accepting ADR.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
