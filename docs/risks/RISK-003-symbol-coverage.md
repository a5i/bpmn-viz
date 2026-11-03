---
id: RISK-003
type: risk
title: BPMN symbol coverage larger than expected
status: mitigated
relations:
  addresses: [SPEC-001]
  mitigated_by: [ADR-0003]
---
# BPMN symbol coverage larger than expected

## Description

Real-world BPMN files (MIWG) contain many elements beyond the P0 glyph set; silently dropping them would corrupt diagrams or crash the pipeline.

## Likelihood

High — confirmed during the MIWG smoke (boundary events, data shapes, ioSpecification, associations all appear in the selected A/C cases).

## Impact

Medium: missing coverage would produce validation errors or incomplete diagrams for real files.

## Mitigation

Unknown elements are skipped with an `import.ignored_element` warning instead of failing; recognized-but-unrendered elements (data shapes, associations, callActivity/transaction) are modeled for id/DI resolution and render as conspicuous fallback glyphs with `symbol.unsupported` warnings; boundary events received full glyphs because the selected MIWG set uses them. The compatibility matrix in REPORT-001 publishes exact coverage; unsupported symbols are always visible in output.

## Outcome

Mitigated: all seven selected MIWG files validate with 0 errors and render with explicit, documented warnings.
