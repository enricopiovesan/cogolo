#!/usr/bin/env bash

set -euo pipefail

classifier="$(cd "$(dirname "$0")" && pwd)/pr_change_classification.sh"

assert_classification() {
  local expected="$1"
  shift
  local actual
  actual="$(printf '%s\n' "$@" | "$classifier" --paths-stdin | awk -F= '/^full_ci=/{print $2}')"
  [[ "$actual" == "$expected" ]] || {
    echo "expected full_ci=${expected}, got ${actual}" >&2
    exit 1
  }
}

assert_classification false docs/getting-started.md docs/adr/0051-risk-based-ci.md specs/105-example/spec.md
assert_classification true specs/governance/approved-specs.json
assert_classification true crates/traverse-runtime/src/lib.rs
assert_classification true .github/workflows/ci.yml
assert_classification true scripts/ci/rust_checks.sh

echo "PR change classification tests passed."
