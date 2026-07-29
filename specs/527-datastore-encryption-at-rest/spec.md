# Feature Specification: DataStore Encryption at Rest for Private Records

**Feature Branch**: `527-datastore-encryption-at-rest`  
**Created**: 2026-07-29  
**Status**: Draft  
**Canonical governing ID**: `084-datastore-encryption-at-rest`  
**Extends**: `518-durable-local-datastore`, `519-embedder-owned-datastore-integration`  
**Input**: Project 1 Specify encryption ticket; planning locks recorded in Decision 40.

## Purpose

Define at-rest encryption for `private` DataStore records, a pluggable
`KeyProvider` port, and fail-closed behavior when keys are unavailable.
Public records remain integrity-protected only. OS/KMS providers, in-place
rotation, and classification mutation are out of scope for v1.

## Decisions

- Only records with classification `private` are encrypted at rest.
- Keys are obtained through a host-supplied `KeyProvider`. Traverse never
  persists key material beside the store in v1.
- v1 ships host-callback / in-memory providers suitable for tests and host
  wiring. OS keychain/KMS adapters are successor tickets.
- The mandatory algorithm is AES-256-GCM. AAD MUST bind key id, record key,
  and classification. Nonce uniqueness rules are normative and fail closed on
  misuse.
- v1 supports a single active key for writes. Envelopes record `key_id` for
  future rotation. There is no in-place re-encrypt/rotate API. Operational
  re-key uses maintenance backup/restore under a new provider (Spec 083).
- Opening a store without a `KeyProvider` MUST fail closed on private
  read/write and MUST allow public CRUD.
- Classification is immutable per record identity. Changing classification
  requires delete + write of a new record.

## Functional Requirements

- **FR-001**: Writes of `private` records MUST encrypt payload bytes with
  AES-256-GCM using material from `KeyProvider` before durability commit.
- **FR-002**: Reads of `private` records MUST decrypt and authenticate before
  returning values; authentication failure MUST return a stable integrity /
  crypto error and MUST NOT return plaintext.
- **FR-003**: `public` records MUST continue to use the existing integrity
  envelope without encryption.
- **FR-004**: `KeyProvider` MUST expose stable secret-free errors for missing
  key, revoked key id, and provider failure.
- **FR-005**: Absent `KeyProvider`, private read/write MUST return
  `key_provider_required`. Public operations MUST proceed.
- **FR-006**: Persisted private envelopes MUST store `key_id`, nonce, and
  ciphertext necessary for authenticated decrypt; they MUST NOT store raw key
  bytes.
- **FR-007**: APIs MUST NOT provide classification mutation of an existing
  record identity in v1.
- **FR-008**: Evidence/telemetry MUST NOT include keys, nonces, ciphertext, or
  plaintext payloads.

## Acceptance Scenarios

1. Given a provider and a private write, when the process restarts and reopens
   with the same provider, then the private value decrypts successfully.
2. Given no provider, when a private write is attempted, then the call fails
   with `key_provider_required` and no record is committed.
3. Given a public write without a provider, when read back, then integrity
   verification succeeds.
4. Given tampered ciphertext, when read, then authentication fails closed.
5. Given a host attempts to “reclassify” in place, when using public APIs, then
   no such operation exists; only delete+write can change secrecy.

## Out of Scope

- OS keychain / KMS adapters (Future).
- In-place key rotation / dual-key read re-encrypt (Future).
- Encrypting public records or whole-store encryption.
- IndexedDB-specific providers (see Spec 085 follow-on).

## Compatibility

Additive. Stores that never write private records behave as today.
