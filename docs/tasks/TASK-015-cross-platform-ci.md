---
id: TASK-015
type: task
title: Cross-platform CI
status: in_progress
relations:
  parent: EPIC-003
  implements: [SPEC-001]

---
# Cross-platform CI

## Goal

Run fmt, clippy, tests, DocVera and supply-chain checks on every push (AC-10).

## Implementation

Wrote `.github/workflows/ci.yml` with SHA-pinned actions, minimal permissions, Linux/macOS/Windows matrix, cargo-deny job and a scheduled nightly fuzz job placeholder; DocVera gate installed via `cargo install docvera-cli --locked`. Committed fixture suite is fully self-contained (no network needed per PR).

## Verification

Workflow verified by construction; local equivalence proven by running every CI step (`cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, `docvera check`, `cargo deny check`, `cargo audit`) green on Linux. Remaining: first green CI run on all three runners.

## Done

- Local run of every CI step green.
- Open: CI runner evidence for macOS/Windows.
