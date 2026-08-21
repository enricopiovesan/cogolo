# Governed Runtime Workflow Composition — North Star

**Status**: Approved
**Canonical governing ID**: `108-governed-runtime-workflow-composition`  
**Version**: 0.1.0  
**Input**: Issue #1089; decisions recorded on #865.  
**Extends**: `007-workflow-registry-traversal`, `015-capability-discovery-mcp`, `018-event-driven-composition`, `042-mcp-library-surface`, `044-application-bundle-manifest`, `039-connector-plugin-architecture`, `103-application-connector-binding`, `104-mediated-connector-invocation`.

## Purpose

Define the complete, phased destination for UMA-style workflow composition at
runtime. An MCP client or agent discovers declared capabilities, proposes a
workflow, and Traverse remains the authority that validates, authorizes,
executes, and explains the result. This document is architectural and does
not make future phases available before their successor specs are approved.

## North-star invariants

- The planner is an untrusted proposer, never the authority.
- A proposal is constrained by its versioned application manifest; it cannot
  add capabilities, connectors, permissions, placement targets, or secrets.
- Capability authority metadata is portable and immutable. Apps may tighten it
  but cannot weaken it.
- Authority is evaluated on four independent portable dimensions: effect
  class, determinism class, field-level data classification/egress policy,
  and reliability semantics. A single label MUST NOT imply another dimension.
- Given the same canonical proposal, manifest, registry, binding, policy, and
  host inputs, validation, authorization, resolution, and execution planning
  are deterministic.
- Public evidence is bounded and secret-free. Hosts retain credentials,
  private configuration, paths, and raw private data.
- No proposal mutates a manifest, registry, or reusable workflow catalog.
- Runtime execution has explicit node, edge, payload, time, concurrency, and
  resource budgets owned by an authenticated principal/app/workspace, and
  fails closed on a violated bound.

## Phases

| Phase | Governing successor | Outcome | Not included before approval |
|---|---|---|---|
| P1 | `109-runtime-workflow-proposals` | Ephemeral, bounded sequential DAG proposal lifecycle | parallelism, waits, retries, compensation, catalog mutation |
| P2 | `110-bounded-parallel-workflow-scheduling` | Deterministic bounded branches and joins | dynamic fan-out, cycles, durable waits |
| P3 | `111-durable-dynamic-orchestration` | Checkpointed waits, retry, cancellation, and compensation | implicit transactions or unbounded recovery |
| P4 | `112-governed-workflow-promotion` | Export and review-based promotion to reusable workflows | direct runtime catalog/manifest mutation |

## Capability boundary

Traverse provides discovery, compatibility inspection, proposal submission,
validation, authorization, execution, observation, and export surfaces. It
does not provide or require an LLM planner. An MCP client may use any planner
that can submit the governed proposal format.

Every proposal is authenticated and tenant/workspace scoped. Runtime
determinism applies to canonicalization, validation, authorization, artifact
resolution, scheduling, and evidence ordering over pinned inputs; it does not
claim identical external connector, clock, model, or remote-state results.

## Cross-phase acceptance criteria

1. A client can explain every accepted or rejected proposal from stable
   decision evidence without exposing a secret or private host identifier.
2. An app cannot use a proposal to weaken declared authority metadata, add authority, or
   invoke an undeclared capability/connector.
3. Equivalent pinned inputs yield the same graph digest, decision, execution
   order, resolved versions, and stable error codes.
4. Future features remain disabled unless their governing successor is
   approved and their required host/resource controls are present.

## Out of scope

- A Traverse-operated model/provider or storage of planner prompts.
- General-purpose user code or mapping scripts in a proposal.
- Treating a proposal as a transactional distributed workflow.
