#!/usr/bin/env bash
# Visual diff helper for two PNG files (ADR-0004: local review only, not a CI gate).
# Usage: scripts/compare-images.sh actual.png expected.png [diff.png]
set -euo pipefail

actual="${1:?actual png}"
expected="${2:?expected png}"
diff_out="${3:-/tmp/bpmn-image-diff.png}"

if ! command -v magick >/dev/null 2>&1; then
  if ! command -v compare >/dev/null 2>&1; then
    echo "ImageMagick (magick/compare) is required for visual diffs" >&2
    exit 2
  fi
  compare -metric AE "$actual" "$expected" "$diff_out" || true
else
  magick compare -metric AE "$actual" "$expected" "$diff_out" || true
fi

echo "Diff image: $diff_out (AE = absolute error pixel count above)"
