# Governed Workflow Promotion (P4)

Governed by spec [`112-governed-workflow-promotion`](../specs/112-governed-workflow-promotion/spec.md)
and [ADR-0042](adr/0042-phased-dynamic-orchestration-evolution.md). Tracks
issue `#1094`.

P4 defines the non-mutating export and human-reviewed promotion path from a
completed [P1 runtime workflow proposal](workflow-proposal-lifecycle.md) to a
reusable, versioned workflow. It adds no new registration mechanism: export
produces a plain candidate value, and promotion still goes through the
existing `traverse_registry::WorkflowRegistry::register` /
`traverse-cli workflow register` path unchanged — exactly as for any
hand-authored workflow.

## Where the code lives

Spec 112 governs `crates/traverse-registry/`, `crates/traverse-cli/`, and
`crates/traverse-mcp/` — notably not `traverse-runtime` or
`traverse-contracts`. `traverse-registry` is an external, independently
versioned crate (spec `051-registry-extraction`); this PR's actionable
surface is entirely new code in `traverse-mcp::tools::workflow_promotion`,
consuming `traverse_registry`'s existing `WorkflowDefinition`/
`WorkflowRegistry` types via its public API.

## Export (FR-001): a candidate, never a mutation

`export_workflow_candidate(canonical, trace, resolved_nodes)` builds a
`WorkflowCandidateArtifact` from a proposal whose trace reached
`Succeeded` — acceptance scenario 1 requires exporting a *completed*
proposal, since an incomplete or failed execution has no proven reusable
shape to offer a reviewer. The function is pure: it never touches a
registry or app manifest, and returns a plain, serializable value.

The candidate carries:

- `source_proposal_id` / `source_proposal_digest` / `source_snapshot_digest`
  — provenance back to the exact proposal and pinned snapshot that produced
  it (FR-001).
- `nodes` — each with its `capability_id`/`capability_version` and resolved
  `risk: RiskMetadata` (FR-002a: preserve reviewed effect, determinism,
  data-flow, and reliability declarations for the reviewer to see).
- `edges` — copied exactly from the proposal, including any fan-out (see
  below).
- `unconfirmed_mappings` — mappings the export could not reduce cleanly
  (see next section), for the reviewer to resolve by hand.
- `excluded_fields` — a fixed audit list (`initial_input`,
  `authorization.approval_token_id`) naming what was intentionally never
  carried into the candidate, mirroring
  `ApplicationEffectiveConfig::redacted_secret_keys`'s convention.

### Why nothing needs active secret-scanning

FR-002a requires the candidate never carry raw inputs, approval tokens,
private bindings, prompts, or secrets. Rather than scanning for and
stripping such content at export time, the candidate is built exclusively
from fields already proven secret-free by construction: `ProposalTrace` is
itself documented as "a bounded, redacted, immutable projection... no raw
node input/output payloads" (spec 109 FR-009), and the only proposal field
that could carry arbitrary caller-supplied content —
`WorkflowProposal.initial_input` — is simply never read by the exporter.
This is a structural guarantee, not a runtime check.

## The mapping model gap (and why it's surfaced, not hidden)

A proposal mapping is an arbitrary JSON-Pointer-to-JSON-Pointer wire between
two exact node fields. `WorkflowDefinition`'s existing model —
`WorkflowNodeInput.from_workflow_input` / `WorkflowNodeOutput.
to_workflow_state` — instead shares data through named top-level keys in an
accumulated, workflow-wide state bag: a downstream node's
`from_workflow_input` can read any key an upstream node's
`to_workflow_state` (or the workflow's own `inputs`) already published,
matched purely by key name, with no rename step.

A mapping whose source and target JSON-Pointer paths share the same final
segment (`/value` → `/value`, or `/a/value` → `/b/value`) translates
cleanly to a shared state key. Anything else — a field rename, or two
nested paths with different leaf names — cannot be expressed by this model
without guessing, so it is listed in `unconfirmed_mappings` instead: no
state-key wiring is emitted for it, and the reviewer decides how (or
whether) to rewire it by hand. This mirrors spec 113's
`mapping_unconfirmed` convention for the same "don't guess, flag it"
reason.

## The fan-out gap (and why it's surfaced, not hidden)

`WorkflowDefinition`'s own registration validation already rejects more
than one direct outgoing edge from any node with a `DuplicateItem` error —
a real, existing v0.1 constraint this feature does not relax or work
around. P1/P2 proposals fully support branching DAGs; `export_workflow_
candidate` copies a proposal's edges exactly, so a branching proposal's
candidate correctly fails registration if promoted as-is. This is not a bug
in the export — it is an honest reflection of what the target format can
currently express. Only proposals whose nodes each have at most one
outgoing edge can currently become a registrable workflow. A dedicated test
(`a_branching_proposals_candidate_is_honestly_unregistrable`) proves this
is surfaced, not silently swallowed.

## Promotion (FR-002, FR-003): identity is a review decision, not inferred

`finalize_candidate_into_definition(candidate, identity)` assembles a
`WorkflowDefinition` from the candidate plus a `PromotedWorkflowIdentity`
(`id`, `name`, `version`, `owner`, `lifecycle`, `summary`, `tags`) — fields
a human reviewer decides at promotion time, never inferred from the source
proposal (FR-003: "a promoted workflow has its own immutable identity; its
source proposal does not confer ongoing execution authority"). This
function performs no registration itself; the caller still submits the
resulting `WorkflowDefinition` through the unchanged
`WorkflowRegistry::register` / `traverse-cli workflow register` path, which
enforces the same canonical validation and per-`(scope, id, version)`
immutability as any hand-authored workflow (FR-002).

## Yank, rollback, deprecation (FR-004)

Reused entirely: a promoted workflow's `Lifecycle` (`Draft` → `Active` →
`Deprecated` → `Retired` → `Archived`) is the only lifecycle mechanism
`WorkflowRegistry` has, for hand-authored or promoted workflows alike. This
feature introduces no separate yank/rollback verb.

## End-to-end proof

`workflow_promotion_tests.rs`'s
`export_then_promote_then_discover_end_to_end` test proves the full spec
112 Definition-of-Done chain against the real
`traverse_registry::WorkflowRegistry` (not a mock): export a successfully
completed linear proposal's trace, finalize it with a reviewer identity,
register it, then confirm it resolves via `find_exact` and appears in
`discover`.

## Non-goals

Matches spec 112's own "Out of scope": direct runtime publication,
automatic promotion, or bypassing human review. Nothing in this feature
calls `WorkflowRegistry::register` itself, opens a PR, or grants a proposal
any ongoing execution authority.
