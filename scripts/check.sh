#!/usr/bin/env bash
# Full local release gate (SPEC-001 section 10 quality gates + TEST-001 exit criteria).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

echo "== cargo fmt =="
cargo fmt --all -- --check

echo "== cargo clippy =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== cargo test =="
cargo test --workspace

echo "== docvera =="
scripts/docvera-check.sh

if command -v cargo-deny >/dev/null 2>&1; then
  echo "== cargo deny =="
  cargo deny check
else
  echo "cargo-deny not installed; skipping" >&2
fi

if command -v cargo-audit >/dev/null 2>&1; then
  echo "== cargo audit =="
  cargo audit
else
  echo "cargo-audit not installed; skipping" >&2
fi

echo "All checks passed."
