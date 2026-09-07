# ADR-0059: Mixed Local and Registry-Reference Activation

- Status: Proposed
- Proposed governing spec: `130-mixed-registry-reference-activation`
- Related issue: #1258

## Context

Specs 106 and 107 separately govern local executable-artifact activation and
cross-host verified `registry_ref` caches. Spec 107 explicitly excludes
activation-time artifact selection. Consequently, a mixed application can
prepare and validate registry dependencies but has no governed offline path to
activate them alongside local components.

## Proposed Decision

Extend activation with a resolver that treats a prepared verified cache entry
as the sole admissible candidate for a `registry_ref` component. The host
continues to own storage, secrets, fetching, and preparation. Traverse records
only immutable non-secret selected-component and validation evidence, and
execution consumes that evidence without a network request or re-resolution.

The resolver validates the component identity and range, digests, trust
lifecycle, ABI, target, placement, and constraints. Any missing or altered
evidence fails closed. Local-only activation remains unchanged.

## Consequences

This gives mixed applications an auditable offline lifecycle while preserving
the cache and trust boundaries of Spec 107. It adds activation evidence and
failure semantics that must be implemented consistently in Rust and Web and
certified across the native host matrix.

## Alternatives Considered

- Re-resolve from the registry during activation or execution: rejected because
  it weakens offline operation and permits unrecorded selection drift.
- Materialize a local-path substitute from the cache: rejected because it
  obscures provenance and permits an unproven equivalent candidate.
- Rewrite the application manifest after preparation: rejected because it
  changes portable application intent into host state.
