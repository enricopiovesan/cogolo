#!/usr/bin/env bash

set -euo pipefail

output="$(mktemp)"
trap 'rm -f "$output"' EXIT

printf '%s\n' \
  '{"command":"describe_server"}' \
  '{"command":"list_entrypoints"}' \
  '{"command":"describe_entrypoint","entrypoint_kind":"workflow","id":"expedition.planning.plan-expedition","version":"1.0.0"}' \
  '{"command":"validate_entrypoint","entrypoint_kind":"workflow","id":"expedition.planning.plan-expedition","version":"1.0.0","request_path":"examples/expedition/runtime-requests/plan-expedition.json"}' \
  '{"command":"execute_entrypoint","entrypoint_kind":"workflow","id":"expedition.planning.plan-expedition","version":"1.0.0","request_path":"examples/expedition/runtime-requests/plan-expedition.json"}' \
  '{"command":"render_execution_report","entrypoint_kind":"workflow","id":"expedition.planning.plan-expedition","version":"1.0.0","request_path":"examples/expedition/runtime-requests/plan-expedition.json"}' \
  '{"command":"shutdown"}' | cargo run --quiet -p traverse-mcp -- stdio >"$output"

grep -q '"kind":"mcp_stdio_server_startup"' "$output"
grep -q '"kind":"mcp_stdio_server_entrypoint_list"' "$output"
grep -q '"kind":"mcp_stdio_server_entrypoint_description"' "$output"
grep -q '"kind":"mcp_stdio_server_entrypoint_validation"' "$output"
grep -q '"kind":"mcp_stdio_server_entrypoint_execution"' "$output"
grep -q '"kind":"mcp_stdio_server_execution_report"' "$output"
grep -q '"status":"rendered"' "$output"
grep -q '"kind":"mcp_stdio_server_shutdown"' "$output"

echo "MCP stdio server execution report smoke test passed."
