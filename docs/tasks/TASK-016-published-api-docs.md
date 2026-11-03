---
id: TASK-016
type: task
title: Published API and docs
status: in_progress
relations:
  parent: EPIC-004
  implements: [SPEC-001]

---
# Published API and docs

## Goal

Document the public API and prepare crates/docs publication (DoD).

## Implementation

Documented all public APIs (crate-level docs with pipeline overview plus item docs), wrote architecture notes, development/testing/release guides, ADR-0001..0005 accepted with validation evidence, EXP-001..003 concluded, REPORT-001 final with compatibility matrix. Crates remain `publish = false` until crates.io names are confirmed (SPEC-001 open question).

## Verification

`cargo doc --workspace --no-deps` builds without warnings; `docvera check` passes; publication blocked on the crates.io namespace check (recorded as open).

## Done

- Docs complete and validated.
- Open: crates.io publication + docs.rs render check.
