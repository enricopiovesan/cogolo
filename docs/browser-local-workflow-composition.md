# Browser-local workflow composition

**Governing spec**: `1277-browser-local-workflow-composition` · **ADR**: ADR-0062
· **Issue**: #1269

The backend-less `/discover` path turns a structured goal into governed
execution without a Traverse-operated service. This page documents the first
half — the browser planner surface in `traverse-embedder`. The runtime-composed
execution adapter (local handoff, per-node events, failure/replan) is #1270.

## What the browser planner is

`traverse_embedder::browser_local_plan` is a pure, deterministic function. It
reads an already synced, digest-verified public registry snapshot plus the set
of `registry_ref` dependencies the host already prepared and verified, and
returns zero or more **untrusted** workflow proposals. It performs no I/O and
makes no network request; every input is borrowed and every output is derived
from it (spec 1277 FR-003, FR-008).

Candidate derivation is exactly the structural schema-and-event chaining rule of
spec `113-declarative-workflow-planning` FR-002: a chain is valid only when each
node's required inputs are covered, by declared JSON-Schema property name and
type, by the starting facts or by exactly one upstream node's outputs. The
planner never infers a plan from capability names, namespaces, natural-language
goals, model output, recency, or a hidden score. It keeps spec 113's bounds — at
most 5 candidate proposals, at most 8 nodes deep — and reports
`plan_search_truncated` rather than returning a silent partial result.

## Inputs

| Input | Type | Notes |
| --- | --- | --- |
| `snapshot_identity` | `SnapshotIdentity` | `registry_snapshot_digest` (`sha256:` over the canonical `SyncedPublicRegistryState`), `source_release`, `contract_schema_version` |
| `snapshot` | `SyncedPublicRegistryState` | the synced, verified public index snapshot |
| `verified_dependencies` | `&[VerifiedRegistryDependency]` | from `resolve_registry_dependency_offline`; carry the verified contract bytes, artifact digest, and prepare evidence |
| `target` | `BrowserPlanTarget` | `Capability { id, version }` or `EmitsEvent { event_type }` — never a goal string |
| `starting_facts` | `serde_json::Value` | object of the facts available before any node runs |
| `workspace_id`, `app_manifest` | — | bound into each emitted Spec-109 proposal |

## Output

`BrowserPlanResponse { proposals: Vec<BrowserWorkflowProposal>, plan_search_truncated: bool }`.

Each `BrowserWorkflowProposal` is a versioned envelope
(`kind = "browser_workflow_proposal"`, `schema_version = "1.0.0"`) that binds the
source `snapshot_digest` and `source_release` to a Spec-109-compatible
`WorkflowProposal`. `mapping_unconfirmed` is always `true`: the browser planner
never clears a field mapping (spec 1277 FR-004). An empty `proposals` list is a
valid success outcome, not an error.

The caller hands a reviewed proposal to the local governed runtime, which
remains the sole validator, authorizer, and executor (spec 1277 FR-005). A
browser result never enlarges manifest authority or bypasses a runtime denial.

## Error taxonomy

`BrowserPlanError { code, detail }`. `detail` may name a declared capability
identity or a digest; it never carries paths, raw values, URLs, or bytes
(spec 1277 FR-007). `BrowserPlanError::as_value()` is the secret-free
projection.

| Code | Meaning |
| --- | --- |
| `browser_plan_unsupported_contract_schema_version` | `contract_schema_version` (or a prepared contract) is not `1.0.0` |
| `browser_plan_snapshot_digest_mismatch` | `registry_snapshot_digest` does not match the snapshot |
| `browser_plan_snapshot_empty` | the snapshot has no capability records |
| `browser_plan_snapshot_evidence_stale` | `source_release` does not match the snapshot's release tag |
| `browser_plan_verified_dependency_contract_invalid` | a prepared dependency's bytes are not a capability contract |
| `browser_plan_verified_dependency_not_in_snapshot` | a prepared dependency has no matching active record |
| `browser_plan_verified_dependency_digest_mismatch` | a prepared dependency's digests disagree with the snapshot record |
| `browser_plan_verified_dependency_evidence_mismatch` | a prepared dependency was prepared against a different snapshot |
| `browser_plan_starting_facts_too_large` | starting facts exceed 64 KiB |
| `browser_plan_verified_dependency_set_too_large` | more than 128 prepared dependencies |

## Bounds

- 5 candidate proposals, 8 nodes deep, 4 000 search calls (spec 113 FR-004).
- 64 KiB serialized starting facts; 128 prepared dependencies (spec 1277 FR-007).

## Conformance

`crates/traverse-embedder/src/browser_local_plan.rs` tests cover deterministic
identical-snapshot output, ambiguous producers with no automatic winner, no
name/namespace inference, each invalid-snapshot-evidence class, prepared
dependency drift (digest, cross-snapshot, missing record), the size bounds, the
node-depth truncation signal, and evidence redaction.
