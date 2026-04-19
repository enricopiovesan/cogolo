# Feature Specification: Connector Plugin Architecture

**Feature Branch**: `039-connector-plugin-architecture`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Plugin model that allows capabilities to declare external resource dependencies (HTTP, filesystem, environment variables) through a `ConnectorPlugin` trait and a `connector_contract.json`; connectors are registered in the Traverse registry alongside capabilities and injected at execution time. Governs `crates/traverse-contracts/` and `crates/traverse-registry/`. Unblocks GitHub issue #331.

## Purpose

This spec defines the Connector Plugin Architecture for Traverse — the mechanism by which capabilities declare, discover, and use external resource integrations without embedding resource-access logic inside capability modules or the runtime core.

A connector is a discrete, registered artifact that satisfies a typed interface (`ConnectorPlugin` trait) and declares its identity, version, provided capabilities, required configuration, and supported placement targets via a `connector_contract.json`. Capabilities declare their connector requirements in their own contracts; the runtime wires connectors to capabilities at execution time.

The execution model for v0 is hybrid:
- The `ConnectorPlugin` trait defines a single interface for both host-native (Rust) and future WASM connectors.
- v0 ships three host-native reference connectors: HTTP (outbound), local filesystem read, and environment variable reader.
- A WASM connector lane is defined in the trait but not required for v0 compliance.

Connectors are registered in the Traverse registry as first-class entries alongside capabilities, enabling the same discovery, versioning, and scope rules to apply. Connectors are isolated from direct capability state access; config injection flows through the runtime, never through the WASM module boundary directly.

## User Scenarios and Testing

### User Story 1 — Runtime HTTP Connector Wiring (Priority: P1)

As a capability developer, I want to declare `connector_requirements: [{connector_id: "traverse.http", version: "^1.0.0"}]` in my capability contract so that the runtime wires the HTTP connector without me embedding HTTP logic in my WASM module.

**Why this priority**: The connector wiring path is the primary value delivery of this spec; without it, capabilities cannot access external resources through the governed integration surface.

**Independent Test**: Register a capability contract with a valid `connector_requirements` entry for `traverse.http`. Register the HTTP reference connector. Submit a runtime request. Verify the runtime wires the connector and executes the capability successfully.

**Acceptance Scenarios**:

1. **Given** a registered capability contract declaring `connector_requirements: [{connector_id: "traverse.http", version: "^1.0.0"}]` and a registered HTTP connector satisfying that requirement, **When** the runtime resolves the capability for execution, **Then** it wires the HTTP connector and injects its config before execution begins.
2. **Given** the HTTP connector is successfully wired, **When** execution completes, **Then** the runtime trace records the connector id, resolved version, and placement target used for injection.
3. **Given** a capability contract referencing `traverse.http` and a runtime where the HTTP connector is not registered, **When** the capability is submitted for registration, **Then** registration fails with `missing_required_connector` before the capability is written to the registry.

### User Story 2 — Missing Connector Fails at Registration (Priority: P1)

As a CI engineer, I want a capability with a connector requirement that cannot be satisfied to fail at registration time so that broken capability packages never enter the registry.

**Why this priority**: Fail-at-registration prevents runtime surprises; a capability that cannot be wired should never be executable.

**Independent Test**: In a CI environment that registers only the environment-variable connector, attempt to register a capability declaring `traverse.http` as a required connector. Verify the registration returns `missing_required_connector`.

**Acceptance Scenarios**:

1. **Given** a registry that has only the environment-variable connector registered and a capability declaring a requirement for `traverse.http`, **When** the capability is submitted for registration, **Then** the registry returns `missing_required_connector` and the capability is not written.
2. **Given** a registration failure with `missing_required_connector`, **When** the error response is inspected, **Then** it includes the unsatisfied `connector_id` and the version range that could not be resolved.
3. **Given** the required connector is later registered and the capability is resubmitted, **When** registration is retried, **Then** it succeeds and the capability enters the registry.

### User Story 3 — Third-Party Custom Connector (Priority: P2)

As a third-party integrator, I want to implement a custom connector (e.g., a Postgres connector) using the `ConnectorPlugin` trait and register it without modifying the Traverse runtime so that Traverse's integration surface is extensible without core changes.

**Why this priority**: Extensibility without core modification is required for Traverse to be usable as a platform, not just a single-vendor tool.

**Independent Test**: Implement a minimal custom connector satisfying the `ConnectorPlugin` trait. Register it via the registry API. Register a capability that declares it as a requirement. Verify the runtime wires it and executes the capability.

**Acceptance Scenarios**:

1. **Given** a third-party connector implementing `ConnectorPlugin` with a valid `connector_contract.json`, **When** it is submitted to the registry, **Then** it is registered without requiring any changes to the Traverse runtime source.
2. **Given** a registered third-party connector, **When** a capability declares a requirement for it and is registered, **Then** the requirement is satisfied and the capability enters the registry.
3. **Given** a custom connector and a reference connector registered simultaneously, **When** a capability requires both, **Then** the runtime wires both and injects both configs before execution begins.

### User Story 4 — WASM Connector Execution (Priority: P2)

As a platform maintainer, I want a WASM connector to be registered and executed using the same WASM executor as capability modules so that the connector plugin model is symmetric for native and WASM implementations.

**Why this priority**: Defining the WASM connector execution path in the trait now prevents architectural divergence when WASM connectors are implemented.

**Independent Test**: Register a minimal WASM connector. Register a capability that requires it. Execute the capability. Verify the runtime loads and executes the WASM connector using the WASM executor, not a native code path.

**Acceptance Scenarios**:

1. **Given** a WASM connector registered with a valid `connector_contract.json` and a `wasm` placement target, **When** a capability requiring that connector is executed, **Then** the runtime loads the WASM connector through the WASM executor and wires its output to the capability.
2. **Given** a WASM connector that traps during execution, **When** the trap is detected, **Then** the runtime surfaces `ConnectorError` in the capability's execution trace and fails execution without crashing the host.
3. **Given** a WASM connector and a native connector both registered, **When** a capability requires only the native connector, **Then** the WASM connector is never loaded or executed.

## Edge Cases

- Connector version conflict — capability requires `^1.0.0` but registry has only `2.0.0`; fail at registration with `connector_version_incompatible`
- Circular connector dependency — connector A declares a requirement on connector B which declares a requirement on connector A; detect at registration time and reject with `circular_connector_dependency`
- Connector execution failure — surface as `ConnectorError` in the capability's execution trace; the trace MUST include the connector id, the failure description, and the execution step at which the failure occurred
- Missing config fields at execution time — connector config schema declared in `connector_contract.json` is not fully satisfied by the runtime-injected config; reject before execution with `connector_config_invalid`
- Connector registered with an invalid `connector_contract.json` — reject at connector registration time with `invalid_connector_contract` before the connector enters the registry
- Capability declares a connector requirement with a connector_id that does not match any registered connector id prefix — fail at registration with `unknown_connector_id`
- Two capabilities share the same connector requirement with conflicting config schemas — each capability retains its own config injection context; no merging of schemas occurs
- Connector placement target is `wasm` but no WASM executor is available in the current runtime — reject connector registration with `unsupported_placement_target`

## Functional Requirements

- **FR-001**: The runtime MUST define a `ConnectorPlugin` trait as the single interface contract for all connectors, covering both host-native (Rust) and WASM connector implementations.
- **FR-002**: Every connector MUST declare a `connector_contract.json` artifact containing at minimum: `connector_id`, `version`, `capabilities_provided` (list of capability ids the connector can serve), `required_config_schema` (JSON schema), and `supported_placement_targets`.
- **FR-003**: Connectors MUST be registered in the Traverse registry as first-class entries, subject to the same versioning and scope rules as capability registrations.
- **FR-004**: The registry MUST validate `connector_contract.json` structure and required fields at connector registration time, before the connector entry is written.
- **FR-005**: Capability contracts MUST support a `connector_requirements` field as a list of objects each containing `connector_id` and `version` (semver range).
- **FR-006**: At capability registration time, the registry MUST verify that every connector listed in `connector_requirements` is already registered and that at least one registered version satisfies the declared semver range.
- **FR-007**: If any connector requirement cannot be satisfied at capability registration time, the registry MUST reject the capability registration with `missing_required_connector` identifying the unsatisfied `connector_id` and version range.
- **FR-008**: If a connector requirement specifies a semver range that no registered version satisfies due to a major version mismatch, the registry MUST reject with `connector_version_incompatible`.
- **FR-009**: The registry MUST detect circular connector dependency chains at registration time and reject with `circular_connector_dependency`.
- **FR-010**: At capability execution time, the runtime MUST inject connector configuration into each required connector before executing the capability; configuration MUST NOT be exposed to the WASM capability module directly.
- **FR-011**: Connector config injection MUST validate the injected config against the `required_config_schema` declared in the `connector_contract.json` before execution begins; if the config does not satisfy the schema, execution MUST be rejected with `connector_config_invalid`.
- **FR-012**: Connectors MUST run in an execution context isolated from direct capability state; a connector MUST NOT be able to observe or modify the internal state of a capability module.
- **FR-013**: The v0 release MUST include three host-native reference connectors: `traverse.http` (outbound HTTP), `traverse.fs.read` (local filesystem read), and `traverse.env` (environment variable reader).
- **FR-014**: Each reference connector MUST have a corresponding `connector_contract.json` that is versioned and governed under the spec-alignment CI gate.
- **FR-015**: Connector execution failures MUST be surfaced as `ConnectorError` in the capability's execution trace, including the connector id, failure description, and the execution step at which the failure occurred.
- **FR-016**: The `ConnectorPlugin` trait MUST include a defined execution path for WASM connectors; the WASM path MUST use the same WASM executor as capability modules.
- **FR-017**: WASM connector traps MUST be caught by the runtime and surfaced as `ConnectorError` without crashing the host process.
- **FR-018**: The registry MUST expose a query interface that returns all connectors satisfying a given `connector_id` and semver range, to support both registration-time verification and runtime wiring.
- **FR-019**: Connector registration, capability registration with connector requirements, and connector wiring at execution time MUST each produce machine-readable, trace-compatible event records.
- **FR-020**: The runtime MUST NOT permit a capability to execute if any of its declared connector requirements are unresolved at execution time, even if they were satisfied at registration time but subsequently deregistered.

## Non-Functional Requirements

- **NFR-001 Isolation**: Connectors MUST be executed in a context isolated from capability state; no direct memory or state sharing between a connector and a capability module is permitted.
- **NFR-002 Determinism**: Connector requirement resolution and version matching MUST be deterministic for the same registry state and capability contract; identical inputs MUST produce identical outcomes.
- **NFR-003 Extensibility**: The `ConnectorPlugin` trait MUST be defined such that new connectors can be implemented and registered without modifying the Traverse runtime source.
- **NFR-004 Testability**: Connector registration validation, version matching, config schema validation, circular dependency detection, and execution wiring MUST each be independently testable with 100% automated line coverage.
- **NFR-005 Versioning**: Connector contracts and the `ConnectorPlugin` trait MUST be versioned under semver discipline; breaking changes MUST require a major version increment.
- **NFR-006 Observability**: Every connector wiring step and connector execution failure MUST be recorded in the runtime execution trace with sufficient detail to diagnose failures without relying on unstructured logs.
- **NFR-007 Config Security**: Connector configuration MUST be injected by the runtime at execution time and MUST NOT be readable by the capability WASM module; config values MUST not appear in capability execution traces.

## Non-Negotiable Quality Standards

- **QG-001**: A capability declaring a connector requirement that cannot be satisfied MUST be rejected at registration time and MUST NOT enter the registry.
- **QG-002**: Connector config schema validation MUST run on every execution attempt; the execution MUST be rejected if the config does not satisfy the declared schema.
- **QG-003**: A connector execution failure MUST be surfaced as `ConnectorError` in the trace; the capability MUST fail execution and the host process MUST NOT crash.
- **QG-004**: Circular connector dependency detection MUST run at registration time and MUST produce a machine-readable `circular_connector_dependency` error identifying the cycle.
- **QG-005**: Connector registration validation, version matching, config schema validation, and execution wiring logic MUST reach 100% automated line coverage under the quality gate.

## Key Entities

- **ConnectorPlugin**: The Rust trait that defines the single interface contract for all connectors; implemented by both host-native and WASM connectors.
- **Connector Contract**: The `connector_contract.json` artifact that declares connector identity, version, provided capabilities, required config schema, and supported placement targets.
- **Connector Requirement**: An entry in a capability contract's `connector_requirements` list, specifying a `connector_id` and a semver version range.
- **Connector Registry Entry**: The first-class registry record for a registered connector, subject to the same versioning and scope rules as capability entries.
- **ConnectorError**: The structured error type emitted in the execution trace when a connector fails during execution; includes connector id, failure description, and execution step.
- **Config Injection**: The runtime operation that delivers connector configuration to the connector at execution time, without exposing the config to the capability WASM module.
- **Reference Connector**: One of the three v0 host-native connectors shipped with Traverse: `traverse.http`, `traverse.fs.read`, and `traverse.env`.

## Success Criteria

- **SC-001**: A capability declaring `traverse.http` as a connector requirement can be registered and executed; the runtime wires the HTTP connector and injects its config without the capability module accessing config directly.
- **SC-002**: A capability with an unsatisfied connector requirement is rejected at registration time with `missing_required_connector`; it never enters the registry.
- **SC-003**: A third-party connector implementing `ConnectorPlugin` with a valid `connector_contract.json` is registered without modifying the Traverse runtime source.
- **SC-004**: A connector execution failure is surfaced as `ConnectorError` in the trace; the host process does not crash.
- **SC-005**: Connector registration validation, version matching, config schema validation, circular dependency detection, and execution wiring logic each reach 100% automated line coverage.

## Out of Scope

- Outbound connector authentication and secret management (e.g., OAuth, mTLS)
- Bi-directional or event-driven connectors (connectors that push data into capabilities)
- Connector-to-connector chaining beyond circular dependency detection
- Connector hot-reload or live deregistration while capabilities are executing
- Remote connector execution over a network transport
- UI or MCP surfaces for connector management
- Connector marketplace or external distribution registry
