# Feature Specification: Cogolo Event Contracts

**Feature Branch**: `003-event-contracts`  
**Created**: 2026-03-26  
**Status**: Draft  
**Input**: Existing Cogolo foundation specification, constitution, planning artifacts, and source material already reviewed from ECCA, C-DAD, UMA, and UMA Chapter 13.

## Purpose

This specification defines the first implementation-tight event contract slice for Cogolo:

- the event contract artifact
- event contract validation behavior
- event contract lifecycle and versioning rules
- event contract governance metadata
- event contract validation evidence and error behavior

This spec governs the event-facing contract model before event registry, runtime event flow, workflow composition, or browser subscription behavior is implemented.

## User Scenarios & Testing

### User Story 1 - Author a Governed Event Contract (Priority: P1)

As a platform developer, I want to author a machine-readable event contract that describes a governed event artifact so that publishers, subscribers, registries, runtimes, and AI systems can discover and use the same event safely.

**Why this priority**: ECCA makes event contracts first-class governed artifacts, not informal payload schemas. If the event contract is vague, event-driven composition and registry governance drift immediately.

**Independent Test**: A developer can create an event `contract.json`, validate it through the contracts crate or CLI, and receive a canonical parsed event contract object plus no validation errors.

**Acceptance Scenarios**:

1. **Given** an event `contract.json` with all required fields and valid values, **When** the contract is parsed and validated, **Then** the system accepts it as a valid governed event contract artifact.
2. **Given** a valid event contract, **When** the contract is loaded, **Then** the system exposes normalized identity, version, lifecycle, owner, payload schema, semantic metadata, publisher/subscriber metadata, and provenance metadata.
3. **Given** a valid event contract with an optional companion `README.md`, **When** validation occurs, **Then** the JSON event contract remains authoritative and the Markdown remains non-governing documentation.

---

### User Story 2 - Reject Invalid or Unsafe Event Contracts (Priority: P1)

As a platform developer, I want invalid event contracts to fail with precise and actionable validation errors so that ambiguous, unowned, non-versioned, or semantically weak events cannot enter the governed system.

**Why this priority**: Event-driven composition is only safe if event contracts are explicit about meaning, schema, ownership, and publishing/subscribing boundaries.

**Independent Test**: A suite of malformed and semantically invalid event contracts can be validated and each one fails with a stable error code, path, and explanation.

**Acceptance Scenarios**:

1. **Given** a contract missing a required field such as owner, payload schema, classification, or publisher policy, **When** validation runs, **Then** validation fails with the missing field path and error code.
2. **Given** a contract with malformed semantic version, duplicate publisher/subscriber entries, invalid lifecycle, or invalid identity metadata, **When** validation runs, **Then** validation fails with field-specific errors.
3. **Given** a contract that only defines a payload schema but omits semantic meaning, ownership, or event-governance metadata, **When** semantic validation runs, **Then** validation rejects it as an incomplete event contract.

---

### User Story 3 - Treat Published Event Contracts as Immutable Governed Records (Priority: P2)

As a platform steward, I want event contracts to be semantic-versioned, lifecycle-aware, discoverable, and immutable once published so that event ecosystems can evolve safely without hidden drift.

**Why this priority**: ECCA and C-DAD both require contracts to be discoverable, versioned, and immutable governed records once approved.

**Independent Test**: Given existing registered event contract metadata, the system can determine whether a new contract version is valid, duplicate, incompatible, or lifecycle-invalid.

**Acceptance Scenarios**:

1. **Given** a new event contract version for an existing event identity, **When** validation runs against the prior published record, **Then** the system validates semver format and lifecycle compatibility.
2. **Given** a contract with the same identity and version but different governed content, **When** duplicate-version validation runs, **Then** validation fails because published contracts are immutable.
3. **Given** an event contract in lifecycle states such as `deprecated`, `retired`, or `archived`, **When** downstream registry or runtime systems consume it later, **Then** those systems can rely on the lifecycle state being explicit and machine-readable.

## Scope

In scope:

- event contract artifact shape
- exact required and optional fields
- JSON field-level validation rules
- semantic validation rules
- lifecycle states
- semver rules for event contract versions
- event ownership, classification, and discoverability metadata
- publisher/subscriber metadata and policy references
- error and evidence model for event contract validation

Out of scope:

- event registry persistence behavior
- runtime event dispatch behavior
- workflow event edge execution behavior
- browser subscription APIs
- transport or stream implementation details

## Requirements

### Functional Requirements

- **FR-001**: An event contract MUST be authored as a `contract.json` artifact.
- **FR-002**: The event contract artifact MUST be the governing machine-readable source of truth for event semantics and event boundary metadata.
- **FR-003**: The system MUST support an optional non-governing companion `README.md` for human-readable rationale, examples, or adoption notes.
- **FR-004**: An event contract MUST include identity, version, lifecycle, owner, summary, payload schema, semantic meaning metadata, classification metadata, publisher metadata, subscriber metadata, policy references, provenance metadata, and validation evidence metadata.
- **FR-005**: The system MUST validate event contracts in two stages:
  - structural validation of JSON shape and value presence
  - semantic validation of identity, governance, versioning, discoverability, and safety rules
- **FR-006**: Event contract versions MUST be semantic versions using `MAJOR.MINOR.PATCH`.
- **FR-007**: Event contracts MUST expose lifecycle states from a controlled enum.
- **FR-008**: Event contracts MUST expose payload schema metadata in a machine-readable object form suitable for downstream validation.
- **FR-009**: Event contracts MUST include semantic meaning metadata beyond payload schema, including at least an event summary and event classification.
- **FR-010**: Event contracts MUST declare allowed publisher references as governed artifact references rather than free-text labels.
- **FR-011**: Event contracts MUST support declared subscriber references for discovery and impact analysis even when subscriber enforcement is deferred.
- **FR-012**: Event contracts MUST support explicit policy references for publication and subscription behavior even if full evaluation is deferred in `v0.1`.
- **FR-013**: Event contracts MUST carry provenance metadata sufficient to identify authoring source, approval/governance context, and validation evidence linkage.
- **FR-014**: Validation MUST reject contracts that omit required fields, contain invalid enumerations, contain malformed semver values, or use inconsistent identity metadata.
- **FR-015**: Validation MUST reject contracts that only define payload structure without sufficient governance metadata for ownership, lifecycle, semantic meaning, or publish/subscribe boundaries.
- **FR-016**: Validation MUST reject duplicate items where uniqueness is required, including duplicated publisher references, subscriber references, policy references, tag values, and classification values when modeled as arrays.
- **FR-017**: Validation MUST reject contracts whose `id` is inconsistent with `namespace` plus `name`.
- **FR-018**: Validation MUST reject contracts whose published identity and version match an existing published record but whose governed content differs.
- **FR-019**: Validation MUST produce stable machine-readable error records with error code, JSON path, severity, and human-readable explanation.
- **FR-020**: Successful validation MUST produce machine-readable validation evidence linked to the event identity, version, and governing spec.
- **FR-021**: The contracts crate MUST expose normalized event contract domain types suitable for registry, runtime, workflow, and CLI consumers without requiring those downstream systems to re-interpret raw JSON.
- **FR-022**: Event contracts MUST support explicit event classification metadata for governance and discoverability.
- **FR-023**: Event contracts MUST identify a clear owner and MUST NOT be treated as valid if ownership is absent or ambiguous.
- **FR-024**: Approved implementation under this spec MUST be validated against this spec before merge.

### Event Contract Fields

The contract MUST model these top-level fields:

- `kind`
- `schema_version`
- `id`
- `namespace`
- `name`
- `version`
- `lifecycle`
- `owner`
- `summary`
- `description`
- `payload`
- `classification`
- `publishers`
- `subscribers`
- `policies`
- `tags`
- `provenance`
- `evidence`

### Lifecycle Enum

Allowed `lifecycle` values:

- `draft`
- `active`
- `deprecated`
- `retired`
- `archived`

### Key Entities

- **Event Contract**: The governed machine-readable artifact that defines one versioned event boundary.
- **Event Artifact Reference**: A normalized identifier for a versioned publisher, subscriber, or related dependency target.
- **Event Validation Error**: A stable machine-readable error produced when structural or semantic validation fails.
- **Event Validation Evidence**: A stable machine-readable record proving an event contract passed validation against the governing spec.

## Non-Functional Requirements

- **NFR-001 Determinism**: Parsing, normalization, and validation of the same event contract input MUST produce the same results and error ordering.
- **NFR-002 Explainability**: Validation failures MUST be actionable without requiring source inspection of the validator implementation.
- **NFR-003 Discoverability**: The event contract model MUST preserve ownership, classification, and subscriber/publisher metadata needed for catalog-driven discovery.
- **NFR-004 Maintainability**: Event contract types and validator logic MUST be separable into clear modules suitable for full automated test coverage.
- **NFR-005 Compatibility Discipline**: Event contract version interpretation MUST be explicit and predictable.
- **NFR-006 Testability**: Core validation logic under this spec MUST be structured for 100% automated coverage once implemented.

## Non-Negotiable Quality Gates

- **QG-001**: No event contract implementation may merge unless it is aligned with this spec and the governing foundation spec.
- **QG-002**: Structural validation, semantic validation, normalization, and duplicate-version behavior MUST have full automated coverage once implemented.
- **QG-003**: Validation errors and evidence formats MUST be stable enough for CLI, registry, runtime, and CI consumers.
- **QG-004**: The validator MUST reject schema-only or ownerless event declarations that do not meet the governed event contract standard.

## Success Criteria

- **SC-001**: A valid event `contract.json` can be parsed into normalized Rust domain types without ambiguity.
- **SC-002**: Invalid event contracts fail with stable error records covering field path, code, and explanation.
- **SC-003**: Duplicate published version misuse is rejected deterministically.
- **SC-004**: Lifecycle, semver, ownership, and classification rules are all validated automatically.
- **SC-005**: The implementation of this spec can be added to the protected coverage target list and held to 100% coverage.

## Governing Relationship

This specification is governed by:

- `001-foundation-v0-1`
- constitution version `1.2.0`

This specification, once approved, is intended to govern the event-contract implementation slice in:

- `crates/cogolo-contracts`
