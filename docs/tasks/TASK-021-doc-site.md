---
id: TASK-021
type: task
title: Documentation site
status: in_progress
relations:
  parent: EPIC-004
  implements: [SPEC-001]
  verified_by: [TEST-001]
---
# Documentation site

## Goal

Give bpmn-viz a public documentation site (`apps/docs`), modeled on the audited DocVera reference implementation in md-schema `apps/docs`: a static-export Next.js landing page, MDX documentation pages (introduction, quickstart, CLI reference, auto-layout, Node/WASM bindings, architecture) and a live browser playground that executes the real Rust engine compiled to WASM (`crates/bpmn-wasm` pkg-web build, ADR-0009) to render BPMN SVG and diagnostics client-side.

## Implementation

- `apps/docs`: Next.js static export (`output: "export"`), MDX content under `apps/docs/content`, Tailwind 4 + Base UI primitives, next-themes day/night, Geist fonts, Cmd+K page search, sidebar/TOC/pagination docs layout.
- Playground: `scripts/prepare-wasm.mjs` copies the `crates/bpmn-wasm/pkg-web` wasm-pack `web` artifact (a build output, built in CI and locally via `npm --prefix crates/bpmn-wasm run build:web`) into `public/runtime/`; a module Web Worker (`public/bpmn.worker.js`) instantiates `BpmnDocument`, exposes `validate`/`renderSvg`/`renderLayoutSvg`/`layout`, and posts JSON DTOs (camelCase contracts mirroring `npm/bpmn-viz/index.d.ts`). The UI terminates the worker after a 20 s timeout; no input leaves the browser.
- Examples adapted from committed fixtures (`tests/fixtures/minimal/start-task-end.bpmn`, `invalid/duplicate-id.bpmn`, `edge-cases/collaboration-pool-lanes.bpmn`) plus a new no-DI order process for the layout demo.
- CI: `.github/workflows/docs.yml` (paths `apps/docs/**`, `crates/bpmn-wasm/**`) builds the WASM browser runtime, then runs lint, typecheck, build, `npm audit` and Playwright Chromium smoke tests on the static export; build-only with artifact upload, no deploy target yet.
- Deployment (Vercel): the build image has no Rust toolchain and `pkg-web` is never committed, so a fresh-clone deploy fails at the `prebuild` hook. `apps/docs/vercel.json` sets `installCommand` (site `npm ci` plus `crates/bpmn-wasm` `npm ci` for the pinned npm `wasm-pack`) and `buildCommand` → `scripts/vercel-build.sh`, which bootstraps a minimal Rust toolchain pinned to `rust-toolchain.toml`'s version via `RUSTUP_TOOLCHAIN` (skipping its rustfmt/clippy components), adds the `wasm32-unknown-unknown` target, builds `pkg-web`, then runs the site build. Requires the project's Root Directory to be `apps/docs` with "Include source files outside of the Root Directory in the Build Step" (default on) so `../../crates/bpmn-wasm` is reachable; CI remains authoritative for lint/typecheck/e2e gates.
- Playground UX (revised after review): rendering is automatic — the diagram renders on page load and re-renders on a 600 ms debounce as the model is edited; every run supersedes in-flight work (persistent worker, latest input wins), the render button only forces an immediate re-render. Verified in Chromium: no click is required for the default render, for example switches, or for render-mode changes.
- Documentation-only change: no workspace crates, public API, licensing, dependency policy or BPMN DI semantics are affected (npm toolchain is confined to `apps/docs`, mirroring the existing `npm/bpmn-viz` package).

## Acceptance Criteria

- [x] `npm run build` produces the static export under `apps/docs/out` for all routes (`/`, `/docs/`, `/docs/quickstart/`, `/docs/cli/`, `/docs/layout/`, `/docs/bindings/`, `/docs/architecture/`, `/playground/`).
- [x] The playground runs the WASM engine in Chromium: renders the DI example to SVG, auto-layouts the no-DI example, and surfaces diagnostics for the invalid example.
- [x] `npm run lint`, `npm run typecheck` and `npm run test:e2e` pass.
- [x] Default workspace workflows unchanged: `cargo test --workspace`, `cargo clippy -D warnings`, `docvera check` green with `apps/docs` present.

## Verification

- `npm run lint`, `npm run typecheck`, `npm run build`, `npm run test:e2e` in `apps/docs` (see Done for results).
- `scripts/check.sh` unaffected at the Cargo level; `docvera check` re-run after adding this document.

## Done

- Site scaffolding complete: static export of 11 routes (`/`, 6 docs pages, `/playground/`, not-found, icon) built successfully.
- Playground verified in Chromium via Playwright (desktop + mobile, 24/24 passing): DI render shows SVG preview with `provided DI` meta; broken reference produces `1 errors` in the diagnostics tab (then recovers after reset); the no-DI example auto-layouts (`auto-layout` meta) and lists generated DI shapes/edges; malformed XML surfaces the parse error without crashing; day/night themes persist. `npm run lint`, `npm run typecheck` and `npm run build` exit 0; `npm audit --omit=dev` reports 0 vulnerabilities.
- Local gate evidence: `docvera check` — 59 documents, 0 diagnostics; `cargo test --workspace` — exit 0; `cargo clippy --workspace --all-targets -- -D warnings` — exit 0 (apps/docs adds no Rust code).
- Auto-render revision: e2e grown to 28 tests asserting renders happen without clicks (default render, live edit re-render, example switch, render-mode switch, malformed-XML recovery); lint/typecheck/build re-run green.
- Vercel deploy fix: with `crates/bpmn-wasm/pkg-web` removed (fresh-clone state), `bash scripts/vercel-build.sh` in `apps/docs` rebuilt the artifact via the pinned wasm-pack and produced the green static export with `out/runtime/bpmn_wasm_bg.wasm` (536,930 bytes) present; `git status` shows only `vercel.json`, `vercel-build.sh`, the `prepare-wasm.mjs` comment correction and this document. The rustup-bootstrap branch of the script is exercised only on Vercel's Rust-less image (local runs skip it via the `command -v rustup` guard).
