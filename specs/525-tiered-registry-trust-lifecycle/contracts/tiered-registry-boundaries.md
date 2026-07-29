# Tiered Registry Boundary Contracts

- `registry.discover(query, allowed_tiers?)` returns only records eligible for
  the requested trust policy; omitted `allowed_tiers` means `{certified}`.
- `registry.evaluate_admission(record, evidence)` returns Certified eligibility
  and stable rejection evidence without exposing confidential publisher data.
- `registry.publish_lifecycle(artifact, state, policy)` validates immutable
  artifact identity and lifecycle invariants before emitting a signed state
  record.
- `registry.resolve(reference, policy)` returns an exact immutable lock entry
  and structured selection evidence, or a stable error.
- `registry.evaluate_lock(lock_entry, locally_known_lifecycle)` makes the
  offline normal/security-yank outcome without network access.
