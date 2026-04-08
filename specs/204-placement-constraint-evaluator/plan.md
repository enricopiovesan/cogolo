# Implementation Plan: Placement Constraint Evaluator

**Branch**: `204-placement-constraint-evaluator` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)

## Summary

Add a stateless, deterministic three-tier placement evaluator to `traverse-runtime` that resolves caller hints, contract constraints (`service_type` + `permitted_targets`), and `RuntimeSnapshot` load heuristics into a `PlacementDecision` or `PlacementError::NoEligibleTarget`, and records the decision in the execution trace.

## Technical Context

**Language/Version**: Rust 1.94+
**Primary Dependencies**: `serde` (serialization of decision types), `traverse-contracts` (capability contract types including `permitted_targets` from #208)
**Storage**: None — evaluator is stateless; decision written to the in-memory execution trace
**Testing**: 100% branch coverage via deterministic unit tests with injected `RuntimeSnapshot` fixtures; no panics, no unwraps
**Target Platform**: All (local, browser via WASM, cloud)
**Project Type**: Runtime subsystem — new module inside `traverse-runtime`
**Constraints**: No `unsafe`, no `unwrap()`, no `panic!()`, no TODOs; depends on #208 merging first (adds `permitted_targets` to contract types); must not change existing public API of `traverse-runtime` other than exposing the new `placement` module

## Constitution Check

Spec 010 is aligned with spec 006 (extends placement abstraction already reserved there) and does not drift from any approved contract. Gate will require `010` to appear in `approved-specs.json` before merge.

## Files Touched

```text
crates/traverse-runtime/src/placement/mod.rs        # CREATED — public module declaration re-exporting evaluator, types
crates/traverse-runtime/src/placement/evaluator.rs  # CREATED — three-tier evaluate() fn, tier helpers, load scoring
crates/traverse-runtime/src/lib.rs                  # MODIFIED — add `pub mod placement;`
crates/traverse-runtime/tests/placement_tests.rs    # CREATED — integration-level tests, 100% branch coverage
```

## Phase 0: Research

- Confirm `permitted_targets` and `service_type` field names on the contract type after #208 merges; adjust `PlacementRequest` field references accordingly.
- Read `crates/traverse-runtime/src/lib.rs` to identify the correct insertion point for `pub mod placement;` without breaking existing module graph.
- Read existing `RuntimeTrace` and `ExecutionTrace` types to determine the exact field to add the `placement` section to.
- Confirm `RuntimeSnapshot` does not already exist elsewhere in the codebase; if it does, reuse rather than duplicate.
- Check `cargo build` compiles cleanly on the branch before writing any new code.

## Phase 1: Types

Define in `crates/traverse-runtime/src/placement/mod.rs`:

```rust
// Target platform discriminant — extend as new targets are added
pub enum PlacementTarget { Local, Browser, Cloud }

// Explains which tier produced the decision
pub enum PlacementReason { CallerHint, ContractSingleton, RuntimeHeuristic }

// Reflects how much evidence backs the decision
pub enum PlacementConfidence { High, Medium, Low }

// Successful placement decision recorded in trace
pub struct PlacementDecision {
    pub target: PlacementTarget,
    pub reason: PlacementReason,
    pub confidence: PlacementConfidence,
}

// Only failure variant for this spec
pub enum PlacementError { NoEligibleTarget }

// Load scores normalized to [0.0, 1.0]; injected by caller
pub struct TargetLoad {
    pub target: PlacementTarget,
    pub cpu:    f32,
    pub memory: f32,
    pub power:  f32,
}

pub struct RuntimeSnapshot {
    pub loads: Vec<TargetLoad>,
}

// Input to the evaluator
pub struct PlacementRequest {
    pub caller_hint:  Option<PlacementTarget>,
    pub permitted_targets: Vec<PlacementTarget>, // from capability contract (#208)
    pub snapshot:     RuntimeSnapshot,
}
```

All types derive `Debug`, `Clone`, and `serde::{Serialize, Deserialize}` where applicable.

## Phase 2: Logic

Implement in `crates/traverse-runtime/src/placement/evaluator.rs`:

**`pub fn evaluate(req: &PlacementRequest) -> Result<PlacementDecision, PlacementError>`**

Tier 1 — Caller hint:
- If `caller_hint` is `Some(t)` and `t` is in `permitted_targets`, return `PlacementDecision { target: t, reason: CallerHint, confidence: High }`.
- If hint is present but not in `permitted_targets`, discard and continue (hint rejection is surfaced via the returned `PlacementDecision` chain; caller sees `reason` will not be `CallerHint`).

Tier 2 — Contract filtering:
- Intersect `permitted_targets` with the known target universe.
- If result is empty, return `Err(PlacementError::NoEligibleTarget)`.
- If exactly one target remains, return `PlacementDecision { target, reason: ContractSingleton, confidence: High }`.

Tier 3 — Heuristic scoring:
- For each remaining candidate, look up its `TargetLoad` in `snapshot.loads`; compute composite score as `(cpu + memory + power) / 3.0`.
- Reject any candidate whose composite score exceeds `0.9`; if all are rejected, return `Err(PlacementError::NoEligibleTarget)`.
- Sort surviving candidates by composite score ascending, then by target name lexicographically as a stable tie-break.
- Set `confidence` to `High` if the winning score is below `0.5`, `Medium` if below `0.75`, `Low` otherwise.
- Return `PlacementDecision { target: winner, reason: RuntimeHeuristic, confidence }`.

All helpers are private. `evaluate` is the only exported symbol from `evaluator.rs`; `mod.rs` re-exports it.

## Phase 3: Tests

Write in `crates/traverse-runtime/tests/placement_tests.rs` — one test function per scenario, no shared mutable state:

**Tier 1 tests**
- `hint_valid_accepted`: hint in `permitted_targets` → `reason == CallerHint`, `confidence == High`.
- `hint_invalid_falls_through`: hint not in `permitted_targets` → reason is not `CallerHint`; decision still returned.
- `no_hint_skips_tier1`: `caller_hint: None` → evaluator returns a decision via tiers 2/3 without error.

**Tier 2 tests**
- `contract_singleton_selected`: one permitted target → `reason == ContractSingleton`, `confidence == High`.
- `contract_empty_returns_error`: zero permitted targets → `Err(NoEligibleTarget)`.
- `contract_filters_unapproved_targets`: permitted = `[cloud]`, snapshot has `local` healthy → cloud selected.

**Tier 3 tests**
- `heuristic_selects_lowest_load`: two targets with different composite scores → lower-scored target wins.
- `heuristic_tie_break_lexicographic`: identical scores → lexicographically first target name wins, `confidence == Low`.
- `heuristic_all_overloaded_returns_error`: all candidates above `0.9` composite → `Err(NoEligibleTarget)`.
- `deterministic_same_inputs`: call `evaluate` twice with identical inputs → both results are identical.

**Coverage target**: 100% branch coverage across `evaluator.rs`; CI gate blocks merge if coverage drops.

## Implementation Sequence

1. Confirm #208 is merged; read updated contract types to verify `permitted_targets` field names.
2. Read `crates/traverse-runtime/src/lib.rs` — identify insertion point for `pub mod placement;`.
3. Create `crates/traverse-runtime/src/placement/mod.rs` — define all types.
4. Create `crates/traverse-runtime/src/placement/evaluator.rs` — implement `evaluate()` and all tier helpers.
5. Modify `crates/traverse-runtime/src/lib.rs` — add `pub mod placement;`.
6. Create `crates/traverse-runtime/tests/placement_tests.rs` — write all test cases listed in Phase 3.
7. Run `cargo build` — fix any compilation errors before proceeding.
8. Run `cargo test` — all tests must pass with zero warnings.
9. Run `bash scripts/ci/spec_alignment_check.sh` — confirm gate passes (requires spec 010 in `approved-specs.json`).
10. Commit and push; open PR referencing issue #204 and note dependency on #208.
