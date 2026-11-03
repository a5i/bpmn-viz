---
id: TASK-018
type: task
title: Implement native BPMN-aware automatic layout in Rust (M0+M1)
status: review
relations:
  parent: EPIC-005
  implements: [SPEC-001]
  decisions: [ADR-0006, ADR-0007]
  verified_by: [TEST-005]
---
# Implement native BPMN-aware automatic layout in Rust (M0+M1)

## Goal

Deliver milestone M0 (contract + reference harness) and M1 (core process layout) of the EPIC-005 auto-layout program: a deterministic, reusable, pure-Rust BPMN 2.0 layout engine for semantic documents whose DI is absent, incomplete, or explicitly recomputed. Output is typed BPMN DI consumable by `bpmn-render`/`bpmn-svg` and later by a BPMN XML writer. No JavaScript runtime or external layout service at runtime.

> ID note: this work arrived as a draft titled `TASK-017`. `TASK-017` is already a completed task (raster fonts, commit `9c9e2f6`) and DocVera enforces unique, stable ids, so the task is filed as **TASK-018** under EPIC-005. The draft's M2/M3 content, subtask breakdown (TASK-017.1–.9 → 018 subtasks), source references, and DoD carry over unchanged in substance.

Behavior reference: [bpmn-io/bpmn-auto-layout](https://github.com/bpmn-io/bpmn-auto-layout) pinned at `v2.0.0-alpha.2` = `75a7f30958c7050474d48db09afd4cc7d352ccad` (MIT, © Camunda Services GmbH; provenance and pin in EXP-004 and `crates/bpmn-layout/tests/fixtures/LICENSES.md`). The reference governs behavioral requirements (ordering policy, canonical sizes, pipeline stages), not exact generated coordinates. The 1.3.0 release line is a different engine and is explicitly out of contract.

## Implementation

- **Docs/governance (M0):** ADR-0006 (native layered algorithm, no JS runtime), ADR-0007 (Preserve/Generate/Reflow), EXP-004 (reference audit, pin, constants, differential methodology), TEST-005 (fixtures, assertions, exit criteria).
- **`crates/bpmn-layout` (new crate):**
  - `api.rs` — `LayoutEngine` trait, `NativeBpmnLayout`, `ScopeId`, `LayoutOptions` (orientation M1 LeftToRight; `node_gap`, `rank_gap`, `routing_clearance`, `deterministic`, `LayoutLimits`), `LayoutOutcome { diagram: Diagram, warnings }`, typed `LayoutError`/`LayoutWarning` enums with stable machine-readable codes (`layout.*`).
  - `geometry.rs` — integer rectangles/points and helpers (intersection, containment, inset, union); rounding happens once at DI emission.
  - `graph.rs` — scope-local graph with declaration-indexed stable ids; iterative traversals only.
  - `validation.rs` — duplicate ids, missing/cross-scope endpoints, unsupported references, option and limit checks.
  - `semantics.rs` — weakly connected components, start selection, primary spine (reachable-to-end → default flow → declaration order), back-edge marking, longest-path ranks, semantic band assignment, stable ordering.
  - `placement.rs` — rank columns and band rows, within-rank/band collision resolution, disconnected component packing, normalization to outer margin.
  - `routing.rs` — deterministic port selection (same-band right→left; cross-band elbows; feedback channels), Manhattan routes, obstacle avoidance with clearance, redundant-segment elimination, structured `RoutingFailed` on impossible routes.
  - `di.rs` — `bpmn_model::di::Diagram` generation (shapes `id = <bpmnElement>_di`, edges with ordered waypoints) and the invariant verifier (coverage, finiteness, axis-aligned segments, boundary docking, interior avoidance, overlap freedom).
- **Fixtures (project-owned, MIT):** ten core `.bpmn` fixtures under `crates/bpmn-layout/tests/fixtures/core/` and failure fixtures under `failures/`, with `LICENSES.md` provenance.
- **Reference harness (dev-only):** `tools/reference-layout/` Node runner installing pinned `bpmn-auto-layout@2.0.0-alpha.2`, emitting canonical reference DI JSON committed under `tests/fixtures/reference/` (node_modules and generated XML stay untracked).
- **Tests:** `contracts.rs`, `geometry_invariants.rs`, `layout_snapshots.rs`, `determinism.rs`, `errors.rs`; workspace consumability test rendering SVG (and PNG) from generated DI for every accepted fixture; property test for no-panic/limits on generated models.
- **Deferred to follow-up tasks (not in this change):** CLI/renderer mode wiring (`--layout generate`), M2 containers/boundary events/lanes, M3 collaboration/artifacts/labels, XML writer Reflow.

## Acceptance Criteria

- [x] Ten core fixtures pass semantic/geometry tests and approved native snapshots.
- [x] Invalid references and unsupported cross-scope flows return stable errors without panic.
- [x] Every supported node and sequence flow in scope appears exactly once in generated DI with finite, integer geometry.
- [x] Routes are axis-aligned, dock on shape boundaries, and do not intersect unrelated shape interiors.
- [x] Identical input + options produce identical canonical geometry and ordered diagnostics across runs; re-layout never mutates the semantic model.
- [x] `bpmn-render` + `bpmn-svg` produce valid SVG (and `bpmn-raster` a nonempty PNG) from generated DI for each accepted fixture.
- [x] Differential tests against the pinned reference pass on coverage, topology, containment, orthogonal routing, and collision freedom (no coordinate-equality requirement).
- [x] Preserve path unchanged: rendering a no-DI document through the untouched renderer still fails with the structured `RenderError` (no layout fallback).
- [x] Resource limits enforced; property tests show no panic on generated malformed/oversized models.
- [x] `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, `docvera check`, `cargo deny check` green; licenses/attribution recorded.

## Verification

Executed 2026-11-03 on the author workstation (Rust 1.99.1 pinned toolchain, release benchmarks):

- `cargo test --workspace` — all suites green; the layout crate contributes 35 tests across seven binaries:
  - `contracts` 10, `errors` 7 (incl. 256-case proptest no-panic corpus), `geometry_invariants` 6, `consumability` 4 (SVG + PNG from generated DI for all eight fixtures, Preserve regression), `determinism` 3, `reference_parity` 1 (differential vs pinned reference on coverage, sizes, topology rules, geometry rules for *both* engines), `layout_snapshots` 1 (+1 ignored regeneration helper), lib unit tests 3, doc test 1.
- `cargo test -p bpmn-xml` — green including the new self-closing-boundary-event regression (`parses_self_closing_boundary_event_as_boundary_event`).
- `cargo fmt --all -- --check` — clean; `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `scripts/docvera-check.sh` — `Checked 53 documents; 0 diagnostics`.
- `cargo deny check` — licenses/bans/advisories/sources ok; `cargo audit` — no vulnerabilities (136 crates scanned).
- `cargo bench -p bpmn-layout` — corpus medians recorded in EXP-004 (12 nodes 0.012 ms / 102 nodes 0.235 ms / 1,002 nodes 16.859 ms).
- Reference harness executed once: `node tools/reference-layout/run.mjs core/*.bpmn --out tests/fixtures/reference` produced the eight committed canonical JSONs at the pinned commit.

## Done

- Artifacts: `crates/bpmn-layout` (public API, engine, verifier), ten fixtures with provenance (`tests/fixtures/LICENSES.md`), approved native snapshots (`tests/snapshots/`), committed reference parity data (`tests/fixtures/reference/`), dev-only harness (`tools/reference-layout/`), docs (EPIC-005, ADR-0006/0007 proposed, EXP-004 concluded, TEST-005 active), SPEC-001/index/CHANGELOG/development-guide updates, self-closing boundary-event parser fix with regression test.
- Acceptance criteria: all boxes above verified except cross-platform CI (GitHub workflow unaffected; Linux verified locally) and PR review of the two proposed ADRs and the snapshot baselines — review gate for this change.
- Deferred follow-ups (tracked in EPIC-005): CLI/renderer Generate-mode wiring (TASK-017.9 equivalent), M2 containers/boundary events/lanes, M3 collaboration/artifacts/labels, XML-writer Reflow.
