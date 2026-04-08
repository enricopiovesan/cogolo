# Implementation Plan: Schema Alignment Gate — v0.2.0 Update

**Branch**: `212-schema-alignment-gate-v02` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)

## Summary

Update `approved-specs.json` with specs 010–017 and extend `spec_alignment_check.sh` to validate v0.2.0 contract fields and cover new crates. Must land last in v0.2.0.

## Technical Context

**Language/Version**: Bash, jq, JSON
**Primary Dependencies**: jq (CI), `approved-specs.json`, `spec_alignment_check.sh`
**Storage**: N/A
**Testing**: Run the gate script locally with valid and deliberately broken inputs
**Target Platform**: CI (GitHub Actions), developer workstation
**Project Type**: CI gate / governance tooling
**Constraints**: Pure bash + jq; deterministic; no LLM; `approved-specs.json` is immutable once merged

## Constitution Check

Amends spec 004-spec-alignment-gate as declared. Must land last — after 010–017 are merged. All gates pass by construction (this spec updates the gate itself).

## Files Touched

```text
specs/governance/approved-specs.json            # MODIFIED — add specs 010–017
scripts/ci/spec_alignment_check.sh              # MODIFIED — add contract field check + new crate coverage
```

## Phase 0: Research

- Read current `approved-specs.json` to confirm schema: `{ id, name, branch, status }` per entry
- Read current `spec_alignment_check.sh` to understand existing checks and extension points
- Confirm all 8 new spec branch names match the actual merged branches: `204-placement-constraint-evaluator`, `205-wasm-executor-adapter`, `206-execution-trace-tiered`, `207-event-broker`, `208-service-type-taxonomy`, `209-capability-discovery-mcp`, `210-runtime-placement-router`, `211-expedition-wasm-port`
- List all contract files in `contracts/` to know which ones need `service_type` + `artifact_type`

## Phase 1: Update approved-specs.json

Add 8 new entries (IDs 010–017) following the existing format:

```json
{ "id": "010", "name": "placement-constraint-evaluator", "branch": "204-placement-constraint-evaluator", "status": "approved" },
{ "id": "011", "name": "wasm-executor-adapter", "branch": "205-wasm-executor-adapter", "status": "approved" },
{ "id": "012", "name": "execution-trace-tiered", "branch": "206-execution-trace-tiered", "status": "approved" },
{ "id": "013", "name": "event-broker", "branch": "207-event-broker", "status": "approved" },
{ "id": "014", "name": "service-type-taxonomy", "branch": "208-service-type-taxonomy", "status": "approved" },
{ "id": "015", "name": "capability-discovery-mcp", "branch": "209-capability-discovery-mcp", "status": "approved" },
{ "id": "016", "name": "runtime-placement-router", "branch": "210-runtime-placement-router", "status": "approved" },
{ "id": "017", "name": "expedition-wasm-port", "branch": "211-expedition-wasm-port", "status": "approved" }
```

## Phase 2: Extend spec_alignment_check.sh

Add after existing checks:

1. **Contract field check**: For each `.json` file in `contracts/`, use `jq` to verify `service_type` and `artifact_type` fields are present. Exit 1 with file path if missing.

2. **New crate coverage**: Ensure the crate list checked by the gate includes `traverse-expedition-wasm` and `traverse-mcp` in addition to the original five.

## Phase 3: Local validation

Run the gate three times:

1. Against the current repo (should pass)
2. Against a temp copy with one spec removed from `approved-specs.json` (should fail)
3. Against a temp copy with `service_type` removed from one contract (should fail)

## Implementation Sequence

1. Read `approved-specs.json` + `spec_alignment_check.sh`
2. Add 8 entries to `approved-specs.json` (Phase 1)
3. Add contract field check to `spec_alignment_check.sh` (Phase 2, step 1)
4. Update crate list in gate (Phase 2, step 2)
5. `bash scripts/ci/spec_alignment_check.sh` — must exit 0
6. Run failure-mode tests (Phase 3, steps 2–3)
7. `cargo test` (no Rust changes, but verify nothing broke)
8. Commit + push + open PR declaring spec 004 amendment
