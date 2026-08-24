# Runtime Workflow Proposal Lifecycle (P1)

Governed by spec [`109-runtime-workflow-proposals`](../specs/109-runtime-workflow-proposals/spec.md)
and [ADR-0050](adr/0050-governed-runtime-workflow-proposal-authority.md)
(renumbered from ADR-0041 on 2026-08-24). Tracks issue `#1090`.

A **workflow proposal** is an untrusted, externally-authored, ephemeral,
manifest-bound bounded sequential DAG over already-registered capabilities.
An MCP client (or any planner) submits one; Traverse validates, authorizes,
and executes it — the planner is never the authority (ADR-0050).

## Where the code lives

| Layer | Crate | What it owns |
|---|---|---|
| Wire format, canonicalization, digesting, structural validation | `traverse-contracts::proposal` | Pure, no manifest/registry access — a proposer can independently recompute the same digest from the same JSON. |
| Manifest/registry cross-validation, authorization, quotas, execution | `traverse-runtime::proposal` | Everything needing live host state: `traverse_registry::ApplicationBundleManifest`, `CapabilityRegistry`, and the `Runtime` execution engine. |
| Public MCP tool surface | `traverse-mcp::tools::proposals` | Plain, fully-tested Rust functions — the same pattern `tools::capabilities` already established for spec `015`. Not wired into `stdio_server.rs`; that reference host is a separate, single-example-bundle transport (see [docs/browser-hosted-execute-entrypoint-validation.md](browser-hosted-execute-entrypoint-validation.md)), not this spec's governed surface. |

## Wire format

```json
{
  "kind": "workflow_proposal",
  "schema_version": "1.0.0",
  "proposal_id": "proposal-001",
  "workspace_id": "workspace-001",
  "app_manifest": { "app_id": "...", "app_version": "1.0.0", "manifest_digest": "sha256:..." },
  "nodes": [
    { "node_id": "a", "capability_id": "content.comments.create-comment-draft", "capability_version": "1.0.0", "artifact_digest": "sha256:..." }
  ],
  "edges": [ { "from_node_id": "a", "to_node_id": "b" } ],
  "mappings": [
    { "source": { "kind": "node", "node_id": "a" }, "source_path": "/draft_id", "target_node_id": "b", "target_path": "/draft_id" },
    { "source": { "kind": "initial_input" }, "source_path": "/comment_text", "target_node_id": "a", "target_path": "/comment_text" }
  ],
  "initial_input": { "comment_text": "hello", "resource_id": "r1" }
}
```

Every field the runtime authorizes or executes against is explicit here — a
node never implicitly sees another node's full output; every path is a
declared mapping (spec FR-002).

## Canonicalization, digesting, and snapshot binding (FR-003, FR-007a)

- **Structural validation** (`canonicalize_proposal`) rejects, before any host
  lookup: wrong `kind`/`schema_version`, over-limit node/edge/mapping counts
  or `initial_input` byte size, duplicate node ids, dangling or duplicate
  edges, self-loops, cycles, mappings to/from unknown endpoints, a mapping
  with no corresponding declared edge, and an ambiguous multi-writer target
  path (two mappings writing the same `target_path` on the same node).
- **Deterministic execution order**: Kahn's algorithm over the declared
  edges, breaking ties among simultaneously-ready nodes by lexicographic
  `node_id` (FR-007a). A diamond graph (`a → b`, `a → c`, `b → d`, `c → d`)
  always orders as `a, b, c, d`, never `a, c, b, d`.
- **`proposal_digest`**: the proposal is serialized to **canonical JSON**
  (object keys recursively sorted, no insignificant whitespace — a from-
  scratch implementation; no such helper existed anywhere in the workspace
  before this) and hashed with SHA-256, formatted as
  `"1.0.0:sha256:<hex>"`. This is deliberately **not** the `governed_content_digest`
  convention used for published capability/event contracts (`{version}:{fnv1a-hex}`
  over Rust `Debug` output) — that scheme is for "did this contract's Rust
  struct change" and is neither cryptographically strong nor independently
  reproducible from raw JSON by an external party. A digest that an approval
  token binds to for authorization needs both properties.
- **`proposal_snapshot_digest`**: hashes `proposal_digest` together with the
  manifest/registry/binding/policy/budget digests supplied by the caller
  (`SnapshotDigests`). This is the digest an approval token is actually
  scoped to (ADR-0050's "governing snapshots") — it changes if any pinned
  input changes even when the proposal JSON is byte-identical.

## Cross-validation (FR-004, FR-011)

`validate_proposal_against_host_state` runs after structural validation and
requires a loaded `ApplicationBundleManifest` and `CapabilityRegistry`:

1. **Declared capability set**: every node's `capability_id@capability_version`
   must appear in `manifest.components` — mirrors the existing "declared set
   is the only permitted set" pattern already used for manifest connector
   bindings in `traverse-cli`'s `app_activate_at`.
2. **Exact artifact pinning**: the registry's resolved artifact digest
   (`binary_digest`, falling back to `source_digest`) must equal the node's
   declared `artifact_digest`.
3. **Mapping schema compatibility**: source/target JSON Schema fragments at
   each mapping's path are resolved by walking `properties`/`items`
   segments; if both declare a `type`, it must match. This is intentionally
   bounded — not full JSON Schema validation — matching how this codebase
   already treats `inputs`/`outputs` schemas as documentation-grade shape,
   not a validator target.
4. **Field-level data-flow policy** (FR-011 — spec text names the rule but
   not byte-level semantics; this is this issue's concrete, documented
   interpretation): for a mapping from node A's output path to node B's
   input path,
   - A's `risk.data_flow.produced_data_classifications` **must** declare a
     classification at that exact path — an *undeclared* classification is
     never treated as safe (fail closed; this is what "schema compatibility
     alone MUST NOT authorize disclosure" means in practice).
   - B's `risk.data_flow.accepted_data_classifications` **must** declare a
     classification at the target path, and it must be `>=` what A
     produces (the ordering is `Public < Internal < Confidential <
     Restricted`, added to `DataClassification` for exactly this
     comparison).
   - If the produced classification is above `Public` and B's
     `effect_class` is `external_effect`/`irreversible_effect`, B's
     `egress_policy` must not be `Denied` — classified data cannot flow
     into a capability with zero declared legitimate egress surface.
   - Mappings sourced from `initial_input` are exempt from classification
     checks: FR-011 governs capability-to-capability data flow, not
     caller-supplied input, which has no capability-declared classification
     to check against.

## Authorization (FR-006, FR-006a)

`proposal_is_automatic_eligible` is the exact same
`traverse_contracts::is_automatic_eligible` check from spec `109`'s FR-005
risk-metadata work (issue `#1091`), applied to every resolved node — a
proposal is automatic-eligible only if **every** node is. The moment one node
is not (state-write, external/irreversible effect, non-deterministic, or
requires idempotency), the whole proposal requires a verified approval
token.

**Approval tokens** are Ed25519-signed, JWT-shaped tokens (`header.payload.signature`,
base64url, `alg` restricted to `EdDSA` — the same discipline as this repo's
existing HTTP bearer-token verification in `traverse-cli::http_api`, but a
parallel implementation rather than shared code: that code is hard-wired to
one global verification key and an HTTP-specific identity shape, with no
`kid`/multi-key, audience, or digest-binding concept to extend). Claims:

| Claim | Meaning |
|---|---|
| `jti` | Token id — the replay/use-count/revocation key. |
| `iss`, `aud` | Verified against host-configured expected values. |
| `sub` | The approving principal. |
| `workspace_id`, `proposal_digest`, `snapshot_digest` | Exact binding — any mismatch is rejected. |
| `permitted_effects`, `permitted_connectors` | Advisory scope hints carried through to the trace. |
| `max_use_count`, `exp` | Enforced by `ApprovalTokenStore` and time-claim comparison. |

`ApprovalTokenStore` is an in-memory, per-process ledger of use-count and
revocation state, keyed by `jti`. Tokens are short-lived and scoped to one
pinned snapshot, so no persistence across restarts is needed.

**This repo verifies approval tokens; it does not issue them.** Per
ADR-0050, the approving principal/service is external to Traverse — there is
no token-signing code here, only verification.

## Quotas (FR-007b)

`QuotaTracker` enforces independent concurrency ceilings per principal, app,
and workspace (`QuotaLimits`, default 4/16/32 concurrent executions). A
reservation is an RAII guard (`QuotaReservation`) that releases automatically
on drop, so a slot can never leak on an early return.

## Execution and trace (FR-007, FR-008, FR-008a, FR-009)

`execute_proposal` runs nodes one at a time in the canonicalized order.
Before each node, its input is assembled **solely** from the proposal's
declared mappings (JSON-Pointer get/set against `initial_input` and prior
nodes' outputs — no implicit full-output passthrough), then dispatched
through the existing `Runtime::execute` single-capability path (not the
branching/event-driven workflow engine in `workflows.rs`, which has fan-out,
event-wait, and state-merge semantics P1 explicitly excludes). The first
failed node stops execution; remaining nodes are marked
`skipped_after_earlier_failure` — no retry, no compensation, no graph or
catalog mutation.

The resulting `ProposalTrace` is deliberately narrow: proposal/snapshot
digests, the authorization summary (automatic vs. approval-token id — never
the raw token), per-node status, and mapping **paths** (never mapped
*values*). No raw input/output payloads, no secrets. `observe_proposal`
renders it directly to JSON for MCP consumption.

## MCP tool surface

`traverse_mcp::tools::proposals` exposes: `validate_proposal`,
`submit_proposal`, `authorization_state`, `execute_proposal_via_mcp`,
`observe_proposal`, `export_proposal`. A structurally or semantically invalid
proposal, a missing/invalid approval token, and an exhausted quota are all
**normal structured responses** (`valid: false` / `AuthorizationState::Invalid`
/ `ProposalExecutionResponse::Denied { code, message }`), never an `McpError`
— `McpError` is reserved for input that isn't even parseable JSON. Every
denial code is a stable `snake_case` string (FR-010).

## Non-goals (unchanged from spec 109)

Parallel execution, cycles, event waits, durable resume, automatic retries,
sagas, direct registry mutation, and planner implementation remain out of
scope for P1, matching the spec's own "Out of scope" section.
