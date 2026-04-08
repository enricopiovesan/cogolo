#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

required_files=(
  "docs/packaged-traverse-runtime-artifact.md"
  "docs/app-consumable-release-artifact.md"
  "docs/app-consumable-consumer-bundle.md"
  "docs/app-consumable-release-checklist.md"
)

for file in "${required_files[@]}"; do
  test -s "${repo_root}/${file}"
done

grep -q "Traverse v0.1 Packaged Runtime Artifact" "${repo_root}/docs/packaged-traverse-runtime-artifact.md"
grep -q "versioned runtime artifact bundle" "${repo_root}/docs/packaged-traverse-runtime-artifact.md"
grep -q "packaged runtime artifact reference" "${repo_root}/docs/packaged-traverse-runtime-artifact.md"
grep -q "app-consumable quickstart" "${repo_root}/docs/packaged-traverse-runtime-artifact.md"
grep -q "bash scripts/ci/packaged_traverse_runtime_artifact_smoke.sh" "${repo_root}/docs/packaged-traverse-runtime-artifact.md"

echo "Traverse packaged runtime artifact is ready."
