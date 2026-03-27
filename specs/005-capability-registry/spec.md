# Feature Specification: Traverse Capability Registry

**Feature Branch**: `005-capability-registry`  
**Created**: 2026-03-27  
**Status**: Draft  
**Input**: Existing Traverse foundation specification, capability contract and event contract slices, spec-alignment governance, and the registry design decisions agreed during the planning session.

## Purpose

This specification defines the first implementation-tight capability registry slice for Traverse.

The capability registry is responsible for making governed capabilities discoverable, versioned, composable, and stable without weakening the contract as the source of truth.

This spec defines:

- registry storage and indexing behavior for capability contracts
- public registry plus private overlay behavior
- artifact reference handling for source and WASM binary metadata
- semver-aware publication and immutability rules
- explicit composability metadata indexing
- composed capability representation

This specification governs work in `traverse-registry` before runtime execution or workflow traversal logic depends on registry semantics.

## User Scenarios & Testing

### User Story 1 - Register and Discover a Capability Version (Priority: P1)

As a platform developer, I want to register a capability contract and discover it later by identity, version, and metadata so that Traverse can treat capabilities as stable governed building blocks instead of ad hoc files.

**Why this priority**: Registry discovery is the operational bridge between contract authoring and runtime execution.

**Independent Test**: A valid capability contract and artifact manifest can be registered into the registry, indexed, and later discovered by exact identity/version and by metadata queries.

**Acceptance Scenarios**:

1. **Given** a valid approved capability contract and matching artifact metadata, **When** registration succeeds, **Then** the registry stores the contract, derived index entry, artifact reference entry, and discoverability metadata.
2. **Given** a registered capability identity with multiple versions, **When** a lookup requests an exact version, **Then** the registry returns the matching immutable record.
3. **Given** a metadata query such as owner, tags, composition role, or lifecycle, **When** registry discovery runs, **Then** the registry returns deterministic results ordered predictably.

---

### User Story 2 - Preserve Stability Through Semver and Immutability (Priority: P1)

As a capability consumer, I want the registry to preserve semver stability and reject unsafe republishing so that my application or workflow can depend on capability versions with confidence.

**Why this priority**: The registry is where Traverse must do better than the weak honor-system parts of package ecosystems.

**Independent Test**: Registering the same capability version with different governed content fails, while valid forward versions are checked against prior versions and accepted only when semver rules are satisfied.

**Acceptance Scenarios**:

1. **Given** a published capability identity and version, **When** a different governed contract digest is registered under the same identity and version, **Then** registration fails.
2. **Given** a prior published capability version and a new candidate version, **When** semver compatibility validation runs against the contract diff, **Then** registration fails if the declared bump is too small for the detected change class.
3. **Given** a consumer requesting a compatible version range later, **When** the registry resolves versions, **Then** the registry can distinguish exact, compatible, and breaking versions from contract-governed metadata.

---

### User Story 3 - Support Public Ecosystem and Private App Overlays (Priority: P2)

As an application builder, I want to use both shared public capabilities and app-specific private capabilities so that my system can compose common building blocks without giving up private business logic.

**Why this priority**: Traverse must support both an ecosystem model and app-local capability sets.

**Independent Test**: A lookup across a configured public registry and local private overlay resolves deterministic results with private overlay precedence.

**Acceptance Scenarios**:

1. **Given** a capability registered in the public registry and a capability with the same identity in the private overlay, **When** lookup runs in the local application context, **Then** the private overlay entry takes precedence.
2. **Given** a capability that exists only in the public registry, **When** the private overlay has no matching record, **Then** discovery falls back to the public registry.
3. **Given** a private overlay registration, **When** the capability is queried later, **Then** the registry exposes the scope as private/local rather than public.

---

### User Story 4 - Discover Safe Composition Paths (Priority: P2)

As a developer or agent, I want the registry to expose composability metadata and composed capabilities clearly so that I can build reusable workflows and higher-level capabilities without guessing how capabilities fit together.

**Why this priority**: Traverse’s value depends on safe, inspectable composition, not just storage of isolated contracts.

**Independent Test**: The registry can answer queries about atomic versus composed capabilities, composition roles, and downstream/upstream composition metadata deterministically.

**Acceptance Scenarios**:

1. **Given** an atomic capability contract, **When** it is registered, **Then** the registry indexes its explicit composability metadata.
2. **Given** a composed capability contract backed by a workflow definition, **When** it is registered, **Then** the registry records it as a first-class capability with a workflow-backed implementation reference.
3. **Given** a discovery query for capabilities that can participate in enrichment, validation, or event-driven composition, **When** the registry evaluates indexed metadata, **Then** it returns matching capabilities predictably.

## Scope

In scope:

- capability contract storage and indexing
- artifact metadata records for source references and WASM binaries
- public registry plus private overlay model
- deterministic lookup and precedence rules
- contract immutability rules
- semver compatibility enforcement hooks based on contract diffs
- composability metadata indexing
- composed capability representation

Out of scope:

- event registry behavior
- workflow registry behavior beyond composed capability references
- runtime execution
- remote artifact store implementation beyond the abstraction and metadata model
- Backstage implementation

## Requirements

### Functional Requirements

- **FR-001**: The capability registry MUST treat the capability contract artifact as the governing source of truth for capability identity, boundary, lifecycle, and semver.
- **FR-002**: The registry MUST store the original capability contract artifact and MUST NOT replace it with a derived record as the authoritative source.
- **FR-003**: The registry MUST maintain a generated index optimized for discovery and lookup.
- **FR-004**: The registry MUST maintain artifact metadata records separate from the contract artifact.
- **FR-005**: Artifact metadata MUST support source references, WASM binary references, content digests, and provenance metadata.
- **FR-006**: The registry MUST support a public registry scope and a private/local overlay scope using the same contract model.
- **FR-007**: Lookup MUST prefer the private/local overlay over the public registry when both contain matching identities in the same lookup context.
- **FR-008**: Published capability identity plus version pairs MUST be immutable.
- **FR-009**: Registration MUST fail when the same published identity and version are reused with different governed contract content.
- **FR-010**: Registration MUST validate declared semver progression against the previous published version using contract-diff compatibility rules.
- **FR-011**: The registry MUST support exact version lookup and metadata discovery in `v0.1`.
- **FR-012**: The registry MUST persist enough metadata to support future compatible-version resolution without redesigning the registry model.
- **FR-013**: The registry MUST index explicit composability metadata from the capability contract.
- **FR-014**: The registry MUST distinguish atomic capabilities from composed capabilities.
- **FR-015**: A composed capability MUST be represented as a first-class capability contract whose implementation reference points to a deterministic workflow definition rather than a direct executable artifact.
- **FR-016**: The registry MUST store enough implementation metadata to distinguish executable-artifact-backed capabilities from workflow-backed capabilities.
- **FR-017**: Registration MUST preserve lifecycle, owner, scope, provenance, and evidence metadata for each published capability version.
- **FR-018**: The registry MUST produce deterministic results for the same stored records and query inputs.
- **FR-019**: Registration and lookup failures MUST produce stable machine-readable error records with error code, path or target, and explanation.
- **FR-020**: Successful registration MUST produce machine-readable registration evidence linked to the capability identity, version, governing spec, and artifact digests.
- **FR-021**: The registry MUST be designed around an artifact storage abstraction, even though `v0.1` may implement only a local adapter.
- **FR-022**: The registry model MUST be exportable to a future Backstage catalog projection without making Backstage the source of truth.
- **FR-023**: Approved implementation under this spec MUST be validated against this governing spec before merge.

### Key Entities

- **Capability Registry Record**: The stored representation of a versioned capability contract plus governed metadata needed for discovery.
- **Capability Registry Index Entry**: The derived lookup-friendly metadata representation built from a capability contract and related artifact records.
- **Capability Artifact Record**: Metadata describing source references, WASM binaries, digests, provenance, and implementation backing.
- **Capability Scope**: The visibility context for a capability version, initially `public` or `private`.
- **Composition Metadata**: Machine-readable metadata describing how a capability can participate in safe composition.
- **Implementation Kind**: The implementation backing classification for a capability, initially `executable` or `workflow`.

## Non-Functional Requirements

- **NFR-001 Determinism**: Registration and lookup results MUST be deterministic for the same input records.
- **NFR-002 Stability**: The registry model MUST preserve immutable publication semantics and predictable semver governance.
- **NFR-003 Discoverability**: Indexed metadata MUST support human and agent discovery without requiring raw contract inspection for common queries.
- **NFR-004 Maintainability**: The registry MUST clearly separate contract storage, artifact metadata, and derived indexing concerns.
- **NFR-005 Portability**: Artifact metadata and implementation records MUST not assume a single hosting backend or transport mechanism.
- **NFR-006 Testability**: Core registration, immutability, precedence, and lookup logic MUST be structured for full automated coverage once implemented.

## Non-Negotiable Quality Gates

- **QG-001**: No registry implementation may merge unless it preserves contract primacy and immutable publication semantics.
- **QG-002**: Public/private overlay precedence MUST be explicit and tested.
- **QG-003**: Semver validation based on contract diffs MUST be part of registry publication behavior, not a manual-only process.
- **QG-004**: Composed capabilities MUST remain first-class discoverable capabilities and MUST NOT be hidden as undocumented workflow-only artifacts.

## Success Criteria

- **SC-001**: A capability can be registered once and discovered later by exact identity and version.
- **SC-002**: Republishing the same version with different governed content is rejected deterministically.
- **SC-003**: The registry can distinguish public and private capability records with deterministic lookup precedence.
- **SC-004**: Atomic and composed capabilities are both discoverable as first-class capabilities.
- **SC-005**: Registry implementation can be protected by the same spec-alignment and coverage discipline as the contract slice.

## Governing Relationship

This specification is governed by:

- `001-foundation-v0-1`
- `002-capability-contracts`
- `004-spec-alignment-gate`
- constitution version `1.2.0`

This specification, once approved, is intended to govern implementation in:

- `crates/traverse-registry`
