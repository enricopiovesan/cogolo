#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

required_files=(
  "docs/mcp-real-agent-exercise.md"
  "docs/mcp-stdio-server.md"
  "docs/mcp-consumption-validation.md"
  "docs/youaskm3-integration-validation.md"
)

for file in "${required_files[@]}"; do
  test -s "${repo_root}/${file}"
done

grep -q "Traverse MCP Real-Agent Exercise" "${repo_root}/docs/mcp-real-agent-exercise.md"
grep -q "discover_capabilities" "${repo_root}/docs/mcp-real-agent-exercise.md"
grep -q "discover_events" "${repo_root}/docs/mcp-real-agent-exercise.md"
grep -q "discover_workflows" "${repo_root}/docs/mcp-real-agent-exercise.md"
grep -q "execute_entrypoint" "${repo_root}/docs/mcp-real-agent-exercise.md"
grep -q "render_execution_report" "${repo_root}/docs/mcp-real-agent-exercise.md"
grep -q "bash scripts/ci/mcp_real_agent_exercise_smoke.sh" "${repo_root}/docs/mcp-real-agent-exercise.md"
grep -q "docs/mcp-real-agent-exercise.md" "${repo_root}/README.md"
grep -q "docs/mcp-real-agent-exercise.md" "${repo_root}/docs/mcp-consumption-validation.md"
grep -q "docs/mcp-real-agent-exercise.md" "${repo_root}/docs/youaskm3-integration-validation.md"
grep -q "discover_capabilities" "${repo_root}/crates/traverse-mcp/src/lib.rs"
grep -q "discover_events" "${repo_root}/crates/traverse-mcp/src/lib.rs"
grep -q "discover_workflows" "${repo_root}/crates/traverse-mcp/src/lib.rs"

echo "Traverse MCP real-agent exercise is ready."
