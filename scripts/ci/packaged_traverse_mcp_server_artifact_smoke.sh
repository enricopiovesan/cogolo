#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

required_files=(
  "docs/packaged-traverse-mcp-server-artifact.md"
  "docs/mcp-stdio-server.md"
  "docs/mcp-consumption-validation.md"
  "docs/app-consumable-release-checklist.md"
)

for file in "${required_files[@]}"; do
  test -s "${repo_root}/${file}"
done

grep -q "Traverse v0.1 Packaged MCP Server Artifact" "${repo_root}/docs/packaged-traverse-mcp-server-artifact.md"
grep -q "versioned MCP server artifact bundle" "${repo_root}/docs/packaged-traverse-mcp-server-artifact.md"
grep -q "packaged MCP server artifact reference" "${repo_root}/docs/packaged-traverse-mcp-server-artifact.md"
grep -q "MCP consumption validation" "${repo_root}/docs/packaged-traverse-mcp-server-artifact.md"
grep -q "bash scripts/ci/packaged_traverse_mcp_server_artifact_smoke.sh" "${repo_root}/docs/packaged-traverse-mcp-server-artifact.md"

echo "Traverse packaged MCP server artifact is ready."
