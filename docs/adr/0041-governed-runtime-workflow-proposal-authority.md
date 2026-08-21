# ADR-0041: Govern Runtime Workflow Proposals as Untrusted, Manifest-Bounded Snapshots

- Status: Accepted
- Governing specs: `108-governed-runtime-workflow-composition`, `109-runtime-workflow-proposals`
- Related issues: #865, #1089

## Context

MCP clients can discover and invoke Traverse artifacts, but cannot safely
create a runtime workflow from discovered capabilities. UMA's target is an
agent-assisted proposal with runtime authority, not agent-controlled runtime
mutation.

## Decision

The planner is an untrusted external proposer. Traverse accepts a canonical,
immutable proposal only if its graph is within an app manifest's declared
authority and all contract, connector, placement, schema, risk, policy, and
resource checks pass. The runtime is deterministic over pinned snapshots.
Capability risk is immutable portable contract metadata (`read_only`,
`state_write`, `external_side_effect`); app manifests may only tighten it.
P1 is a bounded sequential DAG, stops on first failure, and records a
redacted trace attachment. It has no implicit retries, compensation, graph
mutation, manifest mutation, or catalog registration.

## Consequences

Traverse stays provider-neutral and never needs planner credentials or prompts.
MCP clients can still provide UMA-style planning experiences. Authorization
tokens gate sensitive proposals and bind to snapshot identity. Parallelism,
durability, and promotion remain separately governed phases.

## Alternatives considered

- Embed a Traverse-hosted model planner: rejected for credential, privacy,
  provider lifecycle, latency, and authority expansion.
- Let app manifests classify risk alone: rejected because it permits drift and
  accidental weakening.
- Persist proposals as workflows automatically: rejected because runtime
  inference would mutate deployable governed catalogs.
