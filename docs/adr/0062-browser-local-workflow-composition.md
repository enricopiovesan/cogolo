# ADR-0062: Browser-Local Deterministic Workflow Composition

- Status: Accepted
- Date: 2026-09-08
- Governing spec: `1277-browser-local-workflow-composition` (Approved)
- Related issues: #1271, #1277, #1269, #1270

## Context

The backend-less `/discover` experience needs a real proposal-producing path.
Accepted ADR-0043 requires deterministic, data-grounded planning with no
automatic candidate choice; ADR-0050 makes every planner an untrusted
proposer. The remaining question was whether the browser should plan locally
from a verified snapshot or call a public governed endpoint.

## Decision

Planning runs fully in the browser over an already synced, verified,
digest-pinned public registry snapshot. It produces only untrusted,
versioned workflow proposals; the local governed runtime remains the sole
validator, authorizer, and executor. Exact prepared `registry_ref`
components follow the offline, fail-closed lifecycle of Spec 1258.

Traverse will not add a public remote planning endpoint for this scope. The
browser neither refreshes the registry nor falls back to a bundle during
planning, validation, or execution.

## Consequences

`/discover` can remain backend-less and provider-neutral without hosted model
credentials, CORS policy, anonymous abuse controls, or a service operator.
The browser planner and local composed-execution adapter are explicit
downstream work with bounded inputs and redacted evidence. A remote endpoint
is a future architectural change requiring its own approved spec and ADR.

## Alternatives considered

- A public governed planning endpoint: rejected because it changes the
  backend-less commitment and requires deployment, CORS/auth, abuse controls,
  and an operational owner.
- Browser name/namespace heuristics or automatic choice: rejected by
  ADR-0043's deterministic structural-planning boundary.
- A hosted model planner: rejected by ADR-0050's provider, credential,
  privacy, and authority constraints.
