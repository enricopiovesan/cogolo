# Feature Specification: WebSocket and gRPC Event Transport

**Status**: Approved
**Canonical governing ID**: `097-websocket-grpc-event-transport`
**Version**: 1.1.0
**Extends**: `013-browser-runtime-subscription`, `207-event-broker`, `534-ecca-event-products`
**Input**: Issue #966; ADR-0034; `/brainstorm` session recorded as Decision 47 in `docs/decision-log.md`.

**Amendment (2026-08-06, v1.0.0 -> v1.1.0)**: A pre-implementation happy/unhappy-path
audit found three undefined failure modes: FR-008 only covered failures *before* a
stream starts, leaving mid-stream broker failure undefined; there was no reconnect/
resume story despite real WebSocket connections dropping routinely; and there was no
bound on malformed or oversized client-sent messages. FR-009, FR-010, and FR-011 below
close these. No existing FR text changed. Approved by the repo owner the same day this
gap was raised.

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
- **FR-009** (v1.1.0): If `EventBroker` becomes unavailable *after* a
  WebSocket or gRPC stream has already started delivering events, the
  connection MUST be closed with a structured close frame (WebSocket) or
  trailer/status (gRPC) identifying the failure — never a silent hang or a
  bare TCP reset the client cannot distinguish from "no new events."
- **FR-010** (v1.1.0): A WebSocket client reconnecting after a dropped
  connection MUST be able to resume its subscription by supplying the
  last-seen `EventCursor`, resolved the same way `096`'s FR-003 resolves
  `Last-Event-ID` for SSE (including `096`'s FR-009 malformed/expired-cursor
  behavior, `400`/`410`, applied at the WebSocket handshake instead of an
  HTTP request).
- **FR-011** (v1.1.0): The WebSocket server MUST enforce a maximum incoming
  message size and MUST reject a malformed frame with a structured close
  code rather than crashing the connection handler or silently ignoring it
  — consistent with this codebase's existing bounded-input discipline
  (`MAX_REQUEST_HEADER_BYTES` in `crates/traverse-cli/src/http_api.rs` and
  `MAX_WS_INBOUND_MESSAGE_BYTES` in
  `crates/traverse-cli/src/app_events_websocket.rs`).

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
6. Given an open WebSocket stream, when `EventBroker` fails internally mid-
   stream, then the connection is closed with a structured close frame
   identifying the failure, not a silent hang.
7. Given a WebSocket client reconnects with a last-seen `EventCursor` after
   an unplanned disconnect, when the connection re-establishes, then
   delivery resumes from that cursor without gaps or duplicates.
8. Given a client sends an oversized or malformed WebSocket frame, when the
   server receives it, then the connection is rejected with a structured
   close code rather than crashing the handler.

## Out of Scope

- Re-deciding whether SSE should remain a fallback (settled by ADR-0034).
- `EventBroker`'s internal delivery/durability semantics (governed by
  `207`).
- `browser_adapter.rs`'s disposition — resolved separately by issue #973
  (Decision 53): retired now that this spec's `browser_subscription` mode
  serves spec `013`'s contract in production.
- Client SDK implementations for any specific platform (iOS, Android, web
  framework) — this spec governs the server-side interface only.
