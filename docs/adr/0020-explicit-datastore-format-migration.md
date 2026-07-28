# ADR-0020: Keep DataStore Format Migration Explicit and Host-Owned

- Status: Proposed
- Governing spec: `522-datastore-format-migration` (Draft)
- Extends: ADR-0018 and ADR-0019

## Decision

Only the host that owns a durable DataStore root may explicitly request a
specified, approved source-to-target migration. The migrator validates the
source, creates a verified host-owned backup, verifies the target, and commits
atomically. Failed work preserves the source; restore is explicit and verified.

Generic runtime and CLI execution never discover a root or migrate it. Unknown
and legacy input remains fail-closed; no downgrade is implicit.

## Consequences

Format evolution can become recoverable without assigning retention, backup,
or root ownership to Traverse. A later approved format transition is required
before this ADR can be accepted or implemented.
