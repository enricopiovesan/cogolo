# Durable Trace Journal Boundary Contracts

- `trace.append(record, authorization)` persists only a validated safe
  projection for an authorized workspace.
- `trace.list`, `trace.get`, and `trace.export` require host authorization and
  return redacted safe projections only.
- `trace.prune(policy, authorization)` is deterministic, oldest-first, and
  emits prune evidence.
- Journal storage and authorization decisions are host-provided interfaces;
  Traverse never owns their root, keys, tenancy, or identities.
