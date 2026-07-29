# Data Model

| Entity | Required fields | Invariants |
| --- | --- | --- |
| `RegistryTrustRecord` | namespace, id, version, artifact digest, tier, publisher id, provenance ref, schema version | Artifact identity and provenance are immutable. |
| `PublisherAdmissionEvidence` | publisher id, identity verification ref, signature ref, validation ref, conformance ref, support-policy ref, approved-by | All fields are required for Certified eligibility. |
| `LifecycleStateRecord` | artifact identity, state, published-at, state digest, publisher signature | It references an artifact and never rewrites it. |
| `DeprecationNotice` | replacement or migration path, published-at, effective-at | `effective-at` is at least 90 days after publication. |
| `SecurityYankPolicy` | minimum-safe version and/or enforcement deadline, state digest | Applies deterministically from locally known state. |
| `ProductionLockEntry` | exact identity, digest, publisher, tier, provenance, source/index/lifecycle digests, resolved-at | Immutable after preparation; no storage paths or credentials. |
| `ResolutionEvidence` | allowed tiers, selected/rejected record, policy outcome, admission refs, lifecycle facts | Safe to retain and explain offline decisions. |
