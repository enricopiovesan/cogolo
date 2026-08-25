# Verified Public Registry Metadata Sync

**Status**: Approved (2026-08-24)
**Canonical governing ID**: `117-verified-public-registry-metadata-sync`
**Version**: 0.1.0
**Input**: Traverse #1135; implementation handoff traverse-framework/registry#312; prerequisite for Spec 116.

## Purpose

Define the versioned, verified registry-sync projection that provides the sole
input to the offline public contract-metadata cache. Registry synchronization
owns creation and publication; downstream MCP and browser readers remain local,
read-only consumers.

## Requirements

- **FR-001**: Each public record MUST include namespace, capability id,
  version, artifact digest, source release, sync provenance, public
  description, and `use_cases[].scenario` text.
- **FR-002**: Records MUST NOT include raw contracts, use-case input or output
  examples, private fields or overrides, credentials, or secret material.
- **FR-003**: The projection MUST be bound to the exact public artifact digest
  and the verified source release and sync provenance that produced it.
- **FR-004**: Publication MUST be versioned and atomic. Readers observe one
  complete prior or new generation, never partial or mixed state.
- **FR-005**: Unsupported schema versions, malformed records, and invalid
  digest or provenance bindings MUST fail closed with stable actionable errors.
- **FR-006**: Migration MUST be explicit; readers MUST NOT attempt lossy
  conversion or silently treat an incompatible generation as empty.

## Validation

- Fixtures cover valid records, privacy redaction, digest/provenance mismatch,
  incompatible versions, malformed generations, and interrupted publication.
- Release evidence identifies the published `traverse-registry` version for
  downstream consumers of Spec 116.

## Non-goals

- Implementing MCP search, browser serving, cache reader behavior, implicit
  refresh, or any private registry federation.
