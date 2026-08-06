# Feature Specification: Production SSE Transport Backed by EventBroker

**Status**: Approved
**Canonical governing ID**: `096-runtime-event-sse-transport`
**Version**: 1.1.0
**Extends**: `207-event-broker`, `534-ecca-event-products`
**Input**: Issue #964; `/brainstorm` session recorded as Decision 45 in `docs/decision-log.md`.

**Amendment (2026-08-06, v1.0.0 -> v1.1.0)**: A pre-implementation happy/unhappy-path
audit found this spec left two failure modes undefined — a malformed or expired
`Last-Event-ID`, and an internal `EventBroker` error during subscribe/poll — even
though `EventBroker` already carries typed errors for both (`EventError::InvalidCursor`,
`EventError::CursorExpired`). Leaving these undefined would have left the implementer
to invent the HTTP response shape ad hoc. FR-009 and FR-010 below close both gaps.
No existing FR text changed. Approved by the repo owner the same day this gap was
raised, no separate `/brainstorm` needed since the fix pattern (typed error ->
structured HTTP response) is already established practice in this codebase, not a new
architecture decision.

## Purpose

Make the production HTTP API's app-events SSE endpoint
(`/v1/workspaces/{workspace_id}/apps/{app_id}/events`, `handle_app_events` in
`crates/traverse-cli/src/http_api.rs`) a governed consumer of `EventBroker`,
replacing the ungoverned `AppStateEventRecord` type it streams today.
`AppStateEventRecord` carries no relationship to `TraverseEvent`, event
contracts, or `EventBroker` lifecycle/ownership metadata — it is the one
production-facing surface in Traverse that emits events with no governance
attached to them at all. This spec closes that gap without picking a wire
protocol (transport topology stays out of scope, per `534`'s own boundary);
it governs what the existing SSE endpoint reads from, not a new endpoint or a
new protocol.

## Capability Boundary

This endpoint is a read-only, workspace-scoped view over events already
published to `EventBroker` for capabilities executing in that workspace. It
does not publish events, does not expose the broker's full catalog across
workspace boundaries, and does not introduce delivery guarantees beyond what
`EventBroker`/`DurableBroker` already provide. It is not the browser
subscription surface governed by `013-browser-runtime-subscription` (that
surface's ordered message contract — `subscription_established → state →
trace → terminal_result → stream_completed` — is out of scope here; see
issue #973) and it does not select or introduce a new transport (WebSocket
and gRPC are governed separately by `097-websocket-grpc-event-transport`,
issue #966).

## Requirements

- **FR-001**: `handle_app_events` MUST source its SSE stream from
  `EventBroker`/`TraverseEvent` (via `Subscription`/`SubscriptionPoll`/
  `EventCursor`), not from `AppStateEventRecord`. `AppStateEventRecord` and
  its population call sites MUST be removed once this migration lands — it
  is not kept as a parallel or dual-emit path (per Decision 48, no
  back-compat tax while there are no production consumers).
- **FR-002**: Each SSE `data:` payload MUST serialize the full `TraverseEvent`
  CloudEvents envelope (`id`, `source`, `event_type`, `datacontenttype`,
  `time`, `data`) plus its governance metadata (`owner`, `version`,
  `lifecycle_status`), not a reshaped or partial projection.
- **FR-003**: The endpoint MUST continue to support replay via the
  `Last-Event-ID` request header, resolved against `EventBroker`'s existing
  cursor semantics (`EventCursor`, `subscribe`/`subscribe_for_subject`,
  `poll`) rather than a bespoke replay mechanism.
- **FR-004**: The endpoint MUST remain workspace- and app-scoped: a
  subscriber for `{workspace_id}/{app_id}` MUST only receive events whose
  `source` and/or `subject_id` resolve to that workspace/app pairing.
  Cross-workspace event leakage through this endpoint is a spec violation.
- **FR-005**: Existing authorization (`SCOPE_RUNTIME_EVENTS_READ`, loopback
  and bearer-token modes) MUST be preserved unchanged by this migration.
- **FR-006**: The signals currently carried only by `AppStateEventRecord`
  (`state_changed`, `capability_result`, session-lifecycle events,
  command-dispatch events) MUST each be mapped onto a registered
  `EventCatalog` entry with an explicit owner, version, and lifecycle
  status — they MUST NOT be dropped silently, and MUST NOT bypass catalog
  registration as an informal "system event" carve-out.
- **FR-007**: When no events are available for a subscriber, the endpoint
  MUST continue to emit a heartbeat message so long-lived connections are
  not mistaken for a dead stream.
- **FR-008**: This spec does not require this endpoint's design to remain
  stable indefinitely: `097-websocket-grpc-event-transport` (issue #966)
  is expected to retire this SSE endpoint outright once WebSocket ships
  (Decision 47), not run alongside it as a permanent fallback.
- **FR-009** (v1.1.0): A malformed `Last-Event-ID` (fails `EventBroker`'s cursor
  parsing, surfaced as `EventError::InvalidCursor`) MUST return `400 Bad
  Request` with a `problem+json` body identifying the malformed value. An
  expired cursor (outside the active retention window, surfaced as
  `EventError::CursorExpired`) MUST return `410 Gone` with the oldest
  available cursor included in the response body. Neither case MUST silently
  fall back to a full replay from the start of the retention window.
- **FR-010** (v1.1.0): An internal `EventBroker` error during subscribe or
  poll (for example, a poisoned lock or other non-cursor failure) MUST
  return `503 Service Unavailable` with a `problem+json` body. The
  connection MUST NOT be silently dropped without a terminal error response
  — the client must be able to distinguish "no new events" from "the
  broker failed."

## Acceptance Scenarios

1. Given a `Subscribable` capability whose declared event has been published
   to `EventBroker` (per issue #963's `PlacementRouter` wiring), when a
   client opens `GET /v1/workspaces/{workspace_id}/apps/{app_id}/events`,
   then the response is an SSE stream whose `data:` payloads are full
   `TraverseEvent` envelopes for that workspace/app.
2. Given a client reconnects with a `Last-Event-ID` header, when the request
   is served, then only events with a cursor after that id are replayed,
   using `EventBroker`'s own cursor resolution.
3. Given an event published for workspace A, when a client subscribed to
   workspace B's endpoint polls, then that event is never delivered.
4. Given no new events are available, when the connection is held open,
   then a heartbeat message is emitted rather than the connection appearing
   silent or dead.
5. Given the migration has landed, when `crates/traverse-cli/src/http_api.rs`
   is inspected, then no `AppStateEventRecord` type or references to it
   remain.
6. Given a client sends a malformed `Last-Event-ID`, when the request is
   served, then the response is `400` with a `problem+json` body — not a
   silent restart from the beginning of the stream.
7. Given a client sends an expired `Last-Event-ID` (older than the active
   retention window), when the request is served, then the response is
   `410 Gone` with the oldest available cursor in the body.
8. Given `EventBroker` fails internally during a poll, when the failure
   occurs, then the client receives a `503` `problem+json` response rather
   than a connection that appears to hang.

## Out of Scope

- Selecting or implementing WebSocket or gRPC transport (governed by
  `097-websocket-grpc-event-transport`, issue #966).
- The browser-subscription ordered message contract and its production
  exposure (`013-browser-runtime-subscription`; tracked separately as issue
  #973, currently deferred).
- Any change to `EventBroker`'s internal delivery, durability, or lifecycle
  semantics — this spec only governs what already reads from it.
