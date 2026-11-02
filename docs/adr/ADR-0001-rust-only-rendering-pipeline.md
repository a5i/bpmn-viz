---
id: ADR-0001
type: adr
title: Pure Rust rendering pipeline
status: proposed
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
---
# Pure Rust rendering pipeline

## Context

The BPMN renderer prototype requires a bounded and auditable engineering decision.

## Decision

Use Rust for parsing, scene representation, SVG serialization and PNG rasterization; do not bundle bpmn-js.

## Alternatives Considered

- Pending evaluation in implementation spike.

## Consequences

- Affects initial scope, implementation and testing contracts.

## Validation

- Confirm with prototype acceptance tests and revisit before accepting ADR.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
