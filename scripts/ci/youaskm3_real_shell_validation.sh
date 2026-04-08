#!/usr/bin/env bash

set -euo pipefail

required_files=(
  "docs/youaskm3-real-shell-validation.md"
  "docs/youaskm3-integration-validation.md"
  "docs/youaskm3-compatibility-conformance-suite.md"
  "docs/app-consumable-consumer-bundle.md"
)

for file in "${required_files[@]}"; do
  test -f "$file"
  test -s "$file"
done

grep -q "youaskm3 real shell validation" docs/youaskm3-real-shell-validation.md
grep -q "bash scripts/ci/youaskm3_real_shell_validation.sh" docs/youaskm3-real-shell-validation.md
grep -q "YOUASKM3_REPO_ROOT" docs/youaskm3-real-shell-validation.md
grep -q "openspec/specs/pwa-shell/spec.md" docs/youaskm3-real-shell-validation.md
grep -q "scripts/smoke.sh" docs/youaskm3-real-shell-validation.md
grep -q "released Traverse consumer artifacts" docs/youaskm3-real-shell-validation.md

if [[ -n "${YOUASKM3_REPO_ROOT:-}" ]]; then
  test -d "${YOUASKM3_REPO_ROOT}"
  test -f "${YOUASKM3_REPO_ROOT}/README.md"
  test -f "${YOUASKM3_REPO_ROOT}/openspec/specs/pwa-shell/spec.md"
  test -f "${YOUASKM3_REPO_ROOT}/scripts/smoke.sh"
  bash "${YOUASKM3_REPO_ROOT}/scripts/smoke.sh"
fi

echo "youaskm3 real shell validation passed."
