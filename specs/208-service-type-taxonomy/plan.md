# Implementation Plan: Service Type Taxonomy on Capability Contracts

**Branch**: `208-service-type-taxonomy` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)

## Summary

Add ServiceType and ExecutionTarget enums to traverse-contracts, add the two new fields to CapabilityContract with serde defaults, add two validation rules, update expedition contracts. Backward-compatible throughout.

## Technical Context

**Language/Version**: Rust 1.94+
**Primary Dependencies**: serde (existing), traverse-contracts (modified in place)
**Storage**: N/A
**Testing**: Unit tests in traverse-contracts/tests/
**Target Platform**: All
**Project Type**: Contract schema extension
**Constraints**: All existing tests must pass unchanged; serde defaults are mandatory; no unsafe

## Constitution Check

Amends approved spec 002-capability-contracts. All gates pass. No runtime code touched.

## Files Touched

```text
crates/traverse-contracts/src/types.rs          # MODIFIED — add ServiceType, ExecutionTarget enums
crates/traverse-contracts/src/capability.rs     # MODIFIED — add service_type, permitted_targets, event_trigger fields
crates/traverse-contracts/src/validation.rs     # MODIFIED — add two validation rules
contracts/expedition/                            # MODIFIED — add service_type: stateless to contract files
crates/traverse-contracts/tests/contract_tests.rs # MODIFIED — add new test cases
```

## Phase 0: Research

- Locate current CapabilityContract definition (capability.rs) and confirm field names
- Locate existing contract files in contracts/ and note their format (JSON or TOML)
- Check if event_trigger field already exists; if not, add as Option<String>
- Check current ContractError variants; add InvalidPlacementConstraint and MissingEventTrigger if absent
- Confirm serde default attribute syntax used in the codebase

## Phase 1: Enums

Add to `types.rs`:
- `ServiceType` enum: Stateless (default) | Subscribable | Stateful — derive Deserialize, Serialize, Debug, Clone, PartialEq, Display
- `ExecutionTarget` enum: Browser | Edge | Cloud | Device — same derives
- `impl Default for ServiceType { fn default() -> Self { ServiceType::Stateless } }`
- `impl Default for Vec<ExecutionTarget>` via a free function returning all four targets (used as serde default)

## Phase 2: CapabilityContract fields

In `capability.rs`:
- Add `#[serde(default)] service_type: ServiceType`
- Add `#[serde(default = "default_all_targets")] permitted_targets: Vec<ExecutionTarget>`
- Add `#[serde(default)] event_trigger: Option<String>`
- `fn default_all_targets() -> Vec<ExecutionTarget>` returns all four variants

## Phase 3: Validation rules

In `validation.rs`:
- Rule: if service_type == Stateful && permitted_targets.contains(Browser) → Err(ContractError::InvalidPlacementConstraint)
- Rule: if service_type == Subscribable && event_trigger.is_none_or_empty() → Err(ContractError::MissingEventTrigger)
- Add ContractError variants if missing

## Phase 4: Contract files

Update expedition contract JSON/TOML to add `"service_type": "stateless"` explicitly.

## Phase 5: Tests

- test_existing_contract_parses_without_new_fields — confirm backward compat
- test_stateful_browser_rejected — ContractError::InvalidPlacementConstraint
- test_subscribable_missing_trigger_rejected — ContractError::MissingEventTrigger
- test_stateless_all_targets_default — permitted_targets has 4 entries
- test_subscribable_with_trigger_valid — passes validation

## Implementation Sequence

1. Read current types.rs and capability.rs
2. Add ServiceType + ExecutionTarget enums (Phase 1)
3. Add fields to CapabilityContract (Phase 2)
4. Run cargo test — must still pass before validation changes
5. Add validation rules + ContractError variants (Phase 3)
6. Update expedition contract files (Phase 4)
7. Add new test cases (Phase 5)
8. cargo test all crates
9. bash scripts/ci/spec_alignment_check.sh
10. Commit + push + open PR declaring spec 002 amendment
