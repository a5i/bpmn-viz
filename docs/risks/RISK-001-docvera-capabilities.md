---
id: RISK-001
type: risk
title: DocVera capabilities unverified
status: closed
relations:
  addresses: [SPEC-001]
  mitigated_by: [ADR-0001]
---
# DocVera capabilities unverified

## Description

SPEC-001 section 12 required typed, schema-validated Markdown documentation managed through DocVera, but the exact CLI syntax, schema dialect and engine capabilities were unknown at specification time. Inventing commands would have broken CI.

## Likelihood

Certain if unaddressed (guessed syntax cannot work); resolved before any CI wiring.

## Impact

High: the whole documentation-as-code workflow (SPEC, ADR, EPIC, TASK, TEST, REPORT, RISK lifecycle) depended on it.

## Mitigation

Executed EXP-003 against the installed `docvera 0.1.2` CLI: inspected the embedded manual, authored `docvera.toml` and nine schemas with verified DSL features, validated 33 documents with `docvera check`, and wired `scripts/docvera-check.sh` into CI. Engine limitations (no transition enforcement) are documented rather than guessed around.

## Outcome

Closed: integration verified end-to-end; residual gap (transition enforcement via SHACL) is tracked in REPORT-001 next actions.
