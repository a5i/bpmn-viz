---
id: RISK-005
type: risk
title: Malformed input or large geometry exhausting resources
status: mitigated
relations:
  addresses: [SPEC-001]
  mitigated_by: [ADR-0001]
---
# Malformed input or large geometry exhausting resources

## Description

Untrusted BPMN XML could crash the renderer (deep nesting), exhaust memory (huge files, NaN/huge coordinates) or leak file contents (entity expansion).

## Likelihood

Medium for accidental malformed files; high for hostile input without protections.

## Impact

High: CLI crashes or hangs are prototype-blocking; content leaks are security-relevant.

## Mitigation

- Hard input limits: 64 MiB byte cap and 512 depth cap (`xml.size_limit`, `xml.depth_limit`).
- No entity expansion: undefined entities are rejected as syntax errors; DOCTYPE internal subsets are never resolved (`tests/fixtures/malformed/xxe.bpmn` proves `/etc/passwd` never appears in output).
- Geometry numbers are parsed as doubles and validated (`geometry.invalid`) instead of trusted; renderer outputs use `format_number` with non-finite → 0 fallback.
- PNG export enforces width/height/total pixel budgets (`RasterError::TooLarge`), demonstrated by the 1,000-element bench row refusing a 140096x6960 output.
- `fuzz/` harnesses exist for `xml_parse` and `render_document`; cargo-deny and cargo-audit are clean (REPORT-001).

## Outcome

Mitigated for the prototype; residual risk is that libfuzzer execution evidence is still pending (TASK-014).
