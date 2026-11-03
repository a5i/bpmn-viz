#!/usr/bin/env bash
# Downloads pinned external corpora into gitignored external/ (ADR-0005, ADR-0008).
# Provenance: tests/manifests/external-fixtures.lock.json (per-dataset license and
# attribution; no corpus bytes are committed). Local/scheduled use only — per-PR CI
# stays synthetic-only.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/tests/manifests/external-fixtures.lock.json"
out="$root/external"

mkdir -p "$out" "$out/archives"

python3 - "$manifest" "$out" << 'PYEOF'
import hashlib
import json
import pathlib
import sys
import urllib.request
import zipfile

manifest_path, out_root = (pathlib.Path(a) for a in sys.argv[1:3])
manifest = json.loads(manifest_path.read_text())
user_agent = {"User-Agent": "bpmn-viz-download-fixtures"}


def fetch(url: str) -> bytes:
    request = urllib.request.Request(url, headers=user_agent)
    with urllib.request.urlopen(request, timeout=300) as response:
        return response.read()


def digest(data: bytes, algorithm: str) -> str:
    return hashlib.new(algorithm, data).hexdigest()


def ensure_file(rel: str, expected_sha256: str, producer) -> None:
    """Materialize external/<rel>; skip download when the digest already matches."""
    target = out_root / rel
    if target.exists() and digest(target.read_bytes(), "sha256") == expected_sha256:
        print(f"ok   {rel} (cached)")
        return
    data = producer()
    got = digest(data, "sha256")
    if got != expected_sha256:
        raise SystemExit(f"checksum mismatch for {rel}: {got} != {expected_sha256}")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    print(f"ok   {rel} {got[:16]}…")


archive_cache: dict[str, zipfile.ZipFile] = {}


def open_archive(dataset: dict) -> zipfile.ZipFile:
    rel = dataset["fetch"]["archive"]
    cached = archive_cache.get(rel)
    if cached is not None:
        return cached
    archive_path = out_root / rel

    def producer() -> bytes:
        return fetch(dataset["fetch"]["archive_url"])

    expected = dataset["fetch"]["archive_sha256"]
    if archive_path.exists() and digest(archive_path.read_bytes(), "sha256") == expected:
        print(f"ok   {rel} (cached)")
    else:
        data = producer()
        got = digest(data, "sha256")
        if got != expected:
            raise SystemExit(f"checksum mismatch for archive {rel}: {got} != {expected}")
        archive_path.parent.mkdir(parents=True, exist_ok=True)
        archive_path.write_bytes(data)
        print(f"ok   {rel} {got[:16]}…")
    upstream_md5 = dataset["fetch"].get("archive_md5")
    if upstream_md5 and digest(archive_path.read_bytes(), "md5") != upstream_md5:
        raise SystemExit(f"upstream md5 mismatch for archive {rel}")
    handle = zipfile.ZipFile(archive_path)
    archive_cache[rel] = handle
    return handle


for dataset in manifest["datasets"]:
    print(f"== {dataset['name']} ({dataset['license']}) ==")
    fetch_spec = dataset["fetch"]
    if fetch_spec["type"] == "github-raw":
        for name in sorted(dataset["files"]):
            url = f"{fetch_spec['base']}/{name.replace('_', '.')}"
            ensure_file(name, dataset["files"][name], lambda url=url: fetch(url))
    elif fetch_spec["type"] == "zenodo-archive":
        archive = open_archive(dataset)
        entries = {i.filename: i for i in archive.infolist() if not i.is_dir()}
        selected = tuple(fetch_spec["extract_selected"])
        for rel in sorted(dataset["files"]):
            zip_path = rel.split("/", 1)[1]  # "pmo/bpmn/01.bpmn" -> "bpmn/01.bpmn"
            if not zip_path.startswith(selected):
                raise SystemExit(f"manifest file {rel} is outside extract_selected")
            if zip_path not in entries:
                raise SystemExit(f"archive entry missing: {zip_path}")
            ensure_file(rel, dataset["files"][rel], lambda z=zip_path: archive.read(z))
    else:
        raise SystemExit(f"unknown fetch type: {fetch_spec['type']}")

# Full verification pass over everything on disk.
checked = 0
for dataset in manifest["datasets"]:
    for rel, expected in dataset["files"].items():
        target = out_root / rel
        if not target.exists() or digest(target.read_bytes(), "sha256") != expected:
            raise SystemExit(f"verification failed for {rel}")
        checked += 1
print(f"verified {checked} file(s) against {manifest_path.relative_to(out_root.parent)}")
PYEOF

echo "Done: corpora under external/ (gitignored)."
