# Public MCP Capability Search

**Status**: Approved (2026-08-24)
**Canonical governing ID**: `114-mcp-capability-search`
**Version**: 0.1.0
**Input**: Traverse #1123; unblocks #876.

## Purpose

Define an additive `search_capabilities` MCP discovery tool that lets an
agent find published capabilities by what they do, without changing existing
`list_capabilities` or `get_capability` behavior. The tool is a read-only
view of the host's locally synced, digest-verified public registry cache.

## Scope and boundary

The business capability is **discover published capabilities by declared
behavior**. The MCP layer owns query validation, deterministic matching, and
safe result projection. Registry sync, contract verification, and cache
lifecycles remain owned by the existing registry surfaces.

This spec succeeds Spec 015 for this additive tool and reuses the local-cache
posture of Spec 081. It does not amend either approved specification.

## Requirements

- **FR-001**: `search_capabilities` accepts one required `query` string and
  returns a JSON result envelope containing `records`, cache provenance, and
  a boolean `stale`.
- **FR-002**: The query MUST be trimmed, case-folded, and split on Unicode
  whitespace. An empty or whitespace-only query MUST fail with stable
  `invalid_query` rather than returning all records.
- **FR-003**: Every query token MUST case-insensitively match a substring of
  either a capability `description` or one `use_cases[].scenario`. No
  stemming, synonym expansion, semantic ranking, or fuzzy matching is
  performed.
- **FR-004**: Search MUST read only the locally synced, digest-verified public
  cache. Private overrides, in-process development registrations, and network
  refresh during a search are prohibited.
- **FR-005**: Each record MUST use the public capability-summary projection:
  identity, version, display metadata, service type, permitted targets, and
  public lifecycle/provenance metadata. It MUST NOT include raw contracts,
  use-case input/output examples, private records, or secret material.
- **FR-006**: Results MUST be deterministic. Higher match quality MAY sort
  first; equal-quality records MUST sort by capability id ascending then
  semantic version descending. The scoring rule MUST be documented and tested.
- **FR-007**: A valid stale cache remains searchable and the envelope MUST set
  `stale: true` with source-release and sync provenance. Missing cache MUST
  fail with `registry_sync_missing`; malformed or unverified cache MUST fail
  closed with a stable registry verification error. Neither condition is an
  empty result.
- **FR-008**: The tool is read-only and MUST produce no registry, contract,
  cache, trace, or network mutation.

## Compatibility

This is an additive MCP surface. Existing `list_capabilities` filters,
`get_capability`, and their response/error behavior remain unchanged. A future
incompatible request or result change requires a successor versioned artifact.

## Acceptance scenarios

1. A query matching a description returns the matching public summary.
2. A query matching only a declared use-case scenario returns the summary
   without returning the scenario's examples.
3. A multi-token query returns a record only when every token matches an
   allowed field.
4. Whitespace-only input returns `invalid_query`.
5. Repeated searches over the same cache return identically ordered results.
6. A valid stale cache returns results with `stale: true`; missing and invalid
   cache states return their stable actionable errors without network access.
7. A private-only registration and raw contract example payload never appear.

## Validation

- MCP unit and integration tests cover every acceptance scenario.
- Cache fixtures prove public-only, digest-verified, stale, missing, and
  invalid-cache branches.
- `bash scripts/ci/spec_alignment_check.sh` passes.

## Non-goals

- Hosted search, fuzzy/semantic search, embeddings, or a new dependency.
- Registry index schema changes or execution-time sync.
- Returning raw contracts or example payloads.
