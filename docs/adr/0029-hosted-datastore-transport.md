# ADR-0029: Managed Hosted Transport Behind a Provider-Neutral Boundary

- Status: Accepted
- Date: 2026-07-30
- Governing spec: `535-hosted-datastore-transport` / `087-hosted-datastore-transport`
- Decision evidence: Traverse #887
- Extends: ADR-0025, ADR-0028

## Context

The approved DataStore synchronization protocol has an in-process transport
only. Real multi-OS product applications need a hosted path, but a hosted
provider must not become part of portable event/DataStore semantics or own
canonical application data.

## Decision

Use a managed WebSocket relay as the first hosted transport family. Ably is the
first optional, application-selected adapter target. Core contracts and runtime
depend only on a provider-neutral port. Applications issue short-lived scoped
credentials and encrypt operations before relay entry. The relay provides ordered
bounded replay and no authoritative state; an application-owned sync authority
handles expired-cursor recovery. The same conformance suite proves the Ably and
local/in-memory adapters across supported hosts.

## Consequences

- Real-time multi-OS synchronization has a product-ready hosted path.
- Provider replacement is a tested property, not an architectural promise.
- Offline local operation and provider-blind payload confidentiality are retained.
- Applications must operate authorization, encryption keys, and sync-authority
  recovery separately from the relay.

## Alternatives Considered

- Self-hosted NATS/JetStream first: deferred; it adds server operation before
  the portable product path is proven.
- Serverless polling first: rejected; reconnect and ordered replay are weaker.
- Relay as canonical store: rejected; breaks offline-first portability and
  couples data correctness to the provider.
- Provider SDK in core: rejected; makes host/capability portability untestable.
