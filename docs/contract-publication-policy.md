# Contract Publication Policy

This document defines the initial publication policy for Traverse capability, event, and workflow contracts.

## Core Rule

A contract is not publishable just because it exists or validates locally.

Publication requires both:

- passing automated validation and governance checks
- explicit manual approval before publication

This is the default Traverse rule for `v0.1`.

## Lifecycle Model

The practical lifecycle for publishable contracts is:

1. `draft`
2. approved for publication
3. published
4. `deprecated`
5. `retired`
6. `archived`

`draft` means the contract may still evolve and is not yet considered publishable.

Published contracts are immutable records once released under a given identity and version.

## Publication Gate

For a contract to move from `draft` toward publication, the following must be true:

- the governing spec is approved and declared
- required contract validation passes
- required CI and merge-gating checks pass
- semver and immutability rules pass
- required ownership, provenance, and evidence metadata are present
- the change is reviewed and explicitly approved by a human steward

Traverse does not allow fully automatic publication based only on CI success in `v0.1`.

## Approval Rule

The publication decision is:

- CI-gated
- manually approved

CI proves that a candidate contract is technically valid.
Manual approval confirms that the contract is ready to become a governed published artifact.

## Registry Semantics

The registry should treat published contracts as:

- immutable per identity and version
- discoverable
- versioned
- provenance-aware

The registry must reject republishing the same identity and version when governed content differs.

## Applies To

This publication policy applies to:

- capability contracts
- event contracts
- workflow contracts or workflow-backed governed artifacts

## Non-Goals for v0.1

This policy does not yet define:

- a full multi-stage approval workflow engine
- delegated approval roles
- distributed/federated publication
- signed publication attestations

Those may be added later without changing the core rule that publication is CI-gated plus manual approval.
