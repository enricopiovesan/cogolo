# ADR-0023: IndexedDB as a Same-Port Public DataStore Backend

- Status: Accepted
- Date: 2026-07-29
- Governing spec: `528-datastore-indexeddb` / `085-datastore-indexeddb` (Draft)
- Extends: ADR-0018, ADR-0019
- Related: ADR-0022 (private encryption deferred on this backend for v1)

## Context

Web embedders need durable local state without implying a second, browser-only
application API. Full parity with native maintenance and private encryption in
one slice would delay browser permanence indefinitely.

## Decision

Ship IndexedDB as a backend behind the existing DataStore port and envelope
semantics. v1 is public CRUD plus exclusive ownership using Web Locks, with
typed quota/persistence errors. Private records and maintenance operations are
explicitly unsupported until successor specs/tickets land.

## Consequences

- Web apps can persist public integrity-checked state with familiar host APIs.
- Private-at-rest and zip backup on IDB remain follow-ons.
- Conformance focuses on lock behavior, integrity, and error mapping.

## Alternatives Considered

- Separate browser-only port: rejected (splits embedder learning/conformance).
- Private plaintext in IDB: rejected (violates encryption decision).
- Full maintenance parity in v1: deferred to keep the slice approvable.
