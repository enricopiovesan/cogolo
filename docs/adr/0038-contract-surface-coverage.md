# ADR-0038: Contract Surface Must Be Covered by Use Cases

- Status: Accepted (amended 2026-08-10 for Decision 58)
- Governing spec: `102-contract-surface-coverage` (v1.1.0 Approved)
- Related issues: traverse#1014, #1015, #1016, #1040; registry#192, #193, #215

## Context

Capability contracts combine:

1. Free-text `summary` / `description`
2. JSON Schema input/output surface (enums and required properties)
3. `use_cases[]` with concrete input/output examples
4. An executable WASM artifact verified by package smoke

Only (3) and (4) are mechanically exercised. Two failure modes appeared:

- `core.process-comment@1.0.0` overclaimed `action` enum values beyond its use-case/smoke matrix.
- Loop batch publish validated use cases from raw JSON, then wrote a normalized `CapabilityContract` **without** a `use_cases` field, so registry copies lost them while CI still allowed missing use cases.

## Decision

Adopt **schema ⊆ use_cases ⊆ smoke** for the full checkable schema surface (Decision 58):

- Every input schema string enum value MUST appear in at least one use case input example.
- Every `inputs.schema.required` property MUST appear in at least one use case input example (no cartesian product).
- Every `reason_code` / `status` output enum value MUST appear in at least one use case output example; those fields MUST be enums when coverage is required.
- `use_cases` MUST be non-empty and MUST survive `capability publish` into the registry record.
- Each use case MUST have a matching smoke fixture asserting its `reason_code` / key outputs.
- Description claims beyond use cases MUST be called out under **Known limitations** or removed.
- An enum value MUST NOT be “implemented” solely as an undocumented generic unsupported stub.

For already-published gaps: do not edit immutable versions; publish an honesty bump and deprecate the dishonest version with an explicit reason.

## Alternatives Considered

- **Minimum use-case counts** — rejected: owner requires coverage of the declared capability surface, not N examples.
- **Cartesian required-field matrices** — rejected as an impractical publish gate.
- **Description-only linting (NLP)** — rejected for v1: high false-positive risk; use cases are the executable contract.
- **CLI-only or registry-only enforcement** — rejected: authors need fail-fast publish dry-run; registry must still reject bypasses.
- **Parallel new coverage spec** — rejected: amend Spec 102 / registry FR-011 instead.

## Consequences

- Traverse Spec 102 v1.1.0 + expanded publish coverage checker (#1040).
- Registry FR-011 becomes MUST for new/changed contracts; CI mirror (#215).
- Honesty patch-bumps for stripped Loop capabilities follow under FR-010.
