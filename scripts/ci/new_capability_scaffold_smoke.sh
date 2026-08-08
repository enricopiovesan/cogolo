#!/usr/bin/env bash
set -euo pipefail

# scripts/scaffold/new-capability.sh is retired (spec
# 100-capability-package-authoring FR-009): it must now fail loudly and
# redirect authors to `traverse-cli capability new` rather than silently
# emitting the stale, non-loadable scaffold shape it used to produce.

REPO_ROOT="${TRAVERSE_REPO_ROOT:-$(pwd)}"

set +e
OUTPUT="$(bash "$REPO_ROOT/scripts/scaffold/new-capability.sh" --name smoke-test --namespace ci.smoke 2>&1)"
STATUS=$?
set -e

if [[ "$STATUS" -eq 0 ]]; then
  echo "expected scripts/scaffold/new-capability.sh to exit non-zero (retired script)" >&2
  exit 1
fi

if [[ "$OUTPUT" != *"traverse-cli capability new"* ]]; then
  echo "expected retirement message to redirect to 'traverse-cli capability new':" >&2
  echo "$OUTPUT" >&2
  exit 1
fi

echo "Scaffold retirement smoke: PASS"
