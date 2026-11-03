#!/usr/bin/env bash
# Vercel build for apps/docs (ADR-0009, TASK-021).
#
# The playground serves the wasm-pack `web` artifact from
# crates/bpmn-wasm/pkg-web; it is a build output and is never committed, and
# Vercel's build image ships no Rust toolchain, so this script bootstraps one,
# compiles the artifact, then runs the normal site build (whose prebuild hook
# copies the artifact into public/runtime/).
#
# Requires the "Include source files outside of the Root Directory in the
# Build Step" project setting (enabled by default) so ../../crates/bpmn-wasm
# is reachable when the Root Directory is apps/docs.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
wasm_dir="$script_dir/../../../crates/bpmn-wasm"

# Pin the compiler to the workspace version (rust-toolchain.toml) without
# pulling its rustfmt/clippy components; the deploy only compiles.
export RUSTUP_TOOLCHAIN="1.99.1"

export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v rustup >/dev/null 2>&1; then
  echo "Installing Rust toolchain $RUSTUP_TOOLCHAIN (minimal profile, wasm32 target)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain "$RUSTUP_TOOLCHAIN" \
      --target wasm32-unknown-unknown --no-modify-path
fi
rustup target add wasm32-unknown-unknown

echo "Building bpmn-wasm browser runtime (wasm-pack, pinned via $wasm_dir/package.json)"
npm --prefix "$wasm_dir" run build:web

if [ ! -f "$wasm_dir/pkg-web/bpmn_wasm_bg.wasm" ]; then
  echo "wasm-pack did not produce $wasm_dir/pkg-web/bpmn_wasm_bg.wasm" >&2
  exit 1
fi

cd "$script_dir/.."
npm run build
