---
id: ADR-0008
type: adr
title: External test corpora under the pinned-manifest policy
status: proposed
created: 2026-11-03
relations:
  implements: [SPEC-001]
  supersedes: []
---
# External test corpora under the pinned-manifest policy

## Context

The quality program (EPIC-003) currently pins only seven MIWG files (TASK-012), which is too narrow for parser robustness on real-world models, interoperability claims, and layout-quality evaluation (EPIC-005). A corpus audit on 2026-11-03 verified candidate datasets against the MIT tree and the ADR-0005 policy (nothing third-party committed; external content gitignored and pinned):

- **BPMN MIWG** (CC-BY-3.0, attribution already recorded): full `Reference` tree spans dozens of models and many tool exports; only 7 files are pinned today.
- **PMo Dataset** v1.0.0 (Zenodo `10.5281/zenodo.15857589`, license `cc-by-4.0` verified via Zenodo API, rev 10): 55 models, 1 MB, each as ground-truth BPMN **with** DI plus a `BPMN process` variant **without** the diagram definition — a direct Preserve-vs-Generate evaluation pair for EPIC-005. Citation of the AI4BPM 2025 paper required.
- **CODECS T3.3 process models** v1 (Zenodo `10.5281/zenodo.16372882`, license `cc-by-4.0` verified via Zenodo API, rev 4): **rejected after download audit (2026-11-03)** — the 52 MB archive contains zero machine-readable BPMN; all 127 real files are raster exports (PNG/JPG), one guide PDF and metadata.
- **Rejected after audit:** BPMAI (29,810 models but JSON in Signavio/PromniCat format, not BPMN XML; "research purposes" licensing with NC site terms; would need a converter project), Hugging Face `pritamdeka/bpmn-image-xml-pairs` (gated access, license not visible before acceptance, documented split leakage; pixel-level visual regression would also violate ADR-0004), Fundamentals-of-BPM textbook models (site under CC BY-NC-SA 3.0), Flowable/Camunda vendor samples (deferred; vendor extensions out of the supported profile for now), `bpmn-moddle`/`bpmn-js`/`bpmn-auto-layout` fixtures (MIT but redundant with project-owned synthetic fixtures and the pinned reference-JSON differential).

## Decision

Adopt two corpora — **the full MIWG `Reference` tree** and **PMo Dataset v1.0.0** — strictly through the ADR-0005 mechanism: pinned upstream revision and per-file checksums in `tests/manifests/*.lock.json`, scripted download into gitignored `external/`, runs local or scheduled only, per-PR CI stays synthetic-only. Attribution and citation obligations (CC-BY-3.0 / CC-BY-4.0) are recorded in the manifests and the release guide. Dataset provenance lives in manifests and reports — not in new DocVera document types; the DocVera contract (DOC-002) fixes nine schemas and stays unchanged.

## Alternatives Considered

- **Commit corpora into the repository:** mixes CC-BY/NC content into an MIT tree and bloats history; rejected per ADR-0005.
- **Fetch corpora per PR in CI:** network flakiness and upstream drift; rejected per ADR-0005 (scheduled/local only).
- **Adopt BPMAI as a mass parser corpus:** JSON, not BPMN XML — a converter (PromniCat/jBPT ecosystem) is a project of its own; research-only licensing; rejected for now, revisit only with an explicit converter task.
- **Adopt HF image/XML pairs for visual regression:** gated, license unknown, split leakage documented upstream; pixel-equality testing violates ADR-0004; rejected.
- **Adopt CODECS T3.3 as a real-world corpus:** license acceptable (CC-BY-4.0), but the archive carries no machine-readable BPMN (raster exports only); rejected after the download audit recorded in REPORT-002.
- **A DocVera `test-dataset` document type:** the engine has exactly nine schemas with `additionalProperties: false`; inventing types would violate the verified integration contract; rejected.

## Consequences

- `tests/manifests/external-fixtures.lock.json` grows to the full MIWG `Reference` tree at the pinned revision; PMo joins as an additional dataset with archive and per-extracted-file checksums (the upstream Zenodo MD5 is pinned alongside our SHA-256).
- The PMo corpus is consumed through a gated suite (`crates/bpmn-layout/tests/corpus_pmo.rs`) that self-skips without `external/`, keeping per-PR CI synthetic-only; corpus-backed claims cite pinned revisions via [REPORT-002](../reports/REPORT-002-external-corpus-smoke.md).
- Layout-quality and robustness claims in reports become citable to exact corpus revisions; the Preserve regression (ADR-0007) and synthetic-only per-PR CI are unchanged.
- Attribution/citation text must be kept in manifests and `docs/guides/release.md`; CC-BY obligations never stripped.
- Corpus bytes never enter Git; manifest verification (`sha256sum -c`, script-side digest checks) is the integrity gate.

## Validation

- Licenses verified 2026-11-03 from primary sources: PMo `cc-by-4.0` and CODECS `cc-by-4.0` from the Zenodo API (revisions 10 and 4); MIWG CC-BY-3.0 already verified in TASK-012; rejections carry the observed blockers recorded above.
- Executed 2026-11-03 (TASK-019): 133 files pinned, `external/` reproduced from scratch by the generalized script, and [REPORT-002](../reports/REPORT-002-external-corpus-smoke.md) records the smoke baseline (MIWG 16/22 validate-clean with strict findings on 6; PMo Preserve render 55/55; PMo no-DI layout 43/55 invariant-clean and deterministic, 12 typed `layout.routing_failed`). Promotion to accepted cites this report as evidence.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
- [ADR-0005](ADR-0005-third-party-fixture-policy.md), [ADR-0004](ADR-0004-golden-visual-tests.md), [ADR-0007](ADR-0007-generated-di-and-layout-modes.md)
- [EPIC-003](../epics/EPIC-003-testing.md), [EPIC-005](../epics/EPIC-005-auto-layout.md)
- [TASK-012](../tasks/TASK-012-miwg-manifest.md), [TASK-019](../tasks/TASK-019-external-corpus-manifests.md)
- [TEST-001](../testing/TEST-001-test-plan.md), [TEST-005](../testing/TEST-005-layout-test-plan.md), [REPORT-001](../reports/REPORT-001-conformance-baseline.md), [REPORT-002](../reports/REPORT-002-external-corpus-smoke.md)
- PMo Dataset: <https://doi.org/10.5281/zenodo.15857589> (v1.0.0, CC-BY-4.0)
- CODECS T3.3 Process Models: <https://doi.org/10.5281/zenodo.16372882> (audited 2026-11-03; rejected — no machine-readable BPMN in the archive)
- BPMN MIWG test suite: <https://github.com/bpmn-miwg/bpmn-miwg-test-suite> (CC-BY-3.0)
