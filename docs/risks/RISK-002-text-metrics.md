---
id: RISK-002
type: risk
title: Text metrics instability across environments
status: mitigated
relations:
  addresses: [SPEC-001]
  mitigated_by: [ADR-0004]
---
# Text metrics instability across environments

## Description

Font metrics vary across platforms; label wrapping and pixel output could differ between environments, breaking deterministic output claims and golden tests.

## Likelihood

High if glyph metrics were used; medium as mitigated (resvg fontdb fallbacks differ per OS).

## Impact

Medium: visual differences are acceptable per ADR-0004, but non-deterministic *structure* would break tests and reproducibility.

## Mitigation

Scene construction uses no font loader at all: wrapping uses a fixed 0.6 × font-size average width (EXP-002), DI label bounds take precedence, and golden tests pin the SVG (not pixels). PNG tests assert dimensions and non-background content only; cross-platform pixel equality is explicitly not asserted. resvg resolves `sans-serif` via fontdb with DejaVu Sans preferred where installed.

## Outcome

Mitigated: deterministic structure proven by tests; residual visual variance across OSes is documented and accepted for the prototype.
