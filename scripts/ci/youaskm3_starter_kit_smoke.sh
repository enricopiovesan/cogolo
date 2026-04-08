#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

test -s "${repo_root}/apps/youaskm3-starter-kit/README.md"
test -s "${repo_root}/apps/youaskm3-starter-kit/package.json"
test -s "${repo_root}/docs/youaskm3-starter-kit.md"
grep -q "youaskm3 Traverse Starter Kit" "${repo_root}/apps/youaskm3-starter-kit/README.md"
grep -q "versioned Traverse consumer bundle" "${repo_root}/apps/youaskm3-starter-kit/README.md"
grep -q "browser-targeted consumer package" "${repo_root}/apps/youaskm3-starter-kit/README.md"
grep -q "youaskm3 Starter Kit and Integration Guide" "${repo_root}/docs/youaskm3-starter-kit.md"
grep -q "starter kit and integration guide" "${repo_root}/docs/youaskm3-starter-kit.md"
grep -q "bash scripts/ci/youaskm3_integration_validation.sh" "${repo_root}/docs/youaskm3-starter-kit.md"

echo "youaskm3 starter kit smoke passed."
