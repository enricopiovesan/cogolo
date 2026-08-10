# Feature Specification: Contract Surface Coverage

**Status**: Approved
**Canonical governing ID**: `102-contract-surface-coverage`
**Version**: 1.1.0
**Extends**: `002-capability-contracts`, `100-capability-package-authoring`, `056-capability-publish`, `516-agent-artifact-execution`
**Input**: Issues #1014–#1016; #1040; registry#192; registry#215; Decision 57; Decision 58; ADR-0038.
**Incident**: `core.process-comment@1.0.0` overclaimed `action` enum values; Loop batch publish stripped `use_cases` from registry copies because publish serialized a `CapabilityContract` without that field.

## Purpose

Prevent capability contracts from declaring an input/behavior surface larger than what published `use_cases` demonstrate and what package-level verification exercises. Free-text `description` and JSON Schema enums are otherwise treated as marketing, while only use cases are executable promises. Coverage is measured against the **entire declared schema surface**, not a minimum use-case count.

## Capability Boundary

In scope:

- Coverage rules relating `inputs.schema` enums and required properties to `use_cases[].input_example`
- Coverage rules relating `outputs.schema` enums for `reason_code` and `status` (when declared) to `use_cases[].output_example`
- Requirement that `use_cases` is non-empty and preserved through `capability publish` into the registry record
- Publish / dry-run failure behavior when coverage is incomplete
- Package smoke rule: each use case has a matching executable fixture
- Cross-repo expectation that registry validation mirrors the same rule for newly ADDED or CHANGED contracts

Out of scope:

- NLP verification that every sentence in `description` is implemented (human honesty checklist + Known limitations only)
- Cartesian product coverage of all required-field combinations
- Rewriting immutable already-published registry versions in place (honesty patch-bumps instead)
- Expanding Host ABI or guest runtime features
- Requiring 100% branch coverage of guest code beyond the use-case smoke matrix

## Requirements

- **FR-001**: Every string `enum` value declared under `inputs.schema` (at any property path used as a closed vocabulary, including but not limited to `action`, `intensity`, `tone`, and nested config enums) MUST appear in at least one `use_cases[i].input_example` at the corresponding path.
- **FR-002**: Every property listed in `inputs.schema.required` MUST appear (with a concrete value) in at least one `use_cases[i].input_example`. Full cartesian coverage of required combinations is NOT required.
- **FR-003**: When `outputs.schema.properties.reason_code` or `outputs.schema.properties.status` declares an `enum`, every enum value MUST appear in at least one `use_cases[i].output_example` at that field. Authors who need checkable failure/success vocabulary MUST declare these as enums (free-string `reason_code` is not coverage-checkable and MUST NOT be used to evade FR-003).
- **FR-004**: `use_cases` MUST be a non-empty array on every contract submitted to `capability publish` / `--dry-run` and on every newly ADDED or CHANGED registry `contract.json`.
- **FR-005**: `traverse-cli capability publish` MUST preserve `use_cases` (and author-supplied `evidence`) in the registry-bound contract JSON. Round-tripping through normalization MUST NOT strip fields required by this spec.
- **FR-006**: `traverse-cli capability publish` and `capability publish --dry-run` MUST fail with an actionable error listing uncovered enum values, missing required properties, missing output enum coverage, or empty `use_cases` when FR-001–FR-004 are violated.
- **FR-007**: Governed example / capability packages MUST include a smoke fixture for **each** `use_cases[]` entry that exercises that use case and asserts its `reason_code` (and other key outputs named in the use case's `output_example`).
- **FR-008**: Capability authoring documentation MUST state the coverage rule and require an explicit **Known limitations** section whenever description prose mentions behavior not represented in `use_cases`.
- **FR-009**: Schema MUST NOT list an enum value that the executable artifact answers only with a generic `unsupported_*` / equivalent fail-closed stub unless that failure mode itself is a documented use case with a stable `reason_code`.
- **FR-010**: Already-published registry versions that lack `use_cases` or fail FR-001–FR-003 MUST be corrected by an honesty patch-bump (new immutable version), not by editing the published file in place.

## Success Criteria

- A contract that declares an input enum value with no covering use case fails publish dry-run.
- A contract with empty or missing `use_cases` fails publish dry-run.
- Publishing a contract that has use cases locally results in a registry PR whose `contract.json` still contains those use cases.
- A governed example package with a use case but no matching smoke fixture fails the package/smoke gate.
- Registry CI rejects newly ADDED/CHANGED contracts that violate FR-001–FR-004.
- Stripped Loop capabilities are honesty-bumped under FR-010.

## Quality Gates

- QG-001: Unit tests for the publish coverage checker (pass/fail fixtures) covering enums, required props, output reason_code/status, empty use_cases, and preserve-on-publish.
- QG-002: Spec-alignment maps this spec onto CLI publish paths, package smoke conventions, and authoring docs once Approved.
- QG-003: No silent weakening of host input-schema validation.
- QG-004: Registry mirror tests reject missing use_cases on the new/changed-contract path.

## Approval Note

v1.0.0 Approved 2026-08-08 (owner direction). v1.1.0 Approved 2026-08-10 (owner direction on Decision 58 brainstorm closeout — full schema ⊆ use_cases ⊆ smoke gate).
