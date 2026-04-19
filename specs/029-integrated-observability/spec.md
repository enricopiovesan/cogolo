# Feature Specification: Integrated Observability (OTel Traces, Logs, Metrics)

**Feature Branch**: `029-integrated-observability`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Observability layer for `traverse-runtime` and `traverse-mcp`, covering OpenTelemetry signal export, W3C Trace Context propagation, attribute naming conventions, and deterministic test-mode identifiers.

## Purpose

This spec defines the integrated observability model for Traverse.

It narrows the broad intent of "OTel-compatible telemetry" into a concrete, testable set of requirements covering:

- emitting distributed traces (spans) for all runtime execution paths
- propagating W3C Trace Context across runtime request objects, event publication envelopes, and browser adapter subscriptions
- exporting structured log records with trace correlation
- emitting named metrics for execution counts, latency, and error rates
- pluggable OTLP export without vendor lock-in
- a deterministic seeded identifier mode for CI and snapshot testing

Compliance tiers are defined explicitly: distributed traces are REQUIRED for v0 compliance; structured logs and metrics are STANDARDIZED (testable and spec-governed) but optional for v0 compliance. All three signals are written as first-class requirements to support progressive adoption.

This spec does not define a UI or visualization layer. It governs only signal generation and export.

## User Scenarios and Testing

### User Story 1 - Full Execution Trace Visible in Standard OTel Backend (Priority: P1)

As an agent submitting a capability execution request, I want to receive an OTLP-compatible trace showing all spans from request intake to execution completion so that I can diagnose slow or failing executions without Traverse-specific tooling.

**Why this priority**: Distributed tracing is the primary observability primitive for request-oriented systems. Without it, no cross-service correlation is possible and execution failures are opaque.

**Independent Test**: Submit one valid runtime request end-to-end, capture the OTLP export, and verify that a root span and all child spans are present with correct parent-child relationships, attribute names, and status codes.

**Acceptance Scenarios**:

1. **Given** a valid runtime request is submitted, **When** the runtime processes the request through discovery, selection, and execution, **Then** a root span is created for the request and child spans are created for each named execution phase, each carrying at least the required `traverse.*` attributes.
2. **Given** a completed span tree, **When** it is exported via OTLP, **Then** the export payload is parseable by any standards-compliant OTel collector without custom schema extensions.
3. **Given** a request that fails at any phase, **When** the span for that phase is closed, **Then** the span status is set to `ERROR` and the span records the error classification as a structured event, not only in unstructured log output.

### User Story 2 - Standard Attribute Names Enable Backend Filter/Search Without Custom Parsers (Priority: P1)

As a team integrating Traverse with Jaeger or Grafana, I want to filter and search traces using standard OTel semantic convention attribute names so that I do not need to write custom parsers or know Traverse internals.

**Why this priority**: Non-standard attribute names break every OTel-aware backend query and defeat the purpose of adopting the OTel specification.

**Independent Test**: Run several requests with varying capability IDs and intents, export spans to a test collector, and verify that every span's attribute keys conform to OTel semantic conventions or the `traverse.` namespace prefix rule and that no ad hoc keys appear.

**Acceptance Scenarios**:

1. **Given** a span for a capability execution phase, **When** the span attributes are enumerated, **Then** every key either maps to a defined OTel semantic convention or begins with the `traverse.` namespace prefix.
2. **Given** a trace filtered by `traverse.capability.id` in a standard OTel backend, **When** the filter is applied, **Then** all spans related to that capability are returned without custom index configuration.
3. **Given** spans for both success and failure cases, **When** their attribute sets are compared, **Then** the same attribute keys are present (error spans carry additional error attributes, never different base keys).

### User Story 3 - Structured Log Records Carry Trace Correlation IDs (Priority: P2)

As an operator enabling log export, I want to receive structured log records with trace and span IDs so that logs from Traverse are joinable with trace data in any OTel-aware log management system.

**Why this priority**: Log-trace correlation is a standard expectation in production observability stacks. Without it, logs and traces are disconnected silos.

**Independent Test**: Enable log export, run a request, capture the exported log records, and verify each record carries a valid `trace_id` and `span_id` matching the enclosing span in the concurrent trace.

**Acceptance Scenarios**:

1. **Given** a runtime execution that emits at least one internal log record, **When** log export is enabled, **Then** each exported log record includes `trace_id` and `span_id` fields matching the OTel log data model.
2. **Given** a log record emitted during a failure, **When** it is exported, **Then** the severity is set to `ERROR` or higher and the `traverse.error.classification` attribute is present.
3. **Given** log export is disabled, **When** the runtime runs, **Then** no export errors are raised and internal log records are silently dropped without affecting trace export.

### User Story 4 - OTLP Export Endpoint Configurable Without Code Changes (Priority: P2)

As an SRE deploying Traverse to production, I want to configure the OTLP export endpoint through configuration alone so that I can point telemetry at any collector without recompiling Traverse.

**Why this priority**: Configuration-driven export is the minimum bar for production operability. Code changes for endpoint configuration are a deployment blocker.

**Independent Test**: Start the runtime with an OTLP endpoint set only via config (not baked in), submit a request, and verify spans arrive at the configured collector. Change the endpoint in config and verify spans route to the new target after restart.

**Acceptance Scenarios**:

1. **Given** an OTLP endpoint specified in the runtime config, **When** the runtime starts, **Then** the OTel SDK is initialized with that endpoint without any code change.
2. **Given** the OTLP endpoint is unreachable, **When** the runtime processes requests, **Then** export failures are logged internally but do not propagate as errors to the caller or cause request failures.
3. **Given** no OTLP endpoint is configured, **When** the runtime starts, **Then** telemetry is collected in-process but silently dropped (no-op exporter) without error.

## Edge Cases

- WASM boundary: spans MUST propagate correctly when execution crosses into a WASM module; the W3C `traceparent` header MUST be injected into the WASM execution context before invocation and extracted from the WASM response envelope after completion.
- Browser context: W3C Trace Context carrier MUST function for in-browser subscriptions; the carrier extraction logic MUST handle the absence of HTTP headers by falling back to the subscription envelope.
- Offline/air-gapped: OTLP export failures MUST NOT crash the runtime; the export path is fire-and-forget with a bounded retry budget; exhausted retries MUST be counted in the `traverse.telemetry.export.dropped_total` metric.
- Test mode: seeded deterministic IDs MUST produce stable trace IDs and span IDs for identical input sequences, enabling snapshot-style assertions in CI.
- Missing `traceparent` on inbound request: the runtime MUST create a new root span rather than failing the request.
- Concurrent requests: span context MUST not leak between concurrent execution threads or async tasks.
- Very high attribute cardinality (e.g., large input payloads): attribute values MUST be truncated to a bounded length rather than dropped entirely; truncation MUST be recorded as a span event.

## Functional Requirements

- **FR-001**: The runtime MUST create an OTel root span for every runtime request at the point of request intake, before any discovery or validation logic runs.
- **FR-002**: The runtime MUST create child spans for each named execution phase: request validation, capability discovery, candidate evaluation, capability selection, execution, contract output validation, and request completion.
- **FR-003**: Every span MUST carry the `traverse.request.id` and `traverse.execution.id` attributes on all child spans.
- **FR-004**: Every span MUST carry `traverse.capability.id` and `traverse.capability.version` attributes once a capability has been selected.
- **FR-005**: Span and log attribute keys MUST either conform to an OTel semantic convention or begin with the `traverse.` namespace prefix; no ad hoc or vendor-specific key names are permitted.
- **FR-006**: The runtime MUST set span status to `OK` on success and `ERROR` on any failure, and MUST attach the error classification as a structured span event.
- **FR-007**: The runtime MUST propagate W3C Trace Context (`traceparent` / `tracestate`) as a carrier field on outbound runtime request objects, event publication envelopes, and browser adapter subscription payloads.
- **FR-008**: The runtime MUST extract W3C Trace Context from inbound carriers (runtime request, event envelope, subscription) and resume the parent span when a valid `traceparent` is present.
- **FR-009**: When no inbound `traceparent` is present, the runtime MUST start a new root span rather than failing the request.
- **FR-010**: Span IDs and trace IDs MUST be generated using the OTel SDK default (random) in production mode.
- **FR-011**: The runtime MUST support a deterministic seeded ID mode, toggled by a config flag, that produces stable trace IDs and span IDs for identical input sequences; this mode MUST be restricted to test and CI contexts.
- **FR-012**: The runtime MUST export all three OTel signals (traces, logs, metrics) via OTLP over gRPC or HTTP, selectable via config.
- **FR-013**: The OTLP export endpoint, protocol (gRPC or HTTP), and TLS settings MUST be fully configurable without code changes.
- **FR-014**: The runtime MUST support a no-op exporter mode when no OTLP endpoint is configured; no-op mode MUST collect spans in-process and silently drop them.
- **FR-015**: OTLP export failures MUST NOT propagate as errors to the request caller; the export path is fire-and-forget with a configurable bounded retry budget.
- **FR-016**: Exhausted OTLP export retries MUST increment the `traverse.telemetry.export.dropped_total` metric counter.
- **FR-017**: The runtime MUST emit at least the following named metrics: `traverse.request.count` (counter), `traverse.request.duration_ms` (histogram), `traverse.request.error.count` (counter), `traverse.capability.execution.count` (counter), `traverse.capability.execution.duration_ms` (histogram).
- **FR-018**: Structured log records emitted during request processing MUST carry `trace_id` and `span_id` fields conforming to the OTel log data model when log export is enabled.
- **FR-019**: The runtime MUST never include JWT access tokens or raw credential material in any span attribute, log record, or metric label; only the derived `subject_id`, `actor_id`, and token reference hash are permitted.
- **FR-020**: Span attribute values exceeding the configured maximum length MUST be truncated and a span event MUST be recorded indicating truncation occurred.
- **FR-021**: The observability initialization MUST complete before the runtime accepts the first request; partial initialization MUST cause startup failure.
- **FR-022**: The runtime MUST expose the current OTel SDK configuration (endpoint, protocol, signal enablement) as a structured config dump accessible to operators without process restart.

## Non-Functional Requirements

- **NFR-001 Compliance Tiers**: Distributed traces are REQUIRED for v0 spec compliance; structured logs and metrics are STANDARDIZED and testable but optional for v0 compliance gate passage.
- **NFR-002 Determinism**: In seeded test mode, trace IDs and span IDs MUST be byte-identical across runs given the same seed and input sequence, enabling snapshot assertions.
- **NFR-003 Vendor Neutrality**: No vendor SDK, proprietary exporter, or third-party tracing library beyond the canonical OTel Rust SDK is permitted; OTLP is the only required export protocol.
- **NFR-004 Performance**: Span creation and attribute attachment MUST not add more than 1 ms of overhead per request under normal operating conditions; the OTel SDK sampling configuration MUST be respected.
- **NFR-005 Testability**: The telemetry layer MUST be injectable with a test exporter that captures spans and log records in-memory for assertion without a live OTLP collector.
- **NFR-006 Isolation**: Span context MUST be scoped to the request; context MUST NOT leak between concurrent requests regardless of async executor scheduling.
- **NFR-007 WASM Safety**: W3C Trace Context carrier injection and extraction for WASM boundaries MUST be defined at the WASM execution adapter level so that WASM modules do not need to depend on the OTel SDK directly.
- **NFR-008 Failure Tolerance**: The observability subsystem MUST tolerate OTLP endpoint unavailability, export timeout, and serialization errors without affecting the runtime request path.

## Non-Negotiable Quality Standards

- **QG-001**: Raw credential material (JWT, private keys, secrets) MUST NEVER appear in any exported span, log record, or metric label; this gate MUST be enforced by automated test.
- **QG-002**: Every runtime request MUST produce a root span and at least one child span; requests that produce no telemetry MUST fail the observability unit test suite.
- **QG-003**: Span attribute key naming MUST be validated by an automated lint rule that rejects any key not matching an OTel semantic convention or the `traverse.` prefix pattern.
- **QG-004**: The deterministic seeded ID mode MUST be explicitly gated to test/CI config and MUST NOT be activatable in a production configuration file.
- **QG-005**: OTLP export failures MUST NOT surface as request errors; any test that shows a dropped export causing a request error MUST block merge.
- **QG-006**: 100% automated line coverage MUST be maintained for span creation, context propagation, carrier injection/extraction, and seeded ID generation logic.

## Key Entities

- **OTel Root Span**: The top-level span created at request intake that parents all phase spans for one runtime execution attempt.
- **Phase Span**: A child span scoped to a single named execution phase (validation, discovery, selection, execution, etc.).
- **W3C Trace Context Carrier**: The structured field set (`traceparent`, `tracestate`) injected into or extracted from runtime request objects, event envelopes, and subscription payloads.
- **Seeded ID Mode**: A runtime configuration mode that replaces the OTel SDK's random ID generator with a deterministic seeded generator for CI and snapshot testing.
- **OTLP Exporter**: The pluggable export adapter that forwards OTel signals to a collector via gRPC or HTTP; no-op when unconfigured.
- **Traverse Metric**: A named OTel metric instrument (counter or histogram) under the `traverse.*` namespace covering request throughput, latency, and error rates.
- **Log Record**: An OTel-structured log entry emitted during request processing, carrying `trace_id` and `span_id` for correlation.
- **Token Reference Hash**: A non-secret stable hash of a JWT used for correlation in telemetry without exposing credential material.

## Success Criteria

- **SC-001**: A valid runtime request produces a complete, OTLP-exportable span tree with correct parent-child relationships and `traverse.*` attributes on all spans.
- **SC-002**: W3C Trace Context is correctly propagated across runtime request objects, event publication envelopes, and browser adapter subscriptions, verified by round-trip extraction tests.
- **SC-003**: OTLP export failures are absorbed silently by the runtime; no request returns an error due to a telemetry export failure.
- **SC-004**: The deterministic seeded ID mode produces byte-identical trace IDs and span IDs across test runs for the same seed, confirmed by snapshot assertions.
- **SC-005**: No span, log record, or metric label contains raw credential material; automated tests enforce this gate.
- **SC-006**: All required metrics (`traverse.request.count`, `traverse.request.duration_ms`, `traverse.request.error.count`, `traverse.capability.execution.count`, `traverse.capability.execution.duration_ms`) are emitted and captured by the in-memory test exporter.

## Out of Scope

- Visualization UI or tracing dashboards
- Proprietary APM agent integration (Datadog, New Relic, Dynatrace)
- Sampling strategy configuration (deferred to a dedicated sampling spec)
- Log aggregation pipeline design (only the OTel log record format is governed here)
- Alerting or SLO definitions
- Span-based billing or cost attribution
- Multi-tenant trace isolation (governed by spec 030)
