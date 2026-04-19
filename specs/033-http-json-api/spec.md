# Feature Specification: HTTP JSON API

**Feature Branch**: `033-http-json-api`
**Created**: 2026-04-19
**Status**: Draft
**Input**: HTTP+JSON external adapter for Traverse, covering `traverse-cli serve`, endpoint definitions, authentication tiers, wire format, versioning, and error envelope for v0 synchronous request/response.

## Purpose

This spec defines the HTTP JSON API for Traverse.

It narrows the broad transport-agnostic runtime intent into a concrete, testable model for:

- embedding an HTTP/1.1 server in `traverse-cli` via a `serve` subcommand
- exposing a versioned `/v1/` endpoint surface for capability execution, listing, registration, and health
- enforcing a tiered authentication model that is safe by default for loopback and requires Bearer JWT for non-loopback or production bindings
- producing structured JSON error envelopes for all failure paths
- maintaining wire format and versioning discipline suitable for semver governance

This slice does **not** define async or streaming execution, WebSocket transport, or `/v2/` endpoint shapes. It is intentionally limited to synchronous request/response over HTTP/1.1 so the external API surface can be verified before async patterns are introduced.

## User Scenarios and Testing

### User Story 1 - Execute a Capability via HTTP POST from Any Language (Priority: P1)

As a capability developer, I want to run `traverse-cli serve` and submit a capability execution request via `curl` or any HTTP client so that Traverse is usable from any language without the Rust SDK.

**Why this priority**: Without an HTTP boundary, Traverse is only accessible from Rust callers; the HTTP API is the primary cross-language integration surface.

**Independent Test**: Start `traverse-cli serve`; register one capability; submit `POST /v1/capabilities/execute` with a valid JSON body; verify the response contains a structured trace and result with HTTP 200.

**Acceptance Scenarios**:

1. **Given** the server is running with a registered capability, **When** a client sends `POST /v1/capabilities/execute` with a valid JSON body, **Then** the server returns HTTP 200 with a JSON body containing a trace and result.
2. **Given** a request body that fails runtime contract validation, **When** the server processes it, **Then** it returns HTTP 422 with a `{"error": {"code": "...", "message": "..."}}` envelope.
3. **Given** a request that causes an ambiguous match, **When** the server processes it, **Then** it returns HTTP 409 with an error envelope listing all matching candidates.

### User Story 2 - Call the HTTP API from a Browser Application (Priority: P1)

As a browser application developer, I want to call `POST /v1/capabilities/execute` over HTTP from a JavaScript client so that Traverse capabilities are usable from browser-hosted agents.

**Why this priority**: Browser integration is a named consumer of this API surface and must be verified before the API is considered stable.

**Independent Test**: Simulate a browser-originated HTTP request (including CORS preflight); verify the server responds correctly to both preflight and the actual POST.

**Acceptance Scenarios**:

1. **Given** a browser client sends an OPTIONS preflight to `/v1/capabilities/execute`, **When** the server processes it, **Then** it returns the correct CORS headers for the configured allowed origins.
2. **Given** a browser client sends `POST /v1/capabilities/execute` with content-type `application/json`, **When** the server processes it, **Then** it returns a JSON response with correct content-type and a structured trace.

### User Story 3 - Require Bearer JWT for Non-Loopback Bindings (Priority: P2)

As an operator, I want the server to require a Bearer JWT for any non-loopback binding so that production deployments are not accidentally exposed without authentication.

**Why this priority**: The tiered auth model is a safety-critical design decision; unauthenticated production access is a blocking security defect.

**Independent Test**: Start `traverse-cli serve --bind 0.0.0.0:8080`; submit a request without an `Authorization` header; verify the server returns HTTP 401.

**Acceptance Scenarios**:

1. **Given** the server is bound to a non-loopback address, **When** a client sends a request without an `Authorization: Bearer <token>` header, **Then** the server returns HTTP 401 with a structured error envelope.
2. **Given** the server is bound to a non-loopback address and the client provides a valid JWT, **When** the server processes the request, **Then** it authorizes the request and returns the normal response.
3. **Given** `--allow-unauthenticated` is set on a non-loopback binding, **When** the server starts, **Then** it logs a clearly visible warning that the binding is unsafe.

### User Story 4 - Health Check Returns 200 Without Credentials (Priority: P2)

As an operator, I want `GET /v1/health` to return 200 without credentials so that load balancers and orchestrators can probe liveness without managing tokens.

**Why this priority**: Health probes are infrastructure-level; requiring auth on a health endpoint breaks standard orchestration patterns.

**Independent Test**: Start the server with auth required; send `GET /v1/health` without any `Authorization` header; verify HTTP 200 with a JSON liveness body.

**Acceptance Scenarios**:

1. **Given** the server is running with Bearer JWT auth required, **When** a client sends `GET /v1/health` without any `Authorization` header, **Then** the server returns HTTP 200 with a JSON body indicating liveness.
2. **Given** the server is running, **When** `GET /v1/health` is called, **Then** the response includes at minimum `{"status": "ok"}` and a server version field.

## Edge Cases

- Concurrent requests: the server MUST handle multiple simultaneous requests without corrupting shared registry state.
- Large request payloads exceeding the configured maximum (default 8MB) MUST be rejected with HTTP 413 before body parsing.
- Auth token expiry during a long-running synchronous execution: the request is already authorized at intake; mid-execution expiry MUST NOT abort the in-progress execution.
- Bind failure (port already in use) MUST produce a clear error message to stderr and exit with a non-zero code.
- Request body that is not valid JSON MUST return HTTP 400 with a structured error envelope, not an unhandled panic.
- Missing `Content-Type: application/json` header on POST endpoints MUST return HTTP 415.
- Unknown endpoint paths MUST return HTTP 404 with a structured error envelope.
- Server started with `--bind` on a non-loopback address without `--allow-unauthenticated` and without a JWT signing key configured MUST fail at startup with a clear configuration error.

## Functional Requirements

- **FR-001**: `traverse-cli` MUST expose a `serve` subcommand that starts a long-running HTTP/1.1 server process.
- **FR-002**: The server MUST bind to `127.0.0.1:8080` by default; the bind address and port MUST be configurable via `--bind` flag.
- **FR-003**: All endpoints MUST be versioned under the `/v1/` URL prefix.
- **FR-004**: The server MUST expose `POST /v1/capabilities/execute` accepting a JSON body and returning a runtime trace and result.
- **FR-005**: The server MUST expose `GET /v1/capabilities` returning a JSON array of registered capabilities.
- **FR-006**: The server MUST expose `POST /v1/capabilities/register` accepting a capability contract JSON body and registering the capability.
- **FR-007**: The server MUST expose `GET /v1/health` returning liveness status and server version without requiring authentication.
- **FR-008**: All JSON responses MUST use `Content-Type: application/json`.
- **FR-009**: All error responses MUST use the envelope `{"error": {"code": "<machine_code>", "message": "<human_readable>"}}`.
- **FR-010**: When the server is bound to loopback (`127.0.0.1` or `::1`) with no explicit `--require-auth` flag, it MUST allow unauthenticated requests (dev mode).
- **FR-011**: When the server is bound to any non-loopback address, it MUST require `Authorization: Bearer <jwt>` for all endpoints except `GET /v1/health`.
- **FR-012**: When `--allow-unauthenticated` is passed on a non-loopback binding, the server MUST start but MUST emit a clearly visible startup warning.
- **FR-013**: The server MUST validate Bearer JWTs against a configured OIDC-compatible signing key; invalid or expired tokens MUST return HTTP 401.
- **FR-014**: Request bodies exceeding the configured maximum payload size (default 8MB, configurable via `--max-body-bytes`) MUST be rejected with HTTP 413 before full body parsing.
- **FR-015**: Requests with `Content-Type` other than `application/json` on POST endpoints MUST be rejected with HTTP 415.
- **FR-016**: Invalid JSON request bodies MUST return HTTP 400 with a structured error envelope.
- **FR-017**: Bind failures MUST produce a human-readable error on stderr and exit the process with a non-zero code.
- **FR-018**: The server MUST handle concurrent requests without corrupting registry state; registry access MUST be protected by appropriate synchronization.
- **FR-019**: CORS headers MUST be configurable; the default MUST allow loopback origins only; non-loopback allowed origins MUST be explicitly configured.
- **FR-020**: Unknown paths MUST return HTTP 404 with a structured error envelope.
- **FR-021**: The `POST /v1/capabilities/execute` endpoint MUST delegate to the same runtime execution path used by `traverse-cli run`; it MUST NOT implement a separate execution path.

## Non-Functional Requirements

- **NFR-001 Safety**: Non-loopback bindings MUST require explicit auth configuration; the server MUST NOT silently default to unauthenticated on non-loopback bindings.
- **NFR-002 Determinism**: The HTTP layer MUST not alter the determinism of the runtime execution path; same request body MUST produce the same trace and result.
- **NFR-003 Portability**: The HTTP server implementation MUST be decoupled from the runtime core; `traverse-runtime` MUST remain a library crate with no HTTP dependency.
- **NFR-004 Testability**: Endpoint handler logic MUST be testable without a live TCP listener; integration tests MUST test via in-process HTTP client.
- **NFR-005 Compatibility**: The `/v1/` wire format MUST be stable and semver-versioned; breaking changes MUST be introduced under `/v2/` and MUST NOT modify `/v1/` response shapes.
- **NFR-006 Observability**: The server MUST log each inbound request with method, path, response status, and latency in a structured format.
- **NFR-007 Startup Correctness**: The server MUST validate its own configuration at startup and fail fast with a clear error if any required configuration is missing or invalid.

## Non-Negotiable Quality Standards

- **QG-001**: Auth bypass on non-loopback bindings MUST be impossible without the explicit `--allow-unauthenticated` flag; any code path that allows unauthenticated non-loopback access without this flag is a spec violation.
- **QG-002**: All error responses MUST use the structured envelope; free-form error strings or unhandled panics surfacing as HTTP 500 are blocking defects.
- **QG-003**: The HTTP endpoint handlers MUST delegate to `traverse-runtime` without duplicating execution logic; code duplication between `traverse-cli run` and `traverse-cli serve` execution paths is a spec violation.
- **QG-004**: Core handler logic MUST reach 100% automated line coverage.
- **QG-005**: The server MUST pass all endpoint contract tests before merge; deviations from the declared wire format are blocking CI failures.

## Key Entities

- **HTTP Server**: The embedded HTTP/1.1 server process started by `traverse-cli serve`.
- **Endpoint**: A versioned HTTP route under `/v1/` that accepts JSON requests and returns JSON responses.
- **Auth Tier**: The authentication policy applied based on bind address — dev mode (loopback, no auth) or production mode (non-loopback, Bearer JWT required).
- **Error Envelope**: The canonical JSON error response shape `{"error": {"code": "...", "message": "..."}}` used by all error responses.
- **Bearer JWT**: A JSON Web Token presented in the `Authorization` header for production-mode authentication, validated against a configured OIDC-compatible signing key.
- **Max Payload**: The configurable upper bound on request body size; requests exceeding this limit are rejected with HTTP 413.
- **CORS Configuration**: The set of allowed origins for cross-origin requests; defaults to loopback only.
- **Wire Format**: The stable JSON request and response shapes versioned under `/v1/`.

## Success Criteria

- **SC-001**: A developer runs `traverse-cli serve`, registers a capability, and executes it via `curl` or any HTTP client without using the Rust SDK.
- **SC-002**: A non-loopback binding without `--allow-unauthenticated` rejects all non-health requests without a valid Bearer JWT.
- **SC-003**: `GET /v1/health` returns HTTP 200 with a liveness payload regardless of auth configuration.
- **SC-004**: A request with a payload exceeding the configured maximum is rejected with HTTP 413 before body parsing completes.
- **SC-005**: Core endpoint handler logic reaches 100% automated line coverage.

## Out of Scope

- Asynchronous or streaming execution (request/response is synchronous in v0)
- WebSocket or Server-Sent Events transport
- `/v2/` endpoint shapes
- Fine-grained role-based access control beyond the dev/production auth tier
- TLS termination (assumed to be handled by a reverse proxy)
- API rate limiting
- OpenAPI/Swagger schema generation
- Multi-tenant isolation at the HTTP layer
