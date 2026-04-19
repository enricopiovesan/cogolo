# Feature Specification: Event Subscription and Replay

**Feature Branch**: `036-event-subscription-replay`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Event delivery model for Traverse covering at-least-once subscription, cursor-based replay, bounded buffer retention, backpressure handling, and browser adapter compatibility. Unblocks GitHub issues #308 and #312.

## Purpose

This spec defines the subscription and replay model for Traverse event delivery.

The existing event registry (spec 003) defines event schemas and emitter contracts. This spec defines how consumers subscribe to event streams, how they recover from disconnections using cursors, and how the broker handles bounded retention and backpressure.

The model introduced here:

- delivers events to subscribers with at-least-once semantics and stable per-subscription ordering
- assigns opaque server-side cursors so late-joining or reconnecting subscribers can replay missed events
- bounds the broker's event buffer by a configurable `retention_window`; events outside the window are not guaranteed replayable
- requires callers to handle duplicates by providing stable `event_id` values on all emitted events
- supports both in-process Rust consumers and browser-hosted consumers via the existing browser adapter
- explicitly defers distributed cross-process delivery to a future spec

This spec does **not** define workflow orchestration triggers, fan-out routing topology, or persistent event log storage beyond the retention window.

## User Scenarios and Testing

### User Story 1 — Reconnecting Subscriber Replays Missed Events (Priority: P1)

As a platform developer, I want a subscriber that disconnects and reconnects to receive all events it missed during the disconnection interval so that transient network or process interruptions do not cause silent event loss.

**Why this priority**: Replay on reconnect is the foundational recovery property. Without it, subscribers must treat every disconnection as a potential data loss event.

**Independent Test**: Subscribe to `CapabilityExecuted` events. Record the cursor after receiving the first batch. Disconnect. Emit three more events. Reconnect with the recorded cursor. Verify the subscriber receives all three missed events in emission order, with no duplicates if event_ids are unique.

**Acceptance Scenarios**:

1. **Given** a subscriber that recorded a cursor `C` before disconnecting, **When** it reconnects and provides `C` as `from_cursor`, **Then** the broker delivers all events emitted after `C` in the order they were emitted.
2. **Given** a subscriber replaying from cursor `C`, **When** the replay includes events the subscriber already processed before disconnect, **Then** the subscriber receives those events again (at-least-once) and is responsible for idempotent handling using `event_id`.
3. **Given** a cursor that references a point within the active retention window, **When** the subscriber reconnects, **Then** the broker begins delivery from that cursor without error.

### User Story 2 — Two Independent Subscribers Receive Ordered Copies (Priority: P1)

As a platform developer, I want two agents subscribing to the same event type to each receive an independent, ordered copy of all events so that event consumption by one agent does not affect the other.

**Why this priority**: Independent delivery is required for fan-out use cases where multiple agents react to the same runtime events.

**Independent Test**: Create two subscriptions to the same event type. Emit five events. Verify each subscription receives all five events in emission order independently.

**Acceptance Scenarios**:

1. **Given** two active subscriptions to the same event type, **When** an event is emitted, **Then** both subscriptions receive the event in their respective ordered streams.
2. **Given** subscriber A acknowledges and advances past an event, **When** subscriber B has not yet received that event, **Then** subscriber B still receives the event without loss.
3. **Given** subscriber A is slow and has a large undelivered backlog, **When** subscriber B is fast, **Then** subscriber B's delivery is not blocked or slowed by subscriber A's state.

### User Story 3 — Browser App Late-Join Replay (Priority: P2)

As a browser application developer, I want to connect after a capability has executed and replay from cursor 0 so that the browser UI can reconstruct state from the beginning of the retention window.

**Why this priority**: Browser adapters are a first-class consumer of Traverse events. Late-join replay is essential for UI state reconstruction.

**Independent Test**: Emit a `CapabilityExecuted` event. Connect a browser subscriber using `from_cursor: "0"`. Verify the subscriber receives the event despite connecting after emission.

**Acceptance Scenarios**:

1. **Given** a browser subscriber that connects with `from_cursor: "0"`, **When** events exist within the retention window, **Then** the subscriber receives those events from the beginning of the available buffer.
2. **Given** a browser subscriber connected via the existing browser adapter, **When** the broker delivers events, **Then** the delivery mechanism is compatible with the browser adapter's message framing.
3. **Given** a browser subscriber replaying from cursor 0 with no events in the buffer, **When** the broker evaluates the subscription, **Then** it returns an empty stream (not an error) and the subscription remains active for future events.

### User Story 4 — Operator Configures Retention Window (Priority: P2)

As a platform operator, I want to configure the broker's retention window to a value appropriate for my compliance requirements so that events are replayable for the required duration.

**Why this priority**: The default retention window of 5 minutes is insufficient for compliance use cases that require longer replay windows.

**Independent Test**: Configure `retention_window` to 30 minutes. Emit an event. Wait 6 minutes. Connect a new subscriber with `from_cursor: "0"`. Verify the event is still replayable.

**Acceptance Scenarios**:

1. **Given** a broker configured with `retention_window: 30m`, **When** an event was emitted 20 minutes ago, **Then** a new subscriber can replay that event using a cursor within the retention window.
2. **Given** a broker configured with `retention_window: 30m`, **When** an event was emitted 35 minutes ago, **Then** a subscriber requesting replay from a cursor referencing that event receives a `cursor_expired` error.
3. **Given** a broker with `retention_window` modified at runtime, **When** the new window is shorter than the previous one, **Then** events outside the new window are immediately ineligible for replay.

## Edge Cases

- Cursor from a buffer window that has expired — return a `cursor_expired` error with the expiry timestamp; MUST NOT deliver a silent empty stream as if no events had been emitted.
- Subscriber that disconnects with an unbounded backlog — the broker drops oldest un-delivered events from that subscription's queue; on reconnect the subscriber receives a `backlog_dropped` advisory followed by the oldest available event.
- Two events emitted with identical `event_id` values — the second event MUST be discarded by the broker; MUST NOT deliver a duplicate to any subscriber.
- Subscription created for an event type with no registered emitter — valid operation; the broker creates an empty active subscription and delivers events if an emitter registers later.
- A subscriber advancing a cursor that it has already advanced past — idempotent; the broker treats cursor advancement as monotonic and ignores non-advancing cursor updates.
- Retention window set to zero or a negative duration — reject at configuration validation with `invalid_retention_window`; do not silently default.
- Broker receives an event with a missing or empty `event_id` — reject the emission with `event_id_required`; do not assign a broker-generated id silently.
- A subscription's delivery queue reaches capacity before any events are dropped — the broker MUST emit a `subscription_backpressure` warning event to the operator audit channel before dropping.

## Functional Requirements

- **FR-001**: The broker MUST support named subscriptions; each subscription is identified by a `subscription_id` and targets one event type.
- **FR-002**: The broker MUST deliver events to all active subscriptions for a given event type; delivery to one subscription MUST NOT depend on the delivery state of any other subscription.
- **FR-003**: Events delivered to a given subscription MUST be ordered by emission time; the broker MUST not reorder events within a subscription's delivery queue.
- **FR-004**: The broker MUST assign an opaque server-side cursor to each event at emission time; cursors MUST be stable and monotonically increasing within the broker's buffer.
- **FR-005**: A subscription request MAY include a `from_cursor` field; when present the broker MUST deliver all events emitted after that cursor in order before delivering new events.
- **FR-006**: When `from_cursor` is absent, the broker MUST deliver only events emitted after the subscription was created; no historical replay occurs.
- **FR-007**: Every emitted event MUST carry a stable `event_id`; the broker MUST reject emission requests that omit or provide an empty `event_id`.
- **FR-008**: The broker MUST detect duplicate `event_id` values within the retention window; a duplicate event MUST be silently discarded and MUST NOT be delivered to any subscriber.
- **FR-009**: The broker MUST enforce an at-least-once delivery guarantee; it MUST NOT drop events from active subscriptions with available queue capacity.
- **FR-010**: The broker MUST bound each subscription's delivery queue; when a subscription's queue is full, the broker MUST drop the oldest un-delivered events and MUST record the drop count.
- **FR-011**: On reconnect with a cursor that falls within a dropped range, the broker MUST deliver a `backlog_dropped` advisory to the subscriber before resuming delivery from the oldest available event.
- **FR-012**: The broker MUST enforce a configurable `retention_window` (minimum: 1 minute, default: 5 minutes, no maximum enforced by spec); events older than the window are ineligible for replay.
- **FR-013**: When a subscriber provides a `from_cursor` that references a point outside the retention window, the broker MUST return a `cursor_expired` error and MUST NOT deliver a silent empty stream.
- **FR-014**: The cursor value `"0"` MUST be treated as a request to replay from the beginning of the available retention window.
- **FR-015**: Subscriptions MUST be independent of workspace_id isolation per spec 035; a subscriber MUST only receive events emitted within its authorized workspace.
- **FR-016**: The broker MUST support in-process Rust consumers and browser-hosted consumers via the existing browser adapter without requiring distinct subscription APIs.
- **FR-017**: Subscription creation for an event type with no registered emitter MUST succeed and return an empty active subscription; this is not an error condition.
- **FR-018**: The broker MUST emit a `subscription_backpressure` warning to the operator audit channel when a subscription's queue reaches 80% of capacity.
- **FR-019**: The broker MUST emit a `subscription_backlog_dropped` audit event recording the subscription id, drop count, oldest available cursor, and timestamp whenever events are dropped.
- **FR-020**: The `retention_window` MUST be validatable at configuration load time; a zero or negative value MUST produce an `invalid_retention_window` error and MUST prevent broker startup.
- **FR-021**: The broker MUST support subscription cancellation; cancelled subscriptions MUST be removed from the delivery set and their queues freed.
- **FR-022**: The broker MUST preserve cursor semantics across broker restarts within the retention window; cursors issued before a restart MUST remain valid if the referenced events are within the window.

## Non-Functional Requirements

- **NFR-001 Ordering**: Event ordering within a subscription MUST be deterministic and based on emission order; no subscriber-visible reordering is permitted.
- **NFR-002 Idempotency**: All emitted events MUST carry stable `event_id` values; the broker MUST enforce this at the emission boundary so consumers can safely deduplicate.
- **NFR-003 Bounded Memory**: The broker MUST not accumulate unbounded state; both the retention buffer and per-subscription queues MUST be bounded by configuration.
- **NFR-004 Testability**: Subscription delivery, cursor replay, duplicate suppression, backpressure handling, and retention expiry MUST each be independently testable without a running network stack.
- **NFR-005 Browser Compatibility**: The subscription framing and cursor protocol MUST be compatible with the existing browser adapter without requiring adapter changes in this spec.
- **NFR-006 Observability**: Backpressure events, dropped events, cursor expiry, and subscription lifecycle changes MUST each produce structured audit events suitable for operator monitoring.
- **NFR-007 Determinism**: For the same broker state and sequence of emissions, subscription delivery order and cursor assignments MUST be deterministic and reproducible in tests.

## Non-Negotiable Quality Standards

- **QG-001**: A `cursor_expired` error MUST be returned when the caller references a cursor outside the retention window; a silent empty stream is a blocker defect.
- **QG-002**: Events with duplicate `event_id` values MUST be discarded and MUST NOT be delivered to any subscriber; duplicate delivery via the broker is a blocker defect.
- **QG-003**: Backpressure-induced event drops MUST be recorded in the structured audit log and MUST produce a `backlog_dropped` advisory to the subscriber on reconnect; silent drops are a blocker defect.
- **QG-004**: 100% automated line coverage is required for subscription creation, cursor replay, duplicate suppression, backpressure handling, and retention expiry logic.
- **QG-005**: Event subscription and replay behavior MUST align with this governing spec and fail the spec-alignment CI gate when drift occurs.

## Key Entities

- **Subscription**: A named, subject-scoped registration of interest in a specific event type. Identified by `subscription_id`. Maintains an ordered delivery queue and a current delivery cursor.
- **subscription_id**: The stable identifier for a subscription, server-assigned at creation time.
- **Event Cursor**: An opaque, monotonically increasing server-assigned string attached to each event at emission time. Used by subscribers to identify their position in the event stream.
- **from_cursor**: The cursor value provided by a subscriber on connect or reconnect to request replay from a specific position. The value `"0"` requests replay from the start of the retention window.
- **event_id**: A stable, caller-supplied identifier for an emitted event. Used by the broker to detect and discard duplicate emissions. Required on all emitted events.
- **Retention Window**: The configurable duration for which the broker guarantees event replayability. Default: 5 minutes. Events older than the window may not be replayable.
- **Delivery Queue**: The per-subscription bounded buffer holding events awaiting delivery. When full, the broker drops the oldest un-delivered events and records the drop.
- **Backlog Dropped Advisory**: A structured advisory delivered to a subscriber on reconnect when events were dropped from its queue due to backpressure during disconnection.

## Success Criteria

- **SC-001**: A subscriber that reconnects with a valid cursor receives all missed events in emission order with no broker-induced gaps.
- **SC-002**: Two independent subscribers to the same event type each receive a complete, ordered, independent copy of all emitted events.
- **SC-003**: A `cursor_expired` error is returned for any cursor referencing a point outside the active retention window; no silent empty stream is ever returned in its place.
- **SC-004**: Duplicate `event_id` events are silently discarded at the broker and never delivered to any subscriber.
- **SC-005**: Backpressure-induced drops produce both an operator audit event and a subscriber advisory; no drop is silent.
- **SC-006**: 100% automated line coverage is achieved for all subscription, cursor, duplicate suppression, and backpressure paths.

## Out of Scope

- Distributed cross-process event delivery (multi-node brokers, Kafka-style fan-out)
- Persistent event log storage beyond the in-memory retention window
- Workflow orchestration triggers driven by event subscriptions
- Event schema versioning and migration
- Per-subscription replay authorization beyond workspace isolation (spec 035)
- Event fan-out routing topology configuration
- Dead-letter queues for persistently failing subscribers
