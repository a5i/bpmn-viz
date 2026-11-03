---
id: TASK-014
type: task
title: Fuzz and security
status: in_progress
relations:
  parent: EPIC-003
  implements: [SPEC-001]

---
# Fuzz and security

## Goal

Protect against malformed and hostile input (AC-08, RISK-005).

## Implementation

Authored `fuzz/fuzz_targets/{xml_parse.rs,render_document.rs}` (libfuzzer harnesses, nightly-only, not built by the workspace), added hard limits in the parser (size/depth), entity rejection, and XXE fixture `tests/fixtures/malformed/xxe.bpmn` proving no expansion; ran `cargo deny check` and `cargo audit` locally (see REPORT-001).

## Verification

`cargo deny check` and `cargo audit` pass locally; malformed-input CLI tests assert no crash and no content leak. Remaining: execute libfuzzer targets on a nightly CI job and record corpus findings.

## Done

- `cargo deny check` and `cargo audit` clean locally.
- Open: libfuzzer execution evidence.
