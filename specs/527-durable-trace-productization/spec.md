# Feature Specification: Durable Trace Productization

**Feature Branch**: `codex/issue-847-durable-trace-productization`
**Status**: Draft — successor specification requiring maintainer approval before implementation.
**Input**: Issue #847, Decision 38, `524-production-app-readiness`, and Spec 079.

## Purpose

Define the host-authorized durable trace-journal policy for production embeds:
safe evidence persists across restart, is retained within deterministic limits,
and is exported only as explicitly redacted evidence. This does not implement
trace storage or change application-state ownership.

## Capability Boundary

The trace journal accepts safe execution evidence for an authorized workspace,
applies retention, and exports a redacted evidence view. The host owns roots,
keys, tenancy mapping, identity authentication, and transport; Traverse never
persists raw request or output payloads.

## Functional Requirements

- **FR-001**: A durable trace record MUST contain only execution identity,
  timestamps, capability identity/version, outcome, policy/constraint result,
  canonical hashes, and retention/pruning evidence; raw request, output,
  credentials, cache paths, and private payloads MUST be rejected.
- **FR-002**: The journal MUST require a host-supplied workspace authorization
  decision before append, read, or export; denied access fails closed as
  `trace_workspace_unauthorized` without revealing record existence.
- **FR-003**: Default retention is the earlier of 30 days or 10,000 records
  per workspace. Pruning is deterministic oldest-first and emits a safe
  `trace_pruned` record naming count, boundary, policy, and completion time.
- **FR-004**: Restart recovery retains valid committed records, discards only
  an incomplete final record, and emits deterministic recovery evidence.
- **FR-005**: Export requires explicit host authorization and a declared
  redaction profile. Export contains only safe evidence and reports the
  profile, record count, and excluded field classes.
- **FR-006**: Stable secret-free errors are `trace_workspace_unauthorized`,
  `trace_payload_forbidden`, `trace_retention_invalid`, `trace_export_denied`,
  and `trace_recovery_failed`.

## Acceptance Scenarios

1. Given an authorized workspace and an auditable execution, when the host
   restarts, then its safe trace remains readable and no raw request/output is
   present.
2. Given 10,001 records or a record older than 30 days, when retention runs,
   then it prunes oldest-first and records `trace_pruned` evidence.
3. Given an unauthorized caller, when it reads or exports, then it receives
   `trace_workspace_unauthorized` without record metadata.
4. Given an authorized export with a redaction profile, when it completes,
   then it emits only allowed evidence and declares exclusions.

## Compatibility and Governed Files

This is additive to Spec 079. Existing journal records remain readable; new
optional retention/export evidence is additive. A follow-up implementation is
limited to `crates/traverse-runtime/src/trace_journal.rs`,
`crates/traverse-runtime/tests/durable_trace_productization.rs`,
`contracts/trace/durable-trace-export.schema.json`, and conformance fixtures
under `specs/527-durable-trace-productization/`. This issue implements none.

## Out of Scope

- Encryption/key lifecycle, remote trace synchronization, and raw payload
  persistence.
- DataStore migration or ownership changes.
- Automatic approval registry changes.
