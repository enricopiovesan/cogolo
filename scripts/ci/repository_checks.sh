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
  "docs/contract-publication-policy.md"
  "docs/expedition-example-authoring.md"
  "docs/expedition-example-smoke.md"
  "examples/expedition/runtime-requests/plan-expedition.json"
  "docs/exception-process.md"
  "docs/project-management.md"
  "docs/multi-thread-workflow.md"
  "docs/ticket-standard.md"
  "docs/planning-board.md"
  "docs/ai-review-process.md"
  "docs/adr/README.md"
  "docs/adr/0001-rust-wasm-foundation.md"
  ".github/ISSUE_TEMPLATE/task.yml"
  "specs/001-foundation-v0-1/spec.md"
  "specs/001-foundation-v0-1/plan.md"
  "specs/001-foundation-v0-1/research.md"
  "specs/001-foundation-v0-1/data-model.md"
  "specs/004-spec-alignment-gate/spec.md"
  "specs/004-spec-alignment-gate/data-model.md"
  "specs/governance/approved-specs.json"
)

for file in "${required_files[@]}"; do
  test -f "$file"
  test -s "$file"
done

if rg -n "Cogollo|Cogolo" . --hidden -g '!.git' -g '!scripts/ci/repository_checks.sh'; then
  echo "Found stale project name references; expected 'Traverse'." >&2
  exit 1
fi

grep -q "GitHub Project 1" README.md
grep -q "Apache-2.0" README.md
grep -q "personal research" README.md
grep -q "Definition of Done" docs/ticket-standard.md
grep -q "in-progress" docs/ticket-standard.md
grep -q "active branch, PR, or an explicitly assigned developer" docs/ticket-standard.md
grep -q "Validation" docs/ticket-standard.md
grep -q "future" docs/project-management.md
grep -q "in-progress" docs/project-management.md
grep -q 'Potential parallel candidates should stay `Ready`' docs/project-management.md
grep -q "Note" docs/project-management.md
grep -q "separate Codex threads" docs/project-management.md
grep -q "Blocked" docs/planning-board.md
grep -q "In Progress" docs/planning-board.md
grep -q "Only tickets with real active execution" docs/planning-board.md
grep -q "Note" docs/ticket-standard.md
grep -q "One Codex thread is one active worker" docs/multi-thread-workflow.md
grep -q "Starter Prompts" docs/multi-thread-workflow.md
grep -q "bash scripts/ci/expedition_artifact_smoke.sh" docs/expedition-example-smoke.md
grep -q "bash scripts/ci/expedition_execution_smoke.sh" docs/expedition-example-smoke.md
grep -q "bash scripts/ci/expedition_trace_smoke.sh" docs/expedition-example-smoke.md
grep -q "TRAVERSE_REPO_ROOT" docs/expedition-example-smoke.md
grep -q "cargo run -p traverse-cli -- bundle inspect examples/expedition/registry-bundle/manifest.json" docs/expedition-example-authoring.md
grep -q "cargo run -p traverse-cli -- expedition execute examples/expedition/runtime-requests/plan-expedition.json" docs/expedition-example-authoring.md
grep -q "cargo run -p traverse-cli -- trace inspect" docs/expedition-example-authoring.md
grep -q "workflows/examples/expedition/plan-expedition/workflow.json" docs/expedition-example-authoring.md
grep -q "label: Definition of done" .github/ISSUE_TEMPLATE/task.yml
grep -q "label: Validation" .github/ISSUE_TEMPLATE/task.yml
grep -q "Specs Are Versioned, Immutable, and Merge-Gating" .specify/memory/constitution.md
grep -q "Non-Functional Requirements" .specify/memory/constitution.md
grep -q "Enterprise Quality Standards" .specify/memory/constitution.md
grep -q "Non-Functional Requirements" specs/001-foundation-v0-1/spec.md
grep -q "Non-Negotiable Quality Standards" specs/001-foundation-v0-1/spec.md
grep -q "AI Review Process" docs/ai-review-process.md
grep -q '"schema_version": "1.0.0"' specs/governance/approved-specs.json
grep -q "Spec-alignment gate implementation" docs/quality-standards.md
grep -q "## Governing Spec" .github/pull_request_template.md

echo "Repository checks passed."
