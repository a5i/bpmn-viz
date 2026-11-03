---
id: EPIC-003
type: epic
title: Quality
status: in_progress
relations:
  implements: [SPEC-001]
  children: [TASK-001, TASK-012, TASK-013, TASK-014, TASK-015, TASK-019]
---
# Quality

## Outcome

A quality gate that makes regressions loud: layered tests, MIWG conformance against pinned fixtures, fuzzing harnesses, supply-chain checks and CI. Covers AC-08, AC-10 and the SPEC-001 section 10 quality gates.

## Scope

Fixture suite under `tests/fixtures/`, external MIWG pinning under `tests/manifests/`, `fuzz/` harnesses, `deny.toml` + `cargo audit`, CI workflow. MIWG smoke results are recorded in [REPORT-001](../reports/REPORT-001-conformance-baseline.md).

## Acceptance Criteria

- AC-08: Unicode labels, XML escaping, negative/large coordinates and broken inputs do not crash.
- AC-10: CLI verified on the development platform; CI runs fmt/clippy/tests/DocVera per PR.
- Performance baseline measured and recorded (REPORT-001).

## Tasks

- [x] [TASK-001](../tasks/TASK-001-repo-bootstrap.md) — repository bootstrap (done)
- [x] [TASK-012](../tasks/TASK-012-miwg-manifest.md) — MIWG fixture manifest (done)
- [x] [TASK-013](../tasks/TASK-013-golden-tests.md) — golden tests (done)
- [ ] [TASK-014](../tasks/TASK-014-fuzz-security.md) — fuzz/security (in_progress: harnesses authored, libfuzzer runs pending nightly CI)
- [ ] [TASK-015](../tasks/TASK-015-cross-platform-ci.md) — cross-platform CI (in_progress: workflow written, first non-Windows-runner run pending)
- [ ] [TASK-019](../tasks/TASK-019-external-corpus-manifests.md) — external corpus manifests (in review: MIWG full set + PMo pinned, smoke baseline in REPORT-002; CODECS rejected after audit)
