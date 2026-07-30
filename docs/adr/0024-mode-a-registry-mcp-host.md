# ADR-0024: Mode A as a Local Registry-Backed Stdio MCP Host

- Status: Accepted
- Date: 2026-07-30
- Governing spec: Approved `086-mode-a-registry-mcp` / `530-mode-a-registry-mcp`
- Decided in: Traverse #865; governed by #906

## Context

The existing MCP stdio server loads an expedition bundle, executes hardcoded
Rust examples, and requires filesystem request paths. That conflicts with the
LLM façade product claim that local clients use the same catalog and artifact
identity as OS consumers. Designing a hosted gateway or the embedded MCP mode
at the same time would delay the smallest useful truthful path.

## Decision

Adopt Mode A as a versioned local stdio binary for Claude Desktop and Cursor.
It discovers and executes only prepared, digest-verified public registry
entries through the normal runtime path. It accepts one canonical inline
`RuntimeRequest` JSON value or the temporary compatibility path input, never
both. It performs no execution-time network retrieval and fails closed when
the verified cache is absent.

The local process boundary is Mode A's authorization boundary; it adds no
bearer-token protocol. Its public result surface is limited to runtime-owned
structured results and redacted trace summaries. Expedition is a demo-only
path, not a fallback authority.

## Consequences

- Local LLM façades gain an installable, reproducible, offline-capable path
  that shares artifact identity with production consumers.
- Cache preparation remains explicit and host/deployment-owned, avoiding a
  second MCP-specific supply chain.
- Existing path-based callers can migrate to inline JSON without immediate
  breakage, while ambiguity is rejected deterministically.
- Remote/browser clients, their authentication, and tenancy remain blocked on
  a distinct hosted-MCP decision rather than leaking into local stdio design.
- Raw payload diagnostics and durable evidence remain separately governed.

## Alternatives considered

- **Specify local, embedded, and hosted modes together**: rejected because
  unresolved gateway and embedded-cache work would block the local product.
- **Use live registry fetches at execution time**: rejected because it weakens
  reproducibility, provenance, and offline behavior.
- **Keep expedition as fallback/default**: rejected because it creates two
  catalog authorities and breaks the same-artifact claim.
- **Require bearer authentication in local stdio**: rejected because it adds
  secret handling without defining the actual remote trust boundary.
- **Return raw payloads for debugging**: rejected because the MCP surface is
  a public client contract, not a privileged diagnostic channel.

## Approval

Accepted following explicit Enrico approval on 2026-07-30; the companion spec
is registered as immutable approved Spec 086.
