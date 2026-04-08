#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

required_files=(
  "docs/wasm-microservice-authoring-guide.md"
  "docs/adapter-boundaries.md"
  "docs/compatibility-policy.md"
  "docs/oss-pattern-extraction.md"
  "examples/templates/executable-capability-package/manifest.template.json"
)

for file in "${required_files[@]}"; do
  test -s "${repo_root}/${file}"
done

grep -q "Traverse WASM Microservice Authoring Guide" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "governed executable package" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "package_id" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "capability_ref" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "workflow_refs" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "binary" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "constraints" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "model_dependencies" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "docs/adapter-boundaries.md" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "docs/compatibility-policy.md" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "docs/oss-pattern-extraction.md" "${repo_root}/docs/wasm-microservice-authoring-guide.md"
grep -q "bash scripts/ci/wasm_microservice_authoring_guide_smoke.sh" "${repo_root}/docs/wasm-microservice-authoring-guide.md"

echo "Traverse WASM microservice authoring guide is ready."
