#!/usr/bin/env bash

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -d crates/traverse-registry ]]; then
  echo "traverse-registry must be consumed from crates.io, not this workspace" >&2
  exit 1
fi

cargo tree -p traverse-runtime -i traverse-registry | grep -q 'traverse-registry v0.9.1'

echo "registry consumer cutover uses published traverse-registry v0.9.1."
