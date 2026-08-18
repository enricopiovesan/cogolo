# Universal connector follow-up: immutable objects, append-only state, and local-day scheduling

## Context

Callweave exposed three generic host authorities that must not be implemented as
application capabilities: immutable object storage, append-only/idempotent
state transitions, and timezone-aware daily scheduling. They belong behind
portable Traverse connector contracts and application bindings.

## Proposed connector contracts

| Connector ID | Authority | Required guarantees |
|---|---|---|
| `traverse.object-store` | Store and resolve immutable byte assets | content digest, atomic finalization, configured-root/provider confinement, bounded I/O, no ambient path disclosure |
| `traverse.state-store` | Resolve typed records and atomically append transitions | idempotency key, append-only history, version conflict result, replay-safe result, non-secret trace |
| `traverse.scheduler` | Request a bounded, named future invocation | declared timezone/calendar policy, idempotency key, duplicate/late result, cancellation, no ambient timer authority |

## Composition rules

1. A capability declares an abstract connector requirement and never chooses a
   concrete host instance, local path, device, provider, credential, or timer.
2. An application bundle binds a requirement to an installed compatible
   connector and a non-secret configuration reference, as required by Spec 103.
3. Activation validates target, configuration, limits, and placement before
   execution.
4. Guest invocation uses only the mediated ABI in Spec 104. Ambient WASI file,
   network, environment, clock, and device access remain denied.
5. Every authorization or execution result emits a stable, non-secret trace.

## Capability-facing request boundaries

### `traverse.object-store`

Input: content stream/reference, expected media type, retention class, maximum
bytes. Output: immutable asset reference, digest, size, result class.

### `traverse.state-store`

Input: typed record references, transition request, idempotency key, expected
version. Output: result reference, successor/version reference, replay flag,
stable conflict/failure class.

### `traverse.scheduler`

Input: declared job kind, bounded local-day/timezone policy reference, logical
deadline, idempotency key. Output: invocation reference or stable duplicate,
late, unavailable, or policy-denied class.

## Explicit non-goals

- No concrete filesystem, database, cloud bucket, OS timer, or credential
  format is standardized here.
- No capability receives a local file path, device identifier, or secret.
- Audio capture and model runtimes require separate connector contracts.

## Required acceptance coverage

- missing/incompatible/unconfigured binding fails closed before invocation;
- oversized object payload and out-of-policy request are rejected without host
  leakage;
- duplicate state transition returns the original result without a new record;
- stale state version returns a stable conflict;
- scheduler duplicate and late invocation are deterministic;
- public trace output contains no configuration, credential, path, or device
  identifier.
