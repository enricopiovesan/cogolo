# Feature Specification: Host-Owned Authoring Telemetry Aggregation

**Status**: Approved
**Canonical governing ID**: `123-host-owned-authoring-telemetry-aggregation`
**Extends**: `122-manifest-governed-authoring-outcome-telemetry`
**Decision evidence**: Traverse #1189.

## Purpose

Define the host-owned lifecycle for privacy-preserving authoring outcome
telemetry after a manifest has been accepted as eligible under Spec 122. This
spec does not change the manifest boundary; manifests still cannot configure
collection, identity, transport, retention, or access.

## Requirements

- **FR-001**: A host MAY submit only these normalized milestone kinds to its
  portable authoring-outcome adapter: `authoring_started`, `review_finding`,
  `revision`, and `terminal_outcome`. The envelope MUST contain only the
  applicable Spec 122 `aggregate-v1` fields and MUST reject all other fields.
- **FR-002**: Collection is default-off. A host MUST fail closed when opt-in,
  manifest eligibility, ticket issuance, storage, or collector configuration
  is missing, malformed, or unavailable. It MUST not affect authoring, review,
  build, or publication.
- **FR-003**: An opted-in host issues at most one opaque, window-bound
  contributor ticket per reporting bucket. The aggregation store retains only
  a one-way hash of that ticket for deduplication; neither the ticket nor an
  identity mapping may be exported or stored in telemetry data.
- **FR-004**: Local storage contains only approved aggregate counters,
  bucket timestamps, and opaque ticket hashes. It MUST NOT retain raw
  milestone events, prompts, source, contracts, artifacts or digests,
  repository paths, review text, personal identifiers, or per-contributor
  metric rows.
- **FR-005**: A bucket is eligible for remote publication only when it has at
  least 20 distinct retained ticket hashes. The remote payload contains only
  the bounded reporting period, approved aggregate category counts, and
  threshold evidence; no ticket hash, event, or contributor record may leave
  the host.
- **FR-006**: An unpublished bucket MUST be deleted after successful
  publication or at 90 days, whichever occurs first. An opt-out MUST delete
  every unpublished bucket to which that contributor contributed immediately.
  Published aggregates are non-reidentifying and irreversible; this limitation
  MUST be disclosed at opt-in.
- **FR-007**: Aggregate access and export are restricted to a named,
  least-privilege analytics role. The host keeps a separate append-only audit
  record with role, purpose, timestamp, bucket reference, and allow/deny
  result, and records a quarterly access-review attestation. Audit records
  MUST NOT contain authoring-event content.

## Acceptance Scenarios

1. A valid eligible manifest with host opt-in disabled records and exports
   nothing.
2. A host adapter rejects an envelope containing a repository path, prompt,
   contributor identity, or a field outside the `aggregate-v1` allowlist.
3. Nineteen distinct opaque ticket hashes leave a bucket unpublished; the
   twentieth permits only the bounded aggregate payload.
4. A restart retains counters and ticket hashes but no raw milestone event.
5. Opt-out deletes the affected unpublished bucket; an expired bucket is
   pruned within the 90-day retention limit.
6. Every aggregate access or export produces a separate allow/deny audit
   record, and the quarterly review is independently inspectable.

## Compatibility and Quality Gates

- This is additive to Spec 122; applications that omit the manifest policy
  remain unaffected and collection remains disabled.
- The portable adapter, aggregate store, export predicate, retention/opt-out
  behavior, and audit evidence require deterministic contract tests.
- Tests must prove that prohibited content cannot enter local storage or a
  remote payload, and that threshold publication fails closed below 20.
