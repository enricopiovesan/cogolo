# Feature Specification: Placement Constraint Evaluator

**Feature Branch**: `204-placement-constraint-evaluator`
**Spec ID**: 024
**Created**: 2026-04-08
**Status**: Draft
**Input**: GitHub issue #204

> **Governance note**: This spec governs a new deterministic evaluation subsystem inside `traverse-runtime`. It extends spec 006-runtime-request-execution by replacing the stub `local`-only placement constant with a three-tier evaluator. It depends on issue #208 landing first to add `service_type` and `permitted_targets` fields to capability contracts — no placement evaluation can be implemented against contract constraints until those fields exist.

## Context

Spec 006 deferred placement to a fixed `local` constant, preserving the abstraction without implementing it. As Traverse targets browser, edge, and cloud runtimes, the runtime must decide deterministically which execution target to use for each request. The decision depends on three ordered tiers: a caller-supplied hint (cheap to check, honored when valid), contract-level constraints (`service_type` and `permitted_targets` fields from #208, authoritative), and runtime heuristics (CPU, memory, and power load scores from a `RuntimeSnapshot`, used only when the contract permits multiple targets).

The evaluator is a stateless, injected function — it receives a `PlacementRequest` and an immutable `RuntimeSnapshot` and returns either a `PlacementDecision` (target, reason, confidence) or a `PlacementError::NoEligibleTarget`. The result is recorded in the public execution trace so every placement choice is explainable and auditable, consistent with the tiered trace model established in spec 006.

## User Scenarios & Testing

### User Story 1 — Caller hint accepted when valid (Priority: P1)

As a runtime consumer, I want my target hint to be honored when the contract permits it and runtime load is acceptable, so that I can steer execution toward a preferred target without the runtime ignoring my intent.

**Why this priority**: Hint acceptance is the fast path and establishes the basic three-tier contract. Without it, callers have no way to influence placement.

**Independent Test**: Construct a `PlacementRequest` with a caller hint matching one of the contract's `permitted_targets`. Inject a `RuntimeSnapshot` with low load scores for that target. Verify the evaluator returns `PlacementDecision { target: <hint>, reason: CallerHint, confidence: High }`.

**Acceptance Scenarios**:

1. **Given** a capability with `permitted_targets: [local, browser]` and a caller hint of `local`, **When** the evaluator runs tier 1, **Then** it returns `PlacementDecision { target: local, reason: CallerHint, confidence: High }` without consulting tier 3.
2. **Given** a caller hint that names a target not listed in `permitted_targets`, **When** the evaluator runs tier 1, **Then** it discards the hint, records the rejection reason in the trace, and advances to tier 2.
3. **Given** a `PlacementRequest` with no caller hint, **When** the evaluator runs, **Then** tier 1 is skipped cleanly and tier 2 runs immediately.

### User Story 2 — Contract constraints filter ineligible targets (Priority: P1)

As a platform developer, I want the evaluator to enforce `service_type` and `permitted_targets` from the capability contract so that a capability is never placed on a target its contract does not allow.

**Why this priority**: Contract authority is non-negotiable. Heuristics and hints are advisory; contracts are law.

**Independent Test**: Construct a capability contract with `permitted_targets: [cloud]`. Inject a `RuntimeSnapshot` that has healthy scores for `local` and `browser`. Supply no caller hint. Verify the evaluator eliminates `local` and `browser` at tier 2, leaving only `cloud` as the candidate set before tier 3 runs.

**Acceptance Scenarios**:

1. **Given** a contract with `permitted_targets: [cloud]`, **When** tier 2 runs, **Then** only `cloud` remains in the candidate set regardless of runtime load scores.
2. **Given** a contract with `permitted_targets` that yields an empty candidate set after filtering, **When** tier 2 completes, **Then** the evaluator returns `PlacementError::NoEligibleTarget` without entering tier 3.
3. **Given** a contract with `permitted_targets: [local, browser, cloud]`, **When** tier 2 runs, **Then** all three remain as candidates and tier 3 proceeds to score them.

### User Story 3 — Heuristics select the lowest-load eligible target (Priority: P2)

As a platform developer, I want the evaluator to score eligible targets by CPU, memory, and power load from the `RuntimeSnapshot` and select the least-loaded one, so that execution is placed where the runtime has capacity.

**Why this priority**: Heuristic selection prevents hot targets from being overloaded when the contract permits alternatives.

**Independent Test**: Construct a `RuntimeSnapshot` with `local` at 90% CPU load and `browser` at 20% CPU load. Provide a contract permitting both. Verify the evaluator selects `browser` with `reason: RuntimeHeuristic` and a `confidence` that reflects the load delta.

**Acceptance Scenarios**:

1. **Given** two eligible targets with measurably different composite load scores, **When** tier 3 runs, **Then** the evaluator selects the target with the lowest composite score and sets `reason: RuntimeHeuristic`.
2. **Given** two eligible targets with identical composite load scores, **When** tier 3 runs, **Then** the evaluator selects deterministically (lexicographic target name order) and sets `confidence: Low`.
3. **Given** a `RuntimeSnapshot` where all eligible targets exceed a maximum load threshold, **When** tier 3 runs, **Then** the evaluator returns `PlacementError::NoEligibleTarget` rather than placing on an overloaded target.

## Requirements

- **FR-001**: The evaluator MUST accept a `PlacementRequest` (containing an optional caller hint, the resolved capability contract, and a reference to the current `RuntimeSnapshot`) and return either a `PlacementDecision` or a `PlacementError`.
- **FR-002**: Tier 1 MUST check the caller hint against `permitted_targets`; if the hint is absent or invalid it MUST be discarded with a structured rejection note and evaluation MUST continue to tier 2.
- **FR-003**: Tier 2 MUST filter the full target universe to only those listed in the capability contract's `permitted_targets` field; if the result is empty the evaluator MUST return `PlacementError::NoEligibleTarget` immediately.
- **FR-004**: Tier 3 MUST score remaining candidates using CPU load, memory load, and power load from the `RuntimeSnapshot`; the composite score MUST be computed deterministically from those three fields.
- **FR-005**: The evaluator MUST be a stateless function — it MUST NOT retain mutable state between calls and MUST produce identical output for identical inputs.
- **FR-006**: The `PlacementDecision` MUST carry `target`, `reason` (one of `CallerHint`, `ContractSingleton`, `RuntimeHeuristic`), and `confidence` (one of `High`, `Medium`, `Low`); the evaluator MUST record this decision in the execution trace.

## Success Criteria

- **SC-001**: The evaluator honors a valid caller hint without consulting tier 3, confirmed by unit test inspecting the returned `reason` field.
- **SC-002**: The evaluator returns `PlacementError::NoEligibleTarget` when contract filtering eliminates all targets, confirmed by unit test with a zero-candidate contract.
- **SC-003**: The evaluator selects the least-loaded target from the `RuntimeSnapshot` when multiple targets are permitted, confirmed by unit test with controlled load scores.
- **SC-004**: The evaluator is deterministic — running it twice with identical `PlacementRequest` and `RuntimeSnapshot` inputs always returns the same `PlacementDecision`, confirmed by property-based or repetition test.

## Assumptions

- Issue #208 lands before this spec is implemented; `service_type` and `permitted_targets` are present on the capability contract type before any placement evaluation code is written.
- `RuntimeSnapshot` is injected by the caller and treated as immutable by the evaluator; the evaluator does not query runtime state directly.
- The initial target universe is `{ local, browser, cloud }` for scoring purposes; new targets added later extend this list without changing evaluator logic.
- Load scores in `RuntimeSnapshot` are normalized to `[0.0, 1.0]`; values above `0.9` are considered overloaded.
- The execution trace plumbing from spec 006 is already in place; this spec extends the trace with a `placement` section, it does not replace the trace model.
