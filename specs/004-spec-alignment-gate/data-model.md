# Data Model: Traverse Spec-Alignment CI Gate

## Purpose

This document defines the exact `v0.1` data model for the first deterministic spec-alignment merge gate.

It is intentionally narrow. The model covers approved spec records, declared PR spec references, governed path coverage, and gate outputs.

## Approved Spec Registry

Location:

- `specs/governance/approved-specs.json`

Top-level shape:

```json
{
  "schema_version": "1.0.0",
  "specs": [
    {
      "id": "002-capability-contracts",
      "version": "1.0.0",
      "status": "approved",
      "immutable": true,
      "path": "specs/002-capability-contracts/spec.md",
      "governs": [
        "crates/traverse-contracts/"
      ]
    }
  ]
}
```

## Registry Field Definitions

### `schema_version`

- type: string
- required: yes
- allowed value in `v0.1`: `1.0.0`

### `specs`

- type: array
- required: yes
- minimum items: 1

Rules:

- each `id` MUST be unique
- each `path` MUST be unique

## Approved Spec Record

Required fields:

- `id`
- `version`
- `status`
- `immutable`
- `path`
- `governs`

### `id`

- type: string
- required: yes
- format: numeric-prefixed kebab-case spec id

Examples:

- `001-foundation-v0-1`
- `002-capability-contracts`
- `004-spec-alignment-gate`

### `version`

- type: string
- required: yes
- format: semantic version `MAJOR.MINOR.PATCH`

### `status`

- type: string
- required: yes
- allowed values in `v0.1`:
  - `approved`

Rule:

- only `approved` specs may appear in the approved spec registry

### `immutable`

- type: boolean
- required: yes
- required value in `v0.1`: `true`

### `path`

- type: string
- required: yes

Rules:

- MUST point to an existing `spec.md`
- MUST match the canonical spec directory for the given `id`

### `governs`

- type: array
- required: yes
- minimum items: 1

Rules:

- each item MUST be a repository-relative path prefix
- values MUST be unique
- values SHOULD end with `/` when representing a directory prefix

## PR Governing Spec Declaration

Input source:

- PR body markdown under the `## Governing Spec` section

Allowed shape:

```md
## Governing Spec
- 001-foundation-v0-1
- 004-spec-alignment-gate
```

Rules:

- each declaration line MUST begin with `- `
- declared ids MUST be unique after normalization
- blank lines inside the section are allowed
- parsing ends at the next markdown heading of the form `## `

## Governed Path Matching

Given a changed file path:

- if it starts with one or more configured `governs` prefixes, it is a governed file
- the required governing spec set for the PR is the union of all approved spec ids that govern any changed file

Rules:

- matching is prefix-based and case-sensitive
- if a governed file matches no approved spec record, the gate MUST fail
- if multiple approved specs govern the same changed file, the PR MUST declare all of them in `v0.1`

## Spec Alignment Result

The gate must be able to report:

- `status`
- `changed_files`
- `governed_files`
- `required_spec_ids`
- `declared_spec_ids`
- `failures`

### `status`

- type: string
- allowed values:
  - `passed`
  - `failed`

### `failures`

- type: array

Each failure record must contain:

- `code`
- `path`
- `message`

Allowed failure code families:

- `input.*`
- `registry.*`
- `pr.*`
- `coverage.*`

Examples:

- `input.pr_body_missing`
- `registry.spec_missing`
- `registry.spec_not_approved`
- `registry.spec_not_immutable`
- `coverage.file_unmapped`
- `coverage.spec_not_declared`

## Semantic Rules

- The approved spec registry is the machine-readable source of truth for merge gating.
- Specs not present in the approved spec registry are not valid governing references for CI gating.
- A PR body declaration alone is never sufficient without registry coverage.
- A governed file diff must be fully covered by approved immutable specs or the gate fails.
- Non-governed changes should not require unnecessary spec declarations.
