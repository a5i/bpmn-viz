---
id: TASK-019
type: task
title: External corpus manifests (full MIWG, PMo)
status: review
relations:
  parent: EPIC-003
  implements: [SPEC-001]
  decisions: [ADR-0008]
  verified_by: [TEST-001, TEST-005]
---
# External corpus manifests (full MIWG, PMo)

## Goal

Pin the corpora adopted by [ADR-0008](../adr/ADR-0008-external-corpus-manifests.md) — the full MIWG `Reference` tree and PMo Dataset v1.0.0 — under the ADR-0005 mechanism, and record a corpus smoke baseline so corpus-backed claims cite exact upstream revisions.

> Scope note: CODECS T3.3 was adopted by the original ADR draft but **dropped after the download audit**: its archive contains no machine-readable BPMN (raster exports only; evidence in [REPORT-002](../reports/REPORT-002-external-corpus-smoke.md) §Gaps).

## Implementation

- **Manifest:** `tests/manifests/external-fixtures.lock.json` now pins two datasets with per-file SHA-256: MIWG `Reference` expanded from 7 to all 22 `.bpmn` files at revision `85240b8689d835d8bd1a96fbea6af46558ca0568` (enumerated from the revision via the GitHub tree API; the original 7 digests verified unchanged), and PMo `pmo-dataset.zip` (Zenodo v1.0.0) with archive SHA-256 + upstream MD5 and digests for 111 extracted files (`bpmn/`, `bpmn_process/`, `README.md`).
- **Download tooling:** `scripts/download-fixtures.sh` generalized to iterate all `datasets[]` (github-raw and zenodo-archive fetchers, digest-verified cache, full verification pass); end-to-end reproduction proven by deleting `external/` and rebuilding (133 files).
- **Corpus suite:** `crates/bpmn-layout/tests/corpus_pmo.rs` — gated on `external/pmo/` (self-skips without it, so per-PR CI stays synthetic-only per ADR-0005); covers PMo no-DI layout totality (typed failures only, no panics), TEST-005 geometry invariants, double-run canonical determinism, and Preserve rendering of the with-DI variants.
- **Attribution:** MIWG CC-BY-3.0 and PMo CC-BY-4.0 attribution/citation recorded in the manifest and `docs/guides/release.md`.
- **Report:** [REPORT-002](../reports/REPORT-002-external-corpus-smoke.md) records pinned revisions, per-corpus validate/render/layout outcomes and the audit findings.

## Acceptance Criteria

- [x] Both corpora reproduce from the manifest via `scripts/download-fixtures.sh` with digest verification; no corpus bytes are committed to Git (`external/` gitignored).
- [x] MIWG manifest covers every `.bpmn` file of the pinned `Reference` revision (22) with per-file SHA-256.
- [x] PMo pinned with archive and extracted-file checksums; citation and CC-BY attribution present in the manifest and release guide.
- [x] Smoke results recorded in REPORT-002: MIWG 16/22 validate-clean (6 strict findings), PMo Preserve render 55/55, PMo no-DI layout 43/55 invariant-clean and deterministic (12 typed `layout.routing_failed`), Preserve regression green.
- [x] Per-PR CI remains synthetic-only (corpus suite self-skips without `external/`); `cargo test --workspace`, `scripts/docvera-check.sh`, `cargo deny check` green at implementation time.

## Verification

Executed 2026-11-03 on the author workstation:

- `rm -rf external && scripts/download-fixtures.sh` — 133/133 files verified against the manifest; rerun served from digest-verified cache.
- `bpmn validate` loops: MIWG 16/22 clean; PMo with-DI 18/55 clean (37 × `di.dangling_element_ref` — corpus-internal DI inconsistency, verified on 01.bpmn); PMo no-DI 55/55 `di.missing_interchange` (Preserve contract).
- `cargo test -p bpmn-layout --test corpus_pmo` — 2 passed: no-DI 43/55 laid out (failures: 12 × `layout.routing_failed`), with-DI 55/55 rendered via Preserve.
- CLI gates: `bpmn render` refuses validate-dirty files (exit 3, no output); clean sample 06.bpmn renders SVG 2606×240 + PNG (2 `raster.font_substituted` warnings, RISK-002).
- `scripts/docvera-check.sh` — `Checked 56 documents; 0 diagnostics`; `cargo deny check` — advisories/bans/licenses/sources ok; `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean (final gate runs, see Done).

## Done

- Working-tree change on top of the TASK-018 review state: manifest + script + corpus suite + ADR-0008 (amended: CODECS rejected) + TASK-019 + REPORT-002 + guide/index updates. PR and review pending; ADR-0008 promotion to accepted uses REPORT-002 as Validation evidence.
