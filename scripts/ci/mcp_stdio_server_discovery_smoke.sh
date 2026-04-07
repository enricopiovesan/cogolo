#!/usr/bin/env bash

set -euo pipefail

stdout_log=$(mktemp)
stderr_log=$(mktemp)

cleanup() {
  rm -f "${stdout_log}" "${stderr_log}"
}

trap cleanup EXIT

printf '%s\n' \
  '{"command":"list_content_groups"}' \
  '{"command":"describe_content_group","content_group_id":"core-runtime-example"}' \
  '{"command":"describe_content_group","content_group_id":"wasm-agent-example"}' \
  '{"command":"list_entrypoints"}' \
  '{"command":"describe_entrypoint","entrypoint_kind":"capability","id":"expedition.planning.capture-expedition-objective","version":"1.0.0"}' \
  '{"command":"describe_entrypoint","entrypoint_kind":"workflow","id":"expedition.planning.plan-expedition","version":"1.0.0"}' \
  '{"command":"shutdown"}' \
  | cargo run --quiet -p traverse-mcp -- stdio >"${stdout_log}" 2>"${stderr_log}"

grep -q '"kind":"mcp_stdio_server_startup"' "${stdout_log}"
grep -q '"kind":"mcp_stdio_server_content_group_list"' "${stdout_log}"
grep -q '"kind":"mcp_stdio_server_content_group_description"' "${stdout_log}"
grep -q '"content_group_id":"core-runtime-example"' "${stdout_log}"
grep -q '"content_group_id":"wasm-agent-example"' "${stdout_log}"
grep -q '"kind":"mcp_stdio_server_entrypoint_list"' "${stdout_log}"
grep -q '"artifact_kind":"capability"' "${stdout_log}"
grep -q '"artifact_kind":"workflow"' "${stdout_log}"
grep -q '"kind":"mcp_stdio_server_shutdown"' "${stdout_log}"

test ! -s "${stderr_log}"

echo "MCP stdio server discovery smoke passed."
