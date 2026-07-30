# ADR-0027: Host-Owned Durable Trace and Audit Persistence

- Status: Proposed
- Date: 2026-07-30
- Governing draft: `527-durable-trace-productization`

## Decision

Production hosts own the append-only durable trace journal root, its access
authorization, encryption keys, tenancy mapping, retention configuration, and
deletion authority. Traverse writes only canonical UTF-8 JSON Lines containing
safe public trace evidence and canonical hashes; it never persists raw
request/output payloads or private trace entries.

An auditable execution succeeds only after its line is `fsync` committed.
Startup discards only an incomplete final line and emits recovery evidence;
earlier malformed or hash-mismatched records fail closed. Host-explicit,
workspace-scoped retention or deletion is oldest-first and emits
`trace_pruned` evidence.

## Consequences

- Restart, partial-write, corruption, retention, deletion, and private-trace
  behavior have deterministic, testable boundaries.
- Private trace persistence and remote journal synchronization remain separate
  governed capabilities.
- This ADR and its draft require maintainer approval before implementation.
