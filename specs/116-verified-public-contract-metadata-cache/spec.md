# Verified Public Contract-Metadata Cache

**Status**: Approved (2026-08-24)
**Canonical governing ID**: `116-verified-public-contract-metadata-cache`
**Version**: 0.1.0
**Input**: Traverse #1132; unblocks #876 and #1105.

## Purpose

Define a local, public-only metadata cache that binds searchable capability
metadata to the same digest-verified public registry generation used for
artifact resolution. It enables offline MCP discovery and browser entrypoint
resolution without turning either reader into a sync or network client.

## Ownership and boundary

Explicit registry sync or prepare work owns fetching, verification, projection,
and cache publication. MCP search and `traverse-cli serve` are read-only
consumers. They MUST NOT fetch, sync, refresh, repair, or fall back to an
in-process/private registry while reading this cache.

## Requirements

- **FR-001**: A cache generation is a versioned, atomically replaced envelope.
  Readers observe either one complete verified generation or a stable failure;
  they never observe partial, mixed, or uncommitted records.
- **FR-002**: Each record binds its public metadata to exact published
  namespace, capability id, version, artifact digest, source release, and sync
  provenance. A record whose binding cannot be verified MUST fail closed.
- **FR-003**: The public metadata projection contains only identity/version,
  display metadata, description, and declared `use_cases[].scenario` text.
  It MUST exclude raw contracts, use-case inputs/outputs, private fields,
  private overrides, credentials, and secret material.
- **FR-004**: Cache preparation is explicit and consumes only verified public
  registry content. It writes the metadata generation only after every record
  and its digest/provenance binding pass validation.
- **FR-005**: A valid stale generation remains readable and returns safe
  provenance with `stale: true`. Staleness does not weaken verification.
- **FR-006**: Missing preparation, malformed generation data, unsupported
  schema versions, and digest/provenance mismatches fail closed with stable,
  actionable registry errors. They MUST NOT be represented as an empty catalog.
- **FR-007**: Readers perform no network I/O or cache mutation. They may
  select an exact public entrypoint only when its metadata binding and the
  existing artifact verification gate both succeed.
- **FR-008**: Cache-version migration is explicit. An incompatible reader or
  generation fails with a stable compatibility error rather than attempting
  lossy conversion or partial recovery.

## Consumer contracts

### MCP search (#876)

MCP search reads only this projection to match public descriptions and
scenario text. It returns summary-only records and safe cache provenance; it
must not return any cached raw contract or example payload.

### Browser entrypoint execution (#1105)

`traverse-cli serve` resolves an exact public entrypoint only from a ready
generation. It may use cache identity and provenance to locate the declared
artifact, but still applies the normal artifact verification gate before
runtime handoff.

## Acceptance scenarios

1. A complete verified generation exposes public description and scenario text
   while omitting all example payloads and private records.
2. A reader observes the prior complete generation during a failed update and
   never a partially written new generation.
3. A stale but valid generation returns deterministic records and safe stale
   provenance without network access.
4. Missing, malformed, mismatched, and unsupported generations fail closed
   with stable actionable errors.
5. MCP search and browser entrypoint resolution cannot read private/in-process
   registrations as a fallback.

## Validation

- Unit and integration fixtures cover valid, stale-valid, missing, malformed,
  digest-mismatch, provenance-mismatch, incompatible-version, and partial-write
  states.
- Privacy fixtures prove no raw contracts, example inputs/outputs, credentials,
  or private records appear in the projection or reader output.
- Offline fixtures prove readers make no network request and do not mutate the
  cache.

## Non-goals

- Hosted registry search, semantic/fuzzy discovery, private-cache federation,
  implicit refresh, or remote/browser MCP hosting.
- Replacing artifact verification, execution authorization, or registry sync.
