#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

required_files=(
  "specs/023-downstream-publication-strategy/spec.md"
  "specs/023-downstream-publication-strategy/data-model.md"
  "specs/023-downstream-publication-strategy/checklists/requirements.md"
  "docs/app-consumable-release-checklist.md"
  "docs/app-consumable-release-artifact.md"
  "docs/app-consumable-consumer-bundle.md"
)

for file in "${required_files[@]}"; do
  test -s "${repo_root}/${file}"
done

grep -q "Downstream Publication Strategy for Packaged Traverse Runtime and MCP Artifacts" "${repo_root}/specs/023-downstream-publication-strategy/spec.md"
grep -q "supported published artifact forms" "${repo_root}/specs/023-downstream-publication-strategy/spec.md"
grep -q "release-critical artifact forms" "${repo_root}/specs/023-downstream-publication-strategy/spec.md"
grep -q "youaskm3" "${repo_root}/specs/023-downstream-publication-strategy/spec.md"
grep -q "Publication Strategy" "${repo_root}/specs/023-downstream-publication-strategy/data-model.md"
grep -q "Published Artifact Form" "${repo_root}/specs/023-downstream-publication-strategy/data-model.md"
grep -q "Release-Critical Artifact" "${repo_root}/specs/023-downstream-publication-strategy/data-model.md"
grep -q "Downstream Consumer Target" "${repo_root}/specs/023-downstream-publication-strategy/data-model.md"
grep -q "docs/app-consumable-release-checklist.md" "${repo_root}/docs/app-consumable-release-artifact.md"
grep -q "docs/app-consumable-consumer-bundle.md" "${repo_root}/docs/app-consumable-release-checklist.md"

echo "Traverse downstream publication strategy is ready."
