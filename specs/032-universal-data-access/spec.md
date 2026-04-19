# Feature Specification: Universal Data Access

**Feature Branch**: `032-universal-data-access`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Offline-first state access layer for capabilities running across browser, edge, and cloud environments, covering the `DataStore` trait, schema validation, deterministic conflict resolution, and sync lifecycle.

## Purpose

This spec defines the universal data access layer for Traverse capabilities.

It narrows the broad capability portability intent into a concrete, testable model for:

- providing capabilities with a uniform `DataStore` trait regardless of execution environment
- validating reads and writes against a schema declared in the capability contract
- resolving offline conflicts deterministically using Lamport clock ordering
- triggering explicit sync on reconnect without corrupting already-persisted local state
- supporting pluggable storage adapters (browser IndexedDB, cloud KV, local SQLite) that conform to the same trait

This slice does **not** define distributed replication topology or background sync scheduling. It is intentionally limited to the trait boundary, contract schema declaration, Lamport-clock conflict resolution, and reconnect-triggered sync so the data access control plane can be built and verified before replication policy is added.

## User Scenarios and Testing

### User Story 1 - Write and Read Capability State Offline Then Merge On Reconnect (Priority: P1)

As a capability developer, I want my capability to store state via the `DataStore` trait while offline and have that state automatically merged on reconnect so that the same capability implementation runs correctly in the browser and the cloud without environment-specific code.

**Why this priority**: Without a portable, offline-safe data access layer, capabilities cannot be authored once and deployed anywhere — the core portability promise of Traverse breaks.

**Independent Test**: Register a capability with a declared state schema; write state offline via a local adapter; simulate reconnect; verify the runtime triggers sync and the merged state reflects the Lamport-clock winner deterministically.

**Acceptance Scenarios**:

1. **Given** a capability with a declared state schema and a local adapter configured, **When** the capability writes state while offline, **Then** the write succeeds locally, is stamped with a Lamport clock value, and is not lost when the adapter returns to connected mode.
2. **Given** two offline instances that each wrote the same key with different values, **When** the runtime triggers sync on reconnect, **Then** the key with the higher Lamport clock value wins and the merged state is identical on both instances.
3. **Given** a reconnect event, **When** the runtime initiates sync, **Then** it does not corrupt already-committed local writes regardless of sync outcome.

### User Story 2 - Validate Capability Writes Against Declared State Schema (Priority: P1)

As a platform developer, I want the runtime to reject capability writes that do not match the declared state schema so that malformed state never reaches the storage adapter.

**Why this priority**: Schema-validated writes are a non-negotiable correctness guarantee; without them, corrupt state can silently propagate across environments.

**Independent Test**: Declare a state schema in a capability contract; submit a write that violates the schema; verify the runtime rejects the write with a structured validation error before the adapter is called.

**Acceptance Scenarios**:

1. **Given** a capability contract with a declared typed JSON state schema, **When** the capability attempts to write a value that violates the schema, **Then** the runtime rejects the write with a structured `schema_validation_error` and does not invoke the adapter.
2. **Given** a valid write that conforms to the schema, **When** the runtime processes the write, **Then** the adapter is invoked and the write succeeds.
3. **Given** a capability that declares no state schema (read-only or stateless), **When** any write is attempted, **Then** the runtime rejects the write with a `no_state_schema_declared` error.

### User Story 3 - Resolve Conflict on Same Logical Timestamp Deterministically (Priority: P2)

As a platform developer, I want Lamport clock tie-breaks to be deterministic so that two instances merging the same state always converge to the same result.

**Why this priority**: Non-deterministic conflict resolution breaks the portability and reproducibility guarantees of Traverse.

**Independent Test**: Produce two writes with identical Lamport clock values and different payloads; trigger merge; verify the runtime always resolves the tie in the same direction using the defined tie-break rule.

**Acceptance Scenarios**:

1. **Given** two writes to the same key with equal Lamport clock values, **When** the runtime merges them, **Then** it applies the canonical tie-break rule (lexicographic order of writer identity) and always selects the same winner.
2. **Given** a tie-break resolution, **When** the merged state is inspected, **Then** it records which writer won and the Lamport clock value used.

### User Story 4 - Swap Storage Adapter Without Changing Capability Implementation (Priority: P2)

As an operator, I want to configure a cloud KV adapter instead of local SQLite without touching the capability contract or implementation so that the same capability artifact runs on different infrastructure.

**Why this priority**: Adapter portability is a second-class but important concern; it must be validated before the data access layer ships.

**Independent Test**: Instantiate the same capability with two different adapters (local SQLite stub and cloud KV stub); verify both adapters satisfy the `DataStore` trait contract and produce identical read/write results for the same inputs.

**Acceptance Scenarios**:

1. **Given** a capability configured with a cloud KV adapter, **When** the capability writes state, **Then** the write is routed to the cloud KV adapter without any change to the capability source.
2. **Given** adapter failure during a sync operation, **When** the sync fails, **Then** already-committed local state is intact and the runtime surfaces a structured sync error without data loss.

## Edge Cases

- Conflict on identical Lamport clock value — tie-break MUST use a deterministic, documented secondary rule (lexicographic writer identity).
- State schema evolution: new optional fields added to the schema MUST be readable by capabilities written against the old schema without error.
- Adapter failure mid-sync MUST leave already-written local state intact; partial remote writes MUST be rolled back or marked incomplete.
- Capabilities that declare no state schema MUST be rejected at write time with a clear error; reads return empty.
- A Lamport clock overflow (exceeding `u64::MAX`) MUST be handled without silent wraparound.
- A write to a key not declared in the state schema MUST be rejected even if the value is otherwise valid JSON.
- Sync triggered when no remote adapter is configured MUST fail with a clear `no_remote_adapter` error, not a panic.
- Read of a key that has never been written MUST return a typed empty/absent result, not an error.

## Functional Requirements

- **FR-001**: The runtime MUST expose a `DataStore` trait that capabilities use for all state reads and writes; capabilities MUST NOT access storage implementations directly.
- **FR-002**: The `DataStore` trait MUST support at minimum the operations: `read(key) -> Result<Value>`, `write(key, value) -> Result<()>`, `delete(key) -> Result<()>`, and `list_keys() -> Result<Vec<Key>>`.
- **FR-003**: Every write through the `DataStore` MUST be stamped with a Lamport clock value maintained by the runtime, not by the capability.
- **FR-004**: The runtime MUST validate every write against the typed JSON schema declared in the capability contract before invoking the adapter.
- **FR-005**: The runtime MUST reject any write whose value does not conform to the declared state schema and MUST return a structured `schema_validation_error` without invoking the adapter.
- **FR-006**: Capabilities that declare no state schema MUST receive a `no_state_schema_declared` error on any write attempt.
- **FR-007**: The state schema MUST be declared as a typed JSON schema field in the capability contract; its presence is required for any capability that writes state.
- **FR-008**: The runtime MUST support at least three storage adapters: local SQLite, browser IndexedDB, and cloud KV; all adapters MUST implement the `DataStore` trait.
- **FR-009**: The active storage adapter MUST be selected by operator configuration and MUST NOT require changes to the capability contract or implementation.
- **FR-010**: The conflict resolution default MUST be last-write-wins by Lamport clock; the runtime MUST select the entry with the higher Lamport clock value when merging concurrent writes.
- **FR-011**: When two conflicting writes carry identical Lamport clock values, the runtime MUST apply a deterministic tie-break: lexicographic ordering of the writer identity field.
- **FR-012**: Custom merge functions MAY be declared in the capability contract as an extension point; when declared, the runtime MUST invoke the custom merge function instead of last-write-wins.
- **FR-013**: Sync MUST be triggered explicitly by the runtime on reconnect; background sync is not required in this slice.
- **FR-014**: The runtime MUST NOT corrupt already-committed local state when sync fails; adapter failure during sync MUST leave the pre-sync local state intact.
- **FR-015**: The runtime MUST surface sync failures as structured errors containing the adapter identity, failure reason, and the list of keys that were not synced.
- **FR-016**: Schema evolution MUST support adding new optional fields to the state schema without breaking capabilities written against the previous schema version.
- **FR-017**: Reads of keys that have never been written MUST return a typed absent result; they MUST NOT return an error.
- **FR-018**: The runtime MUST record merge decisions in a structured merge trace that includes key, winning value, Lamport clock values of all candidates, and the resolution rule applied.
- **FR-019**: Lamport clock values MUST be `u64`; the runtime MUST detect and reject any increment that would overflow `u64::MAX` rather than wrapping.
- **FR-020**: The `DataStore` trait boundary MUST be stable and semver-versioned; breaking changes MUST increment the governing spec version.

## Non-Functional Requirements

- **NFR-001 Determinism**: Conflict resolution, tie-breaking, and merge trace generation MUST be deterministic for the same set of concurrent writes and writer identities.
- **NFR-002 Portability**: The `DataStore` trait MUST compile to WASM target without adapter-specific native dependencies leaking through the trait boundary.
- **NFR-003 Correctness**: Schema validation MUST occur on every write path; there MUST be no code path that bypasses validation and reaches the adapter.
- **NFR-004 Testability**: The conflict resolution logic, schema validation, and Lamport clock management MUST be testable independently of any concrete adapter implementation.
- **NFR-005 Adapter Isolation**: Adapter failures MUST be isolated from the capability execution path; a failing adapter MUST surface as a structured error, not a panic.
- **NFR-006 Explainability**: Every merge decision MUST produce a machine-readable merge trace artifact that is stable enough for future audit and MCP consumption.
- **NFR-007 Compatibility**: State schema declarations in capability contracts MUST be versionable and subject to semver discipline.

## Non-Negotiable Quality Standards

- **QG-001**: Writes MUST NEVER reach a storage adapter without passing schema validation; any code path that bypasses validation is a spec violation.
- **QG-002**: Conflict resolution MUST be deterministic; non-deterministic tie-break behavior is a blocking defect.
- **QG-003**: Adapter failure during sync MUST NOT corrupt local state; data loss from a sync failure is a blocking defect.
- **QG-004**: Core data access logic (validation, Lamport clock management, conflict resolution) MUST reach 100% automated line coverage.
- **QG-005**: The `DataStore` trait boundary MUST be verified against the governing spec before merge; drift from the declared interface is a blocking CI failure.

## Key Entities

- **DataStore Trait**: The uniform Rust trait exposed by the runtime for all capability state access; implemented by concrete storage adapters.
- **State Schema**: A typed JSON schema declared in the capability contract that defines the valid shape of all values the capability may write.
- **Lamport Clock**: A monotonic logical timestamp maintained by the runtime per writer identity, used to order concurrent writes for conflict resolution.
- **Storage Adapter**: A concrete implementation of the `DataStore` trait targeting a specific backend (local SQLite, browser IndexedDB, cloud KV).
- **Merge Trace**: A structured artifact recording the inputs, resolution rule, and winner for one conflict merge operation.
- **Sync Trigger**: The runtime-initiated operation that propagates offline-written state to the remote adapter on reconnect.
- **Writer Identity**: A stable identifier scoped to an offline instance used as the secondary tie-break key when Lamport clocks collide.
- **Custom Merge Function**: An optional capability-declared extension point that overrides the default last-write-wins conflict resolution strategy.

## Success Criteria

- **SC-001**: A capability reads and writes state through the `DataStore` trait in both local and cloud adapter configurations without any environment-specific code in the capability.
- **SC-002**: A write that violates the declared state schema is rejected before the adapter is called and returns a structured error.
- **SC-003**: Two offline instances with conflicting writes converge to the same merged state deterministically after sync.
- **SC-004**: Adapter failure during sync leaves all previously committed local writes intact and surfaces a structured error.
- **SC-005**: Core data access logic reaches 100% automated line coverage under the protected coverage gate.

## Out of Scope

- Background sync scheduling and sync policy configuration
- Multi-hop replication topology across more than two instances
- Conflict resolution strategies beyond last-write-wins and custom merge functions
- Encryption at rest or transport-layer security for adapter communication
- Fine-grained per-key access control within a single capability
- CRDT-based merge semantics
- Schema migration tooling for breaking schema changes
