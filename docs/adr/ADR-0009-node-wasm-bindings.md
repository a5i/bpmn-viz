---
id: ADR-0009
type: adr
title: Node.js and WebAssembly bindings on the Rust pipeline
status: proposed
created: 2026-11-03
relations:
  implements: [SPEC-001]
  supersedes: []
---
# Node.js and WebAssembly bindings on the Rust pipeline

## Context

SPEC-001 lists WASM integration as a P1 extension. The Rust pipeline (`bpmn-model` → `bpmn-xml` → `bpmn-validate` → `bpmn-render`/`bpmn-layout` → `bpmn-svg`, plus `bpmn-raster` for PNG) is complete through TASK-018, but it is reachable only through the CLI. Embedders (web editors, documentation tooling, CI services) need the same parsing, validation, auto-layout and SVG rendering without spawning a process, while the prototype goal "usable without Node.js or a browser" (SPEC-001 section 1) must remain true for the CLI itself.

A reference implementation was audited in the DocVera project (md-schema checkout, 2026-11-03): `docvera-node` (napi-rs 3 `cdylib`, async work on `tokio::task::spawn_blocking`, explicit `close()`), `docvera-wasm` (wasm-bindgen + serde-wasm-bindgen, `wasm-pack` builds for `nodejs` and `web` targets), and one npm package (`@a5i/docvera`) that carries all platform binaries plus the WASM runtime, loaded through a `platforms.cjs` table. DocVera deliberately keeps its WASM build narrower than the native addon (no SHACL evaluation); the same split applies here: `resvg` rasterization is native-only, browsers rasterize the produced SVG on canvas.

Adding `napi`/`napi-derive`/`napi-build`/`tokio` and `wasm-bindgen`/`serde-wasm-bindgen` changes the dependency policy (SPEC-001 section 7) and adds workspace members, so this ADR is authored before implementation per `AGENTS.md`. BPMN DI semantics, crate public APIs and existing dependency choices are unchanged; the bindings are additive adapter crates.

## Decision

Add two adapter crates and one npm package, mirroring the audited DocVera pattern:

- **`bpmn-node`** — napi-rs 3 `cdylib` (`publish = false`), exposing async methods (`tokio::task::spawn_blocking`) that wrap the full pipeline including `bpmn-raster` PNG export and `bpmn-layout` auto-layout. Owned handles follow the explicit-lock + `close()` discipline.
- **`bpmn-wasm`** — wasm-bindgen `cdylib,rlib` for `wasm32-unknown-unknown`, exposing synchronous parse/validate/layout/SVG-render. **PNG rasterization is excluded**: `resvg` and its font subsystem stay out of the WASM dependency graph.
- **`npm/bpmn-viz`** — one npm package `@a5i/bpmn-viz` containing every supported platform `.node` binary plus the `wasm-pack` runtime, with `platforms.cjs` platform resolution, a `build.cjs` cargo bridge, hand-written `index.d.ts`, and a CLI-parity test gate (`BPMN_BIN`), like `DOCVERA_BIN` in the reference project.

The WASM crate never gains filesystem access: callers pass XML text and receive strings/JSON, matching the "callers supply all input, no side channels" adapter shape of the reference. Workspace `default-members` keeps the core crates as the default build so root `cargo build/test/clippy/deny` behavior is unchanged; bindings are built by their dedicated CI jobs and npm scripts.

## Alternatives Considered

- **wasm-bindgen for Node too (skip napi-rs):** one artifact for both runtimes, but Node calls become synchronous and PNG export would need resvg-in-WASM; the reference project chose native addons for Node precisely to keep the full pipeline, and its parity tests pin behavior. Rejected.
- **`napi-rs` CLI scaffolding (`napi build`/`--js` bindings):** generates a per-platform package layout with `optionalDependencies`; heavier release choreography than one staged package, and the audited single-package `platforms.cjs` model is already reviewed. Rejected for now.
- **wasm-bindgen-rayon / threaded WASM:** unbounded scope for an SVG-only binding; rejected.
- **Embed bpmn-js for web rendering:** violates ADR-0001 (watermark condition, JS runtime); rejected.
- **Publish the binding crates to crates.io:** bindings are useless without the npm artifact; crates stay `publish = false` like every other member pending the SPEC-001 crates.io namespace question. Rejected for now.

## Consequences

- New allowed dependencies: `napi`, `napi-derive`, `napi-build`, `tokio` (rt feature), `wasm-bindgen`, `serde-wasm-bindgen` — all MIT OR Apache-2.0, covered by the existing `deny.toml` allow list; `cargo deny check` must stay green.
- Root workspace workflows stay default-member-only; the bindings compile under dedicated jobs (Node 22+, `wasm32-unknown-unknown` via wasm-pack) and through `npm run build` in `npm/bpmn-viz`.
- WASM binary size is bounded by excluding `resvg`/`fontdb`; browser integrations rasterize SVG themselves (canvas), and PNG remains a native-addon/CLI capability.
- The npm package name `@a5i/bpmn-viz` must be verified on the registry before first publish (same class of open question as SPEC-001 section 15 for crates.io); the release pipeline stages — never auto-publishes — exactly one package per tag pending approval.
- JS API surface (`index.d.ts`) is a new public contract owned by this ADR; breaking changes to it require a superseding ADR, like the Rust API.
- Diagnostics cross the boundary as plain JSON objects (`code`, `severity`, `message`, optional `elementId`/`line`/`column`), defined by hand since `bpmn-model` has no serde dependency.

## Validation

- `cargo build -p bpmn-node` and `cargo clippy -p bpmn-node` succeed on the three CI operating systems; `npm test` in `npm/bpmn-viz` passes on Linux with Node 22, including the CLI-parity fixture test.
- `wasm-pack build` for `nodejs` and `web` targets succeeds; `crates/bpmn-wasm` node test renders the committed minimal fixture to byte-identical SVG as the CLI, and the Playwright Chromium check loads the bundle in a browser.
- `cargo test --workspace` (default members), `cargo fmt --check`, `cargo clippy -D warnings`, `cargo deny check` and `docvera check` all pass with the new crates present.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md) (P1 WASM extension, section 7 dependency policy)
- [ADR-0001](ADR-0001-rust-only-rendering-pipeline.md) (pure Rust pipeline; no embedded bpmn-js)
- [ADR-0006](ADR-0006-native-bpmn-aware-layered-layout.md), [ADR-0007](ADR-0007-generated-di-and-layout-modes.md) (layout exposed through the bindings)
- [EPIC-004](../epics/EPIC-004-release.md), [TASK-020](../tasks/TASK-020-node-wasm-bindings.md)
- [EXP-003](../experiments/EXP-003-docvera-integration.md) (DocVera integration audit; reference implementation provenance)
- napi-rs: <https://napi.rs/>; wasm-bindgen: <https://rustwasm.github.io/docs/wasm-bindgen/>; wasm-pack: <https://rustwasm.github.io/docs/wasm-pack/>
