---
id: TASK-012
type: task
title: MIWG fixture manifest
status: done
relations:
  parent: EPIC-003
  implements: [SPEC-001]

---
# MIWG fixture manifest

## Goal

Pin external MIWG fixtures with provenance per ADR-0005 and smoke the pipeline against real files.

## Implementation

Wrote `scripts/download-fixtures.sh` and `tests/manifests/external-fixtures.lock.json` pinning upstream revision `85240b8689d835d8bd1a96fbea6af46558ca0568` (2026-10-18) with per-file SHA-256 for A.1.0, A.2.0, A.3.0, A.4.0, A.4.1, C.4.0, C.8.0. Files live in gitignored `external/`; findings drove boundary-event and data-shape support (REPORT-001).

## Verification

`bpmn validate` returns 0 errors for all seven files against the pinned revision; results recorded in REPORT-001.

## Done

- Manifest hashes verified with `sha256sum` after download.
- Attribution (CC-BY-3.0) kept in manifest and release guide.
