#!/usr/bin/env bash
# Release pre-flight: runs scripts/check.sh plus release-specific checks (release guide).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

scripts/check.sh

echo "== docs build =="
cargo doc --workspace --no-deps

echo "== locked release build =="
cargo build --locked --release -p bpmn-cli

echo "== fixture suite sanity =="
target/release/bpmn render tests/fixtures/minimal/start-task-end.bpmn --output /tmp/release-smoke.svg
grep -q "bpmn-shape-Task_1" /tmp/release-smoke.svg

echo "Release checks passed. Tag with: git tag -s v0.1.0"
