# ADR-0050: Govern Runtime Workflow Proposals as Untrusted, Manifest-Bounded Snapshots

- Status: Accepted
- Renumbered: 2026-08-24 from ADR-0041 (that number collided with the
  already-Accepted `docs/adr/0041-cross-host-embedded-registry-cache.md`)
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
Capability authority metadata is immutable and portable: effect class,
determinism class, field-level data classification/egress policy, and
reliability semantics. App manifests may only tighten it.
P1 is a bounded sequential DAG, stops on first failure, and records a
redacted trace attachment. It has no implicit retries, compensation, graph
mutation, manifest mutation, or catalog registration.

Approval tokens are verified, scoped, bounded-use credentials tied to the
canonical proposal and governing snapshots. JSON-schema compatibility never by
itself permits a classified field to reach a target or external connector.

## Consequences

Traverse stays provider-neutral and never needs planner credentials or prompts.
MCP clients can still provide UMA-style planning experiences. Authorization
tokens gate sensitive proposals and bind to snapshot identity. Parallelism,
durability, and promotion remain separately governed phases.
Runtime determinism is limited to decisions, resolution, scheduling, and
evidence ordering over pinned inputs; external results are declared separately.

## Alternatives considered

- Embed a Traverse-hosted model planner: rejected for credential, privacy,
  provider lifecycle, latency, and authority expansion.
- Let app manifests classify risk alone: rejected because it permits drift and
  accidental weakening.
- Persist proposals as workflows automatically: rejected because runtime
  inference would mutate deployable governed catalogs.
