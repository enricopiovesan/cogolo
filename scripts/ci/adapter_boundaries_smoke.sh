#!/usr/bin/env bash

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"

doc="${repo_root}/docs/adapter-boundaries.md"
readme="${repo_root}/README.md"
compat="${repo_root}/docs/compatibility-policy.md"

test -f "$doc"
test -s "$doc"

grep -q "core runtime narrow and topology-agnostic" "$doc"
grep -q "capability, event, and workflow contracts" "$doc"
grep -q "trace and runtime evidence generation" "$doc"
grep -q "browser-subscription and MCP-facing payload contracts" "$doc"
grep -q "transport bindings such as HTTP, browser, IPC" "$doc"
grep -q "not adopting a mandatory sidecar topology" "$doc"
grep -q "optional adapter choices" "$doc"
grep -q 'specs/013-browser-runtime-subscription/spec.md' "$doc"
grep -q 'specs/014-mcp-surface/spec.md' "$doc"

grep -q 'docs/adapter-boundaries.md' "$readme"
grep -q 'docs/adapter-boundaries.md' "$compat"

printf 'Adapter boundaries smoke passed.\n'
