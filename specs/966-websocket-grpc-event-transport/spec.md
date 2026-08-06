# Feature Specification: WebSocket and gRPC Event Transport

**Status**: Approved
**Canonical governing ID**: `097-websocket-grpc-event-transport`
**Extends**: `013-browser-runtime-subscription`, `207-event-broker`, `534-ecca-event-products`
**Input**: Issue #966; ADR-0034; `/brainstorm` session recorded as Decision 47 in `docs/decision-log.md`.

## Purpose

Define the governed WebSocket and gRPC interfaces through which an external
client — browser, mobile, or another runtime instance — receives events
published to `EventBroker`, replacing the SSE endpoint governed by
`096-runtime-event-sse-transport` (issue #964) once implemented. Both
transports read from the same `TraverseEvent` source and carry the same
CloudEvents-shaped envelope; neither is a lower-priority fallback for the
other. This spec also defines how `013-browser-runtime-subscription`'s
ordered browser message contract (`subscription_established → state →
trace → terminal_result → stream_completed`) maps onto the WebSocket
transport, since `013` explicitly excludes transport protocols from its own
scope.

## Capability Boundary

This spec governs the wire interface and connection/session lifecycle for
both transports. It does not govern `EventBroker`'s internal delivery,
durability, or catalog semantics (those stay governed by `207`), does not
re-litigate whether SSE should remain a fallback (ADR-0034 already decided
no), and does not include the actual implementation (tracked separately as
issues #967 WebSocket and #968 gRPC).

## Requirements

- **FR-001**: A WebSocket connection MUST require the same authorization
  scope semantics as the SSE endpoint it replaces (equivalent to
  `SCOPE_RUNTIME_EVENTS_READ`), asserted during the connection handshake,
  not per-message.
- **FR-002**: Once authorized, a client MUST be able to subscribe to one or
  more event types (or the browser-subscription ordered stream for one
  `request_id`/`execution_id`, per `013`) over the same connection, mapping
  onto `EventBroker::subscribe`/`subscribe_for_subject`.
- **FR-003**: Every message delivered over WebSocket MUST carry the full
  `TraverseEvent` CloudEvents envelope plus governance metadata, matching
  `096`'s FR-002 requirement for the SSE transport it replaces.
- **FR-004**: When a WebSocket connection is used for the browser
  subscription contract (`013`), message ordering MUST preserve `013`'s
  existing guarantees: `subscription_established` first, all governed
  runtime state events in order, then the terminal result, then
  `stream_completed` last.
- **FR-005**: The gRPC service MUST define `.proto` message types whose
  fields are a 1:1 mapping of `TraverseEvent`'s CloudEvents fields
  (`specversion`, `id`, `source`, `type`, `time`, `data`, and the governance
  metadata envelope), matching UMA's reference `EventService` shape
  (`SendEvent`, `StreamEvents`) as a starting point, not a binding
  requirement to match it exactly.
- **FR-006**: gRPC connections MUST use TLS for transport encryption; token-
  based client authentication (matching the WebSocket authorization model in
  FR-001) MUST be enforced before any event is streamed.
- **FR-007**: Once WebSocket ships, the SSE endpoint governed by `096` MUST
  be removed, not kept running in parallel (ADR-0034).
- **FR-008**: A connection or stream failure (auth rejection, malformed
  subscription request, broker unavailability) MUST return a structured,
  machine-readable error before any partial event stream begins, matching
  `013`'s existing `invalid_request`/`not_found` error-message pattern
  rather than a raw protocol-level error.

## Acceptance Scenarios

1. Given an authorized WebSocket client subscribes to an event type, when
   `EventBroker` publishes a matching event, then the client receives the
   full `TraverseEvent` envelope over the open connection.
2. Given a WebSocket client opens a browser-subscription stream for one
   `execution_id`, when the stream completes, then the message sequence is
   `subscription_established`, governed state events in order,
   `terminal_result`, `stream_completed` — matching `013`'s ordering
   guarantees.
3. Given a gRPC client calls `StreamEvents`, when events are published,
   then each streamed message deserializes into the same logical
   `TraverseEvent` fields a WebSocket client would receive for the same
   event.
4. Given an unauthorized connection attempt on either transport, when the
   handshake completes, then the connection is rejected with a structured
   error before any event data is sent.
5. Given WebSocket has shipped, when `crates/traverse-cli/src/http_api.rs`
   is inspected, then the SSE app-events endpoint from `096` no longer
   exists.

## Out of Scope

- Re-deciding whether SSE should remain a fallback (settled by ADR-0034).
- `EventBroker`'s internal delivery/durability semantics (governed by
  `207`).
- `browser_adapter.rs`'s disposition (tracked separately as issue #973,
  blocked on this spec's implementation landing).
- Client SDK implementations for any specific platform (iOS, Android, web
  framework) — this spec governs the server-side interface only.
