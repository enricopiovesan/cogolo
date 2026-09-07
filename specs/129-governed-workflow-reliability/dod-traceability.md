# Spec 129 — DoD Traceability for Traverse #1235

**Status**: Non-normative. This note does **not** amend the approved, immutable
`spec.md`. It records where each Definition-of-Done item from Traverse #1235
("Specify governed compensation, retry, and transaction boundaries for
workflows") is satisfied, so the ticket's spec phase can be closed and the issue
rescoped to implementation without ambiguity. See decision-log Decision 67.

**Approving change**: PR #1245 (2026-09-06) — Spec `129-governed-workflow-reliability`
(status `approved`, `immutable`, in `specs/governance/approved-specs.json`) and
ADR-0058 ("Explicit Sequential Workflow Recovery", status Accepted).

## DoD item → evidence

| # | DoD item (from #1235) | Satisfied by |
|---|---|---|
| 1 | A governing spec states the supported retry, compensation, and transaction vocabulary and explicitly lists out-of-scope distributed-transaction behavior. | Spec 129 **Purpose** (bounded explicit retry + sequential compensation; "not a distributed transaction facility"); **FR-001** (retry vocabulary), **FR-002 / FR-003** (compensation vocabulary); **Out of Scope** (distributed atomic transactions, two-phase commit, exactly-once effects). |
| 2 | Every automatic action has declared authority, idempotency, budget, placement, and trace requirements. | **authority** — FR-004 (retry + compensation bound into the reviewed canonical proposal; proposer/planner/caller may not alter them) + ADR-0058 Decision. **idempotency** — FR-001 (retry only for contract-declared idempotent steps) + FR-006. **budget** — FR-005 (exhausted approval/budget is a pre-execution rejection category). **placement** — inherited unchanged from Spec `109-runtime-workflow-proposals` and Spec `111-durable-dynamic-orchestration`, which Spec 129 **Extends**: a retry attempt or compensation action is an ordinary workflow step (FR-002 requires it be an explicit pinned step with its own contract/policy validation) and is placement-evaluated by 109/110/111 like any other step. Spec 129 does not restate the placement requirement; it does not remove it. **trace** — FR-007. |
| 3 | Compensation eligibility is contract-declared and validated before scheduling; missing compensation fails closed where required. | **FR-002** (compensation must be an explicit, pinned workflow step with declared inputs and its own contract/policy validation). **FR-005** (validation must reject *missing or cyclic compensation* and *invalid compensation mappings* before any forward step executes — i.e. fail closed). Acceptance Scenario 3. |
| 4 | A failure matrix covers retryable, non-retryable, partially completed, expired-approval, and interrupted cases. | Enumerated across FR-005, FR-006, and Acceptance Scenarios 1–4 rather than as a single labeled table: **retryable** — FR-001 + Acceptance Scenario 1 (fail once, succeed within bound). **non-retryable** — FR-001 (retry permitted only for declared-idempotent steps) + FR-005 (non-idempotent retry rejected pre-execution). **partially completed** — FR-003 + Acceptance Scenario 2 (terminal failure compensates earlier eligible completed steps in reverse completion order; compensation failure reported distinctly). **expired-approval** — FR-005 (exhausted approval/budget). **interrupted** — FR-006 + Acceptance Scenario 4 (resume from durable checkpoint evidence; no silent replay of a non-idempotent completed effect). |
| 5 | Trace evidence distinguishes original execution, retry, compensation, and terminal outcome without raw payloads. | **FR-007** (trace must emit redacted, ordered evidence for each forward attempt, retry, compensation, skipped compensation, and terminal outcome). "Redacted" = no raw payloads. |
| 6 | An ADR records the selected consistency model and why alternatives were rejected. | **ADR-0058** — Decision (bounded retries for declared-idempotent steps only; recovery as explicit sequential compensating capability steps in reverse completion order; no distributed atomicity claim) + Alternatives Considered (retries only; distributed atomic transactions; implicit best-effort rollback — each rejected with reason). |
| 7 | No runtime implementation starts until the spec and ADR are approved. | Satisfied on 2026-09-06: Spec 129 `approved`/`immutable` and ADR-0058 Accepted, both merged in PR #1245. Implementation is tracked by the rescoped #1235 (decision-log Decision 67); its DoD is Spec 129 QG-001..QG-003 + Acceptance Scenarios 1–4, scoped to `crates/traverse-runtime/` and `crates/traverse-mcp/`. |

## Notes on the two inherited / distributed items

- **Placement (item 2)** is deliberately not restated in Spec 129. The specs it
  extends already govern where a workflow step runs, and reliability steps are
  workflow steps. If a future reviewer wants placement named explicitly in the
  reliability slice, that is a successor-spec change, not a defect in the
  approved text.
- **Failure matrix (item 4)** is covered in substance but not as a single table.
  The implementation ticket's QG-001 ("tests cover every FR-005 rejection
  category") and QG-002 (integration tests for success-after-retry,
  reverse-order compensation, compensation failure, interruption recovery)
  operationalise the same set of cases.
