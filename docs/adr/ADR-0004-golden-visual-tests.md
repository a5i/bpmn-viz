---
id: ADR-0004
type: adr
title: Structural plus visual regression testing
status: accepted
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
  informed_by: [EXP-002]
---
# Structural plus visual regression testing

## Context

Pixel-perfect comparison with other renderers is not a conformance test (fonts and drawing conventions differ). Pure structural checks would miss visual regressions. The prototype needs both, without heavyweight golden-image infrastructure that requires per-platform font pinning.

## Decision

Layer testing by cost: (1) unit tests per crate, (2) fixture-based integration tests with structural SVG assertions (ids, markers, escapes), (3) one committed golden SVG pinning the deterministic serializer byte-for-byte, (4) PNG checks that assert dimensions and non-background pixel content at 1× and 2× without pixel equality, (5) MIWG smoke validation against pinned external files.

## Alternatives Considered

- **Golden PNG pixel equality:** requires pinned fonts and toolchain per platform; fails the cross-platform gate (AC-10). Rejected for CI; local diffs remain possible via `scripts/compare-images.sh`.
- **Snapshot-everything:** reviewing hundreds of goldens is noisy; only the minimal pipeline is pinned byte-exact.

## Consequences

- Visual regressions in stroke width, positions or markers surface through structural assertions or reviewer-diffable golden SVG changes.
- Cross-platform runs only need a fontdb fallback; no font assets are committed (license risk avoided).

## Validation

- `cargo test --workspace` runs all five layers locally in under two seconds (integration suite); the golden test failed and was corrected once during development, demonstrating it catches serializer drift.
- EXP-002 documents the determinism contract of text layout used by the structural assertions.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
- [EXP-002](../experiments/EXP-002-text-rendering.md)
- [TEST-001](../testing/TEST-001-test-plan.md)
