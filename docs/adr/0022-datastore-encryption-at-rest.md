# ADR-0022: Encrypt Only Private DataStore Records via Host KeyProvider

- Status: Accepted
- Date: 2026-07-29
- Governing spec: `527-datastore-encryption-at-rest` / `084-datastore-encryption-at-rest`
- Extends: ADR-0018, ADR-0019

## Context

Classification metadata already distinguishes `public` and `private` records,
but privacy is not enforced cryptographically. Building OS-specific KMS into
the first slice would stall approval. Silently encrypting all records would
over-constrain public cache-like state.

## Decision

Encrypt only `private` records at rest using AES-256-GCM with AAD binding key
id, record key, and classification. Hosts supply key material through a
`KeyProvider` port. v1 providers are host-callback/in-memory; OS/KMS adapters
come later.

No KeyProvider means private operations fail closed while public CRUD works.
Classification is immutable per record identity. No in-place rotation API in
v1; envelopes carry `key_id` so a later rotation design can proceed. Re-key
runbooks use Spec 083 backup/restore under a new provider.

## Consequences

- “Private” gains a real confidentiality boundary without blocking public use.
- Implement work waits on Spec 084 approval.
- Browser private support waits on a KeyProvider adapter after IndexedDB CRUD.

## Alternatives Considered

- Whole-store encryption: rejected for public data ergonomics.
- Host-only plaintext encryption outside Traverse: rejected for conformance.
- In-place rotation in v1: deferred as higher complexity.
