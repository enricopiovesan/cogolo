# ADR-0034: Adopt WebSocket and gRPC as the Governed Runtime Event Transport, Replacing SSE

- Status: Accepted
- Governing spec: `097-websocket-grpc-event-transport`

## Context

`534-ecca-event-products` deliberately scopes out "selecting a broker vendor
or transport topology," leaving how a governed domain event reaches an
external client (browser, mobile, or another runtime) undecided. A
repo-wide dependency search found no WebSocket or gRPC dependency anywhere
in the workspace (no `tonic`/`prost`, no websocket library); the only
transport in the codebase is SSE, and it currently reads from an ungoverned
type (closed by `096-runtime-event-sse-transport`, issue #964), not
`EventBroker`. Traverse has no production users or production capabilities
yet (Decision 48, `docs/decision-log.md`), so this decision is not
constrained by an existing client base.

## Decision

Traverse adopts WebSocket and gRPC together as the governed event transport,
both reading from the same `EventBroker`/`TraverseEvent` source, and both
decided in the same increment rather than sequenced as primary/secondary —
UMA's own reference architecture treats them as peers a client selects
between per platform and workload (WebSocket via browser-native APIs or
libraries such as Starscream/OkHttp; gRPC via `grpc-swift`/`grpc-java` for
mobile clients that need structured, typed contracts and lower battery
overhead). SSE is retired outright once WebSocket ships — it does not
remain as a permanent parallel transport, consistent with Decision 48's
no-back-compat-tax principle.

## Consequences

The workspace takes on a new dependency for WebSocket handling and for gRPC
(`tonic`/`prost` or equivalent), plus `.proto` contract authorship and
versioning for the event service. Connection lifecycle, auth-over-socket,
and message framing must all be designed (governed by the spec this ADR
pairs with) rather than reusing the existing HTTP request/response and SSE
patterns. The SSE endpoint built under `096-runtime-event-sse-transport`
is explicitly transitional: its removal is in scope for whichever ticket
implements WebSocket (issue #967), not a separate cleanup task.
`browser_adapter.rs`'s eventual disposition (issue #973) is blocked on this
transport existing, since its governed message contract (`013`) doesn't yet
have a production transport to run on.

## Alternatives Considered

- Keep SSE as a permanent fallback alongside WebSocket — rejected. Traverse
  has no installed base to protect, and permanently maintaining two
  transports for one source of truth adds surface area for a compatibility
  constraint that doesn't exist.
- WebSocket only, defer gRPC until a concrete native/mobile client exists —
  this was the recommended option going into the decision, but was not
  chosen; the user opted to decide both together rather than speculatively
  defer gRPC, accepting the larger upfront design cost.
- Decide no transport now, leave it an open "TBD" — rejected. UMA's model
  and Traverse's own capability contracts (`emits`/`consumes`,
  `service_type: Subscribable`) already assume governed events reach a live
  client somewhere; leaving the transport permanently undecided leaves that
  assumption unfulfillable.
