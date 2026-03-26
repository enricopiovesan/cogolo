#!/usr/bin/env bash

set -euo pipefail

body_file="${1:-}"

if [[ -z "${body_file}" ]]; then
  echo "Usage: $0 <pr-body-file>" >&2
  exit 1
fi

test -s "$body_file"

grep -q "## Governing Spec" "$body_file"
grep -q "## Project Item" "$body_file"
grep -q "## Validation" "$body_file"

echo "PR body check passed."
