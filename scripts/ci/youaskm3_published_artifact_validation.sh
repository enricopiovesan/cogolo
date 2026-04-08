#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

required_files=(
  "docs/youaskm3-published-artifact-validation.md"
  "docs/packaged-traverse-runtime-artifact.md"
  "docs/packaged-traverse-mcp-server-artifact.md"
  "docs/app-consumable-release-artifact.md"
  "docs/app-consumable-package-release-pointer.md"
  "docs/youaskm3-integration-validation.md"
  "docs/youaskm3-real-shell-validation.md"
  "docs/app-consumable-release-checklist.md"
  "docs/app-consumable-requirements-traceability.md"
)

for file in "${required_files[@]}"; do
  test -s "${repo_root}/${file}"
done

bash "${repo_root}/scripts/ci/packaged_traverse_runtime_artifact_smoke.sh"
bash "${repo_root}/scripts/ci/packaged_traverse_mcp_server_artifact_smoke.sh"
bash "${repo_root}/scripts/ci/youaskm3_real_shell_validation.sh"

grep -q "Traverse v0.1 Published Artifact Consumption Validation for \`youaskm3\`" "${repo_root}/docs/youaskm3-published-artifact-validation.md"
grep -q "packaged runtime artifact" "${repo_root}/docs/youaskm3-published-artifact-validation.md"
grep -q "packaged MCP server artifact" "${repo_root}/docs/youaskm3-published-artifact-validation.md"
grep -q "Traverse v0.1.0" "${repo_root}/docs/youaskm3-published-artifact-validation.md"
grep -q "consumer_name: youaskm3" "${repo_root}/docs/youaskm3-published-artifact-validation.md"
grep -q "validated_flow_id: youaskm3_published_artifact_validation" "${repo_root}/docs/youaskm3-published-artifact-validation.md"
grep -q "bash scripts/ci/packaged_traverse_runtime_artifact_smoke.sh" "${repo_root}/docs/youaskm3-published-artifact-validation.md"
grep -q "bash scripts/ci/packaged_traverse_mcp_server_artifact_smoke.sh" "${repo_root}/docs/youaskm3-published-artifact-validation.md"
grep -q "bash scripts/ci/youaskm3_published_artifact_validation.sh" "${repo_root}/docs/youaskm3-published-artifact-validation.md"
grep -q "docs/youaskm3-published-artifact-validation.md" "${repo_root}/docs/app-consumable-release-checklist.md"
grep -q "docs/packaged-traverse-runtime-artifact.md" "${repo_root}/docs/app-consumable-release-artifact.md"
grep -q "docs/packaged-traverse-mcp-server-artifact.md" "${repo_root}/docs/app-consumable-release-artifact.md"
grep -q "docs/youaskm3-published-artifact-validation.md" "${repo_root}/docs/app-consumable-requirements-traceability.md"
grep -q "published-artifact validation" "${repo_root}/docs/youaskm3-integration-validation.md"

echo "Traverse published-artifact validation for youaskm3 is ready."
