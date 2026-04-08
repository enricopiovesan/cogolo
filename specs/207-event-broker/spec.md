# Feature Specification: In-Process Event Broker with ECCA-Aligned Catalog

**Feature Branch**: `207-event-broker`
**Spec ID**: 026
**Created**: 2026-04-08
**Status**: Draft
**Input**: GitHub issue #207

> **Governance note**: Governed by spec `003-event-contracts` (event contract format — defines the contract schema that every published event must conform to). Extends spec `006-runtime-request-execution` (execution path that produces events). Depends on issue #208 for the `subscribable` service_type on capabilities. The EventBroker and EventCatalog live in `traverse-runtime`; the MCP surface is exposed via `traverse-mcp`. ECCA principle — events as products — mandates that ownership metadata (owner, version, lifecycle_status) is non-negotiable on every published event. No raw event data is stored in the catalog; only schema reference + governance metadata.

## Context

Traverse v0.2.0 introduces subscribable capabilities (service_type: `subscribable`, from spec 014 / issue #208). These capabilities emit typed events that other capabilities or agents can subscribe to. This spec governs the in-process runtime infrastructure that makes that possible:

- **EventBroker**: the runtime component that routes published events to registered subscribers, synchronously and in-process. No external queue, no network hop in v0.2.0.
- **EventCatalog**: a registry of known event types, each carrying the governance metadata required by ECCA Level 3 — owner (capability_id), version, lifecycle_status, and a consumer list. The catalog makes event types discoverable without exposing raw event payloads.
- **CloudEvents schema**: every event published through the broker carries the standard CloudEvents fields (`source`, `type`, `id`, `time`, `datacontenttype`, `data`) plus a governance metadata envelope (`owner`, `version`, `lifecycle_status`).
- **MCP surface**: two tools — `list_event_types` (catalog query) and `subscribe` (agent subscription) — make the broker accessible to MCP clients at ECCA Level 3.

Spec 003 (`003-event-contracts`) defines the format of event contracts on disk. This spec governs the runtime broker that enforces those contracts at publish time and routes events to subscribers.

## User Scenarios & Testing

### User Story 1 — Subscribable capability publishes an event; subscribers receive it synchronously (Priority: P1)

As a capability author, I want to publish a CloudEvents event through the EventBroker so that all registered subscribers receive it synchronously in the same process, with all CloudEvents fields and governance metadata populated.

**Why this priority**: This is the foundational primitive. Without it no downstream capability or agent can react to events produced by subscribable capabilities.

**Independent Test**: Register a subscribable capability's event type in the EventCatalog. Register a subscriber handler for that event type via the EventBroker. Publish a `TraverseEvent` of that type. Assert the subscriber's handler was invoked exactly once with the full event, including all CloudEvents fields and the governance metadata envelope.

**Acceptance Scenarios**:

1. **Given** an event type is registered in the EventCatalog with lifecycle_status `active`, **When** a `TraverseEvent` of that type is published via `EventBroker::publish`, **Then** every registered subscriber handler is called synchronously with the full event before `publish` returns.
2. **Given** a `TraverseEvent` received by a subscriber, **When** its fields are inspected, **Then** `source`, `type`, `id`, `time`, `datacontenttype`, and `data` are all present and non-empty, and the governance metadata envelope carries non-empty `owner`, `version`, and `lifecycle_status` values.
3. **Given** two subscribers registered for the same event type, **When** an event of that type is published, **Then** both subscribers receive the event; neither receives events of a different type.

---

### User Story 2 — Event contract enforces owner, version, and lifecycle_status on every published event (Priority: P1)

As a platform operator, I want every published event to carry governance metadata derived from the event contract so that I can identify who owns each event type, what version it is, and whether it is still active — without inspecting raw event payloads.

**Why this priority**: ECCA treats events as products. Ownership metadata is the mechanism by which accountability is enforced at runtime, not just at contract-definition time.

**Independent Test**: Register an event type in the EventCatalog with a specific owner (capability_id), version, and lifecycle_status `active`. Publish an event of that type. Assert the received event's governance metadata matches the catalog entry exactly — owner, version, lifecycle_status — regardless of what the publisher passed in those fields.

**Acceptance Scenarios**:

1. **Given** an event contract specifying `owner = "capability-a"`, `version = "1.0.0"`, `lifecycle_status = active`, **When** a `TraverseEvent` of the corresponding type is published, **Then** the event's governance metadata carries `owner = "capability-a"`, `version = "1.0.0"`, `lifecycle_status = active` as populated by the broker from the catalog — not trusting the caller's supplied values.
2. **Given** a `TraverseEvent` with a missing or empty `owner` field in the governance metadata supplied by the caller, **When** `publish` is called, **Then** the broker fills `owner` from the EventCatalog entry rather than rejecting the event.
3. **Given** an event type with no matching catalog entry, **When** `publish` is called for that type, **Then** `EventBroker::publish` returns `Err(EventError::UnregisteredEventType)`.

---

### User Story 3 — MCP client calls list_event_types and receives the event catalog (Priority: P2)

As an MCP client (agent or developer tool), I want to call `list_event_types` on the MCP surface so that I can discover which event types are registered, who owns them, what version they are at, their lifecycle status, and how many consumers are subscribed.

**Why this priority**: ECCA Level 3 requires that event types are discoverable by agents through the MCP surface. Without this tool, the catalog is invisible to any consumer outside the runtime process.

**Independent Test**: Register three event types in the EventCatalog (one deprecated, two active). Register one subscriber for one of the active types. Call the `list_event_types` MCP tool. Assert the response lists all three entries, each with `event_type`, `owner`, `version`, `lifecycle_status`, and `consumer_count` fields. Assert the entry with a subscriber shows `consumer_count = 1`.

**Acceptance Scenarios**:

1. **Given** the EventCatalog contains registered event types, **When** `list_event_types` is called, **Then** the response is a JSON array of `EventCatalogEntry` objects, each containing `event_type`, `owner`, `version`, `lifecycle_status`, and `consumer_count`.
2. **Given** a subscriber is registered for event type X, **When** `list_event_types` is called, **Then** the entry for event type X shows `consumer_count` equal to the number of registered subscribers.
3. **Given** `list_event_types` is called on an empty catalog, **When** the response is received, **Then** an empty array is returned with no error.

---

### User Story 4 — A deprecated event type is rejected at publish time (Priority: P3)

As a platform operator, I want the EventBroker to refuse to publish events of deprecated types so that capabilities cannot silently continue emitting events that have been marked for retirement.

**Why this priority**: Lifecycle enforcement at the broker level makes deprecation a runtime contract, not just a documentation note. This prevents stale producers from continuing to emit events after a type has been deprecated.

**Independent Test**: Register an event type in the EventCatalog with `lifecycle_status = deprecated`. Attempt to publish a `TraverseEvent` of that type. Assert `publish` returns `Err(EventError::LifecycleViolation)` and no subscriber is invoked.

**Acceptance Scenarios**:

1. **Given** an event type with `lifecycle_status = deprecated` in the EventCatalog, **When** `EventBroker::publish` is called for that type, **Then** the result is `Err(EventError::LifecycleViolation)` and no subscriber handler is invoked.
2. **Given** an event type with `lifecycle_status = draft` in the EventCatalog, **When** `EventBroker::publish` is called for that type, **Then** the result is `Err(EventError::LifecycleViolation)` and no subscriber handler is invoked.
3. **Given** an event type transitions from `deprecated` back to `active` in the catalog (re-registered), **When** `EventBroker::publish` is called for that type, **Then** the event is delivered successfully to subscribers.

---

## Requirements

- **FR-001**: Every `TraverseEvent` MUST carry the full CloudEvents mandatory fields: `source` (String), `type` (String), `id` (String, UUID v4), `time` (RFC 3339 String), `datacontenttype` (String), and `data` (JSON value). These fields MUST be non-empty on every event that passes through the broker.
- **FR-002**: Every `TraverseEvent` MUST carry a governance metadata envelope (`EventMetadata`) with non-empty `owner` (capability_id String), `version` (semver String), and `lifecycle_status` (`LifecycleStatus` enum: `Draft`, `Active`, `Deprecated`). The broker MUST populate these fields from the EventCatalog at publish time, overriding any caller-supplied values.
- **FR-003**: `EventBroker` MUST be a trait with at least three methods: `publish(event: TraverseEvent) -> Result<(), EventError>`, `subscribe(event_type: &str, handler: SubscriberFn) -> SubscriptionId`, and `unsubscribe(id: SubscriptionId) -> Result<(), EventError>`.
- **FR-004**: The in-process implementation (`InProcessBroker`) MUST deliver events synchronously to all subscribers of the matching event type before `publish` returns. Delivery order among subscribers of the same event type MUST be deterministic (insertion order).
- **FR-005**: `EventCatalog` MUST support registering event types with `register(entry: EventCatalogEntry) -> Result<(), EventError>`, listing all entries with `list() -> Vec<&EventCatalogEntry>`, and querying a single entry with `get(event_type: &str) -> Option<&EventCatalogEntry>`. Each `EventCatalogEntry` MUST carry `event_type` (String), `owner` (String), `version` (String), `lifecycle_status` (LifecycleStatus), and `consumer_count` (usize). No raw event data or payload schema is stored in the catalog.
- **FR-006**: MCP tool `list_event_types` MUST return the full contents of the EventCatalog as a JSON array of `EventCatalogEntry` objects. The response MUST include `event_type`, `owner`, `version`, `lifecycle_status`, and `consumer_count` for each registered event type.
- **FR-007**: MCP tool `subscribe` MUST allow an MCP client (e.g., an agent) to register interest in an event type by name. The subscription MUST increment `consumer_count` on the corresponding `EventCatalogEntry`. The tool MUST return a `subscription_id` the caller can use to unsubscribe.
- **FR-008**: `EventBroker::publish` MUST return `Err(EventError::LifecycleViolation)` when the event type's `lifecycle_status` in the EventCatalog is `Deprecated` or `Draft`. No subscriber MUST be invoked in this case.
- **FR-009**: `EventBroker::publish` MUST return `Err(EventError::UnregisteredEventType)` when the event type has no corresponding entry in the EventCatalog.
- **FR-010**: No raw event `data` payload MUST be stored in the EventCatalog. The catalog holds only schema reference + governance metadata.

## Success Criteria

- **SC-001**: `cargo test` passes with no panics, `unwrap()` calls, or TODOs across the events module and MCP tools.
- **SC-002**: Tests assert all CloudEvents fields (`source`, `type`, `id`, `time`, `datacontenttype`, `data`) are non-empty on every `TraverseEvent` delivered to a subscriber.
- **SC-003**: Tests assert lifecycle enforcement: publishing a `Deprecated` or `Draft` event type returns `Err(EventError::LifecycleViolation)` and no subscriber is invoked.
- **SC-004**: Tests assert the EventCatalog is queryable via the `list_event_types` MCP tool and returns correct `consumer_count` values after subscriptions are registered.
- **SC-005**: Tests assert that `EventBroker::publish` returns `Err(EventError::UnregisteredEventType)` for event types with no catalog entry.
- **SC-006**: No raw event data appears in any `EventCatalogEntry` field at any point during any test.

## Assumptions

- The `subscribable` service_type on capabilities is defined in spec 014 / issue #208. This spec assumes that type is available or will be available before this feature is merged; the EventBroker does not enforce service_type directly — it operates on event type strings.
- In v0.2.0 the broker is single-threaded or protected by a `Mutex`; async delivery and backpressure are out of scope.
- `SubscriberFn` is a `Box<dyn Fn(&TraverseEvent) + Send + Sync>` or equivalent; the exact signature is finalized in the implementation phase.
- CloudEvents `source` is set to `traverse-runtime/<capability_id>` by the publishing capability; the broker does not override `source`.
- The `uuid` crate (v4 feature) and `chrono` (or equivalent RFC 3339 source) are already present in the workspace or added in this spec's implementation phase.
- MCP tool `subscribe` registers a catalog-level consumer count increment for v0.2.0; actual agent callback delivery over MCP is out of scope for this slice.
