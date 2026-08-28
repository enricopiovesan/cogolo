# ADR-0053: Manifest-Governed Authoring Outcome Telemetry

- Status: Accepted
- Governing spec: `122-manifest-governed-authoring-outcome-telemetry`
- Supersedes: ADR-0052 for the manifest/host configuration boundary

## Decision

Manifests declare only an eligibility profile and a closed allowlist of
non-identifying authoring-outcome categories. The host exclusively owns consent,
collector endpoint and credentials, transport, aggregation, retention, and
report access.

## Rationale

This follows the useful part of mobile privacy-manifest design: an app or SDK
declares what data category and purpose it requires, while the platform enforces
the permitted shape. It avoids allowing a content manifest to become an
exfiltration mechanism by supplying a destination or privacy override.

## Consequences

Applications and capabilities can be measured consistently, but cannot broaden
collection or choose where it is sent. Host configuration is the sole trusted
boundary for remote publication.
