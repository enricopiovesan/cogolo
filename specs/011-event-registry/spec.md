# Feature Specification: Event Registry

**Feature Branch**: `011-event-registry`  
**Created**: 2026-03-30  
**Status**: Draft  
**Input**: Dedicated registry slice for governed event contract publication, storage, lookup, indexing, and compatibility-aware discovery.

## Purpose

This spec defines the dedicated event-registry slice for Traverse.

It narrows the broader foundation and event-contract requirements into a concrete, testable model for:

- registering governed event contract artifacts
- preserving immutable published event versions
- storing event artifact records plus derived discovery metadata
- supporting public and private scope lookup
- exposing publisher, subscriber, lifecycle, and schema information for downstream runtime and workflow consumers
- validating semver progression and duplicate-version immutability at registration time

This slice does **not** define event emission delivery, message brokers, or event-driven runtime traversal. It is intentionally limited to registry semantics so event publication and discovery remain governed before event-driven execution features expand.

## User Scenarios and Testing

### User Story 1 - Register a Governed Event Contract (Priority: P1)

As a platform developer, I want to register an approved event contract into a governed event registry so that events become discoverable first-class artifacts rather than loose JSON files.

**Why this priority**: Traverse’s event-driven architecture depends on event contracts being governed, indexed artifacts that runtime, workflow, and UI consumers can discover safely.

**Independent Test**: Register one valid event contract into an empty registry and verify the registry stores the authoritative artifact, derived index record, and validation evidence.

**Acceptance Scenarios**:

1. **Given** a valid event contract artifact, **When** event registration succeeds, **Then** the event registry stores the authoritative contract, a derived registry record, and validation evidence.
2. **Given** a duplicate registration for the same `(scope, id, version)` with identical governed content, **When** registration is repeated, **Then** the registry behaves idempotently and returns the existing record.
3. **Given** a duplicate registration for the same `(scope, id, version)` with different governed content, **When** registration is attempted, **Then** the registry rejects it as an immutable version conflict.

### User Story 2 - Discover Event Contracts Deterministically (Priority: P1)

As a workflow or runtime developer, I want to resolve event contracts deterministically by id, version, and scope so that workflows, emitters, and subscribers do not depend on hidden registry heuristics.

**Why this priority**: Event references in workflows and later event-driven runtime features are only safe if registry discovery is predictable and explainable.

**Independent Test**: Register event contracts in both private and public scopes, then verify exact lookup, list lookup, and overlay precedence behave according to the governed rules.

**Acceptance Scenarios**:

1. **Given** matching private and public event registrations, **When** lookup uses `prefer_private`, **Then** the private registration wins deterministically.
2. **Given** only public event registrations, **When** lookup uses `public_only`, **Then** the registry returns only public results.
3. **Given** no registered event matches an exact lookup, **When** resolution is attempted, **Then** the registry returns an explicit no-match result instead of inventing a fallback.

### User Story 3 - Preserve Compatibility and Governance Metadata (Priority: P2)

As a platform steward, I want the event registry to preserve lifecycle, compatibility, publisher, and subscriber metadata so that downstream consumers can reason about event evolution safely.

**Why this priority**: Traverse needs event contracts to evolve with the same rigor as capability contracts, including version discipline and discoverability.

**Independent Test**: Register successive versions of an event contract and verify the registry records semver progression, lifecycle metadata, and discovery metadata without mutating published versions.

**Acceptance Scenarios**:

1. **Given** a valid new event version, **When** it is registered, **Then** the registry stores it alongside prior versions without mutating older published records.
2. **Given** an invalid semver progression, **When** registration is attempted, **Then** the registry rejects the new record with explicit compatibility evidence.
3. **Given** a consumer inspects a registered event, **When** the derived record is read, **Then** lifecycle, publisher, subscriber, summary, and payload-schema metadata are machine-readable and stable.

## Edge Cases

- What happens when an event contract references publishers or subscribers that are not yet implemented as capabilities?
- What happens when the same event id/version exists in both public and private scope?
- What happens when a contract is valid structurally but lacks required governance metadata for registry indexing?
- What happens when a later event version changes lifecycle to `deprecated` or `retired`?
- What happens when a workflow references an event version that exists only in private scope but the lookup policy is `public_only`?
- What happens when a registry already contains one version lineage and a new registration attempts an incompatible semver bump?

## Functional Requirements

- **FR-001**: The system MUST accept a validated event `contract.json` artifact as the authoritative registration input for one event version.
- **FR-002**: The event registry MUST preserve immutable publication semantics per `(scope, id, version)`.
- **FR-003**: Event registration MUST store the authoritative event contract artifact and MUST NOT replace it with only derived metadata.
- **FR-004**: Event registration MUST also produce a derived event registry record for deterministic discovery and downstream consumption.
- **FR-005**: The event registry MUST support at least the lookup scopes `public` and `private`.
- **FR-006**: The event registry MUST support runtime lookup policies that distinguish `public_only` and `prefer_private`.
- **FR-007**: When the same event `(id, version)` exists in both scopes, `prefer_private` lookup MUST resolve the private record first.
- **FR-008**: Exact event lookup by `(scope policy, id, version)` MUST return either one deterministic match or an explicit no-match result.
- **FR-009**: The event registry MUST preserve lifecycle, owner, publisher, subscriber, summary, tags, classification, and payload-schema metadata in derived discovery records.
- **FR-010**: Event registration MUST record validation evidence that ties the stored record to the approved event-contract rules.
- **FR-011**: Event registration MUST reject any event contract that fails structural or semantic validation under the approved event-contract slice.
- **FR-012**: Event registration MUST reject a governed-content change for an already published `(scope, id, version)`.
- **FR-013**: Event registration MUST allow idempotent re-registration of the same governed content for the same `(scope, id, version)`.
- **FR-014**: The event registry MUST support version-aware lineage queries for one event id across published versions in one scope.
- **FR-015**: The event registry MUST validate semver progression against prior published versions before accepting a new version.
- **FR-016**: Derived event registry records MUST remain machine-readable and stable enough for runtime, workflow, CLI, UI, and future MCP consumers.
- **FR-017**: The event registry MUST expose enough metadata for workflows to validate event-edge references without reparsing raw contract JSON in every consumer.
- **FR-018**: The event registry MUST preserve public/private scope boundaries without mutating the underlying event contract model.
- **FR-019**: The event registry MUST distinguish authoritative stored artifacts from derived index metadata.
- **FR-020**: The event registry MUST keep storage, indexing, compatibility validation, and lookup behavior explicit rather than hidden in ad hoc helper logic.

## Non-Functional Requirements

- **NFR-001 Determinism**: Registration results, duplicate handling, lookup precedence, lineage ordering, and derived metadata generation MUST be deterministic for identical inputs.
- **NFR-002 Explainability**: Registration failures and lookup outcomes MUST remain explainable through machine-readable evidence rather than unstructured logs alone.
- **NFR-003 Compatibility**: Event version handling MUST support semver discipline and immutable publication semantics.
- **NFR-004 Testability**: Core event-registry logic MUST be separable enough to achieve 100% automated line coverage when implemented.
- **NFR-005 Maintainability**: Authoritative artifact storage, derived record generation, compatibility checks, and lookup policy behavior MUST remain clearly separated in the implementation.
- **NFR-006 Portability**: Event registry artifacts and index records MUST not assume a specific broker, transport, or cloud-hosted registry backend.

## Non-Negotiable Quality Standards

- **QG-001**: Event registry lookup MUST remain deterministic and MUST NOT silently choose among multiple records without an approved precedence rule.
- **QG-002**: Published event versions MUST remain immutable within one scope.
- **QG-003**: No consumer may be forced to reinterpret raw event contract JSON to discover basic registry metadata already governed by this slice.
- **QG-004**: Core event-registry logic MUST reach 100% automated line coverage when implemented.
- **QG-005**: Event registry behavior MUST align with the governing approved specs and fail merge validation when drift occurs.

## Key Entities

- **Event Registry Store**: The authoritative persisted collection of registered event contract artifacts by scope, id, and version.
- **Event Registry Record**: The stored event artifact plus derived machine-readable metadata and validation evidence.
- **Event Lineage Record**: The ordered set of registered versions for one event id in one scope.
- **Event Lookup Policy**: The deterministic rule set governing public-only or private-preferred lookup behavior.
- **Event Registration Evidence**: The machine-readable validation and compatibility evidence produced during event registration.

## Success Criteria

- **SC-001**: A valid event contract can be registered and discovered as a first-class registry artifact.
- **SC-002**: Duplicate immutable-version conflicts are rejected predictably and idempotent re-registration succeeds safely.
- **SC-003**: Public/private lookup precedence behaves deterministically for identical registered inputs.
- **SC-004**: Derived event registry records expose sufficient metadata for downstream workflow and runtime use.
- **SC-005**: The dedicated event-registry slice becomes the authoritative registry reference for event storage and lookup semantics.

## Out of Scope

- message delivery or broker configuration
- event emission runtime behavior
- event-driven workflow traversal semantics
- browser subscription transport
- MCP event transport details
