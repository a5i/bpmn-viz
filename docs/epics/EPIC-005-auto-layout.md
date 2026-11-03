---
id: EPIC-005
type: epic
title: Native BPMN automatic layout
status: in_progress
relations:
  implements: [SPEC-001]
  children: [TASK-018]
---
# Native BPMN automatic layout

## Outcome

A deterministic, pure-Rust, BPMN-aware layout engine (`bpmn-layout`) that turns semantic BPMN documents without usable Diagram Interchange into consumable typed DI, without any JavaScript or external layout runtime. Behavior contract and differential testing reference `bpmn-io/bpmn-auto-layout` 2.x (pinned; EXP-004). ADR-0006 records the algorithm decision; ADR-0007 records the Preserve/Generate/Reflow mode semantics with the renderer's DI-required contract (ADR-0003) unchanged.

## Scope

Milestones:

- **M0 — contract and reference harness:** public Rust types independent of XML/SVG/PNG (`LayoutEngine`, `LayoutOptions`, `LayoutOutcome`, error/warning taxonomy), pinned reference commit with license/provenance (EXP-004), dev-only Node reference runner, canonical comparison methodology, deterministic ordering policy, supported-element matrix, structured error codes, performance budgets.
- **M1 — core process layout (prototype acceptance scope):** single-process scope extraction and validation; weakly connected components, start selection, primary spine, branches/joins, cycles/back edges; left-to-right ranks and semantic bands; canonical shape sizes; disconnected-component packing; orthogonal sequence-flow routing with deterministic docking; typed DI generation + invariant verifier; determinism guarantees; ten core fixtures with invariants, errors, snapshots, differential data, and renderer consumability (SVG/PNG from generated DI).
- **M2 — containers/exceptions (follow-up):** expanded/collapsed subprocesses with recursive child layout, boundary events and exception-handler branches, lanes, event subprocesses.
- **M3 — collaboration/artifacts (follow-up):** participants/pools, message flows, text annotations, associations, data objects/stores, groups, label placement.

M2/M3 are tracked as independent follow-up tasks and are not part of M1 completion criteria.

## Acceptance Criteria

- M1 DoD (TASK-018): ten core fixtures pass semantic/geometry tests and approved snapshots; stable structured errors without panic for invalid references and cross-scope flows; every supported node and flow appears in generated DI with finite geometry; orthogonal routes avoid unrelated shape interiors; identical input produces identical geometry; the renderer produces SVG and PNG from generated DI for each accepted fixture; deterministic ordering documented; no JS/network/runtime service in the released library; provenance recorded; ADR-0006, ADR-0007, EXP-004, TEST-005 linked and reviewed; `scripts/check.sh` green.

## Tasks

- [ ] [TASK-018](../tasks/TASK-018-bpmn-auto-layout.md) — implement native BPMN-aware automatic layout M0+M1 (in review; ADR-0006/0007 proposed for PR review)
- [ ] Follow-up (M2): containers, boundary events, lanes — tracked when M1 review completes
- [ ] Follow-up (M3): collaboration, artifacts, labels — tracked when M2 review completes
