# Changelog

All notable changes to this project are documented here. Format based on [Keep a Changelog](https://keepachangelog.com/); the project does not yet follow a release cadence.

## [0.1.0] — prototype (unreleased)

### Added

- `bpmn-model`: semantic BPMN 2.0 subset (events incl. boundary events, tasks and common kinds, gateways, sequence flows, pools/lanes, subprocesses, text annotations), Diagram Interchange types, and stable diagnostic codes.
- `bpmn-xml`: namespace-URI-based importer (never prefix-based), `xsi:type` handling, subprocess bodies, collaborations, lane sets, data shapes/associations at id/DI resolution level, bounded input (size/depth), entity-expansion rejection.
- `bpmn-validate`: duplicate ids, dangling/foreign references, invalid geometry, missing Diagram Interchange, boundary attachment checks, collapsed-subprocess exemptions.
- `bpmn-scene`: renderer-agnostic scene primitives with deterministic paint order and approximate label wrapping.
- `bpmn-render`: glyphs for the P0 symbol set plus boundary events, pools/lanes with rotated labels, expanded/collapsed subprocesses, flow decorations (default tick, conditional diamond), dashed message/association edges, text annotations, conspicuous fallback glyphs.
- `bpmn-svg`: owned deterministic scene-to-SVG serializer.
- `bpmn-raster`: resvg-based PNG export with scale validation and pixel budgets.
- `bpmn-cli`: `render`, `inspect`, `validate` with documented exit codes (0/2/3/4/5), text and JSON diagnostics, atomic writes, diagram selection by index or id.
- Test suite: 18 suites (90+ tests) with committed golden SVG, MIWG smoke against pinned external fixtures, fuzz harnesses, and DocVera-validated documentation set (SPEC/ADR/EPIC/TASK/EXP/TEST/REPORT/RISK).
- `bpmn-layout`: deterministic, pure-Rust BPMN-aware automatic layout (M0+M1 of EPIC-005): single-process scope extraction and validation, semantic spine/band analysis with declaration-order tie-breaking, rank columns and semantic band rows, orthogonal routing with obstacle avoidance, typed DI generation plus an invariant verifier, structured `layout.*` errors and warnings, and renderer-consumable output (TASK-018; ADR-0006/ADR-0007; reference parity against pinned `bpmn-io/bpmn-auto-layout@2.0.0-alpha.2` via the dev-only Node harness under `tools/reference-layout/`).
- `bpmn-xml`: self-closing `<boundaryEvent/>` elements now parse as boundary events instead of degrading to the fallback element kind (regression-tested; found by the TASK-018 layout contract tests).
- Documentation: EPIC-005, TASK-018, ADR-0006 (native layered layout, no JS runtime), ADR-0007 (Preserve/Generate/Reflow modes), EXP-004 (reference pin `75a7f309` + performance baseline), TEST-005 (layout test plan).
- `bpmn-layout` tests: twenty complex stress fixtures under `tests/fixtures/complex/` (nested XOR/AND, wide fan-in/fan-out, inclusive gateways, intermediate events, multiple starts/ends, disconnected components, scrambled declaration order, cross-band links, subprocess cascades) running through the geometry-invariant, determinism, warning-contract (`complex.rs`) and renderer-consumability suites, plus a `failures/routing-feedback-leg-blocked.bpmn` regression pinning the structured `layout.routing_failed` behavior of the M1 feedback router (TEST-005 §Fixtures).
- `bpmn-node`: Node.js bindings (napi-rs 3) over the full pipeline — parse, validate, inspect, renderSvg, renderPng (native fonts), auto-layout and layout-rendered SVG for no-DI models; async off the JS thread, explicit `close()`; shipped via the single `@a5i/bpmn-viz` npm package with all four platform binaries (ADR-0009; TASK-020).
- `bpmn-wasm`: synchronous WebAssembly bindings (wasm-bindgen) for parse/validate/inspect/renderSvg/auto-layout, built for `nodejs` and `web` targets via wasm-pack; PNG rasterization stays native-only (ADR-0009; TASK-020).
- `npm/bpmn-viz` (`@a5i/bpmn-viz`): single npm package with per-platform `.node` loading (`platforms.cjs`), a source-checkout dev fallback, TypeScript contracts, assembled WASM runtime under the `/wasm` subpath, release packaging validation, and CLI parity tests (`BPMN_BIN`); `ci.yml` gains `bindings`/`bindings-wasm` jobs and `release.yml` stages one npm package per tag pending 2FA approval (ADR-0009; TASK-020).

### Fixed

- `bpmn-raster`: PNG text no longer silently disappears when DejaVu Sans is absent; `sans-serif`/`serif` resolve over installed fonts with a preferred fallback chain, substitution and missing fonts are reported as `raster.font_substituted`/`raster.missing_fonts` warnings via the new `render_png_with_diagnostics` (TASK-017).
- `bpmn-render`: the diagram viewport now unions DI `BPMNLabel` bounds, so labels placed outside their shape (MIWG A.1.0 "Start Event") are no longer clipped in SVG/PNG output (TASK-017).
- `bpmn-model`: diagnostic codes `raster.missing_fonts` and `raster.font_substituted` (TASK-017).
