---
id: REPORT-002
type: report
title: External corpus smoke baseline (MIWG full set, PMo)
status: final
relations:
  based_on: [TEST-001, TEST-005, EPIC-003]
---
# External corpus smoke baseline (MIWG full set, PMo)

## Method

Corpora pinned per [ADR-0008](../adr/ADR-0008-external-corpus-manifests.md) and executed locally on 2026-11-03 (bpmn-viz working tree at TASK-018 review state, debug profile, toolchain 1.99.1):

- **MIWG full `Reference` set**: 22 `.bpmn` files at revision `85240b8689d835d8bd1a96fbea6af46558ca0568` (2026-10-18), per-file SHA-256 in `tests/manifests/external-fixtures.lock.json`; expanded from the 7 files pinned in [REPORT-001](REPORT-001-conformance-baseline.md).
- **PMo Dataset v1.0.0** (Zenodo `10.5281/zenodo.15857589`, CC-BY-4.0): archive `pmo-dataset.zip` pinned with SHA-256 `9667c465…` and upstream MD5 `414d9fbc…`; 55 ground-truth BPMN models with DI (`external/pmo/bpmn/`) and the same 55 without the diagram definition (`external/pmo/bpmn_process/`), digests pinned per extracted file (111 entries).
- **Reproduction:** `external/` deleted and rebuilt from scratch by `scripts/download-fixtures.sh` (133 files verified; second run serves from digest-verified cache).
- **Probes:** `bpmn validate` per file (exit 0/3 recorded); `bpmn render` CLI samples; PMo no-DI through `bpmn-layout` via the new gated suite `crates/bpmn-layout/tests/corpus_pmo.rs` (invariants, double-run determinism, typed-failure classification; self-skips when `external/` is absent so per-PR CI stays synthetic-only per ADR-0005).

## Results

- **MIWG validate: 16/22 clean, 6 with errors.** All findings are strict, correct diagnostics about upstream files: `di.dangling_element_ref` on B.1.0, B.2.0, C.6.0, C.7.0 (DI references elements our importer classifies as unsupported/unknown and drops, after `import.ignored_element` warnings) and `geometry.invalid` label-bounds on C.10.0, C.9.1 (zero/negative label bounds in upstream DI). `bpmn render` refuses these files by design (exit 3, no output written).
- **PMo with-DI validate: 18/55 clean, 37 with `di.dangling_element_ref`.** Root cause verified on 01.bpmn: the PMo ground-truth DI references `TextAnnotation_*` (and DataObject/Association) elements that are absent from the semantic XML — the corpus is internally inconsistent, not the validator. The direct renderer (`render_first_diagram`, no validation gate) renders **55/55** to valid SVG; the CLI gate refuses the 37 (by design).
- **PMo no-DI validate: 55/55 `di.missing_interchange`** — the documented Preserve contract (ADR-0007) holds uniformly on real data.
- **PMo no-DI layout: 43/55 laid out.** Every success passes TEST-005 geometry invariants (valid integer bounds, no overlap, axis-aligned boundary-docked routes) and double-run canonical determinism. The 12 failures are uniformly typed `layout.routing_failed` (05, 25–27, 29, 32–34, 36, 38, 39, 47) — the pinned M1 router limitation (TEST-005 failure 11 class), no panics, no silent skips.
- **Render smoke:** clean PMo 06.bpmn → SVG 2606×240 (13 KB) and PNG (24 KB, 2 `raster.font_substituted` warnings — known RISK-002 environment substitution).

## Gaps

- **CODECS T3.3 (Zenodo 16372882) audited and dropped from adoption:** the archive contains zero machine-readable BPMN — only PNG/JPG raster exports, one guide PDF and metadata (127 real files; verified by full listing). The ADR-0008 draft's "BPMN subset requires a filter" assumption was wrong; the draft (still `proposed`) is amended accordingly.
- **BPMAI remains out** (JSON in Signavio/PromniCat format, research-only licensing) and **HF image/XML pairs remain out** (gated, license invisible) — unchanged from the ADR-0008 audit.
- **Artifact-DI strictness question:** real-world tools emit DI shapes for text annotations/data objects; our import drops unsupported elements and validation then reports dangling DI. Correct-but-lossy; a decision on retaining artifact DI (M3 scope) is untracked.
- **12/55 PMo models exceed the M1 router** (`layout.routing_failed`); TEST-005 pins the router replacement as M2 work.
- DocVera runtime is 0.2.0 while DOC-002 records verification against 0.1.2 (drift observed during this run; unrelated to corpora).

## Next Actions

1. Review [ADR-0008](../adr/ADR-0008-external-corpus-manifests.md) as amended (MIWG full set + PMo; CODECS rejected) and promote to accepted with this report as Validation evidence (TASK-019).
2. File a decision task on artifact/data-shape DI retention (dangling `di.dangling_element_ref` on real-world files; candidates: ignore-with-warning vs. retain for DI resolution) before M3.
3. M2 router work replaces the `layout.routing_failed` pins; re-run this corpus suite as the acceptance harness (target: >43/55 on PMo no-DI).
4. Keep the corpus suite out of per-PR CI; schedule it (or run at review time) wherever MIWG/PMo-backed claims are made.
