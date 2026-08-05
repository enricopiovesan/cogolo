#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
artifact_dir="$script_dir/artifacts"
artifact_path="$artifact_dir/validate-team-readiness-agent.wasm"
repo_root="$(cd "$script_dir/../../.." && pwd)"

mkdir -p "$artifact_dir"

PATH="/Users/enricopiovesan/.rustup/toolchains/1.94.0-aarch64-apple-darwin/bin:$PATH" \
  RUSTFLAGS="-C link-self-contained=no" \
  cargo build --manifest-path "$repo_root/crates/traverse-expedition-wasm/Cargo.toml" \
  --target wasm32-wasip1 --release
cp /Users/enricopiovesan/.cargo-target-shared/wasm32-wasip1/release/traverse-expedition-wasm.wasm "$artifact_path"

printf 'built %s\n' "$artifact_path"
