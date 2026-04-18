# Unified Architectural Blueprint (UAB)

This document is the minimal cross-spec skeleton that keeps Traverse from drifting into a set of incompatible “governing specs.”

It defines **interfaces between specs**, shared vocabulary, and the cross-cutting invariants that every governing spec must respect.

It intentionally does **not** define the specs themselves.

## Timebox And Output

- Target: a first usable draft in **7 days** (timeboxed).
- Output: stable interface notes + a short list of explicit decisions (with owners).

If it takes longer than a week, it is trying to become a book. Stop and cut scope.

## Non-Goals

- No attempt to fully specify OpenTelemetry, identity, registry behavior, or persistence in this document.
- No attempt to define “the perfect” taxonomy; only what other specs must plug into.

## The Problem This Solves

Several critical specs are coupled:

- Observability signals must cross the WASM boundary.
- Identity must be propagated through runtime execution and event publication/subscription.
- Enforcement needs contract registries (capability + event) and error taxonomy.
- The insulation layer affects runtime, host interfaces, and tooling.

If those specs are written independently, they will conflict and create “spec blockade.”

## Shared Vocabulary (Canonical Terms)

These terms should be used consistently across governing specs and implementation:

- **Capability**: an executable unit registered with a contract and an implementation artifact.
- **Contract**: the governed JSON document defining inputs/outputs/constraints and lifecycle.
- **Event Contract**: governed JSON defining an emitted event’s schema and metadata.
- **Workflow**: a governed composition of capability steps.
- **Runtime Request**: a request to execute a capability (or workflow) with input and placement intent.
- **Execution ID**: stable identifier for an execution attempt (used for trace + subscription).
- **Trace**: deterministic execution artifact with public/private tiers (see trace-tiering specs).
- **Host**: the environment providing bindings (filesystem/network/storage/clock) and enforcement.
- **Module**: a WASM artifact that runs under a host with explicit declared needs/permissions.

### Identity Terms

- **Subject**: end-user identity (human or service principal).
- **Actor**: the calling agent/tool identity (the invoker).
- **Module Identity**: the cryptographic identity of a module (origin/provenance).
- **Host Identity**: the identity of the environment enforcing policy.

## Cross-Spec Interface Map (What Must Plug Into What)

Each section below lists:

- the interface surface that other specs must depend on, and
- the invariants that must stay stable.

### Observability Interfaces (Owned By #329)

Inputs from other specs:

- Runtime Request: execution id, capability id, workflow id (if any)
- Identity: subject/actor/module identity tags for attribution
- Errors: stable classification

Outputs to other specs:

- Stable signal naming (attribute keys) and propagation rules across WASM boundary
- Minimal deterministic span tree model for a canonical execution

Invariants:

- Prefer OpenTelemetry semantic conventions whenever applicable.
- Do not create proprietary attribute names when an OTel convention exists.

### Security And Identity Interfaces (Owned By #337, #339)

Inputs from other specs:

- Registry: module/capability identifiers, provenance fields, digests
- Runtime: execution boundary and host bindings

Outputs to other specs:

- Module trust contract: how a host validates origin (signing/provenance), what is verified, and when.
- Identity propagation contract: how subject/actor identity flows from invoker to runtime to events.
- Threat model: explicit blast radius assumptions (module compromise, host compromise, registry tamper).

Invariants:

- Verification must be deterministic and auditable.
- Identity must be cryptographically attributable where required (no “stringly typed” identity).

### Contractual Rigor Interfaces (Owned By #332)

Inputs from other specs:

- Registry specs (capability + event) define what “registered” means.
- Runtime request spec defines what must be validated pre-execution.

Outputs to other specs:

- Enforcement points: compile time (authoring), registration time, execution time.
- Error taxonomy: which failures are validation vs conflict vs execution and how they surface.

Invariants:

- No execution/deploy without satisfying the registered contract and event catalog when applicable.
- Validation failures must be actionable without log archaeology.

### WASI / Component Model Insulation Interfaces (Owned By #330)

Inputs from other specs:

- Host bindings surface (filesystem, clock, network, storage) needs stable abstraction.
- Build/packaging flow for modules.

Outputs to other specs:

- A stable “Traverse Host ABI” surface that hides WASI churn.
- Version policy: how insulation changes without breaking modules.

Invariants:

- Insulation must not become the bottleneck for new WASI features: define an extension mechanism.

### Ecosystem / Connectors Interfaces (Owned By #331, #338)

Inputs from other specs:

- Registry resolution rules
- Identity and permission model

Outputs to other specs:

- Plugin contract: what a connector is, how it is declared, versioned, and discovered.
- Registry compatibility model: module A vX can depend on connector B vY under host Z.

Invariants:

- Dependency resolution must be deterministic and reproducible.

### Universal Data Access Interfaces (Owned By #335)

Inputs from other specs:

- Identity model (data access is identity-scoped)
- Host bindings surface

Outputs to other specs:

- Portable persistence contract: capability declares needs; host provides binding.
- Consistency stance: AP leaning vs CP leaning and the required primitives for sync/merge.

Invariants:

- Offline-to-online sync must be explicitly defined for the portable knowledge app use case.
- Conflict handling must be deterministic.

## Minimal Error Taxonomy (Cross-Cutting)

Every spec that defines failure must map to one of these classes:

- **Usage**: user input/CLI shape errors
- **Validation**: contract or schema violations (deterministic)
- **Conflict**: registry immutability / duplicate version conflicts
- **I/O**: filesystem and host binding errors
- **Execution**: runtime or module execution failures

If a spec invents a new category, it must justify it and update this section.

## Decision Log

When this document forces a decision, capture it with:

- Decision:
- Alternatives considered:
- Chosen:
- Why:
- Owner:
- Date:
- Follow-ups (tickets):

## Current Explicit Decisions Needed (To Unblock Work)

- Observability: traces-only first vs traces+logs vs traces+metrics (owner: #329).
- Identity: module signing scheme and verification policy (owner: #337/#339).
- Consistency stance for portable knowledge sync (owner: #335).
- Extension story for insulation layer when WASI adds new features (owner: #330).

