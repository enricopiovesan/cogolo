# Feature Specification: Schema Alignment Gate — v0.2.0 Update

**Feature Branch**: `212-schema-alignment-gate-v02`
**Spec ID**: 028
**Created**: 2026-04-08
**Status**: Draft
**Input**: GitHub issue #212

> **Governance note**: Amends spec 004-spec-alignment-gate. Must land last in the v0.2.0 sequence — after all other specs (010–017) are approved and merged.

## Context

The spec-alignment gate exists to ensure that every piece of code merged into Traverse is traceable to an approved, registered spec. `approved-specs.json` is the authoritative registry: the gate reads it at CI time and compares spec IDs referenced in source and PR metadata against the registered set. When new specs are introduced without updating `approved-specs.json`, the gate has no way to detect drift — a PR could reference spec 014 in a source comment, the gate would find no matching entry, and either silently pass (if the check is absent) or produce a spurious failure with no clear cause. Either outcome undermines the governance guarantee the gate is designed to enforce.

The v0.2.0 release introduces eight new specs (010–017, governing issues #204–#211). Each of these specs governs new crates, new contract fields, and new runtime subsystems. Until `approved-specs.json` is updated and `spec_alignment_check.sh` is extended to check v0.2.0-specific contract fields (`service_type`, `artifact_type`) and new crate paths (`traverse-expedition-wasm`, `traverse-mcp`), the gate cannot provide the v0.2.0 coverage guarantee. This spec defines exactly what changes are required to restore full alignment coverage for the v0.2.0 surface area.

## User Scenarios & Testing

### User Story 1 — New v0.2.0 specs registered in approved-specs.json (Priority: P1)

**Acceptance Scenarios**:

1. **Given** specs 010–017 are approved, **When** the alignment gate runs, **Then** all new spec IDs appear in `approved-specs.json` and the gate passes.

### User Story 2 — PR without spec registration is blocked (Priority: P1)

**Acceptance Scenarios**:

1. **Given** a PR that references a spec ID not in `approved-specs.json`, **When** the CI gate runs, **Then** the gate fails with a clear error message identifying the unregistered spec.

### User Story 3 — Contract field coverage check for v0.2.0 fields (Priority: P2)

**Acceptance Scenarios**:

1. **Given** a contract file in `contracts/` missing `service_type` or `artifact_type`, **When** the gate runs, **Then** it reports the missing required fields.

## Requirements

- **FR-001**: `approved-specs.json` MUST include entries for specs 010–017 (IDs and branch names matching #204–#211).
- **FR-002**: `spec_alignment_check.sh` MUST validate that all spec IDs declared in source comments/PR bodies resolve to an entry in `approved-specs.json`.
- **FR-003**: `spec_alignment_check.sh` MUST check that contract files in `contracts/` include `service_type` and `artifact_type` fields.
- **FR-004**: The gate MUST cover `traverse-expedition-wasm` crate and `traverse-mcp` crate (not just the original five crates).
- **FR-005**: Gate failure messages MUST identify the specific spec ID or contract file causing the failure.
- **FR-006**: The gate MUST remain deterministic and AI-agnostic (pure bash + jq, no LLM calls).

## Success Criteria

- **SC-001**: `approved-specs.json` contains 18 entries (001–009 existing + 010–017 new) after this spec lands.
- **SC-002**: `bash scripts/ci/spec_alignment_check.sh` exits 0 on a fully compliant repo.
- **SC-003**: A test run with a deliberately missing spec entry exits non-zero with an identifying error message.
- **SC-004**: CI spec-alignment job passes on the v0.2.0 merge.

## Assumptions

- Specs 010–017 are all approved and merged before this spec is implemented.
- `approved-specs.json` format follows the existing schema: array of `{ id, name, branch, status }`.
- `jq` is available in the CI environment.
- The gate does not validate WASM binary contents — only contract field presence.
