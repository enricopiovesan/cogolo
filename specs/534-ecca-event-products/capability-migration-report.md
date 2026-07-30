# ECCA Existing-Catalog Migration Report

**Governing spec**: `534-ecca-event-products@1.0.0` (FR-020)
**Governing issue**: traverse-framework/traverse#899
**Machine-readable source of truth**: `contracts/governance/ecca-capability-inventory.json`
**Mechanically checked by**: `crates/traverse-contracts/tests/ecca_capability_inventory.rs`

## Scope

Every currently published `capability_contract` under `contracts/examples/` and
`contracts/inference/` received a validator-backed classification before its
next publication, per FR-020. `connector_contract` entries under
`contracts/connectors/` are out of scope: connectors are resource adapters,
not capabilities, under the constitution's capability-first boundary
principle (Principle I).

## Outcome summary

| Outcome | Count |
| --- | --- |
| Compliant — governed event product already declared | 6 |
| No event required (documented evidence) | 9 |
| Blocked | 0 |
| Exception-free | 15 |
| **Total published capabilities** | **15** |

No artificial event was created to satisfy a quota. Every `no-event-required`
classification is backed by a `side_effects`/`emits` inspection recorded in
the inventory manifest, consistent with QG-004 and the FR-020 non-goal.

## No-event-required capabilities (9)

All nine have `side_effects` containing only `memory_only`, `state_change`, or
`external_call`, and an empty `emits` list — i.e. their result returns
directly to the caller (or is written to their own capability-local state)
with no domain fact broadcast to independent consumers:

- `doc-approval.analyze`
- `doc-approval.recommend`
- `hello.world.say-hello`
- `meeting-notes.process`
- `traverse-starter.pipeline`
- `traverse-starter.process`
- `traverse-starter.summarize`
- `traverse-starter.validate`
- `traverse.inference.generate`

## Governed-event-declared capabilities (6)

All six declare `side_effects: [event_emission]` and a non-empty `emits`
referencing one of the five published expedition-domain event contracts under
`contracts/examples/expedition/events/`:

- `expedition.planning.capture-expedition-objective` → `expedition-objective-captured`
- `expedition.planning.interpret-expedition-intent` → `expedition-intent-interpreted`
- `expedition.planning.assess-conditions-summary` → `conditions-summary-assessed`
- `expedition.planning.validate-team-readiness` → `team-readiness-validated`
- `expedition.planning.assemble-expedition-plan` → `expedition-plan-assembled`
- `expedition.planning.plan-expedition` → `expedition-plan-assembled` (second producer)

## Declared/observed drift found and closed during this inventory

Building the inventory surfaced two categories of drift between capability
declarations and the event contracts' `publishers`/`subscribers` records
(the exact drift class FR-016 requires the catalog to be able to show). Both
were closed as part of this migration, using only the existing capability and
event contract schema fields — no new ECCA descriptor fields were introduced,
since the descriptor/schema work itself belongs to #896 and #897:

1. **Undeclared second producer**: `expedition.planning.plan-expedition` is a
   composite workflow capability whose terminal step re-emits
   `expedition-plan-assembled` (see its `side_effects`, `emits`, and
   `dependencies` fields), but the event contract's `publishers` list
   contained only `assemble-expedition-plan`. Fixed by adding
   `plan-expedition` as a second publisher.
2. **Missing subscriber declarations**: four capabilities (`assess-conditions-summary`,
   `interpret-expedition-intent`, `validate-team-readiness`,
   `assemble-expedition-plan`) declare a `consumes` reference to another
   capability's event, but the corresponding event contract's `subscribers`
   list was empty. Fixed by adding the matching subscriber entry to each of
   the four upstream event contracts (`expedition-intent-interpreted`,
   `expedition-objective-captured`, `conditions-summary-assessed`,
   `team-readiness-validated`).

## What is explicitly deferred, and why

Per ADR-0028 ("the first implementation is deliberately incremental") and to
avoid duplicating #896 (registry catalog and capability discovery) and #897
(runtime validation, lineage, and conformance), this migration does not:

- Add the new ECCA-only descriptor fields (CloudEvents-explicit
  source/subject/time mapping, `exposure` class, per-field controlled data
  classification, retention/deprecation) to the five existing event
  contracts. Those fields are introduced by #896/#897's schema work; adding
  them here ahead of that landing would risk conflicting field definitions.
- Implement runtime enforcement that rejects republication of a
  non-compliant capability (FR-011/FR-013). That requires the ECCA validator
  built in #897, which has not landed yet.
- Add OpenTelemetry lineage evidence (FR-019); that is runtime instrumentation
  scoped to #897.

The regression test added in this change enforces the part of FR-020 that is
independent of that future work: every published capability must have an
inventory entry, and every capability declaring `event_emission`/`emits` must
have a validator-parseable event contract that lists it as a publisher (and,
where a `consumes` reference exists, the upstream event must list it as a
subscriber). A capability added later without an inventory entry, or an
undeclared producer/consumer relationship, fails this test.

## Maintainer review note

No capability's classification was changed to `governed-event-declared` by
fabricating an event; every `governed-event-declared` entry already had a
side-effect and `emits` declaration in its checked-in contract prior to this
inventory. This report only makes existing, self-declared relationships
mechanically checkable and closes the drift listed above.
