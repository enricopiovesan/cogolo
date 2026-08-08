# Feature Specification: Capability Package Authoring CLI

**Status**: Approved
**Canonical governing ID**: `100-capability-package-authoring`
**Version**: 1.0.0
**Extends**: `044-application-bundle-manifest` (FR-015 scaffold authority), `017-ai-agent-packaging`, `516-agent-artifact-execution`, `091-no-std-wasi-guest-profile`
**Input**: Issues #988–#991; Decision 54 in `docs/decision-log.md`.

## Purpose

Define the governed CLI create path for a new standalone capability package
so developers and LLMs can author packages that match the production
`capability_package` model used by `capability-package inspect` and
`capability-package execute`, without relying on tribal Host ABI knowledge
or landing on empty/draft scaffolds that cannot become executable.

This slice makes `traverse-cli capability new` the canonical create command.
It reassigns the practical meaning of `044` FR-015's `component new` surface
to a redirect/deprecation path so authors are not offered two competing
scaffolds.

## Capability Boundary

In scope:

- CLI command `capability new <capability-id>`
- Required scaffold artifacts and guest-profile constraints for new packages
- Redirect/deprecation behavior for `component new` and
  `scripts/scaffold/new-capability.sh`
- CLI success/failure messaging for create (next-step guidance vs claim of
  executability)

Out of scope:

- Expanding Host ABI v1 or granting new WASI imports
- Rewriting existing catalog or example packages
- Registry publish, catalog UI, or FR-020 republication rules
- Changing `capability-package execute` result rendering (separate bug #987
  under already-approved `516`)
- Wiring advertised `capability inspect` (separate bug #986 under already-
  approved `017`)

## Relationship to Spec 044 FR-015

`044` FR-015 required `traverse-cli component new` to generate a component
package structure suitable for real WASM implementation without fake product
behavior. That command today emits a layout that is not a
`kind: capability_package` manifest and is not aligned with the no-std guest
profile required for Host ABI v1 execution.

This spec **does not edit** immutable `044` text. It **supersedes the
practical create-path authority** of FR-015 for new standalone capability
authoring:

- Canonical create command: `capability new`
- `component new` MUST redirect or fail with guidance to `capability new`
  (see FR-008)
- `044` QG-004 (no fake product behavior) remains in force for all scaffolds

App-bundle composition that still needs a component slot inside an
application bundle remains governed by `044`; this spec does not invent a
second app-bundle model.

## Requirements

- **FR-001**: `traverse-cli capability new <capability-id>` MUST create a
  new package directory for a valid capability id and MUST refuse to run
  when the target directory already exists (no overwrite).
- **FR-002**: The generated package MUST include a package manifest with
  `kind: capability_package` that is loadable by the same loader used by
  `capability-package inspect` / `capability-package execute`.
- **FR-003**: The generated package MUST include a capability contract with
  explicit, authorable input and output JSON Schemas (not silently empty
  `properties: {}` with no author guidance). Placeholder field names are
  allowed; empty schema objects that look complete are not.
- **FR-004**: The generated Rust guest stub MUST be structured for the
  no-std WASI guest profile governed by `091` (and the stdout/stdin FFI
  boundary governed by `090` where applicable): it MUST NOT guide authors
  toward a `std`-linked WASI binary that fails Host ABI validation.
- **FR-005**: The scaffold MUST include an artifacts directory and a sample
  runtime request fixture path documented in CLI output so authors can run
  `capability-package execute` after building a real artifact.
- **FR-006**: CLI success output for `capability new` MUST print next steps
  (implement logic, build wasm, inspect, execute) and MUST NOT claim the
  package is executable while the wasm artifact is missing or still a
  placeholder digest.
- **FR-007**: Invalid capability ids MUST fail with a usage/validation error
  before any files are written.
- **FR-008**: `traverse-cli component new` MUST NOT continue to emit the
  pre-Spec-100 empty component layout as a silent success path. It MUST
  either (a) delegate to the same generator as `capability new`, or (b) exit
  non-zero with a clear message directing authors to `capability new`.
- **FR-009**: `scripts/scaffold/new-capability.sh`, if retained, MUST either
  invoke/match the Spec 100 scaffold or exit non-zero directing authors to
  `traverse-cli capability new`. It MUST NOT remain the primary documented
  path while emitting stale contract shapes (`input_schema`, draft-only
  traps, or non-`capability_package` manifests).
- **FR-010**: A malformed or incomplete scaffold that cannot be inspected as
  a capability package MUST fail with one clear, machine-readable error
  class — not a cascade of unrelated schema mismatches — when passed to
  `capability-package inspect`.

## Acceptance Scenarios

1. **Given** a unique valid capability id, **When** a developer runs
   `traverse-cli capability new example.domain.my-cap`, **Then** a package
   directory is created with `kind: capability_package` manifest, contract,
   no-std-oriented guest stub, artifacts directory, and sample request
   fixture.
2. **Given** that scaffold, **When** the author builds a Host-ABI-valid
   artifact and updates the manifest digest per existing package rules,
   **Then** `capability-package inspect` and `capability-package execute`
   succeed using the production package path.
3. **Given** an invalid capability id or an existing target directory,
   **When** `capability new` runs, **Then** it exits non-zero and writes no
   (or no additional) scaffold files.
4. **Given** `component new` or the bash scaffold script, **When** invoked
   after this spec ships, **Then** the author is redirected to or blocked
   toward `capability new` rather than receiving the pre-Spec-100 empty
   layout as success.
5. **Given** a freshly scaffolded package with no real wasm artifact,
   **When** CLI create output is inspected, **Then** it does not claim the
   capability is executable.

## Edge Cases / Unhappy Paths

- Target path exists → refuse overwrite.
- Capability id with path separators or `..` → validation error, no writes.
- Missing wasm / placeholder digest → inspect/execute fail clearly; create
  success must not imply execute success.
- Authors editing only `std`-based templates → prevented by FR-004 scaffold
  shape; ABI verification remains the execution gate (`516` / Host ABI).

## Quality Gates

- **QG-001**: Scaffolds MUST NOT include fake product business logic that
  can pass as a finished capability (preserves `044` QG-004).
- **QG-002**: Scaffolds MUST NOT expand Host ABI or add WASI imports beyond
  the approved guest profile.
- **QG-003**: Create, redirect, and validation failure modes MUST be covered
  by CLI tests.
- **QG-004**: Spec-alignment gate MUST list this spec's governed paths.

## Success Criteria

- **SC-001**: A new author can create a package with CLI alone and reach
  inspect/execute after building, without copying tribal ABI setup from
  outside the scaffold.
- **SC-002**: No primary create path remains that emits the pre-Spec-100
  empty `component new` layout as success.
- **SC-003**: `044` QG-004 continues to hold for generated packages.

## Assumptions

- Production package loading remains the `capability_package` model already
  used by shipped examples and `capability-package` CLI commands.
- `#986` and `#987` close adjacent CLI correctness gaps under existing
  approved specs and do not require this slice to ship first.
- Decision 48 (no pre-production backward-compatibility tax) applies:
  redirecting `component new` does not require a long dual-scaffold era.

## Out of Scope

- Registry publication and catalog UX
- Multi-capability app bundle redesign beyond redirecting standalone create
- Demo-only execute modes
- New ADRs for Host ABI (none required for this CLI surface)

## Implementation Tickets

- Traverse #989 — approve this specification (this ticket)
- Traverse #990 — implement `capability new` + redirect/deprecate scaffolds
- Traverse #991 — align CLI help + skill docs with this create path
- Traverse #988 — umbrella tracking
- Traverse #986 / #987 — adjacent Ready CLI correctness (not gated on this spec)
