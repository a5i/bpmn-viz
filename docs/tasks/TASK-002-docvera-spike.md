---
id: TASK-002
type: task
title: DocVera CLI and schema spike
status: done
relations:
  parent: EPIC-004
  implements: [SPEC-001]
  decisions: []

---
# DocVera CLI and schema spike

## Goal

Resolve RISK-001 by verifying the installed DocVera CLI and defining the repository's document schemas without invented syntax.

## Implementation

Inspected `docvera guide` (embedded manual) of the installed `docvera 0.1.2`; wrote `docvera.toml` and nine schemas under `docs/docvera/schemas/`; added frontmatter to previously untyped docs; fixed H1/title mismatches the checker reported; verified exit codes 0/1/2 by exercising real failures.

## Verification

`docvera check` returns `Checked 15 documents; 0 diagnostics`, exit 0. Full evidence recorded in [EXP-003](../experiments/EXP-003-docvera-integration.md).

## Done

- DocVera contract documented in `docs/docvera/README.md`.
- `scripts/docvera-check.sh` added and used by CI.
