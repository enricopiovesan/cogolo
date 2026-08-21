# Runtime Workflow Proposals and Authorization (P1)

**Status**: Approved
**Canonical governing ID**: `109-runtime-workflow-proposals`  
**Version**: 0.1.0  
**Implements phase**: P1 of `108-governed-runtime-workflow-composition`  
**Input**: Issue #1089; decisions on #865.

## Purpose

Define a provider-neutral MCP lifecycle for an ephemeral, manifest-constrained
workflow proposal that executes as a bounded sequential DAG.

## Requirements

- **FR-001**: The public MCP surface MUST support discovery, compatibility
  inspection, proposal submission, validation feedback, authorization state,
  execution, observation, and proposal export; it MUST NOT require a
  Traverse-hosted LLM or planner credential.
- **FR-002**: A proposal MUST identify its app-manifest version/digest,
  workspace scope, capability IDs and versions, explicit directed edges, and
  explicit source/target JSON paths for every data mapping.
- **FR-003**: The runtime MUST canonicalize a proposal and bind its digest to
  pinned manifest, registry, binding, policy, and budget snapshots.
- **FR-004**: Validation MUST reject undeclared capabilities/connectors,
  incompatible schemas, invalid mappings, unavailable/yanked versions,
  disallowed placement, and any attempt to weaken manifest policy.
- **FR-005**: Capability contracts MUST declare exactly one baseline risk:
  `read_only`, `state_write`, or `external_side_effect`. A manifest may only
  tighten the required authorization.
- **FR-006**: A valid proposal containing only automatic-eligible risk classes
  and budgets MAY run automatically. Private connectors/secrets,
  side-effecting operations, or policy/budget exceptions MUST require an
  approval token bound to proposal digest, manifest identity/version,
  workspace scope, and expiry.
- **FR-007**: P1 MUST accept only acyclic graphs below configured node, edge,
  mapping, payload, validation-time, execution-time, and resource limits, and
  execute one node at a time in deterministic topological order.
- **FR-008**: The runtime MUST stop at the first failed node. It MUST NOT
  perform an implicit retry, compensation, graph mutation, or runtime catalog
  registration. A new execution is required for retry.
- **FR-009**: The trace MUST retain a bounded redacted snapshot: graph digest,
  identities/versions/digests, mapping paths, authorization decision, budgets,
  per-node status, and terminal outcome. It MUST exclude prompts, secrets,
  private config, and arbitrary raw payload copies.
- **FR-010**: All denials and failures MUST use stable machine-readable codes
  and secret-free details.

## Acceptance scenarios

1. A client proposes two manifest-declared read-only capabilities with a
   schema-valid explicit mapping; validation deterministically accepts and
   sequential execution produces an auditable redacted trace snapshot.
2. A proposal includes an external-side-effect capability; it cannot execute
   without a valid bound approval token.
3. A proposal maps an undeclared/private field or references a capability not
   allowed by the manifest; validation rejects before invocation.
4. A node fails after an earlier state write; no later node, retry, or
   compensation runs, and the trace distinguishes succeeded, failed, and
   not-started nodes.
5. Re-submitting pinned identical inputs produces the same proposal digest and
   validation decision.

## Out of scope

Parallel execution, cycles, event waits, durable resume, automatic retries,
sagas, direct registry mutation, and planner implementation.
