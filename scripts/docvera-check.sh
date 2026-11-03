#!/usr/bin/env bash
# DocVera validation gate (EXP-003). Exit 0 = clean, 1 = findings, 2 = usage.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v docvera >/dev/null 2>&1; then
  echo "docvera CLI not found on PATH; install with: cargo install --locked docvera-cli" >&2
  exit 2
fi

version="$(docvera --version | awk '{print $2}')"
major="$(echo "$version" | cut -d. -f1)"
minor="$(echo "$version" | cut -d. -f2)"
if [ "$major" -ne 0 ] || [ "$minor" -lt 1 ]; then
  echo "docvera $version is too old; this workspace is validated against 0.1.2+" >&2
  exit 2
fi

cd "$root"
echo "docvera $version — checking docs/ against docvera.toml"
exec docvera check "$@"
