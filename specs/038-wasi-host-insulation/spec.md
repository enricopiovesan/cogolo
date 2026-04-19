# Feature Specification: WASI Host Insulation and Traverse Host ABI

**Feature Branch**: `038-wasi-host-insulation`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Stable host-function boundary for WASM modules compiled for Traverse, insulating module authors from WASI churn and establishing an independently versioned Traverse Host ABI. Governs `crates/traverse-runtime/` and `Cargo.toml`. Unblocks GitHub issue #330.

## Purpose

This spec defines the Traverse Host ABI — a stable, versioned set of host functions that WASM modules call at runtime. The ABI is the sole sanctioned surface between a Traverse WASM module and the host runtime. It insulates module authors from changes in the underlying WASI implementation: a module compiled against ABI v1 continues to execute correctly after the host upgrades or replaces its WASI layer.

The primary compilation target for v0 is `wasm32-wasip1` (WASI Preview 1). This spec restricts v0 to that target only and defines the forward migration path to the Component Model / WASI Preview 2 as a separate adapter-layer strategy. No dual-target support is introduced in v0.

The host-function categories covered by ABI v1 are:

- **stdio**: structured input delivery and structured output capture
- **environment queries**: capability identity, declared version, and runtime-injected configuration
- **execution metadata**: trace context injection, execution-id propagation

This spec establishes how ABI versioning, load-time import validation, and the Component Model migration path are governed so that the boundary remains stable as the underlying WASI implementation evolves.

## User Scenarios and Testing

### User Story 1 — Module Stability Across Host Upgrades (Priority: P1)

As a WASM module author, I want a module compiled with Traverse Host ABI v1 to continue running correctly after the host runtime upgrades its WASI p1 implementation so that I never need to recompile a published module due to host-side WASI churn.

**Why this priority**: ABI stability is the primary contract that allows the Traverse ecosystem to distribute compiled WASM artifacts independent of the host runtime release cycle.

**Independent Test**: Compile a module targeting ABI v1. Simulate a host upgrade by swapping the underlying WASI p1 implementation. Verify the module executes correctly and produces identical output.

**Acceptance Scenarios**:

1. **Given** a WASM module compiled against Traverse Host ABI v1 and a host runtime that has been upgraded to a newer WASI p1 implementation, **When** the module is loaded and executed, **Then** the runtime applies the ABI v1 compatibility layer and the module executes without modification.
2. **Given** a host runtime reporting ABI v1 and a module that imports only `traverse_host::*` symbols, **When** the module is loaded, **Then** the import validation gate passes without error.
3. **Given** a host runtime that has promoted to ABI v2, **When** a v1 module is loaded, **Then** the runtime applies the shim path described under FR-009 and execution proceeds.

### User Story 2 — Enforced Import Boundary (Priority: P1)

As a platform maintainer, I want module authors to import only `traverse_host::*` functions and never directly import raw WASI syscalls so that the ABI boundary is enforced by the runtime, not by convention.

**Why this priority**: Without a load-time enforcement gate, any module can silently bypass the insulation layer, making ABI versioning meaningless.

**Independent Test**: Build a WASM module that imports one raw WASI function not in the ABI whitelist. Attempt to load it in the Traverse runtime. Verify the load fails with `unauthorized_host_import` and the module is never executed.

**Acceptance Scenarios**:

1. **Given** a WASM module that imports a raw WASI syscall not present in the ABI v1 whitelist, **When** the runtime attempts to load the module, **Then** load fails immediately with error code `unauthorized_host_import` before any module code executes.
2. **Given** a WASM module that imports only ABI-whitelisted functions, **When** the runtime loads the module, **Then** load succeeds and execution proceeds normally.
3. **Given** a module load failure due to `unauthorized_host_import`, **When** the runtime produces a trace, **Then** the trace records the specific import symbol that triggered the violation.

### User Story 3 — Component Model Migration Path (Priority: P2)

As a platform maintainer, I want to add Component Model / WASI Preview 2 support behind an adapter layer so that existing ABI v1 modules receive compatibility without recompilation.

**Why this priority**: Locking the platform to WASI Preview 1 indefinitely is not acceptable; the migration path must be specified before v1 artifacts are published at scale.

**Independent Test**: With the adapter-layer strategy documented, verify a test harness can load a WASI p1 module through the adapter layer against a p2 host without modification to the module.

**Acceptance Scenarios**:

1. **Given** a future host runtime that implements the adapter-layer strategy defined in this spec, **When** an ABI v1 module is loaded, **Then** the adapter translates ABI v1 imports to their p2 equivalents without requiring module recompilation.
2. **Given** a platform upgrade that introduces Component Model support, **When** new modules are compiled targeting ABI v2, **Then** they coexist with ABI v1 modules in the same registry without conflict.
3. **Given** the adapter layer fails to translate a specific ABI v1 import, **When** the failure is detected at load time, **Then** the runtime returns `abi_adapter_failure` with the untranslatable symbol identified.

### User Story 4 — CI Import Whitelist Verification (Priority: P2)

As a CI engineer, I want a deterministic job that verifies compiled WASM modules do not import host functions outside the Traverse Host ABI whitelist so that unauthorized imports are caught before artifacts are published.

**Why this priority**: Runtime load-time rejection is the last line of defense; CI verification prevents unauthorized modules from ever reaching distribution.

**Independent Test**: Run the CI verification job against a known-bad WASM binary containing a prohibited import. Verify the job fails and reports the violating symbol.

**Acceptance Scenarios**:

1. **Given** a CI pipeline that runs the ABI whitelist verification job, **When** a compiled WASM artifact contains only whitelisted imports, **Then** the job exits with status 0.
2. **Given** a CI pipeline that runs the ABI whitelist verification job, **When** a compiled WASM artifact contains at least one non-whitelisted import, **Then** the job exits non-zero and reports each violating symbol.
3. **Given** multiple compiled WASM artifacts submitted together, **When** the CI job runs, **Then** it validates all artifacts and reports all violations in a single structured output before exiting.

## Edge Cases

- Module imports a raw WASI function not in the ABI whitelist — reject at load time with `unauthorized_host_import`; module code must never execute
- ABI v1 module loaded on a host that has advanced to ABI v2 — compatibility shim applies transparently; no error surfaced to the module
- ABI v2 module loaded on a host that only supports ABI v1 — reject at load time with `unsupported_abi_version`; module must declare its minimum required ABI version
- Host function panics or traps inside the WASM execution context — caught by the runtime trap handler and surfaced as a structured `ExecutionFailure`; the host process must never crash
- Malformed or truncated WASM binary — caught at the binary parse phase before import validation; error returned as `malformed_wasm_artifact`
- Module declares an ABI version string that does not match any known ABI version — reject at load time with `unknown_abi_version`
- Module imports an ABI function that exists in v1 but has changed signature in v2 — adapter layer must detect the signature mismatch and return `abi_signature_mismatch`
- Two modules loaded concurrently both fail ABI validation — each failure is independent and both are reported without interference

## Functional Requirements

- **FR-001**: The runtime MUST define a versioned "Traverse Host ABI" as the sole sanctioned surface through which WASM modules invoke host capabilities.
- **FR-002**: Traverse Host ABI v1 MUST include the following function categories: stdio (structured input delivery, structured output capture), environment queries (capability id, declared version, runtime-injected configuration), and execution metadata (trace context injection, execution-id propagation).
- **FR-003**: The ABI version MUST be independently versioned using a semver-compatible scheme separate from the Traverse runtime version and the WASI implementation version.
- **FR-004**: Every WASM module MUST declare its target ABI version as a custom section or export in the binary before load-time validation occurs.
- **FR-005**: The runtime MUST maintain a machine-readable ABI whitelist that enumerates all permitted import symbols for each supported ABI version.
- **FR-006**: The runtime MUST perform import validation against the ABI whitelist for every WASM module at load time, before any module code is executed.
- **FR-007**: If a WASM module imports any symbol not present in the whitelisted set for its declared ABI version, the runtime MUST reject the load with error code `unauthorized_host_import` and include the violating symbol in the error detail.
- **FR-008**: The runtime MUST NOT execute any module code after a load-time validation failure.
- **FR-009**: When a module declares ABI v1 and the host supports ABI v2, the runtime MUST apply an ABI compatibility shim that translates v1 import calls to their v2 equivalents without requiring module recompilation.
- **FR-010**: When a module declares an ABI version higher than the maximum supported by the host, the runtime MUST reject the load with `unsupported_abi_version`.
- **FR-011**: Host function implementations MUST catch all WASM traps and runtime panics originating within the module execution context and surface them as structured `ExecutionFailure` results without crashing the host process.
- **FR-012**: The stdio category of ABI v1 MUST provide deterministic functions for delivering structured JSON input to a module and capturing structured JSON output from a module.
- **FR-013**: The environment query category of ABI v1 MUST provide read-only access to the capability id, declared contract version, and runtime-injected configuration; the module MUST NOT be able to modify these values.
- **FR-014**: The execution metadata category of ABI v1 MUST allow the runtime to inject a trace context and execution-id into the module before execution begins.
- **FR-015**: The ABI whitelist MUST be stored as a versioned, machine-readable artifact co-located with the runtime source and subject to the spec-alignment CI gate.
- **FR-016**: A CI verification job MUST be defined that validates compiled WASM artifacts against the ABI whitelist and fails with non-zero exit status when any prohibited import is detected.
- **FR-017**: The migration path to WASI Preview 2 / Component Model MUST be documented in this spec as an adapter-layer strategy: a future spec introduces ABI v2 targeting the Component Model; existing v1 modules are served through a published adapter without recompilation.
- **FR-018**: The v0 implementation MUST use `wasm32-wasip1` as the sole compilation target; no dual-target or Component Model support is introduced in this spec.
- **FR-019**: Load-time errors produced by ABI validation MUST be machine-readable, carry a stable `error_code`, and include sufficient path and symbol information to identify the violating artifact without manual inspection.
- **FR-020**: The runtime MUST expose the supported ABI version range so that tooling and registry consumers can query it without loading a module.

## Non-Functional Requirements

- **NFR-001 Stability**: The Traverse Host ABI version MUST evolve independently of the WASI implementation; a WASI p1 implementation swap MUST NOT require ABI version bumps or module recompilation.
- **NFR-002 Determinism**: ABI whitelist validation MUST be deterministic for the same module binary and ABI version; identical inputs MUST produce identical validation outcomes.
- **NFR-003 Isolation**: Host function implementations MUST be isolated from capability state such that one module's host function invocations cannot observe or modify another module's execution context.
- **NFR-004 Testability**: ABI whitelist validation logic, shim application logic, and host function trap handling MUST each be independently testable with 100% automated line coverage.
- **NFR-005 Observability**: Every load-time ABI validation failure MUST be surfaced in the runtime trace; the absence of an ABI validation step in the trace for a loaded module MUST be treated as a gate failure.
- **NFR-006 Forward Compatibility**: ABI whitelist additions in a minor version increment MUST be backward compatible; removals MUST only occur in a major version increment with a defined migration window.
- **NFR-007 Portability**: The ABI design MUST not encode assumptions about the host operating system beyond what `wasm32-wasip1` guarantees, preserving future portability to browser and edge hosts.

## Non-Negotiable Quality Standards

- **QG-001**: A WASM module that imports any symbol outside the ABI whitelist MUST be rejected at load time and MUST never execute any code.
- **QG-002**: Every load-time ABI validation failure MUST produce a machine-readable error with a stable `error_code` and the violating symbol identified.
- **QG-003**: Host function trap handling MUST catch all traps and surface them as structured `ExecutionFailure`; the host process MUST NOT crash due to a module trap.
- **QG-004**: ABI whitelist validation, shim logic, and host function implementations MUST reach 100% automated line coverage under the quality gate.
- **QG-005**: The ABI whitelist artifact MUST be subject to the spec-alignment CI gate; drift between the spec-declared ABI function set and the whitelist artifact MUST block merge.

## Key Entities

- **Traverse Host ABI**: The versioned, whitelist-governed set of host functions that WASM modules import; the sole sanctioned boundary between module code and the host runtime.
- **ABI Whitelist**: The machine-readable per-version enumeration of all permitted import symbols; stored as a versioned artifact co-located with the runtime source.
- **ABI Version**: An independently versioned semver-compatible identifier assigned to each published Traverse Host ABI revision; declared by modules as a custom section or export.
- **ABI Compatibility Shim**: The host-side adapter that translates import calls from an older ABI version to the current implementation without requiring module recompilation.
- **Load-Time Import Validation**: The gate that runs against every WASM module binary before any module code executes; compares declared imports against the ABI whitelist.
- **Host Function Category**: A logical grouping of related ABI functions (stdio, environment queries, execution metadata) that defines the functional scope of ABI v1.
- **Adapter-Layer Strategy**: The forward migration approach in which a future ABI v2 targets the Component Model; v1 modules are served through a published adapter without recompilation.

## Success Criteria

- **SC-001**: A WASM module compiled against Traverse Host ABI v1 executes correctly on a host that has upgraded its WASI p1 implementation without any modification to the module binary.
- **SC-002**: A WASM module importing any symbol outside the ABI whitelist is rejected at load time with `unauthorized_host_import` before any module code executes.
- **SC-003**: A host function trap or panic inside the WASM execution context is caught and surfaced as a structured `ExecutionFailure` without crashing the host process.
- **SC-004**: The CI ABI whitelist verification job fails with non-zero exit and identifies all violating symbols when presented with a module containing prohibited imports.
- **SC-005**: ABI whitelist validation, shim application, and host function trap handling each reach 100% automated line coverage under the quality gate.

## Out of Scope

- WASI Preview 2 / Component Model implementation (defined as a follow-on spec via the adapter-layer strategy)
- Dual-target compilation (wasm32-wasip1 and Component Model in the same v0 build)
- Browser or edge host adapters
- Module signing or integrity verification
- Custom user-defined host functions outside the three ABI v1 categories
- WASM module caching or artifact distribution
- Network or remote execution of WASM modules
