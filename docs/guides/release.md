---
id: DOC-008
type: note
title: Release guide
status: active
---
# Release guide

## Pre-flight (release gate)

```sh
scripts/check.sh            # fmt, clippy, tests, docvera, deny, audit
cargo doc --workspace --no-deps   # docs build without warnings
```

CI must be green on all three OS runners before tagging (TASK-015).

## Versioning

- Workspace version lives in `[workspace.package].version` in the root `Cargo.toml`; every crate inherits it via `version.workspace = true`, and `npm/bpmn-viz/package.json` must match (release.yml enforces all three on tags).
- `CHANGELOG.md` records user-visible changes per release.
- Crates are `publish = false` until crates.io names are verified (SPEC-001 open question; TASK-016).
- The npm package `@a5i/bpmn-viz` is verified for registry availability before its first publish (ADR-0009 open question; TASK-020).

## Licensing and attribution

- Code is MIT (`LICENSE`).
- BPMN MIWG data is CC-BY-3.0 and is **not** redistributed; if a release artifact bundles diagrams generated from MIWG files, attribute the [BPMN MIWG test suite](https://github.com/bpmn-miwg/bpmn-miwg-test-suite) and cite the pinned revision from `tests/manifests/external-fixtures.lock.json`.
- PMo Dataset (Zenodo 10.5281/zenodo.15857589) is CC-BY-4.0 and is **not** redistributed; attribute "PMo Dataset v1.0.0, Brissard, Cuppens & Zouaq (Polytechnique Montréal)" and cite Brissard et al., *What is the Best Process Model Representation? A Comparative Analysis for Process Modeling with Large Language Models*, AI4BPM Workshop at BPM 2025, alongside the pinned version from `tests/manifests/external-fixtures.lock.json`.
- Dependency license compliance is enforced by `deny.toml` (`cargo deny check licenses`); generate notices with `cargo about` if a binary distribution is produced.

## Reproducibility

- `Cargo.lock` is committed; releases build with `cargo build --locked --release -p bpmn-cli`.
- The toolchain is pinned by `rust-toolchain.toml`.
- SVG output is deterministic by design; PNG depends on the local font environment (documented in ADR-0004/RISK-002) — ship SVG for archival output.

## Tagging

```sh
git tag -s v0.1.0 -m "BPMN renderer prototype v0.1.0"
git push origin v0.1.0
```

## npm package (`@a5i/bpmn-viz`, ADR-0009/TASK-020)

A `v*.*.*` tag runs `.github/workflows/release.yml`: version-consistency gate, workspace tests, release builds on four platforms (CLI archives + checksums), the WASM build with Node and browser smokes, then **one staged npm package** with all platform binaries plus WASM — nothing publishes without the local 2FA approval (`npm stage approve <stage-id>`). One-time setup: register the npm Trusted Publisher for `@a5i/bpmn-viz` (org `a5i`, repo `bpmn-viz`, workflow `release.yml`, allow `npm stage publish` only) after verifying the package name.

Local equivalents:

```sh
npm --prefix npm/bpmn-viz run build            # debug addon (dev)
npm --prefix npm/bpmn-viz run build:release    # release addon
npm --prefix npm/bpmn-viz run build:wasm       # wasm-pack nodejs/web + assemble
npm --prefix npm/bpmn-viz test                 # addon tests (BPMN_BIN=… for parity)
npm --prefix npm/bpmn-viz run test:wasm        # packaged WASM runtime
npm --prefix npm/bpmn-viz run validate:release # staged-package gate
```
