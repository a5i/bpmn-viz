---
id: ADR-0001
type: adr
title: Pure Rust rendering pipeline
status: accepted
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
---
# Pure Rust rendering pipeline

## Context

The BPMN renderer prototype requires a bounded and auditable engineering decision. The dominant web tooling (bpmn.io) needs Node.js/browser runtimes, carries a watermark license for embedded viewers, and does not fit the goal of a reproducible CLI without a JavaScript runtime.

## Decision

Use Rust for parsing, scene representation, SVG serialization and PNG rasterization; do not bundle bpmn-js. Concretely: `quick-xml` for import, an owned scene model, an owned deterministic SVG writer, and `resvg` for rasterization.

## Alternatives Considered

- **Embed bpmn-js (headless browser):** pixel-familiar output but requires Node/Chromium, violates the no-JS constraint, and bpmn.io licensing adds a watermark condition for embedded viewers. Rejected.
- **`svg` crate for serialization:** evaluated during implementation (SPEC-001 section 15 open question); the owned writer gives full control over attribute order, number formatting and escaping, which the determinism gate needs. Rejected in favor of the owned writer.
- **ebi_bpmn for parsing:** see EXP-001; behavioral focus, no DI. Rejected.

## Consequences

- Single-language toolchain: MIT-compatible dependency graph, `cargo audit`/`cargo-deny` enforceable, reproducible builds from `Cargo.lock`.
- Text rendering must be solved in-crate (see EXP-002) rather than inheriting browser text shaping.
- Rendering parity with Camunda/bpmn-js is approximated, not guaranteed (SPEC-001 out-of-scope list).

## Validation

- `cargo build --workspace`, `cargo test --workspace` (90+ tests) and `cargo clippy -D warnings` pass without Node.js or a browser installed.
- The CLI (`target/debug/bpmn`) renders SVG and PNG for all fixtures and the seven selected MIWG reference files on Linux; dependency licenses verified with `cargo deny check` (see REPORT-001).

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
- [EXP-001](../experiments/EXP-001-ebi-bpmn-evaluation.md)
- [bpmn.io license](https://bpmn.io/license/)
