---
id: EXP-001
type: experiment
title: Evaluate ebi_bpmn as a parsing base
status: concluded
relations:
  informs: [SPEC-001]
  decides: [ADR-0002]
---
# Evaluate ebi_bpmn as a parsing base

## Hypothesis

`ebi_bpmn` could replace a custom parser for the semantic layer.

## Setup

Reviewed the published `ebi_bpmn` documentation (docs.rs) against this prototype's requirements: namespace-aware QName resolution, Diagram Interchange (DI) geometry, ordered diagrams, typed diagnostics and tolerance for unknown vendor extensions.

## Results

- `ebi_bpmn` focuses on *behavioral* BPMN models (executions semantics); DI support is not part of its contract, which SPEC-001 section 16 anticipated.
- Its public model is shaped for analysis, not for lossless retention of source order, DI selection or structured diagnostics.
- Embedding it would have forced a second modeling layer for DI and a translation step with no offsetting benefit for a read-only renderer.

## Conclusion

Not adopted. The prototype keeps its own namespace-aware importer (`bpmn-xml`, quick-xml based) and its own DI model (`bpmn-model`), separated per ADR-0002. `ebi_bpmn` remains a reference for behavioral semantics if execution semantics are ever in scope.
