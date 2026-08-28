# ADR-0054: Host-Owned Authoring Telemetry Aggregation

- Status: Accepted
- Governing spec: `123-host-owned-authoring-telemetry-aggregation`
- Extends: ADR-0053 for the collection, aggregation, deletion, and audit
  lifecycle

## Decision

After a manifest is accepted under Spec 122, an integrating host alone emits
a closed set of normalized authoring milestones. It aggregates only allowed
categories locally using opaque, window-bound contributor tickets. The store
holds aggregate counters and one-way ticket hashes—not raw events or identity
records—and exports a bounded aggregate only at an exact threshold of 20
distinct tickets.

An opt-out deletes every affected unpublished bucket. Buckets are also deleted
on publication or at 90 days. Published aggregates are irreversible and that
fact is disclosed at opt-in. A named least-privilege analytics role accesses
only aggregates under a separate append-only audit trail and quarterly review.

## Rationale

This preserves the declarative manifest boundary from ADR-0053 while giving
hosts a portable, verifiable way to collect comparable outcomes. Opaque tickets
provide exact thresholding without turning a telemetry bucket into a durable
developer profile. Aggregate-only persistence limits privacy exposure and
allows failure to remain non-disruptive.

## Consequences

Implementations need a host adapter, ticket issuer, aggregate-only bucket
store, threshold gate, prune/reset operation, and access-audit sink. They must
not scrape Git/CI/PR metadata, accept manifest-provided collector controls, or
export raw events or ticket hashes.
