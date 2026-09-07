# ADR-0061: Offline Activation from Verified Cache Evidence

- Status: Accepted
- Date: 2026-09-07
- Governing spec: `1258-offline-cache-activation` (Approved)
- Related issue: #1258

## Decision

Add a narrow successor to Specs 106 and 107. It governs activation from
already verified, host-owned Registry cache evidence for mixed local/Registry
applications. Activation and execution remain offline and fail closed; neither
may rewrite manifests or re-resolve an alternate candidate.

## Consequences

The cache becomes a supported activation input with immutable redacted evidence
and drift detection, while cache persistence and secrets remain host-owned.
Existing approved specs remain unmodified.

## Alternatives considered

- Amend Specs 106/107: rejected because approved specs are immutable.
- Allow activation-time resolution or fetching: rejected because it weakens
  offline determinism and trust evidence.
