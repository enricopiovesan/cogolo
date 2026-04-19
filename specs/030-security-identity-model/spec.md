# Feature Specification: Security Identity Model

**Feature Branch**: `030-security-identity-model`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Identity and signing layer for `traverse-runtime`, `traverse-contracts`, and `traverse-cli`, covering caller identity propagation, artifact signature verification, OIDC-style JWT handling, and safe identity derivation.

## Purpose

This spec defines the security identity model for Traverse.

It narrows the broad intent of "authenticated execution" into a concrete, testable model covering:

- canonical caller identity format (OIDC-style JWT access token)
- safe identity derivation: `subject_id` and `actor_id` extracted from token claims, never the raw token
- propagation of derived identity through runtime requests, execution contexts, emitted events, trace attributes, and subscription filters
- two-tier artifact signature verification: published/governed artifacts MUST pass Ed25519 signature verification before execution; local/dev artifacts generate a visible warning and are never silently accepted
- pluggable signing scheme: Ed25519 keypair signing as the required baseline, Sigstore (keyless, Fulcio/Rekor) as a supported alternative for published artifacts
- token non-disclosure: JWT access tokens MUST NEVER appear in traces, logs, events, or any exported telemetry

This spec does not define audit logging persistence, multi-tenant data isolation, or network-level authentication (TLS). Those are separate concerns.

## User Scenarios and Testing

### User Story 1 - Published WASM Module Fails Clearly on Signature Mismatch (Priority: P1)

As an enterprise operator, I want a published WASM module configured with an Ed25519 signature to cause a clear, explicit execution failure when verification fails so that tampered or misconfigured artifacts are never silently executed.

**Why this priority**: Silent execution of unsigned or tampered published artifacts is an unacceptable security regression. Clear failure is non-negotiable.

**Independent Test**: Register a published WASM module with a valid Ed25519 signature, then submit a runtime request referencing a corrupted version of the artifact (byte-modified). Verify the runtime returns an explicit `signature_verification_failed` error and produces no execution output.

**Acceptance Scenarios**:

1. **Given** a published artifact with a valid Ed25519 signature in its manifest, **When** the runtime loads the artifact and the signature is valid, **Then** execution proceeds normally with a `signature_verified` trace event recorded.
2. **Given** a published artifact whose signature does not match its content, **When** the runtime attempts to load it, **Then** execution is rejected with a `signature_verification_failed` error before any capability logic runs.
3. **Given** a published artifact with no signature in its manifest, **When** the runtime encounters it, **Then** execution is rejected with a `missing_signature` error — never silently passed.

### User Story 2 - Local/Dev Module Produces Visible Warning, Not Silent Pass (Priority: P1)

As a developer running Traverse locally, I want to receive a visible warning when a module has no signature in dev mode so that I know the module is unverified without the runtime blocking my workflow.

**Why this priority**: Silent acceptance of unsigned dev artifacts creates a false sense of security and risks developers shipping unsigned artifacts to production by accident.

**Independent Test**: Run the runtime in dev mode with an unsigned local module. Verify that a `WARNING: unverified_artifact` message is emitted to stderr and to the runtime trace before execution proceeds, and that the warning is machine-readable (structured field, not only plain text).

**Acceptance Scenarios**:

1. **Given** a local/dev artifact with no signature and dev mode enabled, **When** the runtime loads it, **Then** a structured warning is emitted (to stderr and as a trace event) containing the artifact path and identity before execution proceeds.
2. **Given** a local/dev artifact with no signature and dev mode enabled, **When** execution completes, **Then** the execution result carries a `warnings` field listing the unverified artifact.
3. **Given** dev mode is disabled (i.e., production mode), **When** the runtime encounters an unsigned local artifact, **Then** execution is rejected with `missing_signature`, identical to the published artifact path.

### User Story 3 - Emitted Events Carry subject_id Without Exposing Raw Token (Priority: P2)

As an authenticated user whose JWT is passed to traverse-cli, I want all emitted events to carry my `subject_id` without exposing the raw token so that downstream consumers can filter and audit events by identity without access to credentials.

**Why this priority**: Token leakage via event streams is a confidentiality failure. Derived identifiers are the correct propagation primitive.

**Independent Test**: Pass a valid JWT to traverse-cli, execute a capability, and capture all emitted events. Verify that no event payload contains the raw JWT string and that every event carries `subject_id` matching the `sub` claim of the JWT.

**Acceptance Scenarios**:

1. **Given** a valid JWT passed as caller identity, **When** the runtime derives identity, **Then** `subject_id` is set to the `sub` claim value and `actor_id` is set to the `act.sub` claim value (or omitted if absent).
2. **Given** a derived `subject_id`, **When** events are emitted during execution, **Then** every event envelope includes `subject_id` and (when present) `actor_id` as top-level fields.
3. **Given** any execution path (success or failure), **When** the runtime trace is exported, **Then** the raw JWT string does not appear in any span attribute, log record, event payload, or metric label.

### User Story 4 - Sigstore Verification Usable in CI/CD Without Private Key Management (Priority: P2)

As an operator using Sigstore for published artifact verification in a CI/CD pipeline, I want keyless verification to work when Rekor is reachable so that I do not need to manage Ed25519 private keys in the pipeline.

**Why this priority**: Keyless signing reduces key management burden for CI-published artifacts and is the industry direction for supply chain security.

**Independent Test**: Sign a WASM module artifact using Sigstore (Fulcio/Rekor), configure Traverse with `signing_scheme = "sigstore"`, submit a runtime request referencing the artifact, and verify successful verification. Then simulate Rekor unavailability and verify the runtime fails with `sigstore_unreachable` rather than silently passing.

**Acceptance Scenarios**:

1. **Given** a published artifact signed via Sigstore and Rekor is reachable, **When** the runtime verifies it, **Then** verification succeeds and a `sigstore_verified` trace event is recorded.
2. **Given** a published artifact signed via Sigstore and Rekor is unreachable, **When** the runtime attempts verification, **Then** execution is rejected with `sigstore_unreachable` — never silently passed.
3. **Given** Sigstore is configured but the artifact carries an Ed25519 signature instead, **When** verification runs, **Then** the runtime falls back to Ed25519 verification rather than failing; the signing scheme used is recorded in the trace.

## Edge Cases

- Token expiry during long-running execution: the runtime MUST detect token expiry at the point of identity derivation (request intake); mid-execution expiry MUST NOT cause silent identity loss — the originally derived `subject_id` is pinned for the full execution lifecycle.
- Multi-tenant requests: two concurrent requests with different `subject_id` values MUST NOT share execution context, emitted event envelope, or span attributes; identity fields MUST be scoped to the request.
- Sigstore Rekor unreachable for published artifact verification: the runtime MUST reject the artifact with `sigstore_unreachable` rather than falling back to unsigned execution.
- Offline environments where keyless (Sigstore) verification is impossible: operators MUST be able to configure Ed25519-only mode as the fallback; Sigstore MUST NOT be the only verification path.
- JWT with missing or malformed `sub` claim: the runtime MUST reject the token with `invalid_identity` rather than deriving a null `subject_id`.
- Artifact manifest lists a signature but the signature file is missing from the artifact bundle: the runtime MUST treat this as `signature_verification_failed`, not `missing_signature`.
- Duplicate `subject_id` across distinct physical users (hash collisions or shared service accounts): out of scope for this spec; the runtime treats `subject_id` as opaque.

## Functional Requirements

- **FR-001**: The runtime MUST accept an OIDC-style JWT access token as the canonical caller identity input at the runtime request boundary.
- **FR-002**: The runtime MUST derive `subject_id` from the JWT `sub` claim and `actor_id` from the JWT `act.sub` claim (RFC 8693 token exchange) before any capability logic runs.
- **FR-003**: When the JWT `sub` claim is absent or malformed, the runtime MUST reject the request with `invalid_identity` before discovery.
- **FR-004**: The raw JWT string MUST NEVER appear in any span attribute, log record, event envelope, metric label, or structured trace output; only `subject_id`, `actor_id`, and the token reference hash are permitted in telemetry.
- **FR-005**: The runtime MUST compute a stable token reference hash (SHA-256 of the raw JWT bytes, hex-encoded) for correlation purposes only; this hash MUST be documented as non-secret but also non-reversible.
- **FR-006**: `subject_id` and `actor_id` MUST propagate through: runtime request → execution context → emitted event envelopes → trace span attributes → subscription filter payloads.
- **FR-007**: The runtime MUST implement two-tier artifact signature verification: "Published/Governed" artifacts MUST pass signature verification before execution; local/dev artifacts generate a structured warning and are allowed-but-warned in dev mode only.
- **FR-008**: "Published/Governed" is defined as any artifact referenced by an entry in `contracts/` or `specs/governance/approved-specs.json`.
- **FR-009**: The runtime MUST support Ed25519 keypair signing as the required baseline verification scheme; Ed25519 public keys MUST be stored in the artifact manifest, not in runtime config.
- **FR-010**: The runtime MUST support Sigstore (Fulcio/Rekor) as an additional verification scheme for published artifacts; the signing scheme is selected per-artifact based on the manifest field `signing_scheme`.
- **FR-011**: When Sigstore is configured and Rekor is unreachable, the runtime MUST reject the artifact with `sigstore_unreachable`; no fallback to unsigned execution is permitted.
- **FR-012**: In production mode, unsigned artifacts (regardless of source classification) MUST be rejected with `missing_signature`; dev mode allows-but-warns unsigned local artifacts.
- **FR-013**: Dev mode MUST be explicitly configured and MUST NOT be the default; production mode is the default.
- **FR-014**: Every signature verification outcome (verified, failed, missing, sigstore_unreachable) MUST be recorded as a structured trace event on the artifact loading span.
- **FR-015**: The runtime MUST pin the derived `subject_id` and `actor_id` for the full lifecycle of an execution attempt; mid-execution token expiry MUST NOT alter the pinned identity.
- **FR-016**: Subscription filters MUST support filtering by `subject_id`; the filter MUST be evaluated against the derived identity field in the event envelope, not against the raw token.
- **FR-017**: The CLI (`traverse-cli`) MUST accept a JWT via a dedicated flag or environment variable and MUST pass the derived identity (not the raw token) to the runtime.
- **FR-018**: The contracts layer MUST define a canonical identity context schema (`subject_id: String`, `actor_id: Option<String>`, `token_reference_hash: String`) used uniformly across runtime request, event envelope, and execution context.
- **FR-019**: The runtime MUST expose a machine-readable verification report for each artifact load attempt, accessible via the execution trace.
- **FR-020**: Multi-tenant isolation MUST be enforced at the request scope: identity fields from one request MUST NOT be readable from a concurrent request's execution context.

## Non-Functional Requirements

- **NFR-001 Token Non-Disclosure**: The raw JWT MUST be zeroed from memory immediately after identity derivation and MUST NOT be retained in any heap-allocated structure beyond the derivation phase.
- **NFR-002 Determinism**: Ed25519 signature verification MUST be deterministic; the same artifact and public key MUST always produce the same pass/fail result.
- **NFR-003 Auditability**: Every signature verification outcome and every identity derivation event MUST be recorded in the execution trace with enough detail to reconstruct who ran what and what was verified.
- **NFR-004 Testability**: The signing verification layer MUST be injectable with test fixtures (test keypairs, mock Sigstore responses) for 100% automated coverage without live Rekor access.
- **NFR-005 Offline Operability**: Ed25519 verification MUST work fully offline; Sigstore is explicitly allowed to require network access, and operators in offline environments MUST configure Ed25519-only mode.
- **NFR-006 Pluggability**: The verification scheme is selected per-artifact via the manifest field; adding a new scheme MUST NOT require changes to core runtime dispatch logic.
- **NFR-007 Separation of Concerns**: Identity derivation, signature verification, and identity propagation MUST be implemented as distinct, independently testable modules within their respective crates.

## Non-Negotiable Quality Standards

- **QG-001**: Raw JWT strings MUST NEVER appear in exported telemetry, events, or logs; an automated test MUST scan all export sinks for JWT patterns and fail the suite if any are found.
- **QG-002**: Unsigned published artifacts MUST NEVER execute silently in any mode; automated tests MUST cover the `missing_signature` rejection path for published artifact classification.
- **QG-003**: Local/dev unsigned artifacts MUST emit a structured (machine-readable) warning before execution in dev mode; a plain-text-only warning without a structured field MUST fail the quality gate.
- **QG-004**: Sigstore Rekor unavailability MUST cause rejection, not silent pass; an automated test using a mock unreachable Rekor MUST verify this behavior.
- **QG-005**: Identity context (`subject_id`, `actor_id`, token reference hash) MUST propagate consistently across all four propagation boundaries (runtime request, execution context, event envelope, subscription filter); a cross-boundary propagation test is REQUIRED.
- **QG-006**: 100% automated line coverage MUST be maintained for identity derivation, signature verification dispatch, dev-mode warning emission, and propagation logic.

## Key Entities

- **JWT Access Token**: The OIDC-style bearer token provided by the caller as proof of identity; consumed at request intake and immediately discarded after derivation.
- **subject_id**: The stable string identifier derived from the JWT `sub` claim; the canonical caller identity used for propagation and filtering.
- **actor_id**: The optional string identifier derived from the JWT `act.sub` claim (RFC 8693 delegation chain); absent when the caller is not acting on behalf of another principal.
- **Token Reference Hash**: A SHA-256 hex-encoded hash of the raw JWT bytes, used for correlation across telemetry without exposing credential material.
- **Identity Context**: The canonical structured type (`subject_id`, `actor_id`, `token_reference_hash`) defined in `traverse-contracts` and used uniformly across the runtime boundary.
- **Published/Governed Artifact**: Any artifact referenced by an entry in `contracts/` or `specs/governance/approved-specs.json`; subject to REQUIRED signature verification.
- **Local/Dev Artifact**: Any artifact not referenced by a governed entry; subject to allowed-but-warned behavior in dev mode and REQUIRED rejection in production mode.
- **Ed25519 Signing Scheme**: The baseline artifact signing scheme using Ed25519 keypairs; verifiable offline, deterministic, and required for all published artifact configurations.
- **Sigstore Signing Scheme**: The keyless artifact signing scheme using Fulcio certificate authority and Rekor transparency log; requires network access to Rekor for verification.
- **Verification Report**: The structured trace artifact recording the outcome of signature verification for one artifact load attempt.

## Success Criteria

- **SC-001**: A published WASM module with a valid Ed25519 signature executes successfully; a module with a corrupted signature is rejected with `signature_verification_failed` before execution.
- **SC-002**: A local/dev unsigned module in dev mode produces a structured warning in the execution trace and the result `warnings` field, and then proceeds to execute.
- **SC-003**: All emitted events for an authenticated request carry `subject_id` and no raw JWT string; automated scan of export sinks confirms zero JWT leakage.
- **SC-004**: Sigstore verification with an unreachable Rekor produces `sigstore_unreachable` rejection; no execution output is produced.
- **SC-005**: Identity context propagates correctly across all four boundaries (runtime request → execution context → event envelope → subscription filter) in a cross-boundary propagation test.
- **SC-006**: 100% automated line coverage is confirmed for identity derivation, verification dispatch, dev-mode warning, and propagation modules.

## Out of Scope

- Audit log persistence and long-term audit trail storage
- Multi-tenant data isolation beyond identity scoping at the request level
- Network-level authentication (mTLS, client certificate validation)
- Role-based access control (RBAC) or permission enforcement
- Token refresh or OAuth 2.0 flow management
- Key rotation procedures for Ed25519 keypairs
- Hardware security module (HSM) integration
- Single sign-on (SSO) federation or OIDC provider configuration
