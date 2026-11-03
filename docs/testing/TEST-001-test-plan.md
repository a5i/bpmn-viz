---
id: TEST-001
type: test_plan
title: Prototype test plan
status: active
relations:
  specifies: [SPEC-001]
  verifies: [TASK-003, TASK-004, TASK-005, TASK-006, TASK-007, TASK-008, TASK-009, TASK-010, TASK-011, TASK-013]
  produces: [REPORT-001]
---
# Prototype test plan

## Fixtures

Committed synthetic fixtures (project-owned, MIT) under `tests/fixtures/`:

- `minimal/start-task-end.bpmn` (+ reviewed golden `start-task-end.svg`).
- `edge-cases/`: `custom-prefixes.bpmn` (AC-01, xsi:type, default/conditional flows, lanes, annotation), `collaboration-pool-lanes.bpmn` (AC-05), `expanded-subprocess.bpmn` (AC-05), `multi-diagram.bpmn` (AC-06), `unicode.bpmn` (AC-08).
- `invalid/`: `duplicate-id.bpmn`, `dangling-reference.bpmn`, `missing-di.bpmn`, `nan-bounds.bpmn` (AC-02/AC-09).
- `malformed/`: `broken-xml.bpmn`, `xxe.bpmn` (AC-08, entity-expansion rejection).

External pinned fixtures (CC-BY-3.0, not committed): seven BPMN MIWG cases in gitignored `external/` per ADR-0005.

## Assertions

- Unit tests per crate: model invariants, namespace resolution, geometry checks, scene ordering, wrapping, serializer determinism/escaping, raster limits.
- Integration `parse.rs`: AC-01/02/03/06 behaviors against fixtures.
- Integration `render.rs`: AC-04/05/07/08 — structural SVG assertions, byte-exact golden comparison, PNG dimensions and non-background pixel content at 1x/2x.
- Integration `cli.rs`: CLI contract, exit codes 0/2/3/4/5, atomic writes, JSON report shape, stdout mode.
- MIWG smoke: `bpmn validate` exit 0 on all seven pinned files (see REPORT-001 for the recorded run).

## Coverage

- Parser: namespaces, prefixes, xsi:type, event definitions, subprocess bodies, collaborations, lane sets, text annotations, data shapes/associations at id level, malformed inputs, limits.
- Validator: duplicate ids, dangling/foreign references, invalid geometry, missing DI/shapes/waypoints, boundary attachments, collapsed-subprocess exemption.
- Renderer: all P0 glyphs plus boundary events, flows decorations, label placement, rotated container labels, fallback glyphs.
- Output: deterministic SVG, scaled PNG, CLI contract. Not covered: pixel-equality against other renderers (ADR-0004), PDF output (out of scope).

## Exit Criteria

- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo test --workspace` green (18 suites, 90+ tests).
- `docvera check` exits 0 with 0 diagnostics.
- `cargo deny check` and `cargo audit` pass locally.
- MIWG smoke: 0 validation errors on the pinned set.
