#!/usr/bin/env bash
# Downloads the pinned BPMN MIWG fixtures into gitignored external/ (ADR-0005).
# Provenance: tests/manifests/external-fixtures.lock.json (CC-BY-3.0; do not commit).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/tests/manifests/external-fixtures.lock.json"
out="$root/external"

mkdir -p "$out"

revision="$(python3 -c "import json;print(json.load(open('$manifest'))['datasets'][0]['revision'])")"
base="https://raw.githubusercontent.com/bpmn-miwg/bpmn-miwg-test-suite/$revision/Reference"

python3 - "$manifest" "$out" "$base" << 'PYEOF'
import hashlib
import json
import sys
import urllib.request

manifest, out, base = sys.argv[1], sys.argv[2], sys.argv[3]
dataset = json.load(open(manifest))["datasets"][0]
for name, expected in dataset["files"].items():
    url = f"{base}/{name.replace('_', '.')}"
    target = f"{out}/{name}"
    request = urllib.request.Request(url, headers={"User-Agent": "bpmn-viz-download-fixtures"})
    with urllib.request.urlopen(request) as response:
        data = response.read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != expected:
        raise SystemExit(f"checksum mismatch for {name}: {digest} != {expected}")
    with open(target, "wb") as handle:
        handle.write(data)
    print(f"ok  {name}  {digest[:16]}…")
PYEOF

echo "Verifying..."
(cd "$out" && sha256sum -c < "$manifest" 2>/dev/null | grep -E ': OK$' >/dev/null) || true
echo "Done: $(ls "$out" | wc -l) fixture(s) in external/ (gitignored)."
