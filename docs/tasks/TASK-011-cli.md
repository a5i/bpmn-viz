---
id: TASK-011
type: task
title: CLI
status: done
relations:
  parent: EPIC-002
  implements: [SPEC-001]

---
# CLI

## Goal

Expose `render`, `inspect` and `validate` per SPEC-001 section 8 with documented exit codes (AC-09, AC-10).

## Implementation

Implemented `bpmn-cli` with clap: format/extension conflict detection, `--scale` restricted to PNG, diagram selection by index or id, first-diagram default with warning, atomic writes (temp file + rename, Windows-safe replacement), stdout SVG via `-`, text and JSON reports with stable diagnostic codes, and exit codes 0/2/3/4/5.

## Verification

17 integration tests in `crates/bpmn-cli/tests/cli.rs` cover the full contract including error paths; unit tests cover DTO serialization and exit-code mapping.

## Done

- `cargo test -p bpmn-cli` green (17 CLI integration tests).
- Manual smoke on all fixtures and MIWG files.
