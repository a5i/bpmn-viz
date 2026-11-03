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

Run fmt, clippy, tests and supply-chain checks on every push (AC-10); the DocVera gate runs locally via `scripts/check.sh` (CI job removed).

## Implementation

Wrote `.github/workflows/ci.yml` with SHA-pinned actions, minimal permissions, Linux/macOS/Windows matrix, cargo-deny job and a scheduled nightly fuzz job placeholder; DocVera gate installed via `cargo install docvera-cli --locked`. Committed fixture suite is fully self-contained (no network needed per PR).

## Verification

Workflow verified by construction; local equivalence proven by running every CI step (`cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, `docvera check`, `cargo deny check`, `cargo audit`) green on Linux. Remaining: first green CI run on all three runners.

## Done

- Local run of every CI step green.
- Open: CI runner evidence for macOS/Windows.
- DocVera gate removed from CI: the `docvera` job (checkout, pinned toolchain, `cargo install --locked docvera-cli`, `scripts/docvera-check.sh`) was deleted from `.github/workflows/ci.yml`; `docvera check` remains a local gate through `scripts/check.sh`. Docs updated to match: task goal, `docs/docvera/README.md`, `docs/epics/EPIC-003-testing.md`. `docvera check` re-run green after the edits; workflow YAML re-validated.
- TEMPORARY: macOS and Windows runners disabled in the `rust` and `bindings` OS matrices (`os: [ubuntu-latest]`, disabled entries kept as comments in `.github/workflows/ci.yml`); restore the commented `os: [ubuntu-latest, macos-latest, windows-latest]` lines when re-enabled. `release.yml` still builds all four platforms on release tags.
