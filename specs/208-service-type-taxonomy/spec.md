# Feature Specification: Service Type Taxonomy on Capability Contracts

**Feature Branch**: `208-service-type-taxonomy`
**Spec ID**: 014
**Created**: 2026-04-08
**Status**: Draft
**Input**: GitHub issue #208

> **Governance note**: Amends spec 002-capability-contracts. All existing contract parsing must remain backward-compatible. No runtime or CI gate changes — only traverse-contracts crate and contract files in contracts/.

## Context

The UMA execution model defines three service classes that govern where and how a capability may run. Stateless capabilities are pure functions that execute on any target with no external state. Subscribable capabilities are event-activated and must declare the event that triggers them. Stateful capabilities require managed persistence and are therefore target-constrained — browser environments provide no durable storage guarantee and must be excluded. Without contract-level annotation, the placement evaluator (#204) and the event broker (#207) have no authoritative source for these constraints and must fall back to unsafe defaults.

Adding `service_type` and `permitted_targets` as first-class fields on `CapabilityContract` gives both downstream consumers a single, validated source of truth. Backward-compatible serde defaults — `Stateless` for service type and all four targets for permitted targets — ensure that every existing contract file continues to parse and validate without modification. This preserves the full existing test suite and removes the need for a coordinated migration of all deployed contracts before the feature can land.

## User Scenarios & Testing

### User Story 1 — Backward-compatible parsing (Priority: P1)
**Acceptance Scenarios**:
1. **Given** an existing capability contract without service_type or permitted_targets fields, **When** parsed, **Then** service_type defaults to Stateless and permitted_targets defaults to all four targets.

### User Story 2 — Stateful + Browser constraint violation (Priority: P1)
**Acceptance Scenarios**:
1. **Given** a contract with service_type: Stateful and permitted_targets: [Browser], **When** validated, **Then** returns ContractError::InvalidPlacementConstraint.

### User Story 3 — Subscribable without event_trigger (Priority: P1)
**Acceptance Scenarios**:
1. **Given** a contract with service_type: Subscribable and no event_trigger field, **When** validated, **Then** returns ContractError::MissingEventTrigger.

### User Story 4 — Placement evaluator consumes service_type and permitted_targets (Priority: P2)
**Acceptance Scenarios**:
1. **Given** a contract with permitted_targets: [Cloud, Edge], **When** the placement evaluator runs tier-2, **Then** only Cloud and Edge targets are considered.

## Requirements

- **FR-001**: ServiceType enum (Stateless | Subscribable | Stateful) in traverse-contracts with serde Deserialize + Display.
- **FR-002**: ExecutionTarget enum (Browser | Edge | Cloud | Device) in traverse-contracts with serde Deserialize + Display.
- **FR-003**: CapabilityContract gains `service_type: ServiceType` with `#[serde(default)]` defaulting to Stateless.
- **FR-004**: CapabilityContract gains `permitted_targets: Vec<ExecutionTarget>` with `#[serde(default)]` defaulting to all four targets.
- **FR-005**: Validation rule: Stateful + Browser in permitted_targets → ContractError::InvalidPlacementConstraint.
- **FR-006**: Validation rule: Subscribable + missing/empty event_trigger → ContractError::MissingEventTrigger.
- **FR-007**: All existing contract files in contracts/ parse successfully after the change (no breakage).
- **FR-008**: Expedition example contract updated to explicitly declare service_type: Stateless.

## Success Criteria

- **SC-001**: All existing contract parsing tests pass unchanged.
- **SC-002**: ContractError::InvalidPlacementConstraint triggered by Stateful+Browser in a test.
- **SC-003**: ContractError::MissingEventTrigger triggered by Subscribable+no event_trigger in a test.
- **SC-004**: cargo test passes for all crates.
- **SC-005**: Expedition contract explicitly declares service_type: Stateless.

## Assumptions

- event_trigger field may not yet exist on CapabilityContract — if absent, add it as Option<String> with serde default None.
- The placement evaluator (#204) is implemented after this spec lands, consuming these fields.
- Browser is excluded for Stateful because browser environments cannot provide managed persistence guarantees.
