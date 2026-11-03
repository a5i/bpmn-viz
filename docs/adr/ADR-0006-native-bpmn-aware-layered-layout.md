---
id: ADR-0006
type: adr
title: Native BPMN-aware layered layout without a JavaScript runtime
status: proposed
created: 2026-11-03
relations:
  implements: [SPEC-001]
  supersedes: []
  informed_by: [EXP-004]
---
# Native BPMN-aware layered layout without a JavaScript runtime

## Context

Models without complete Diagram Interchange currently fail loudly (ADR-0003), which is correct for a *renderer*, but leaves no path to visualize semantic-only BPMN. ADR-0003 deferred automatic layout "to a future ADR if no-DI support becomes a requirement"; EPIC-005 makes it one. The candidate approaches differ sharply in dependency class and output quality:

- embed a JavaScript layout engine (bpmn-js / bpmn-auto-layout) at runtime;
- use a generic graph crate (elk, dagre, rust layered graph libs) with BPMN patches;
- implement a BPMN-specific layered algorithm natively in Rust.

BPMN layout quality depends on BPMN semantics (spine, branches, joins, exception paths, container rules). A generic layering followed by BPMN patches produces misleading narrative geometry. A JavaScript runtime violates the distribution constraints that motivated ADR-0001 (no Node, no browser, no embedded bpmn-js with its watermark condition).

[EXP-004](../experiments/EXP-004-reference-layout.md) audited `bpmn-io/bpmn-auto-layout` 2.0.0-alpha.2 (pinned commit `75a7f30958c7050474d48db09afd4cc7d352ccad`, MIT, © Camunda Services GmbH): a documented BPMN-aware constrained layered algorithm (ranks + semantic bands + orthogonal routing) with an executable reference harness available for differential testing. The older 1.3.0 line (the npm `latest` tag) is a different, weaker grid engine and is explicitly *not* the behavior contract.

## Decision

Create a new workspace crate `bpmn-layout` implementing a deterministic, pure-Rust, BPMN-aware layered layout engine. It consumes `bpmn_model::BpmnDocument` (semantic model) and produces `bpmn_model::di::Diagram` (typed BPMN DI) plus typed warnings/errors. No JavaScript, WebAssembly, network, or external layout service executes at runtime; the pinned TypeScript reference is used only by a development-only Node harness under `tools/reference-layout/` for differential tests and never from the library.

The Rust engine mirrors the reference's conceptual pipeline (scope extraction → validation → semantic analysis → placement → routing → DI generation → verification) and its behavior contract (declaration-order tie-breaking, canonical shape sizes, documented constants) as recorded in EXP-004; it does not promise identical coordinates. Upstream MIT attribution is recorded for behavior and fixture provenance. Milestones M2 (containers, boundary events, lanes) and M3 (collaborations, artifacts, labels) extend the same pipeline and are tracked as follow-up tasks, not part of the initial crate contract.

This ADR does not weaken ADR-0003: the renderer keeps consuming authored DI verbatim and keeps failing loudly when DI is missing. Generated DI enters the pipeline only through the explicit layout API (see ADR-0007).

## Alternatives Considered

- **JavaScript engine at runtime (Node/WASM bpmn-auto-layout):** violates ADR-0001's no-runtime distribution goal, adds a watermark/license-encumbered viewer dependency, and cannot run in the existing pure-Rust binary. Rejected for runtime use; retained as a dev-only differential harness.
- **Generic layered layout crate + BPMN fixes:** new dependency class with non-determinism risk (ADR-0003 already flagged), and narrative quality (spine continuity, band meaning) is exactly what generic algorithms lack. Rejected.
- **Grid/force layout:** trivially deterministic but produces misleading BPMN narrative; this is the 1.3.0 approach and is explicitly out of contract. Rejected.
- **No layout (status quo):** semantic-only models stay unrenderable. Rejected by EPIC-005's requirement.

## Consequences

- New public API (`bpmn-layout` crate: `LayoutEngine`, `LayoutOptions`, `LayoutOutcome`, typed error/warning enums with stable codes) — reviewed via this ADR and ADR-0007.
- Runtime dependency additions to the workspace: `thiserror` (already approved); internal geometry uses `bpmn-model` types, so no `kurbo`/`petgraph` dependency is required for M1. Dev-only: `serde`/`serde_json` (differential/snapshot fixtures), `proptest` (invariant/no-panic property tests), `criterion` (performance corpus). All MIT/Apache-2.0 compatible; `cargo deny` gate applies.
- Determinism policy: all ordering derives from BPMN declaration order and documented policies; no hash-map iteration order, no float identity comparisons; internal geometry is integer-based; DI coordinates are rounded once at emission.
- Untrusted-input policy: no panics on malformed models; iterative traversals only (no unbounded recursion); resource limits enforced with structured `LimitExceeded` errors.
- `tools/reference-layout/node_modules/` and generated outputs stay untracked; committed reference data is limited to small canonical JSON fixtures with provenance in `crates/bpmn-layout/tests/fixtures/LICENSES.md`.
- M1 renders a single process scope (events, tasks, gateways, sequence flows); subprocess bodies, boundary events, lanes, collaborations, and artifacts are skipped with explicit warnings, not silently misrepresented.

## Validation

- The ten core fixtures of TEST-005 pass geometry-invariant tests (one shape per node, one edge per flow, axis-aligned routes, docking on shape boundaries, no route through unrelated shape interiors, no shape overlap).
- Determinism tests: identical input + options produce identical canonical geometry and ordered diagnostics across repeated runs.
- Differential tests compare native output against the pinned reference (canonical JSON committed with provenance) on element coverage, topology, containment, orthogonal routing, and collision freedom — not coordinate equality.
- Fuzz/property tests assert no panic and limit enforcement on malformed and oversized models.
- `scripts/check.sh` remains green (fmt, clippy `-D warnings`, workspace tests, docvera, deny).

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
- [ADR-0001](ADR-0001-rust-only-rendering-pipeline.md) — pure Rust runtime
- [ADR-0003](ADR-0003-no-autolayout-in-poc.md) — renderer requires DI; layout is a separate concern
- [ADR-0007](ADR-0007-generated-di-and-layout-modes.md) — Preserve/Generate/Reflow semantics
- [EXP-004](../experiments/EXP-004-reference-layout.md) — reference audit and pin
- Upstream: <https://github.com/bpmn-io/bpmn-auto-layout> (MIT), pinned `v2.0.0-alpha.2` = `75a7f30958c7050474d48db09afd4cc7d352ccad`
