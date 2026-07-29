# Research: Production App Readiness Baseline

## Decision: Embedded offline execution is the production topology

**Rationale**: It avoids runtime network ambiguity and preserves host-owned
identity, storage, and update control. HTTP remains a development/CI surface.

**Alternatives considered**: Sidecar-first production was rejected because it
would reintroduce deployment and network coupling into every host.

## Decision: Resolve once, run from a verified generation

**Rationale**: Exact lock identity plus content-addressed cache yields
reproducible execution, explicit updates, and rollback.

**Alternatives considered**: Runtime range resolution or network fallback were
rejected because they make offline and security outcomes ambiguous.

## Decision: Separate safe trace journal from application state

**Rationale**: Audit retention and redaction have different ownership and
lifecycle from user state. Decision 34 and Spec 079 already establish this.

## Decision: File-backed DataStore v2 before broader adapters

**Rationale**: An explicit v1-to-v2 migration proves recoverable evolution
without prematurely selecting cloud sync, IndexedDB, database, or key systems.

## Decision: Common certification, not weakest-platform support

**Rationale**: Certified status is a product promise. A host failing cache,
rollback, trace-restart, or resource-control conformance is Preview.
