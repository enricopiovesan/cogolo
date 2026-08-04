# Feature Specification: Mode A Registry-Backed Stdio MCP Host

**Feature Branch**: `codex/issue-906-mode-a-mcp-spec`
**Created**: 2026-07-30
**Status**: Approved (2026-07-30 — approved by Enrico; ADR-0024 accepted)
**Version**: 1.0.0
**Input**: Traverse #865 decision records; governing ticket #906; Specs 015,
042, 054, 055, 516, and 520.

## Purpose

Define the smallest supported LLM-facing Traverse host: a versioned local
stdio MCP binary that discovers and executes the same digest-verified public
registry capabilities and workflows used by production consumers. This is a
Mode A product boundary, not a hosted service or embedded-host substitute.

The governed business capability is **execute a prepared governed capability
or workflow through a local MCP client**, with runtime-owned result and trace
evidence. It is reusable by Claude Desktop and Cursor without coupling either
client's UI or prompt logic to Traverse.

## Boundary and ownership

| Concern | Owner |
| --- | --- |
| Configuration, process launch, and local OS access | MCP client / user |
| Cache preparation and artifact acquisition | explicit host or deployment tooling under Spec 520 |
| Capability/workflow metadata, lifecycle, and published digests | public registry |
| Deterministic local resolution and execution | Traverse MCP host and runtime |
| Runtime-owned output, trace summary, and redaction | Traverse runtime |
| Hosted transport, tenants, and remote authentication | successor hosted-MCP governing work |

## User scenarios and testing

### User story 1 — Discover the prepared public kit (Priority: P1)

As a local LLM client, I discover `traverse-starter`, `meeting-notes`, and
other prepared public-kit entries from the local verified cache, so tool
selection uses the same authority and identities as OS consumers.

**Independent test**: Prepare a fixture cache containing kit capabilities and
workflows, then assert discovery returns their published identities and no
expedition-only entry as a default result.

1. **Given** a valid prepared cache, **When** discovery runs, **Then** it
   returns deterministic public-kit metadata and selected versions/digests.
2. **Given** no prepared cache entry, **When** discovery or execution targets
   it, **Then** the host returns a stable cache/preparation failure and makes
   no network request.
3. **Given** an expedition-only bundle exists locally, **When** Mode A starts,
   **Then** it is not a fallback authority or execution path.

### User story 2 — Execute with an inline runtime request (Priority: P1)

As a local LLM client, I execute a discovered entry with inline JSON rather
than a checkout-relative file path, so normal tool calls do not create or
manage temporary request files.

**Independent test**: Submit a serialized `RuntimeRequest` through
`request_json`, execute a published WASM fixture offline, and assert its
runtime-owned result.

1. **Given** exactly one valid `request_json`, **When** execution runs,
   **Then** the host validates and executes that request against the selected
   verified artifact.
2. **Given** both `request_json` and `request_path`, **When** execution is
   requested, **Then** the host fails with `mcp_request_ambiguous` before
   loading or executing either input.
3. **Given** neither input, **When** execution is requested, **Then** the
   host fails with `mcp_request_missing`.
4. **Given** malformed JSON or a request failing runtime validation, **When**
   execution is requested, **Then** the host returns a stable validation
   classification without raw input echoing.

### User story 3 — Install a supported local host (Priority: P2)

As a Claude Desktop or Cursor user, I install one versioned cross-platform
binary and use documented stdio configuration, without a Rust toolchain or a
Traverse source checkout.

**Independent test**: On each released target, start the binary from a clean
fixture configuration and complete discovery plus one offline execution.

1. **Given** a supported release binary and documented client configuration,
   **When** the client launches it over stdio, **Then** the MCP lifecycle and
   capability version are reported deterministically.
2. **Given** a client requiring an incompatible supported MCP capability
   version, **When** it starts the binary, **Then** it receives a stable
   compatibility failure.
3. **Given** an upgrade, **When** the client starts the newer binary with an
   unchanged compatible configuration, **Then** its documented Mode A
   behavior remains compatible.

### User story 4 — Receive safe runtime evidence (Priority: P1)

As a local LLM client, I receive useful structured execution outcomes and
safe trace summaries without obtaining payloads, secrets, or raw failures.

**Independent test**: Execute fixtures with unique secret-like input, output,
and error strings; assert none appear in every MCP result or trace summary.

1. **Given** a successful execution, **When** its result is returned, **Then**
   it contains only the documented runtime-owned result fields and safe trace
   summary.
2. **Given** a failed execution, **When** its result is returned, **Then** it
   contains a stable error category and redacted safe evidence only.
3. **Given** any request or runtime failure contains secrets or raw payloads,
   **When** the host reports it, **Then** those values and unfiltered details
   are absent.

## Functional requirements

- **FR-001**: Mode A MUST be a versioned local stdio MCP binary. Its initial
  supported consumer configurations are Claude Desktop and Cursor.
- **FR-002**: Mode A MUST discover public capabilities and workflows only from
  a locally prepared, digest-verified registry cache compatible with Spec 520.
  It MUST NOT perform network fetches during discovery, validation, or
  execution.
- **FR-003**: Mode A MUST execute published, digest-pinned WASM artifacts
  through the runtime. Expedition bundles and `ExpeditionExampleExecutor` MUST
  NOT be a Mode A default or fallback path.
- **FR-004**: Execution input MUST accept exactly one of `request_json` or the
  compatibility `request_path`. Both inputs MUST produce
  `mcp_request_ambiguous`; neither MUST produce `mcp_request_missing`.
- **FR-005**: `request_json` MUST serialize the canonical `RuntimeRequest`.
  Its parse and semantic validation failures MUST be stable, machine-readable,
  and must not echo raw request data.
- **FR-006**: Missing preparation, cache entries, unavailable versions,
  invalid digests, yanked entries, and unknown public entrypoints MUST fail
  closed using the compatible registry error taxonomy or `mcp_not_found`.
- **FR-007**: Mode A MUST return only documented runtime-owned structured
  result fields and a redacted trace summary. It MUST NOT return raw requests,
  raw outputs outside the allowed runtime result contract, secrets, caller
  identities, private hashes, raw telemetry attributes, or unfiltered errors.
- **FR-008**: Mode A MUST have no built-in bearer-token protocol. The local
  process boundary is the initial authorization boundary; remote authentication
  is out of scope.
- **FR-009**: The binary MUST define its version, supported MCP capability
  version, supported platforms, and compatible configuration/upgrade policy.
- **FR-010**: Documentation MUST label expedition MCP support as demo-only and
  identify Mode A as the kit-default local path after implementation.
- **FR-011**: Mode A MUST provide deterministic fixtures for discovery,
  offline execution, cache-missing behavior, xor-input failures, digest/yank
  failures, and privacy/redaction failures.

## Stable error categories

| Condition | Required code |
| --- | --- |
| Both inline and path inputs | `mcp_request_ambiguous` |
| No input | `mcp_request_missing` |
| Invalid inline JSON/request | `mcp_request_invalid` |
| Unknown catalog entry | `mcp_not_found` |
| Missing verified cache entry | `registry_cache_entry_missing` |
| Sync/preparation absent | `registry_sync_missing` or `registry_prepare_failed` |
| Digest mismatch or yanked entry | Spec 520 registry code |
| Unsupported protocol capability | `mcp_version_incompatible` |

## Success criteria

- **SC-001**: A clean supported client configuration completes discovery and
  one published-WASM execution against a prepared cache without a source
  checkout or network request.
- **SC-002**: 100% of exercised missing-cache, unknown-entry, digest, yanked,
  and xor-input fixtures fail with their documented stable category.
- **SC-003**: Privacy fixtures prove 100% of returned MCP results and trace
  summaries omit supplied unique sensitive values.
- **SC-004**: The same prepared cache and request yield the same selected
  artifact identity and structured error/result classification across repeated
  runs.

## Compatibility and migration

`request_path` remains a documented compatibility input during the initial
Mode A release but is mutually exclusive with `request_json`. Removing it
requires a successor versioned-spec decision. Existing expedition documentation
may remain as a demo path but must not represent the supported kit-default
product path after Mode A lands.

## Out of scope

- Embedded Mode B cache preparation and its host-package implementation.
- Browser, remote, hosted, multi-tenant, ChatGPT, or Grok MCP adapters.
- Live registry fetching, a CLI/HTTP sidecar fallback, or prompt-side business
  logic.
- Bearer-token, OAuth, or remote authorization design.
- Privileged raw-payload diagnostics or durable cross-restart trace history.

## Approval

This immutable artifact governs implementation as approved Spec 086 after
explicit reviewer approval, acceptance of ADR-0024, and registry evidence in
`specs/governance/approved-specs.json`.
