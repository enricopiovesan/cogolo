# ADR-0052: Privacy-Preserving Authoring Outcome Telemetry

- Status: Accepted
- Governing spec: `1183-privacy-preserving-authoring-telemetry`
- Decision evidence: Traverse #1169

## Context

Runtime usage telemetry does not authorize observation of authoring or review.
The ecosystem needs evidence about authoring outcomes without collecting the
content or identity of contributors.

## Decision

Use a separate, default-off telemetry boundary. Opted-in clients aggregate only
permitted categorical outcome data locally. A remote aggregate is allowed only
after a reporting window reaches 20 distinct opted-in contributors. All
under-threshold data stays local. Remote aggregates are retained for no more
than 90 days. Collection failures, missing consent, and prohibited fields fail
closed and never influence authoring or publication.

Remote reports are available only to a named, least-privilege Traverse
maintainer analytics role. Access is logged and role membership is reviewed
quarterly.

## Consequences

This permits carefully bounded aggregate evidence while prohibiting content and
identity collection.

## Alternatives Considered

- Local-only export: stronger privacy but no ecosystem aggregate.
- No instrumentation: lowest collection risk but no reproducible evidence.
- Lower cohort threshold: rejected for avoidable re-identification risk.
