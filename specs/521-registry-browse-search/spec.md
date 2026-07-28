# Feature Specification: Synced Registry Browse and Search

**Status**: Approved
**Canonical governing ID**: `081-registry-browse-search`
**Input**: Traverse #832; extends Specs 054 and 055.

## Purpose

Define deterministic local CLI discovery for the synced public registry index,
without changing its thin contract-first format or introducing runtime network
lookup.

This specification derives from Decision 36 in the decision log.

## Requirements

- `traverse-cli registry list --workspace <id> [--namespace <value>] [--id-prefix <value>] [--json]` lists only the local synced index, ordered by namespace, id, then descending semantic version.
- `traverse-cli registry search <query> --workspace <id> [--namespace <value>] [--json]` performs case-insensitive substring matching over namespace, id, and cached contract summary only; ordering is namespace, id, version.
- Human output is a stable table; JSON output includes `status`, `workspace`, `source_release`, `index_version`, `source_commit`, `synced_at`, `stale`, and records with namespace/id/version/digest/yanked/deprecated.
- No synced index returns `registry_sync_missing`; malformed local state returns `registry_sync_invalid`; unavailable cached summary returns `registry_contract_summary_missing`; offline operation continues against valid local state and never fetches.
- An explicit CLI `registry cache-contract-summary` action MAY fetch a selected published contract, validate it, digest-address it, and cache only summary fields needed for search. Runtime execution never reads the network.
- Stale state remains searchable and reports `stale: true`; invalid refresh never replaces the last valid state.

## Acceptance Scenarios

1. Given a valid synced index, list emits deterministic human and JSON records without network access.
2. Given an offline host with valid stale sync state, search succeeds with provenance and `stale: true`.
3. Given no sync state, list and search return `registry_sync_missing` with sync guidance.
4. Given a cached validated contract summary, search finds its declared display metadata without exposing full contract bytes.

## Out of Scope

- Registry index schema changes, runtime lookup, hosted search, or implementation of #814.
