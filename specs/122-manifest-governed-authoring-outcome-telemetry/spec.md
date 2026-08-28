# Feature Specification: Manifest-Governed Authoring Outcome Telemetry

**Status**: Approved
**Canonical governing ID**: `122-manifest-governed-authoring-outcome-telemetry`
**Supersedes**: `1183-privacy-preserving-authoring-telemetry`
**Decision evidence**: Traverse #1169 and #1186.

## Purpose

Allow an application or capability manifest to declare eligibility for
privacy-preserving authoring outcome telemetry without granting that manifest
control of consent, collection transport, or privacy limits.

## Requirements

- **FR-001**: A manifest MAY declare `authoring_outcome_telemetry` with only
  `eligible`, `profile`, and the approved `allowed_fields` allowlist.
- **FR-002**: The only supported profile is `aggregate-v1`; unknown profiles,
  unrecognized fields, duplicate fields, and prohibited fields MUST be rejected
  before collection.
- **FR-003**: The `aggregate-v1` allowlist is limited to `authoring_route`,
  `terminal_outcome`, `revision_count_bucket`, `elapsed_time_bucket`, and
  `finding_category_counts`.
- **FR-004**: Collector endpoint, credentials, consent state, transport,
  aggregation threshold, retention, analytics-role authorization, and access
  logging are host-owned configuration. A manifest MUST NOT set or override
  any of them.
- **FR-005**: Collection remains default-off and requires explicit host-owned
  opt-in. Disabled, malformed, or unavailable configuration fails closed and
  cannot affect authoring, review, build, or publication.
- **FR-006**: Remote publication remains limited to aggregates of at least 20
  distinct opted-in contributors, retained for no more than 90 days, and
  accessible only by the named least-privilege analytics role with logged,
  quarterly reviewed access.

## Acceptance Scenarios

1. A manifest declaring `aggregate-v1` and only allowed fields is eligible but
   produces no collection without host-owned opt-in.
2. A manifest declaring a collector URL, credentials, threshold, retention, or
   an unknown field is rejected before collection.
3. A host uses its configured collector only after validating the manifest;
   the manifest cannot redirect the transport or expand the field set.
4. An eligible manifest with 19 contributors produces no remote report; 20
   contributors produce the bounded aggregate only.

## Quality Gates

- Contract tests cover allowed manifest declarations and every prohibited
  host-owned override.
- Evidence records contain only the approved aggregate fields.
- Implementation preserves the default-off, fail-closed behavior in Spec 1183.
