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
5. **MIWG smoke** — after `scripts/download-fixtures.sh`: `for f in external/*.bpmn; do bpmn validate "$f"; done` (expect 0 errors).

## Writing a regression test

1. Add the smallest fixture that reproduces the defect under the matching `tests/fixtures/` subdirectory (or extend an existing one).
2. Add the assertion to the matching integration file, referencing stable diagnostic codes (`reference.dangling`) rather than message text.
3. Reference the TASK id in the commit message.

## Fuzzing

`fuzz/fuzz_targets/` contains libfuzzer harnesses for XML parsing and full rendering. They need a nightly toolchain (`cargo +nightly fuzz run xml_parse`); findings should become fixtures under `tests/fixtures/malformed/`.
