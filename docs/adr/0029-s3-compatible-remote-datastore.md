# ADR-0029: Constrain the Remote DataStore Adapter to an S3-Compatible Profile

**Status:** Proposed  
**Governing draft:** `535-s3-compatible-remote-datastore`  
**Extends:** ADR-0024

## Decision

Traverse will support an S3-compatible remote DataStore adapter only through
explicit host injection. A host supplies the client, opaque tenant prefix, and
least-privilege credentials. Stored values use canonical DataStore envelopes
with a Traverse SHA-256 digest; ETags are conditional-write tokens, not
integrity evidence. Conditional conflicts, authorization, outage, timeout,
and ambiguous submission map to stable secret-free failures. Retry remains a
host decision.

## Consequences

MinIO is the required first integration-conformance target. No provider SDK,
credential store, endpoint policy, background queue, synchronization behavior,
or multi-key transaction is introduced. #886 may implement only after this
draft and ADR are explicitly approved.

## Alternatives Considered

- Treating ETags as integrity digests is rejected because ETag meaning is
  provider- and upload-dependent.
- Runtime-owned credentials or retries are rejected because they violate the
  host-owned boundary in ADR-0024.
- Provider-specific implementation before a portable profile is rejected
  because it would create ungoverned behavior and lock in a vendor.
