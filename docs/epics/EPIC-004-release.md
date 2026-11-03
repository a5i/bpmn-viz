---
id: EPIC-004
type: epic
title: Documentation and release
status: in_progress
relations:
  implements: [SPEC-001]
  children: [TASK-002, TASK-016, TASK-020]
---
# Documentation and release

## Outcome

DocVera-managed documentation with verified CLI integration, ADRs, license notices, a reproducible release path, and a documented compatibility matrix. Covers the SPEC-001 definition of done.

## Scope

`docvera.toml` + nine document schemas (EXP-003), `scripts/docvera-check.sh`, ADR-0001..0005, architecture notes, development/testing/release guides, license notices, `scripts/release-checks.sh`.

## Acceptance Criteria

- DocVera schema/reference validation passes (`docvera check` = 0 diagnostics).
- DoD: implementation merged, public API documented, tests pass, docs links valid, ADR updated on architecture changes, third-party attributions recorded, CI green.
- Compatibility documented (supported symbols, known gaps — REPORT-001).

## Tasks

- [x] [TASK-002](../tasks/TASK-002-docvera-spike.md) — DocVera CLI/schema spike (done)
- [ ] [TASK-016](../tasks/TASK-016-published-api-docs.md) — published API and docs (in_progress: docs done, crates.io naming/publish pending)
- [ ] [TASK-020](../tasks/TASK-020-node-wasm-bindings.md) — Node.js and WebAssembly bindings (in_progress; ADR-0009 proposal)
