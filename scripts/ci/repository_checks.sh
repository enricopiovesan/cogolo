#!/usr/bin/env bash

set -euo pipefail

required_files=(
  "README.md"
  "LICENSE"
  "NOTICE"
  "CONTRIBUTING.md"
  "CODE_OF_CONDUCT.md"
  "SECURITY.md"
  "SUPPORT.md"
  ".editorconfig"
  ".specify/memory/constitution.md"
  "docs/quality-standards.md"
  "docs/compatibility-policy.md"
  "docs/exception-process.md"
  "docs/project-management.md"
  "docs/ai-review-process.md"
  "docs/adr/README.md"
  "docs/adr/0001-rust-wasm-foundation.md"
  "specs/001-foundation-v0-1/spec.md"
  "specs/001-foundation-v0-1/plan.md"
  "specs/001-foundation-v0-1/research.md"
  "specs/001-foundation-v0-1/data-model.md"
)

for file in "${required_files[@]}"; do
  test -f "$file"
  test -s "$file"
done

if rg -n "Cogollo" . -g '!scripts/ci/repository_checks.sh'; then
  echo "Found stale project name references; expected 'Cogolo'." >&2
  exit 1
fi

grep -q "GitHub Project 1" README.md
grep -q "Apache-2.0" README.md
grep -q "personal research" README.md
grep -q "Specs Are Versioned, Immutable, and Merge-Gating" .specify/memory/constitution.md
grep -q "Non-Functional Requirements" .specify/memory/constitution.md
grep -q "Enterprise Quality Standards" .specify/memory/constitution.md
grep -q "Non-Functional Requirements" specs/001-foundation-v0-1/spec.md
grep -q "Non-Negotiable Quality Standards" specs/001-foundation-v0-1/spec.md
grep -q "AI Review Process" docs/ai-review-process.md

echo "Repository checks passed."
