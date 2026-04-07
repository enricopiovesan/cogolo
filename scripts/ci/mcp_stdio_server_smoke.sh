#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
stdout_log=$(mktemp)
stderr_log=$(mktemp)
failure_stdout_log=$(mktemp)
failure_stderr_log=$(mktemp)

cleanup() {
  rm -f "${stdout_log}" "${stderr_log}" "${failure_stdout_log}" "${failure_stderr_log}"
}

trap cleanup EXIT

printf '%s\n' '{"command":"describe"}' '{"command":"shutdown"}' \
  | cargo run -p traverse-mcp -- stdio >"${stdout_log}" 2>"${stderr_log}"

grep -q '"kind":"mcp_stdio_server_startup"' "${stdout_log}"
grep -q '"host_mode":"stdio"' "${stdout_log}"
grep -q '"kind":"mcp_stdio_server_description"' "${stdout_log}"
grep -q '"kind":"mcp_stdio_server_shutdown"' "${stdout_log}"

: >"${failure_stdout_log}"
set +e
cargo run -p traverse-mcp -- stdio --simulate-startup-failure \
  >"${failure_stdout_log}" 2>"${failure_stderr_log}"
failure_status=$?
set -e

if [[ ${failure_status} -eq 0 ]]; then
  echo "Expected simulated startup failure to exit non-zero." >&2
  exit 1
fi

grep -q '"kind":"mcp_stdio_server_error"' "${failure_stderr_log}"
grep -q '"code":"startup_failed"' "${failure_stderr_log}"
test ! -s "${failure_stdout_log}"

echo "MCP stdio server smoke passed."
