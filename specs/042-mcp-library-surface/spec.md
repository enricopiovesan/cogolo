# Feature Specification: MCP Library Surface

**Feature Branch**: `042-mcp-library-surface`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Spec slice defining the refactoring of `traverse-mcp` into a dual-surface crate: a public Rust library API callable by agents without MCP wire protocol, and a thin stdio server binary that wraps the library. Covers public function signatures, `McpToolRegistry`, semver stability guarantees, and embedding rules. Unblocks GitHub issue #310.

## Purpose

This spec defines the implementation-governing slice for exposing `traverse-mcp` as a first-class Rust library.

It narrows the intent into a concrete, testable model for:

- restructuring `traverse-mcp` so that its tool handler logic lives in a library crate, not only in the stdio binary
- exposing a public, semver-stable API surface (`discover_capabilities`, `get_capability`, `execute_capability`, `McpToolRegistry`)
- making the existing stdio MCP server a thin binary wrapper over the library
- allowing Rust agents to call tool handler functions directly without MCP wire protocol
- allowing third-party MCP servers to embed `McpToolRegistry` and re-export Traverse's tools as part of a larger tool surface

This slice does not change the MCP wire protocol, the tool schemas, or the underlying runtime execution behavior. It is a structural refactoring that adds a library surface without removing the binary surface.

## User Scenarios and Testing

### User Story 1 — Rust Agent Calls Library Functions Directly (Priority: P1)

As a Rust agent developer, I want to import `traverse_mcp` as a library dependency and call `execute_capability()` directly — without spawning a subprocess or implementing MCP stdio — so that I can integrate Traverse capabilities into my agent without the overhead of process management or wire protocol parsing.

**Why this priority**: Direct library access is the primary value of this spec. Without it, Rust agents must spawn a subprocess, which is fragile and not consistent with Rust-native integration patterns.

**Independent Test**: Write a Rust binary that adds `traverse_mcp` as a `[dependencies]` entry, calls `discover_capabilities()`, selects a known capability from the returned list, and calls `execute_capability()` with a valid input — verifying the result without invoking the stdio server.

**Acceptance Scenarios**:

1. **Given** a compiled Rust agent that imports `traverse_mcp`, **When** the agent calls `discover_capabilities()`, **Then** the function returns the same capability list as the `tools/list` MCP method would return for the same runtime state.
2. **Given** a compiled Rust agent that imports `traverse_mcp`, **When** the agent calls `execute_capability(id, version, input)` with a registered capability, **Then** the function returns the same execution result as the `tools/call` MCP method would return.
3. **Given** a compiled Rust agent that calls `execute_capability()` before the underlying runtime is initialized, **When** the call is made, **Then** the function returns `Err(TraverseMcpError::RuntimeNotInitialized)` without panicking.

### User Story 2 — Existing Stdio Server Binary Continues to Work (Priority: P1)

As a platform operator, I want the existing `traverse-mcp` stdio server to continue to work unchanged after the refactor — producing the same wire protocol output for the same MCP messages — so that existing MCP clients are not broken.

**Why this priority**: Backward compatibility of the stdio server is non-negotiable; the refactor must be invisible to MCP protocol consumers.

**Independent Test**: Run the stdio server binary before and after the refactor, send the same `tools/list` and `tools/call` MCP JSON-RPC messages, and verify the response payloads are byte-identical.

**Acceptance Scenarios**:

1. **Given** the refactored stdio server binary, **When** it receives a `tools/list` MCP JSON-RPC message, **Then** it returns a `tools/list` response with the same tool definitions as before the refactor.
2. **Given** the refactored stdio server binary, **When** it receives a `tools/call` MCP JSON-RPC message for a registered capability, **Then** it returns the same result structure as before the refactor.
3. **Given** the refactored stdio server binary running alongside a Rust agent calling library functions directly, **When** both operate against the same runtime state, **Then** neither corrupts shared state and both produce consistent results.

### User Story 3 — Third-Party MCP Server Embeds McpToolRegistry (Priority: P2)

As a third-party MCP server developer, I want to add `traverse_mcp` as a dependency, embed `McpToolRegistry` into my server, and re-export Traverse's `discover_capabilities` and `execute_capability` tools as part of my own tool surface — so that my server provides a superset of Traverse's MCP tools without duplicating their implementation.

**Why this priority**: Composability of the MCP tool surface is the primary motivation for the registry abstraction; without it the library API does not support the embedding use case.

**Independent Test**: Write a minimal Rust MCP server that creates an `McpToolRegistry`, calls `register_traverse_tools()`, adds one additional custom tool, and then calls `handle_tools_list()` — verifying the response includes both Traverse's tools and the custom tool.

**Acceptance Scenarios**:

1. **Given** a third-party MCP server that embeds `McpToolRegistry`, **When** it calls `register_traverse_tools()`, **Then** Traverse's `discover_capabilities` and `execute_capability` tool schemas are added to the registry.
2. **Given** an `McpToolRegistry` with Traverse tools registered, **When** the third-party server adds an additional tool with a name that does not conflict, **Then** the registry accepts it and `handle_tools_list()` returns all tools including both Traverse and custom tools.
3. **Given** an `McpToolRegistry` with Traverse tools registered, **When** the third-party server attempts to register a tool whose name conflicts with an already-registered Traverse tool, **Then** the registry returns `Err(McpRegistryError::ToolNameConflict)` and does not modify the existing registration.

### User Story 4 — CI Integration Test Calls Library Without Stdio (Priority: P2)

As a CI pipeline author, I want to import `traverse_mcp` as a library in integration tests and call `discover_capabilities()` and `execute_capability()` directly — without starting or connecting to a stdio server process — so that integration tests are faster, hermetic, and free of process lifecycle management.

**Why this priority**: Testability without subprocess overhead is a concrete quality benefit of the library surface; it must work without the stdio server.

**Independent Test**: Write a test in `traverse-mcp/tests/` that calls `discover_capabilities()` directly, verifies the list is non-empty, then calls `execute_capability()` for a known capability and verifies a successful result — all without launching the stdio binary.

**Acceptance Scenarios**:

1. **Given** the library API, **When** a test calls `discover_capabilities()` with a runtime that has registered capabilities, **Then** the function returns the complete list without stdio setup.
2. **Given** a test that calls `execute_capability()` with a valid input, **When** the function completes, **Then** it returns a result consistent with the runtime's contract validation output.

## Edge Cases

- Library caller and stdio server running simultaneously against the same runtime — shared state MUST be protected by the same synchronization primitives used by the runtime; no data races are permitted.
- Library function called before the underlying runtime is initialized — `RuntimeNotInitialized` error returned from all three public functions; no panic.
- Breaking change to a public library function signature (name, parameters, or return type) — requires a new spec slice; existing callers MUST be migrated before the breaking change merges.
- `McpToolRegistry` embedded in a third-party server that registers a tool with the same name as a Traverse tool — `ToolNameConflict` error at registration time; the conflicting tool is not registered.
- `execute_capability()` called with an input that fails the capability's contract input schema — function returns a structured validation error, not a panic; the error is the same error returned by the runtime for schema violations.
- `discover_capabilities()` called on a runtime with no registered capabilities — returns an empty list, not an error.
- `McpToolRegistry` used after the runtime backing it has been shut down — returns `RuntimeNotInitialized` for any tool dispatch call; does not panic.
- Library imported in a `#[cfg(test)]` context that does not initialize the full runtime — functions MUST return `RuntimeNotInitialized`, not an initialization side-effect or panic.

## Functional Requirements

- **FR-001**: The `traverse-mcp` crate MUST be refactored so that all tool handler logic lives in `lib.rs` (or a module tree rooted there), not only in a binary entry point.
- **FR-002**: The `traverse-mcp` crate MUST expose `discover_capabilities()` as a public function in its library API.
- **FR-003**: The `traverse-mcp` crate MUST expose `get_capability(id: &str, version: &str)` as a public function in its library API.
- **FR-004**: The `traverse-mcp` crate MUST expose `execute_capability(id: &str, version: &str, input: serde_json::Value)` as a public function in its library API.
- **FR-005**: All three public functions MUST return `Result` types with a typed `TraverseMcpError` error enum; `unwrap()` and `panic!()` are forbidden on any function call path reachable from the public API.
- **FR-006**: `TraverseMcpError` MUST include at minimum the variants `RuntimeNotInitialized`, `CapabilityNotFound`, `InputSchemaValidationFailed`, `ExecutionFailed`, and `RegistryUnavailable`.
- **FR-007**: The existing stdio server binary MUST be moved to `traverse-mcp/src/bin/server.rs` and MUST be a thin wrapper that calls the library functions for all MCP method handling.
- **FR-008**: The stdio server binary MUST produce byte-identical responses to the same MCP JSON-RPC messages before and after the refactor.
- **FR-009**: The crate MUST expose a `McpToolRegistry` struct with at minimum the methods `register_traverse_tools()`, `register_tool(tool: McpTool)`, `handle_tools_list()`, and `handle_tools_call(name: &str, arguments: serde_json::Value)`.
- **FR-010**: `McpToolRegistry::register_tool()` MUST return `Err(McpRegistryError::ToolNameConflict)` when a tool with the same name is already registered; it MUST NOT silently overwrite the existing registration.
- **FR-011**: `McpToolRegistry::register_traverse_tools()` MUST register Traverse's `discover_capabilities` and `execute_capability` tools into the registry.
- **FR-012**: The public function signatures (`discover_capabilities`, `get_capability`, `execute_capability`, `McpToolRegistry` methods) MUST be considered stable under semver once merged; breaking changes require a new spec slice.
- **FR-013**: The library functions MUST be safe to call from multiple threads simultaneously against the same runtime state; internal synchronization MUST prevent data races.
- **FR-014**: All three public functions MUST return `Err(TraverseMcpError::RuntimeNotInitialized)` when called before the runtime is initialized, without panicking or producing undefined behavior.
- **FR-015**: The crate's `Cargo.toml` `[lib]` section MUST be explicitly declared so that downstream crates can depend on the library without also depending on the binary.
- **FR-016**: The crate MUST NOT expose any internal modules, types, or functions as `pub` beyond what is documented in this spec's public API surface.

## Non-Functional Requirements

- **NFR-001 Backward Compatibility**: The stdio server binary's MCP wire protocol behavior MUST be unchanged after refactoring; existing MCP clients MUST continue to work without modification.
- **NFR-002 API Stability**: Public function signatures are semver-stable once merged; the library's major version MUST be incremented before any breaking signature change ships.
- **NFR-003 Thread Safety**: All public library functions MUST be safe to call from multiple threads against the same runtime; `Send + Sync` bounds MUST be satisfied on all public types.
- **NFR-004 Testability**: All three public tool handler functions MUST be independently testable at unit and integration level without spawning the stdio binary.
- **NFR-005 Minimal Surface**: The public library API is limited to the three functions and `McpToolRegistry`; no additional types, modules, or functions are made public by this spec.
- **NFR-006 No Panic Paths**: No panic, unwrap, or expect on any code path reachable from the public API; all error conditions MUST be surfaced via `Result`.
- **NFR-007 Crate Hygiene**: `traverse-mcp` MUST compile with no warnings under `cargo build` after the refactor; all dead code introduced by the structural change MUST be removed.

## Non-Negotiable Quality Standards

- **QG-001**: The public function signatures (`discover_capabilities`, `get_capability`, `execute_capability`) MUST NOT change after merge without a new governing spec; the CI gate MUST enforce this via the spec-alignment check.
- **QG-002**: All three public functions MUST return typed `Result` errors; `unwrap()` is forbidden on any reachable path from the public API.
- **QG-003**: The stdio server binary MUST pass byte-identical wire protocol tests before and after the refactor; regression in MCP protocol output is a blocking failure.
- **QG-004**: `McpToolRegistry::register_tool()` MUST return `ToolNameConflict` on collision; silent overwrite is not acceptable.
- **QG-005**: Core library logic MUST reach 100% automated line coverage under the protected coverage gate.

## Key Entities

- **Library API**: The set of public Rust functions exposed by `traverse-mcp` as a `[lib]` crate: `discover_capabilities()`, `get_capability()`, `execute_capability()`.
- **TraverseMcpError**: The typed error enum returned by all public library functions, covering runtime initialization, capability lookup, schema validation, and execution failures.
- **McpToolRegistry**: A struct that holds a named set of MCP tools and dispatches `tools/list` and `tools/call` requests; usable by third-party MCP servers to embed Traverse's tools.
- **McpRegistryError**: The typed error enum returned by `McpToolRegistry` methods, including `ToolNameConflict`.
- **Stdio Server Binary**: `traverse-mcp/src/bin/server.rs` — the thin binary wrapper that reads MCP JSON-RPC messages from stdin, dispatches to library functions, and writes responses to stdout.
- **McpTool**: A struct representing a single MCP tool definition with a name, description, and JSON Schema for its arguments; used as the registration unit for `McpToolRegistry`.

## Success Criteria

- **SC-001**: A Rust binary that imports `traverse_mcp` as a library and calls `discover_capabilities()` compiles and runs successfully without spawning the stdio server.
- **SC-002**: The stdio server binary produces byte-identical MCP wire protocol responses before and after the refactor, verified by automated regression tests.
- **SC-003**: `McpToolRegistry` accepts Traverse tools and a non-conflicting custom tool; rejects a tool with a conflicting name with `ToolNameConflict`.
- **SC-004**: All three public functions return `Err(RuntimeNotInitialized)` when called before runtime initialization; no panics occur in this path.
- **SC-005**: Integration tests call `execute_capability()` directly without the stdio binary and produce results consistent with the runtime's contract validation.
- **SC-006**: Core library logic reaches 100% automated line coverage under the protected coverage gate.

## Out of Scope

- Changes to the MCP wire protocol or tool schemas
- Non-Rust library bindings (Python, TypeScript, etc.) — non-Rust agents use the HTTP API from spec 033
- MCP authentication or authorization
- Tool versioning within the MCP protocol surface
- Streaming or incremental response support for `execute_capability`
- Dynamic tool discovery or hot-reload of tools at runtime
- Any change to the runtime execution model or capability contract format
