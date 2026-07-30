# Feature Specification: Hosted DataStore Transport and Adapter Boundary

**Status**: Approved
**Canonical governing ID**: `087-hosted-datastore-transport`
**Extends**: `086-datastore-synchronization`, `534-ecca-event-products`
**Decision evidence**: Traverse #887 decision log

## Purpose

Define the first real hosted transport for provider-neutral, offline-first
DataStore synchronization. It provides an optional managed WebSocket adapter
without making the provider a dependency of the portable product model,
DataStore correctness, capability code, or event contracts.

## Scope

In scope:

- a provider-neutral hosted transport port and initial Ably adapter
- encrypted operation relay, bounded replay, degraded operation, and recovery
- scoped authorization, opaque channels, evidence, and cross-host conformance

Out of scope:

- relay-owned DataStore state, snapshots, or conflict resolution
- provider SDK concepts in portable contracts or generated capability code
- exactly-once delivery, peer discovery, and operating a self-hosted relay

## Requirements

- **FR-001**: Portable synchronization code MUST depend only on a
  provider-neutral hosted-transport port. The first Ably adapter is optional,
  application-selected, and replaceable. Provider SDKs, credentials, and native
  channel names MUST NOT appear in capability contracts, DataStore envelopes,
  generated capability code, or core runtime APIs.
- **FR-002**: The adapter MUST relay the approved `086` synchronization
  envelope inside the approved ECCA-compatible event envelope. It MUST preserve
  operation ID, synchronization-set ID, writer ID, Lamport ordering evidence,
  correlation/causation, and key-version metadata without interpreting plaintext
  DataStore content.
- **FR-003**: The application backend MUST issue short-lived, least-privilege
  credentials for one authorized synchronization scope. The adapter MUST NOT
  mint credentials, embed provider secrets, or use broad service credentials.
- **FR-004**: An authorized channel MUST be an opaque backend-derived scope.
  It MUST NOT derive from a capability ID. Tenant, user, and device-group
  membership MUST be enforced before credentials are issued; channel/credential
  mismatch MUST fail deterministically.
- **FR-005**: The adapter MUST refresh credentials through the application
  backend before expiry. Refresh failure or expiry MUST transition synchronization
  to typed degraded state while allowing durable local writes and capability
  execution to continue.
- **FR-006**: Synchronization payloads MUST be encrypted by the application
  before entering the relay. Every operation MUST contain a key-version ID.
  Applications MUST support a bounded active-and-previous key window during
  rotation; stale keys require normal reconciliation. Provider credentials and
  encryption keys MUST remain separate.
- **FR-007**: The relay is non-authoritative. It MUST NOT store canonical
  DataStore state, create snapshots, resolve conflicts, or decide winners. The
  `086` protocol resolves conflicts after ordered delivery.
- **FR-008**: The adapter MUST offer at least two minutes of ordered replay per
  authorized channel. An expired or unavailable cursor MUST return the stable
  typed outcome `resync_required`; it MUST NOT silently transfer a snapshot.
- **FR-009**: After `resync_required`, an application-owned synchronization
  authority service MUST provide encrypted snapshot or operation catch-up for
  the authorized scope. It is separate from the relay and follows the portable
  synchronization protocol.
- **FR-010**: Relay outages MUST not block local writes. The adapter MUST emit
  typed connection/degraded/recovering evidence and reconnect with bounded
  backoff. At-least-once delivery, receiver idempotency, explicit ordering scope,
  correlation, causation, and deduplication remain mandatory.
- **FR-011**: Observability MAY contain only opaque operation IDs,
  contract/schema/key versions, hashed scope/device identifiers, state changes,
  retry/replay/resync outcomes, and timing metrics. It MUST NOT contain plaintext
  payloads, credentials, raw channel names, or encryption keys.
- **FR-012**: The adapter MUST emit ECCA-compatible declared/observed lineage
  evidence and deterministic diagnostics for unauthorized scope, expiry, replay
  loss, invalid envelope, key mismatch, and provider-unavailable cases.

## Acceptance Scenarios

1. Given an application-selected Ably adapter and a valid short-lived scoped
   credential, when an encrypted operation is published, then the receiving host
   obtains the unchanged portable envelope and records observed lineage.
2. Given a disconnection shorter than the guaranteed window, when a host
   reconnects with its cursor, then ordered missed operations replay exactly as
   the local deterministic adapter fixture specifies.
3. Given an expired cursor, when the host reconnects, then it receives
   `resync_required` and obtains encrypted catch-up from the application-owned
   synchronization authority; the relay provides no snapshot.
4. Given an invalid credential, wrong scope, expired key, or invalid envelope,
   when it reaches the adapter, then the operation is rejected with sanitized,
   deterministic evidence and no plaintext disclosure.
5. Given relay unavailability or credential refresh failure, when local writes
   occur, then they remain durable and execution succeeds while synchronization
   is visibly degraded and later reconnects.

## Conformance and Quality Gates

- **QG-001**: The identical conformance suite MUST pass through the Ably and
  deterministic local/in-memory adapters with no portable-contract or
  capability-code differences.
- **QG-002**: It MUST pass on TypeScript/web, Swift/iOS, Kotlin/Android, and
  .NET/desktop, with Rust as the deterministic reference harness. An unsupported
  host requires an explicit capability profile; silence is not conformance.
- **QG-003**: Fixtures MUST cover authorization isolation, credential rotation,
  encrypted payload privacy, replay, expiry/resync, offline/degraded recovery,
  idempotent retry, conflict-equivalent delivery, telemetry redaction, and
  adapter replacement.
- **QG-004**: This specification permits only hosted-transport work that closes
  the #894 generation gate. It does not lift the broader capability/reference-app
  generation pause until the ECCA validator and three-capability proof pass.

## Implementation Tickets

- Traverse #887 — hosted transport adapter implementation
- Traverse #898 — event-first reference-app proof (after its registry/runtime
  dependencies expose the required interfaces)
