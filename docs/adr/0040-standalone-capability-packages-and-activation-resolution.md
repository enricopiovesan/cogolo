# ADR-0040: Standalone Capability Packages and Activation-Time Artifact Resolution

- Status: Accepted
- Governing specs: `105-standalone-capability-packages`, `106-activation-artifact-resolution`
- Related issues: #1057, #1058, #1059, #1060, #1061

## Context

The capability-package validator requires a package to declare an approved
workflow reference. That reverses the intended dependency: a workflow composes
capability contracts, while a package is one portable executable implementation
of a capability. It blocks independently useful packages and encourages
synthetic workflow references. In addition, workflow registration proves that a
contract exists, but cannot truthfully prove that a specific host can execute it.

## Decision

Capability packages may be standalone. `known_compositions` is optional,
advisory metadata; it is never an execution prerequisite. Schema v1 accepts
`workflow_refs` as a deprecated alias for one supported minor release. Different
values in both fields are invalid, and schema v2 removes the alias.

Workflows continue to reference capability contracts, not package identities.
At host activation, each required contract resolves to a compatible active,
verified executable package. An exact package pin wins; otherwise the resolver
selects the highest compatible active version and then package id. The immutable
activation record contains the selected identity, digest, resolver version, and
eligibility evidence. Execution consumes that record and fails closed on drift.

## Consequences

Standalone packages are inspectable and executable without application knowledge.
Portable workflow registration remains independent of host inventory. Hosts gain
deterministic, auditable eligibility checks before traffic reaches execution.
Existing manifests remain valid for the defined migration window.

## Alternatives Considered

- Keep mandatory workflow references: rejected because it creates reverse
  coupling and false composition metadata.
- Remove workflow metadata immediately: rejected as an unnecessary breaking
  change that loses useful discovery information.
- Resolve a package during workflow registration: rejected because registry
  portability would depend on one host installation.
- Resolve at every execution: rejected because selection could drift silently.
- Require a package pin in every application: rejected because applications
  should remain portable; pins remain an optional strict-reproducibility choice.
