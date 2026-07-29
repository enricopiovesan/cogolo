# Implementation Plan: Production App Readiness Baseline

**Branch**: `524-production-app-readiness` | **Date**: 2026-07-28 | **Spec**: [spec.md](spec.md)
**Input**: Approved Decision 38 and the product-planning specification.

## Summary

Plan the v1 production-app bar as five independently governed delivery slices:
registry trust/lockfiles, embedded cache lifecycle, durable traces,
`local-datastore/2`, and cross-platform certification. The plan preserves
embedded offline execution, host ownership, and deterministic evidence.

## Technical Context

**Language/Version**: Rust 1.94+; host packages in TypeScript, Swift, Kotlin, and .NET
**Primary Dependencies**: Cargo workspace, serde, semver, existing registry and embedder packages
**Storage**: host-owned verified artifact cache, append-only trace journal, host-owned file-backed DataStore v2
**Testing**: cargo test, conformance scripts, cross-platform native artifact certification, deterministic CLI smoke paths
**Target Platform**: Web, Linux/Rust, Apple, Android, Windows/.NET
**Project Type**: runtime library, CLI, cross-platform embedder packages
**Performance Goals**: offline initialization and execution perform no network I/O; bounded retention is deterministic
**Constraints**: host owns roots, keys, tenancy, network preparation, and user authorization; v1 is single-writer per DataStore root
**Scale/Scope**: five successor specs and bounded tickets; no monolithic implementation

## Constitution Check

| Gate | Result | Evidence |
| --- | --- | --- |
| Capability-first boundaries | Pass | Each slice is a bounded host/runtime capability, not an app UI flow. |
| Contracts and immutable specs | Pass | Each implementation slice needs its own approved successor spec and contract changes. |
| Portability | Pass | Host-specific storage/network work is behind public host boundaries. |
| Explainability | Pass | Lock, cache, migration, retention, and yank outcomes require stable evidence. |
| Small verifiable slices | Pass | Delivery order is independently testable; no slice claims the whole baseline. |

## Delivery Sequence

1. **Registry trust and lockfile** — define tier metadata, default discovery,
   Certified admission, lockfile schema, normal/security yank lifecycle, and
   90-day deprecation policy.
2. **Embedded cache lifecycle** — implement explicit prepare, generation
   activation, rollback, offline initialization, and security-yank enforcement.
3. **Durable trace productization** — wire the approved trace journal into the
   embedded host surface, retention defaults, authorization, and safe export.
4. **DataStore v2** — specify exact envelope, backup, restore, encryption
   disclosure, single-writer behavior, and v1-to-v2 migration conformance.
5. **Platform certification** — add common conformance tests and Certified vs
   Preview classification across all five primary hosts.

## Dependency Rules

- Slice 1 must precede Slice 2 because cache preparation consumes lock/tier/yank semantics.
- Slice 2 must precede registry-only Reference Apps cutover.
- Slices 3 and 4 may proceed independently after their successor specs are approved.
- Slice 5 consumes the common acceptance contracts from Slices 2–4.

## Project Structure

```text
crates/traverse-cli/              # registry discovery, preparation, evidence
crates/traverse-runtime/          # offline execution, traces, DataStore contract
crates/traverse-embedder/         # host-facing preparation/init interfaces
packages/{web,swift,kotlin,dotnet}/TraverseEmbedder/
specs/524-production-app-readiness/ # planning baseline and design artifacts
docs/decision-log.md              # accepted product direction
```

**Structure Decision**: preserve the existing multi-package workspace; each
successor spec names exact governed files before any implementation starts.
