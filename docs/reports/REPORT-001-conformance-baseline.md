---
id: REPORT-001
type: report
title: Conformance and performance baseline
status: final
relations:
  based_on: [TEST-001, EPIC-001, EPIC-002]
---
# Conformance and performance baseline

## Method

- Pinned MIWG subset (A.1.0, A.2.0, A.3.0, A.4.0, A.4.1, C.4.0, C.8.0) at upstream revision `85240b8689d835d8bd1a96fbea6af46558ca0568` (2026-10-18) via ADR-0005; ran `bpmn validate` and `bpmn render` (SVG) on each.
- Ran the full synthetic suite (`cargo test --workspace`), `cargo clippy -D warnings`, `cargo deny check`, `cargo audit`, `docvera check`.
- Measured a performance baseline with `cargo bench -p bpmn-cli --bench render` (optimized bench profile) on synthetic linear processes with 100 / 1,000 / 10,000 tasks; timings are a local baseline, not a speed guarantee (SPEC-001 section 10).

## Results

**MIWG smoke (validate + render SVG):** all seven files parse, validate with 0 errors and render; diagnostics limited to `symbol.unsupported`/`import.ignored_element` warnings for elements outside the P0 rendering subset (data shapes) and one `symbol.unsupported` warning per event-based gateway variant where present. Boundary events (A.3.0, C.8.0), data stores, data objects/references, ioSpecification inputs/outputs and data associations resolve and render (associations as dashed edges, data shapes as conspicuous fallbacks).

**Compatibility matrix (rendered with glyphs):** start/end/intermediate events with message, timer, error, escalation, terminate, compensation, conditional, link, signal, cancel, multiple markers; boundary events; tasks plus user/service/manual/script/send/receive/business-rule kinds; exclusive/parallel/inclusive gateways; expanded/collapsed subprocesses; pools, lanes, sequence flows (default tick, conditional diamond), message flows, associations, text annotations. Fallback with warnings: event-based/complex gateways, data shapes, callActivity/transaction. Not modeled: choreographies, conversation links, ad-hoc processes, multi-instance markers.

**Quality gates:** fmt clean; clippy `-D warnings` clean; `cargo test --workspace` 18 suites green (90+ tests); `docvera check` 33 documents, 0 diagnostics; `cargo deny check` and `cargo audit` clean with deny-by-default configs.

**Performance baseline (bench profile, Linux x64, rustc 1.99.1):**

| Elements | Parse | Render (scene) | SVG serialize | PNG (1x) |
| --- | --- | --- | --- | --- |
| 100 (102 nodes) | 0.63 ms | 0.30 ms | 0.91 ms (58 KB) | 161 ms (52 KB) |
| 1,000 (1,002 nodes) | 4.4 ms | 1.9 ms | 6.9 ms (589 KB) | skipped: 140096x6960 exceeds pixel budget |
| 10,000 (10,002 nodes) | 43 ms | 20 ms | 70 ms (5.9 MB) | not attempted |

The 1,000-element row demonstrates the pixel budget refusing a pathologically wide single-row layout instead of exhausting memory (RISK-005).

## Gaps

- libfuzzer targets are authored but not yet executed on CI (TASK-014); malformed-input coverage rests on fixtures and unit tests for now.
- CI has not yet run on macOS/Windows runners (TASK-015).
- DocVera lifecycle *transitions* are not machine-enforced; statuses are enums and transitions rely on review (EXP-003).
- Crates are not published; crates.io names unverified (TASK-016).
- PNG rasterization is skipped by the pixel budget for pathologically wide layouts; a row-aware layout hint is future work.

## Next Actions

1. Execute `fuzz/xml_parse` and `fuzz/render_document` in the nightly CI job; triage and add regression fixtures (TASK-014).
2. Green-run `.github/workflows/ci.yml` on all three OS runners; record evidence in TASK-015.
3. Verify crates.io names `bpmn-*` per SPEC-001 open question, then flip `publish` and tag v0.1.0 (TASK-016).
4. Consider SHACL policies in `docvera.toml` to machine-enforce lifecycle transitions (supersedes pointers, done-with-verification).
5. Optional: font-metric-backed label measurement behind a feature flag if visual fidelity issues are reported (EXP-002).
