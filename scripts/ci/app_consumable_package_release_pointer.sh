#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

required_files=(
  "docs/app-consumable-package-release-pointer.md"
  "docs/app-consumable-release-artifact.md"
  "docs/app-consumable-consumer-bundle.md"
  "docs/app-consumable-release-checklist.md"
)

for file in "${required_files[@]}"; do
  test -s "${repo_root}/${file}"
done

grep -q "package release pointer" "${repo_root}/docs/app-consumable-package-release-pointer.md"
grep -q "versioned package release pointer" "${repo_root}/docs/app-consumable-package-release-pointer.md"
grep -q "release bundle definition" "${repo_root}/docs/app-consumable-package-release-pointer.md"
grep -q "bash scripts/ci/app_consumable_package_release_pointer.sh" "${repo_root}/docs/app-consumable-package-release-pointer.md"
grep -q "docs/app-consumable-package-release-pointer.md" "${repo_root}/README.md"
grep -q "docs/app-consumable-package-release-pointer.md" "${repo_root}/docs/app-consumable-consumer-bundle.md"
grep -q "docs/app-consumable-package-release-pointer.md" "${repo_root}/docs/app-consumable-release-artifact.md"
grep -q "docs/app-consumable-package-release-pointer.md" "${repo_root}/docs/app-consumable-release-checklist.md"

echo "Traverse v0.1 app-consumable package release pointer is ready."
