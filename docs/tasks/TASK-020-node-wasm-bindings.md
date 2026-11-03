---
id: TASK-020
type: task
title: Node.js and WebAssembly bindings
status: in_progress
relations:
  parent: EPIC-004
  implements: [SPEC-001]
  decisions: [ADR-0009]
  verified_by: [TEST-001]
---
# Node.js and WebAssembly bindings

## Goal

Expose the Rust pipeline (parse, validate, auto-layout, SVG render; PNG native-only) to JavaScript runtimes: a Node.js native addon and a browser/Node WASM build, distributed as one npm package `@a5i/bpmn-viz`, per ADR-0009 and the audited DocVera reference implementation.

## Implementation

- `crates/bpmn-node`: napi-rs 3 `cdylib`; async methods on `tokio::task::spawn_blocking`; classes `BpmnDocument`-like handle (`fromXml`, `inspect`, `validate`, `renderSvg`, `renderPng`, `layout`) with explicit `close()`.
- `crates/bpmn-wasm`: wasm-bindgen + serde-wasm-bindgen; synchronous `Checker`-style handle (`fromXml`, `validate`, `renderSvg`, `layout`); no `resvg`/`fontdb` in the WASM graph.
- `npm/bpmn-viz`: single package with `platforms.cjs` (linux-x64-gnu, linux-arm64-gnu, darwin-arm64, win32-x64-msvc), `build.cjs` cargo bridge, `index.js` loader with source-checkout fallback, `index.d.ts` types, `scripts/assemble-wasm.cjs`, `scripts/validate-release.cjs`, tests (`node --test`) including CLI parity via `BPMN_BIN`.
- CI: node/wasm build+test jobs in `ci.yml`; `release.yml` staged npm publish with provenance (single package, all platforms + WASM), modeled on the audited reference.

## Acceptance Criteria

- [ ] `npm test` passes in `npm/bpmn-viz` (loader, platforms table, release packaging, parity against the CLI for the committed minimal fixture).
- [ ] `wasm-pack` builds for `nodejs` and `web`; WASM node test renders byte-identical SVG to the CLI for the minimal fixture; browser smoke via Playwright Chromium.
- [ ] Default workspace workflows unchanged: `cargo test --workspace`, `cargo clippy -D warnings`, `cargo deny check`, `docvera check` green with the new crates present.
- [ ] ADR-0009 promoted or review findings recorded before merge to main.

## Verification

- Execution evidence (2026-11-03, Linux x64, Rust 1.99.1, Node 24, wasm-pack 0.15.0):
  - `cargo build -p bpmn-node`: ok; `cargo clippy -p bpmn-node -- -D warnings`: ok.
  - `cargo clippy -p bpmn-wasm -- -D warnings`: ok; `wasm-pack build --target nodejs|web --release -- --locked`: ok.
  - `crates/bpmn-wasm` `npm test`: PASS (incl. byte parity with `tests/fixtures/minimal/start-task-end.svg`); `npm run test:browser`: PASS (Playwright Chromium).
  - `npm/bpmn-viz` `npm run build` + `npm test`: 18/18 PASS (fixture parity, PNG scale/background/layout/close semantics, CLI inspect+SVG parity, release packaging); `npm run test:wasm`: 1/1 PASS; `npm pack --dry-run`: 14 files.
  - Workspace gate: `cargo fmt --all -- --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` ok; `cargo test --workspace` ok; `cargo deny check` (advisories/bans/licenses/sources) ok; `cargo audit` ok; `scripts/docvera-check.sh`: 58 documents, 0 diagnostics.
  - CI: `bindings` (3 OS) and `bindings-wasm` jobs added to ci.yml; `release.yml` added (staged npm publish, 4-platform CLI archives).
  - Remaining: GitHub Actions run evidence on all three OS runners; ADR-0009 review; `@a5i/bpmn-viz` registry availability check.

## Done

- Implementation, npm scaffold, CI/release wiring and docs (ADR-0009, this task, index, EPIC-004) merged with the evidence above.
- Open: `@a5i/bpmn-viz` registry availability check before first publish; Trusted Publisher registration for staged publishing.
