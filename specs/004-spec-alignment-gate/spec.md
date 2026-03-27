# Feature Specification: Cogolo Spec-Alignment CI Gate

**Feature Branch**: `004-spec-alignment-gate`  
**Created**: 2026-03-27  
**Status**: Draft  
**Input**: Existing Cogolo foundation specification, constitution, quality standards, and issue `#7` for deterministic spec-alignment enforcement.

## Purpose

This specification defines the first deterministic spec-alignment gate for Cogolo pull requests.

The gate exists to make the governance rule enforceable:

- approved specs are versioned and immutable
- governed implementation must align with an approved spec
- pull requests that change governed implementation without approved spec coverage must fail

This first version is intentionally narrow. It focuses on deterministic mapping between changed governed paths and approved spec records instead of attempting full semantic validation of every requirement.

## User Scenarios & Testing

### User Story 1 - Fail PRs That Change Governed Code Without Approved Spec Coverage (Priority: P1)

As a maintainer, I want the CI pipeline to fail when a pull request changes governed implementation without an approved governing spec so that code cannot drift ahead of the documented source of truth.

**Why this priority**: This is the core constitutional rule. Without it, the repo only documents spec alignment rather than enforcing it.

**Independent Test**: A pull request that changes a governed file under a protected path without a matching approved spec record fails the spec-alignment job with an actionable explanation.

**Acceptance Scenarios**:

1. **Given** a pull request that changes a governed file under `crates/` or another protected path, **When** no approved spec record governs that file, **Then** the spec-alignment job fails with the unmatched file path.
2. **Given** a pull request that changes governed files covered by approved spec records, **When** the PR body declares those governing specs, **Then** the spec-alignment job passes.
3. **Given** a pull request that only changes non-governed docs or community files, **When** the spec-alignment job runs, **Then** the job passes without requiring a governing spec declaration.

---

### User Story 2 - Fail PRs That Reference Missing or Unapproved Specs (Priority: P1)

As a maintainer, I want the CI pipeline to fail when a pull request references specs that are unknown, non-immutable, or not approved for implementation so that the governing-spec mechanism stays trustworthy.

**Why this priority**: Spec references are only meaningful if they point to explicit approved artifacts.

**Independent Test**: A pull request whose `## Governing Spec` section references a missing, draft, mutable, or mismatched spec record fails the spec-alignment job.

**Acceptance Scenarios**:

1. **Given** a PR body that references a spec id not present in the approved spec registry, **When** validation runs, **Then** the spec-alignment job fails.
2. **Given** a PR body that references a spec record with `status != approved` or `immutable != true`, **When** validation runs, **Then** the spec-alignment job fails.
3. **Given** a PR body that references an approved immutable spec record, **When** validation runs, **Then** the gate accepts that reference.

---

### User Story 3 - Provide Deterministic Merge-Safe Evidence (Priority: P2)

As a maintainer, I want the spec-alignment gate to produce deterministic, actionable output so that developers can understand exactly why a PR passed or failed and CI results remain stable.

**Why this priority**: A merge gate that is noisy or ambiguous will be bypassed or ignored.

**Independent Test**: Re-running the gate on the same PR content and diff produces the same pass or fail result and the same failure categories.

**Acceptance Scenarios**:

1. **Given** the same approved spec registry, PR body, and file diff, **When** the job runs multiple times, **Then** it produces the same result deterministically.
2. **Given** a failing PR, **When** the job reports failure, **Then** the output identifies the missing or invalid spec mapping clearly enough to fix the PR.
3. **Given** a passing PR, **When** the job completes, **Then** the output identifies the governing spec ids that justified the change set.

## Scope

In scope:

- approved spec registry format
- deterministic mapping from governed path prefixes to approved spec records
- PR-body governing-spec validation
- CI enforcement job for pull requests
- actionable pass/fail output

Out of scope:

- semantic verification that every code change fully implements every requirement in a spec
- automatic approval workflow for specs
- runtime contract-to-spec semantic comparison
- workflow or registry behavioral validation beyond path-based coverage

## Requirements

### Functional Requirements

- **FR-001**: The repository MUST define a machine-readable approved spec registry.
- **FR-002**: Each approved spec registry entry MUST include spec identity, version, status, immutable flag, canonical path, and governed path prefixes.
- **FR-003**: The spec-alignment gate MUST evaluate pull requests using changed file paths and the PR body as inputs.
- **FR-004**: The gate MUST treat only configured governed path prefixes as requiring approved spec coverage.
- **FR-005**: The gate MUST fail when a changed governed file is not covered by any approved immutable spec record.
- **FR-006**: The gate MUST fail when the PR body lacks a `## Governing Spec` section.
- **FR-007**: The gate MUST fail when the PR body references a spec id that is missing from the approved spec registry.
- **FR-008**: The gate MUST fail when the PR body references a spec whose registry record is not `approved` or not immutable.
- **FR-009**: The gate MUST fail when a changed governed file is covered by an approved spec record but that governing spec id is not declared in the PR body.
- **FR-010**: The gate MUST ignore changes outside configured governed path prefixes.
- **FR-011**: The gate MUST produce deterministic, machine-readable, and human-readable output that identifies:
  - changed governed files
  - governing spec ids required by the diff
  - governing spec ids declared by the PR
  - failure reasons
- **FR-012**: The gate MUST support multiple governing spec ids in a single pull request.
- **FR-013**: The gate MUST pass when no governed files changed.
- **FR-014**: The CI workflow MUST run the spec-alignment gate on pull requests.
- **FR-015**: The repository documentation MUST describe the approved spec registry and the default spec-alignment workflow.
- **FR-016**: Approved implementation under this spec MUST itself be validated against this governing spec before merge.

### Key Entities

- **Approved Spec Record**: The machine-readable record for an approved immutable spec and the governed paths it covers.
- **Governed Path Prefix**: A repository path prefix that requires approved spec coverage when changed.
- **Declared Governing Spec Id**: A spec id listed in a PR body under `## Governing Spec`.
- **Spec Alignment Result**: The pass/fail evaluation artifact for one pull request diff.

## Non-Functional Requirements

- **NFR-001 Determinism**: The gate MUST produce the same result for the same approved spec registry, PR body, and changed file list.
- **NFR-002 Explainability**: Failure output MUST identify the specific missing or invalid mapping rather than failing silently.
- **NFR-003 Maintainability**: The first implementation MUST stay simple enough to evolve as the spec model matures.
- **NFR-004 Merge Safety**: The gate MUST be reliable enough to become a required protected check.
- **NFR-005 Testability**: The core gate logic MUST be structured for full automated coverage once implemented as protected core logic.

## Non-Negotiable Quality Gates

- **QG-001**: No governed implementation change may merge unless it is covered by an approved immutable spec record.
- **QG-002**: The approved spec registry MUST remain the machine-readable source of truth for merge gating.
- **QG-003**: The PR body governing-spec declaration MUST not be treated as sufficient without registry validation.
- **QG-004**: The gate MUST fail closed for invalid registry format, missing registry file, or unreadable required inputs.

## Success Criteria

- **SC-001**: A PR that changes governed implementation without approved spec coverage fails deterministically.
- **SC-002**: A PR that changes governed implementation and declares the correct approved immutable specs passes the alignment gate.
- **SC-003**: A PR that changes only non-governed files passes without unnecessary friction.
- **SC-004**: The new CI job can become a required branch-protection check.

## Governing Relationship

This specification is governed by:

- `001-foundation-v0-1`
- constitution version `1.2.0`

This specification, once approved, is intended to govern implementation in:

- `scripts/ci/spec_alignment_check.sh`
- `.github/workflows/ci.yml`
- `specs/governance/approved-specs.json`
