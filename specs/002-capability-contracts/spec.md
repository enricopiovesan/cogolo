# Feature Specification: Cogolo Capability Contracts

**Feature Branch**: `002-capability-contracts`  
**Created**: 2026-03-26  
**Status**: Draft  
**Input**: Existing Cogolo foundation specification, data model, constitution, and project source material derived from UMA, ECCA, and C-DAD.

## Purpose

This specification defines the first implementation-tight contract slice for Cogolo:

- the capability contract artifact
- capability contract validation behavior
- capability contract lifecycle and versioning rules
- capability contract validation evidence and error behavior

This spec governs work in `cogolo-contracts` before broader runtime, registry, or workflow implementation proceeds.

## User Scenarios & Testing

### User Story 1 - Author a Valid Capability Contract (Priority: P1)

As a platform developer, I want to author a machine-readable capability contract that fully describes a portable business capability so that the contract can become the governing source of truth for validation, registration, runtime execution, and AI-assisted development.

**Why this priority**: Cogolo depends on contracts as the primary boundary. If the capability contract is underspecified or ambiguous, registries, runtime behavior, and validation all drift.

**Independent Test**: A developer can create a `contract.json`, validate it through the contracts crate or CLI, and receive a canonical parsed contract object plus no validation errors.

**Acceptance Scenarios**:

1. **Given** a `contract.json` that includes all required fields and valid values, **When** the contract is parsed and validated, **Then** the system accepts it as a valid capability contract artifact.
2. **Given** a valid capability contract, **When** the contract is loaded, **Then** the system exposes normalized identity, version, lifecycle, contract boundary, event references, execution metadata, and provenance metadata.
3. **Given** a valid capability contract with an optional companion `README.md`, **When** validation occurs, **Then** validation treats the JSON contract as authoritative and the Markdown as non-governing documentation.

---

### User Story 2 - Reject Invalid or Unsafe Capability Contracts (Priority: P1)

As a platform developer, I want invalid capability contracts to fail with precise and actionable validation errors so that unsafe, ambiguous, or non-portable capabilities cannot enter the governed system.

**Why this priority**: The contract engine is the first merge and runtime safety boundary.

**Independent Test**: A suite of malformed and semantically invalid contracts can be validated and each one fails with a stable error code, path, and explanation.

**Acceptance Scenarios**:

1. **Given** a contract missing a required field, **When** validation runs, **Then** validation fails with the missing field path and error code.
2. **Given** a contract with an invalid lifecycle transition, malformed semantic version, duplicate event reference, or invalid portability declaration, **When** validation runs, **Then** validation fails with field-specific errors.
3. **Given** a contract that attempts to represent a utility function, CRUD wrapper, or host-specific implementation disguised as a capability, **When** semantic validation runs, **Then** validation rejects it as an invalid capability boundary or portability violation.

---

### User Story 3 - Treat Published Capability Contracts as Versioned Governed Records (Priority: P2)

As a platform steward, I want capability contracts to be semantic-versioned, lifecycle-aware, and immutable once published so that capabilities can evolve safely without hidden drift.

**Why this priority**: C-DAD and the project constitution both require versioned, governed, immutable artifacts.

**Independent Test**: Given existing registered contract metadata, the system can determine whether a new contract version is valid, duplicate, incompatible, or lifecycle-invalid.

**Acceptance Scenarios**:

1. **Given** a new capability contract version for an existing capability identity, **When** validation runs against the prior published record, **Then** the system validates semver format and lifecycle compatibility.
2. **Given** a contract with the same identity and version but different governed content, **When** duplicate-version validation runs, **Then** validation fails because published contracts are immutable.
3. **Given** a contract whose lifecycle is `deprecated`, `retired`, or `archived`, **When** the runtime or registry later consumes it, **Then** downstream systems can rely on the lifecycle state being explicit and machine-readable.

## Scope

In scope:

- capability contract artifact shape
- exact required and optional fields
- JSON field-level validation rules
- semantic validation rules
- lifecycle states
- semver rules for capability contract versions
- portability-related contract declarations
- error and evidence model for contract validation

Out of scope:

- event contract implementation details beyond reference validation shape
- workflow definition semantics
- runtime execution behavior
- registry persistence behavior
- browser/demo behavior

## Requirements

### Functional Requirements

- **FR-001**: A capability contract MUST be authored as a `contract.json` artifact.
- **FR-002**: The capability contract artifact MUST be the governing machine-readable source of truth for capability boundary semantics.
- **FR-003**: The system MUST support an optional non-governing companion `README.md` for human-readable rationale or explanation.
- **FR-004**: A capability contract MUST include identity, version, lifecycle, owner, summary, inputs, outputs, side effects, emitted events, consumed events, permissions, execution metadata, policy references, dependency references, and provenance metadata.
- **FR-005**: The system MUST validate capability contracts in two stages:
  - structural validation of JSON shape and value presence
  - semantic validation of business boundary, portability, versioning, and governance rules
- **FR-006**: Capability contract versions MUST be semantic versions using `MAJOR.MINOR.PATCH`.
- **FR-007**: Capability contracts MUST expose lifecycle states from a controlled enum.
- **FR-008**: Capability contract execution metadata MUST explicitly declare the portable binary format and runtime entry metadata.
- **FR-009**: Capability contracts for `v0.1` MUST declare `wasm` as the binary format.
- **FR-010**: Capability contracts for `v0.1` MUST support placement-facing execution declarations even though only `local` execution exists in the runtime.
- **FR-011**: Capability contracts MUST declare emitted and consumed event references as versioned artifact references rather than free-text labels.
- **FR-012**: Capability contracts MUST support explicit permission references and policy references even if their full evaluation is deferred.
- **FR-013**: Capability contracts MUST carry provenance metadata sufficient to identify authoring source, approval/governance context, and validation evidence linkage.
- **FR-014**: Validation MUST reject contracts that omit required fields, contain invalid enumerations, contain malformed semver values, or use inconsistent identity metadata.
- **FR-015**: Validation MUST reject contracts that declare unsupported binary formats, unsupported `v0.1` portability models, or target-specific API dependence without an approved exception reference.
- **FR-016**: Validation MUST reject duplicate items where uniqueness is required, including duplicated emitted event references, consumed event references, permission references, dependency references, and preferred execution targets.
- **FR-017**: Validation MUST reject contracts whose `id` is inconsistent with `namespace` plus `name`.
- **FR-018**: Validation MUST reject contracts whose published identity and version match an existing published record but whose governed content differs.
- **FR-019**: Validation MUST produce stable machine-readable error records with error code, JSON path, severity, and human-readable explanation.
- **FR-020**: Successful validation MUST produce machine-readable validation evidence linked to the contract identity, version, and governing spec.
- **FR-021**: The contracts crate MUST expose normalized domain types suitable for registry, runtime, and CLI consumers without requiring those downstream systems to re-interpret raw JSON.
- **FR-022**: The capability contract MUST support explicit boundary assertions for:
  - preconditions
  - postconditions
  - side effects
- **FR-023**: The capability contract MUST support declaring at least one meaningful business action summary and MUST NOT be treated as valid if it is only a transport, storage, or utility wrapper.
- **FR-024**: Approved implementation under this spec MUST be validated against this spec before merge.

### Capability Contract Fields

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
- `inputs`
- `outputs`
- `preconditions`
- `postconditions`
- `side_effects`
- `emits`
- `consumes`
- `permissions`
- `execution`
- `policies`
- `dependencies`
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

- **Capability Contract**: The governed machine-readable artifact that defines one portable business capability.
- **Capability Artifact Reference**: A normalized identifier for a versioned contract or dependency target.
- **Capability Validation Error**: A stable machine-readable error produced when structural or semantic validation fails.
- **Capability Validation Evidence**: A stable machine-readable record proving a contract passed validation against the governing spec.

## Non-Functional Requirements

- **NFR-001 Determinism**: Parsing, normalization, and validation of the same contract input MUST produce the same results and error ordering.
- **NFR-002 Explainability**: Validation failures MUST be actionable without requiring source inspection of the validator implementation.
- **NFR-003 Portability**: The contract model MUST preserve portability-first design and MUST NOT require host-specific declarations as normal behavior.
- **NFR-004 Maintainability**: The contract types and validator logic MUST be separable into clear modules suitable for full automated test coverage.
- **NFR-005 Compatibility Discipline**: Contract version interpretation MUST be explicit and predictable.
- **NFR-006 Testability**: Core validation logic under this spec MUST be structured for 100% automated coverage.

## Non-Negotiable Quality Gates

- **QG-001**: No capability contract implementation may merge unless it is aligned with this spec and the governing foundation spec.
- **QG-002**: Structural validation, semantic validation, normalization, and duplicate-version behavior MUST have full automated coverage once implemented.
- **QG-003**: Validation errors and evidence formats MUST be stable enough for CLI, registry, and CI consumers.
- **QG-004**: The validator MUST reject host-coupled or non-portable declarations unless an approved exception path exists.

## Success Criteria

- **SC-001**: A valid `contract.json` can be parsed into normalized Rust domain types without ambiguity.
- **SC-002**: Invalid contracts fail with stable error records covering field path, code, and explanation.
- **SC-003**: Duplicate published version misuse is rejected deterministically.
- **SC-004**: Lifecycle, semver, and portability rules are all validated automatically.
- **SC-005**: The implementation of this spec can be added to the protected coverage target list and held to 100% coverage.

## Governing Relationship

This specification is governed by:

- `001-foundation-v0-1`
- constitution version `1.2.0`

This specification, once approved, is intended to govern the first real implementation slice in:

- `crates/cogolo-contracts`
