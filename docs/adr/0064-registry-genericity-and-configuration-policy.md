# ADR-0064: Registry Genericity and Configuration-Evolution Policy

- Status: Accepted
- Date: 2026-09-08
- Governing spec: `1256-registry-genericity-policy` (Approved)
- Related issue: #1256
- Supersedes numbering: originally filed as ADR-0059, renumbered to 0064 to resolve a collision with `0059-mixed-registry-reference-activation`.

## Context

Registry admission needs a discoverable policy that distinguishes portable
business capabilities from application composition and host authority. Without
one, a domain-shaped record can conceal a product workflow, vendor binding, or
secret-bearing configuration behind an apparently reusable name.

## Decision

Public capabilities represent stable domain operations with typed contracts,
portable configuration semantics, and deterministic or explicitly mediated
behavior. Names describe the operation, not an app, UI, customer, vendor,
model, database, microphone, target, or host binding. Connectors own
unavoidable host authority; applications own composition and concrete binding.

Publication requires two materially distinct portable fixture/configuration
scenarios. A genuinely new primitive may instead present a documented,
reviewed exception rationale. This is evidence of portability, not a demand
for two shipping customers. Configuration is versioned and typed, records only
redacted provenance/reference names, evolves additively when existing meaning
is retained, and otherwise requires a major version plus migration guidance.

Automation rejects only objectively detectable structural leakage. Human
review remains responsible for semantic genericity and exception assessment.

## Consequences

The Registry gains a consistent admission checklist and redaction/compatibility
policy without pretending that a static validator can understand arbitrary
domain semantics. Publication work must supply portable fixture evidence and
may need a schema migration for incompatible configuration changes. Existing
records are audited only for confirmed violations.

## Alternatives considered

- Require two shipping customers: rejected because it blocks new primitives
  despite credible portable evidence.
- Permit one ordinary example: rejected because it does not demonstrate
  portability across materially different contexts.
- Infer genericity from source code: rejected because it is unreliable and
  would create a false approval signal.

## Approval evidence

The maintainer approved Spec 1256 on 2026-09-08. The approved-spec registry
records `1256-registry-genericity-policy` at version 0.1.0. This ADR records
that approved decision; it does not alter the immutable spec.
