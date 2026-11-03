---
id: DOC-006
type: note
title: Development guide
status: active
---
# Development guide

## Requirements

- Rust 1.99.1 (pinned in `rust-toolchain.toml`; rustfmt + clippy components).
- No Node.js, browser or C dependencies for the core build.
- Optional: `cargo-deny`, `cargo audit`, `docvera` CLI (0.1.2 or compatible) for the full gate.

## Daily commands

```sh
cargo build --workspace
cargo test --workspace            # 26 suites incl. bpmn-layout, self-contained fixtures
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo bench -p bpmn-cli --bench render   # performance baseline (REPORT-001)
docvera check                     # documentation gate (needs docvera on PATH)
scripts/check.sh                  # everything above in one shot
```

## Try the CLI

```sh
cargo run -p bpmn-cli -- render tests/fixtures/minimal/start-task-end.bpmn --output out.svg
cargo run -p bpmn-cli -- render diagram.bpmn --format png --scale 2 --output out.png
cargo run -p bpmn-cli -- validate diagram.bpmn --format json
cargo run -p bpmn-cli -- inspect diagram.bpmn
```

Exit codes: 0 success, 2 usage, 3 invalid BPMN/DI, 4 IO, 5 render failure.

## External MIWG fixtures

```sh
scripts/download-fixtures.sh       # populates gitignored external/
bpmn validate external/A_1_0.bpmn
```

Provenance lives in `tests/manifests/external-fixtures.lock.json` (ADR-0005).

## Agent workflow

Read `AGENTS.md` first; every change references a TASK id, and public API, licensing or DI-semantic changes require an ADR proposal (SPEC-001 section 12).
