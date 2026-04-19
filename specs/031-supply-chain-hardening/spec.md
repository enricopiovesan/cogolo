# Feature Specification: Supply Chain Hardening

**Feature Branch**: `031-supply-chain-hardening`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Supply chain security layer for `scripts/ci/`, `.github/workflows/`, and `Cargo.toml`, covering SBOM generation, reproducible builds, SLSA provenance, artifact checksum verification, and nightly supply-chain CI checks.

## Purpose

This spec defines the supply chain hardening model for Traverse.

It narrows the broad intent of "artifact integrity and provenance" into a concrete, testable set of requirements covering:

- SBOM generation in CycloneDX format for each release using `cargo cyclonedx` or equivalent
- reproducible builds: `Cargo.lock` committed, no timestamps in binary artifacts, byte-identical output across runs on the same platform
- SLSA Level 1 provenance for v0: a signed build provenance statement linking a source commit to a produced artifact
- artifact checksum (SHA-256) included in every artifact manifest, verified by the runtime before execution
- a `traverse-cli artifact verify` subcommand returning structured pass/fail evidence
- a nightly CI workflow running SBOM generation, checksum validation, and signature presence checks for governed artifacts

This spec governs CI scripts, GitHub Actions workflows, and top-level `Cargo.toml` workspace settings. It does not define runtime execution logic (governed by spec 006) or the security identity model (governed by spec 030), though it shares the same signing scheme (Ed25519 baseline, Sigstore for published artifacts).

## User Scenarios and Testing

### User Story 1 - CLI Artifact Verification Returns Structured Pass/Fail with Evidence (Priority: P1)

As a security team member, I want to run `traverse-cli artifact verify <path>` and receive a structured pass/fail result with explicit evidence (signature status, checksum match, provenance statement) so that I can verify artifact integrity without reading internal implementation details.

**Why this priority**: A verifiable CLI command is the primary trust boundary for enterprise consumers. Without it, integrity claims cannot be independently checked.

**Independent Test**: Sign and checksum a WASM artifact, place it at a known path, run `traverse-cli artifact verify <path>`, and assert the structured output contains `signature_status: verified`, `checksum_status: matched`, and a `provenance` block with a valid source commit reference.

**Acceptance Scenarios**:

1. **Given** a valid, signed, checksummed artifact with a provenance statement, **When** `traverse-cli artifact verify <path>` is run, **Then** the command exits with code 0 and outputs a structured JSON report with `overall_status: passed` and per-check evidence fields.
2. **Given** an artifact whose SHA-256 checksum does not match the manifest, **When** `traverse-cli artifact verify <path>` is run, **Then** the command exits with a non-zero code and the report includes `checksum_status: mismatch` with the expected and actual hash values.
3. **Given** an artifact with no provenance statement, **When** `traverse-cli artifact verify <path>` is run, **Then** the command exits with a non-zero code and the report includes `provenance_status: missing`; the report MUST NOT suppress the other check results.

### User Story 2 - CI Pipeline Fails on Published Artifact Checksum Mismatch (Priority: P1)

As a CI pipeline operator, I want the CI build to fail when a published artifact's checksum does not match its manifest entry so that compromised or accidentally modified artifacts are caught before deployment.

**Why this priority**: Checksum gating at CI is the last automated line of defense before a modified artifact reaches production.

**Independent Test**: In a CI test environment, modify a single byte in a governed WASM artifact binary without updating its manifest checksum, trigger the supply-chain CI workflow, and verify the workflow step fails with an explicit checksum mismatch error.

**Acceptance Scenarios**:

1. **Given** a governed artifact with a mismatched SHA-256 checksum, **When** the nightly supply-chain CI workflow runs, **Then** the checksum validation step fails with an explicit error naming the artifact path and the expected versus actual hash.
2. **Given** all governed artifacts with matching checksums, **When** the nightly supply-chain CI workflow runs, **Then** the checksum validation step passes and a machine-readable summary is written to the workflow artifact store.
3. **Given** a new WASM module added to the repo without a manifest entry, **When** the nightly workflow runs, **Then** the workflow emits a warning step (not a hard failure) indicating the artifact lacks a manifest entry, and the overall job fails to prompt the developer to register it.

### User Story 3 - Enterprise Consumer Can Independently Verify SBOM and Provenance (Priority: P2)

As an enterprise consumer downloading a Traverse release, I want to independently verify the SBOM and provenance attestation so that I can satisfy my organization's software composition analysis requirements without trusting the Traverse release pipeline blindly.

**Why this priority**: Publicly verifiable provenance and SBOM are table-stakes for enterprise software procurement and compliance programs.

**Independent Test**: Download a release package, extract the included CycloneDX SBOM and SLSA provenance statement, verify the SBOM lists all direct and transitive Rust dependencies with accurate version metadata, and verify the provenance statement links the correct source commit SHA to the release artifact hash.

**Acceptance Scenarios**:

1. **Given** a published Traverse release archive, **When** the CycloneDX SBOM is extracted and parsed by a standards-compliant SBOM tool, **Then** all direct and transitive Cargo dependencies are present with correct package names, versions, and license identifiers.
2. **Given** a published release SLSA Level 1 provenance statement, **When** it is inspected, **Then** it contains the source commit SHA, the build system identifier, the artifact SHA-256, and a build invocation reference.
3. **Given** a release where the source commit SHA in the provenance does not match the artifact, **When** an independent verification tool checks the provenance, **Then** the mismatch is detectable without Traverse-specific tooling.

### User Story 4 - Nightly CI Warns on WASM Module Missing Signature (Priority: P2)

As a developer who has added a new WASM module, I want the nightly CI to warn me that the module is missing a signature so that I am not surprised by a verification failure when the module is promoted to governed status.

**Why this priority**: Early warning reduces the cost of fixing supply chain gaps compared to discovering them at promotion or production deployment.

**Independent Test**: Add an unsigned WASM module to the repo, trigger the nightly supply-chain CI workflow, and verify the workflow emits a structured warning listing the module path and `signature_status: missing` without failing the overall job for ungoverned artifacts.

**Acceptance Scenarios**:

1. **Given** an ungoverned WASM module with no signature in its manifest, **When** the nightly workflow runs the signature presence check, **Then** the check emits a structured warning step output listing the module path and `signature_status: missing`.
2. **Given** a governed WASM module with no signature, **When** the nightly workflow runs, **Then** the signature presence check FAILS the job (not a warning) with an explicit error naming the governed artifact.
3. **Given** all governed modules have valid signatures, **When** the nightly workflow runs, **Then** the signature presence check passes and a summary report is written to the workflow artifact store.

## Edge Cases

- Reproducible build failures on different Rust toolchain versions: the reproducible build gate MUST pin the toolchain version in `rust-toolchain.toml`; builds on a different toolchain version are explicitly not required to be byte-identical and the CI gate MUST document this constraint.
- SBOM accuracy for transitive dependencies: `cargo cyclonedx` MUST be run with the `--all-features` flag to capture conditional dependency trees; the nightly CI MUST fail if the generated SBOM lists zero transitive dependencies (indicates tool misconfiguration).
- Partial provenance (some artifacts signed, some not): the verification report MUST enumerate each artifact's individual status; an overall `partial_provenance` status MUST be surfaced rather than silently passing or failing the whole set.
- Cargo.lock missing from the repository: the CI gate MUST detect a missing or `.gitignore`d `Cargo.lock` and fail with an explicit error.
- Artifact manifest checksum computed with a non-SHA-256 algorithm: the runtime MUST reject manifests with unsupported checksum algorithms rather than silently skipping verification.
- SLSA provenance statement signed by a key not in the trusted keystore: verification MUST fail with `provenance_key_untrusted` rather than `provenance_missing`.
- Race condition in nightly SBOM generation when a dependency is updated between the lock file read and the SBOM write: the CI script MUST use the committed `Cargo.lock` as the authoritative input, not a freshly resolved lock.

## Functional Requirements

- **FR-001**: `Cargo.lock` MUST be committed to the repository and MUST NOT be listed in `.gitignore`; the nightly CI MUST fail if `Cargo.lock` is absent or excluded.
- **FR-002**: The top-level `Cargo.toml` workspace MUST configure `[workspace.metadata.cyclonedx]` to enable `cargo cyclonedx` SBOM generation with all features enabled.
- **FR-003**: The nightly CI workflow MUST run `cargo cyclonedx` (or a pinned equivalent) and write a CycloneDX-format SBOM to the workflow artifact store as `traverse-sbom.cdx.json`.
- **FR-004**: The generated SBOM MUST list all direct and transitive Cargo dependencies with package name, version, and license identifier.
- **FR-005**: The nightly CI workflow MUST fail if the generated SBOM lists zero transitive dependencies.
- **FR-006**: Every release artifact MUST include a SHA-256 checksum in its manifest under the field `checksum_sha256`.
- **FR-007**: The runtime MUST verify the SHA-256 checksum of every artifact against the manifest value before execution; a mismatch MUST cause rejection with `checksum_mismatch`.
- **FR-008**: The nightly CI workflow MUST verify the SHA-256 checksum for every governed artifact and fail the job when any checksum mismatches.
- **FR-009**: Artifact signing MUST use the same scheme as spec 030: Ed25519 keypair as the required baseline, Sigstore (Fulcio/Rekor) as an additional supported scheme for published artifacts.
- **FR-010**: The nightly CI workflow MUST run a signature presence check for all governed artifacts and fail the job when any governed artifact is missing a signature.
- **FR-011**: The nightly CI workflow MUST emit a structured warning (not a hard failure) for ungoverned artifacts missing a signature.
- **FR-012**: The release build pipeline MUST produce a SLSA Level 1 provenance statement for each release artifact; the statement MUST include: source commit SHA, build system identifier, artifact SHA-256, and build invocation reference.
- **FR-013**: The provenance statement MUST be included in the release archive alongside the artifact and the SBOM.
- **FR-014**: Builds MUST be byte-identical across runs on the same platform and toolchain version; the `rust-toolchain.toml` file MUST pin the exact Rust toolchain version used for release builds.
- **FR-015**: Release binaries MUST NOT embed build timestamps or other non-deterministic metadata; CI MUST verify byte-identity by hashing the build output of two identical inputs and asserting SHA-256 equality.
- **FR-016**: The `traverse-cli` MUST implement an `artifact verify <path>` subcommand that outputs a structured JSON report containing: `overall_status`, `signature_status`, `checksum_status`, `provenance_status`, and per-check evidence fields.
- **FR-017**: `traverse-cli artifact verify` MUST exit with code 0 only when all checks pass; any single check failure MUST produce a non-zero exit code.
- **FR-018**: The `artifact verify` report MUST enumerate all checks even when an early check fails; it MUST NOT short-circuit after the first failure.
- **FR-019**: The nightly CI workflow MUST be defined as a separate GitHub Actions workflow file (`.github/workflows/supply-chain.yml`) triggered on schedule and on push to the main branch.
- **FR-020**: The CI workflow MUST write a machine-readable supply-chain summary report to the workflow artifact store after each run, whether passing or failing.
- **FR-021**: The `scripts/ci/` directory MUST contain a `supply_chain_check.sh` script that can be run locally to reproduce the nightly CI checks without GitHub Actions infrastructure.
- **FR-022**: Artifact manifest files MUST reject unsupported checksum algorithm identifiers with an explicit `unsupported_checksum_algorithm` error rather than silently skipping verification.

## Non-Functional Requirements

- **NFR-001 Reproducibility**: Byte-identical builds are REQUIRED on the same platform and pinned toolchain; cross-platform byte identity is explicitly not required and MUST NOT be claimed.
- **NFR-002 Toolchain Pinning**: The exact Rust toolchain version MUST be pinned in `rust-toolchain.toml` and the CI workflow MUST use this pinned version for all release builds.
- **NFR-003 SBOM Completeness**: The SBOM MUST cover transitive dependencies, not only direct Cargo dependencies; SBOM generation tool configuration MUST enable full dependency tree traversal.
- **NFR-004 Provenance Level**: SLSA Level 1 is the target for v0; SLSA Level 2 (signed provenance from a hosted build platform) is deferred to a follow-on spec and MUST NOT block this spec's completion.
- **NFR-005 Verifiability**: The SBOM and provenance artifacts MUST be parseable by standard third-party tools (e.g., `cyclonedx-cli`, `slsa-verifier`) without Traverse-specific tooling.
- **NFR-006 Nightly Gate**: The nightly supply-chain workflow MUST complete within 15 minutes; if it exceeds this bound, the workflow MUST fail with a timeout error rather than producing a partial report.
- **NFR-007 Local Reproducibility**: The `supply_chain_check.sh` script MUST run to completion on a developer workstation with the pinned toolchain installed, producing the same report as the CI workflow.
- **NFR-008 Partial Provenance Surfacing**: When a release contains a mix of signed and unsigned artifacts, the overall status MUST be `partial_provenance` (not `passed` or `failed`); the report MUST enumerate each artifact's individual status.

## Non-Negotiable Quality Standards

- **QG-001**: `Cargo.lock` MUST be present and committed; a CI check MUST explicitly verify this and fail the workflow if it is missing.
- **QG-002**: Every governed artifact MUST have a SHA-256 checksum in its manifest; the nightly CI MUST fail for any governed artifact with a missing or malformed checksum field.
- **QG-003**: The `traverse-cli artifact verify` command MUST output a structured JSON report for every invocation regardless of outcome; plain-text-only output without a machine-readable field MUST fail the quality gate.
- **QG-004**: Reproducible build verification MUST be automated: the CI pipeline MUST build the same artifact twice and assert SHA-256 equality of the outputs; a single non-deterministic build MUST fail the gate.
- **QG-005**: The nightly supply-chain workflow MUST be a first-class CI gate: a failing nightly run MUST create a GitHub issue or send a notification to the maintainers; silent failures are not acceptable.
- **QG-006**: 100% automated line coverage MUST be maintained for `traverse-cli artifact verify` logic including checksum verification, signature dispatch, and provenance parsing.

## Key Entities

- **CycloneDX SBOM**: A software bill of materials in CycloneDX JSON format listing all direct and transitive Cargo dependencies with package name, version, and license identifier.
- **SLSA Level 1 Provenance Statement**: A build provenance document linking a source commit SHA to a produced artifact SHA-256, including build system identifier and build invocation reference.
- **Artifact Manifest**: The structured metadata file accompanying each artifact, containing `checksum_sha256`, `signing_scheme`, `signature`, and provenance reference fields.
- **Reproducible Build**: A build process that produces byte-identical output for the same source, toolchain version, and build environment; non-deterministic elements (timestamps, PIE randomization) are explicitly excluded.
- **Supply-Chain CI Workflow**: The nightly GitHub Actions workflow (`.github/workflows/supply-chain.yml`) running SBOM generation, checksum validation, signature presence checks, and reproducible build verification.
- **artifact verify Command**: The `traverse-cli artifact verify <path>` subcommand that runs all supply-chain checks against a local artifact path and outputs a structured JSON report.
- **Governed Artifact**: Any artifact referenced by an entry in `contracts/` or `specs/governance/approved-specs.json`; subject to REQUIRED checksum, signature, and provenance checks.
- **Partial Provenance**: An overall status indicating a release contains a mix of fully verified and partially verified or unsigned artifacts.
- **rust-toolchain.toml**: The Rust toolchain pin file that specifies the exact toolchain version used for release builds to enable reproducibility.

## Success Criteria

- **SC-001**: `traverse-cli artifact verify <path>` returns a structured JSON report with `overall_status: passed` for a valid, signed, checksummed artifact with a provenance statement; non-zero exit code for any check failure.
- **SC-002**: The nightly CI workflow fails explicitly when any governed artifact's SHA-256 checksum mismatches; the failure message names the artifact path and the expected versus actual hash.
- **SC-003**: A published Traverse release archive contains a CycloneDX SBOM listing all transitive Cargo dependencies and a SLSA Level 1 provenance statement, both parseable by standard third-party tools.
- **SC-004**: The nightly CI workflow emits a structured warning for each ungoverned WASM module missing a signature, and fails the job for any governed module missing a signature.
- **SC-005**: The reproducible build gate produces byte-identical output across two back-to-back builds on the same platform and pinned toolchain, verified by SHA-256 comparison in CI.
- **SC-006**: `supply_chain_check.sh` runs to completion locally and produces a report equivalent to the nightly CI workflow output.

## Out of Scope

- SLSA Level 2 or higher (deferred to a follow-on spec)
- Cross-platform byte-identical reproducible builds
- Hardware security module (HSM) integration for signing keys
- Binary transparency log for Traverse releases beyond SLSA provenance
- Dependency vulnerability scanning (CVE database queries)
- License compliance enforcement (SBOM generation is in scope; enforcement policy is not)
- Container image supply chain hardening (Traverse ships as a Cargo workspace, not a container image in v0)
- Automatic dependency update PRs (Dependabot configuration)
