---
id: DOC-007
type: note
title: Testing guide
status: active
---
# Testing guide

Layers (ADR-0004), cheapest first:

1. **Unit tests** live inside each crate (`#[cfg(test)] mod tests`) and pin invariants: bounds validation, namespace resolution, scene ordering, wrapping, serializer escaping, raster limits.
2. **Fixture integration tests** are in `crates/bpmn-cli/tests/{parse,render,cli}.rs` and reference fixtures from the workspace-root `tests/fixtures/` directory.
3. **Golden test** — `tests/fixtures/minimal/start-task-end.svg` is compared byte-for-byte. To update it intentionally: `cargo run -p bpmn-cli -- render tests/fixtures/minimal/start-task-end.bpmn --output tests/fixtures/minimal/start-task-end.svg`, review the diff, commit.
4. **PNG assertions** check dimensions and non-background pixel counts at 1x/2x — never pixel equality.
5. **External corpora smoke** — after `scripts/download-fixtures.sh`: `for f in external/*.bpmn external/pmo/bpmn/*.bpmn; do bpmn validate "$f"; done`. Findings on real-world files are data diagnostics, not tool crashes; expected counts are recorded in [REPORT-002](../reports/REPORT-002-external-corpus-smoke.md). PMo no-DI layout smoke: `cargo test -p bpmn-layout --test corpus_pmo` (self-skips without `external/`; per-PR CI stays synthetic-only per ADR-0005).

## Writing a regression test

1. Add the smallest fixture that reproduces the defect under the matching `tests/fixtures/` subdirectory (or extend an existing one).
2. Add the assertion to the matching integration file, referencing stable diagnostic codes (`reference.dangling`) rather than message text.
3. Reference the TASK id in the commit message.

## Fuzzing

`fuzz/fuzz_targets/` contains libfuzzer harnesses for XML parsing and full rendering. They need a nightly toolchain (`cargo +nightly fuzz run xml_parse`); findings should become fixtures under `tests/fixtures/malformed/`.
