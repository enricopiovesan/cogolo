#!/usr/bin/env bash

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

cargo test -p traverse-cli-rs --lib app::tests

echo "downstream app bundle registration smoke passed."
