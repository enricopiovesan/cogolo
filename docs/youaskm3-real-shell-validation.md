# youaskm3 Real Shell Validation

This document defines the Traverse-side validation path for the real browser-hosted `youaskm3` shell against released Traverse consumer artifacts.

It is the evidence path for the downstream shell specification at [youaskm3/openspec/specs/pwa-shell/spec.md](https://github.com/enricopiovesan/youaskm3/blob/main/openspec/specs/pwa-shell/spec.md).
This youaskm3 real shell validation is the Traverse-side proof path for the downstream browser-hosted shell.

## Governing Spec

- `specs/019-downstream-consumer-contract/spec.md`
- `specs/020-downstream-integration-validation/spec.md`
- `specs/021-app-facing-operational-constraints/spec.md`

## Purpose

Use one deterministic Traverse-side validation flow to prove that the downstream browser-hosted shell can consume the released Traverse consumer artifacts without relying on private Traverse internals or undocumented setup.

## Prerequisites

- A local Traverse checkout with the released consumer bundle documentation available.
- A checked-out `youaskm3` repository available at `YOUASKM3_REPO_ROOT`.
- The downstream repository exposes the browser-hosted shell spec at `openspec/specs/pwa-shell/spec.md`.
- The downstream repository exposes its own deterministic smoke path in `scripts/smoke.sh`.

## Traverse Validation Path

Run the Traverse-side wrapper:

```bash
bash scripts/ci/youaskm3_real_shell_validation.sh
```

That wrapper validates the documented Traverse release artifacts first and then, when `YOUASKM3_REPO_ROOT` is set, verifies that the downstream shell repository is present and can run its own smoke path.

## Expected Evidence

The validation path should prove:

- the released Traverse consumer bundle is documented
- the downstream browser-hosted shell spec is present
- the downstream shell repo can be located through `YOUASKM3_REPO_ROOT`
- the downstream shell can run its repo-local smoke validation
- the observed path uses only documented public surfaces
- at least one setup or incompatibility failure mode is detectable

## Known Failure Modes

The validation is expected to fail deterministically when:

- the released Traverse consumer bundle documentation is missing
- `YOUASKM3_REPO_ROOT` is unset or points at the wrong repository
- the downstream shell spec is missing
- the downstream smoke path is unavailable
- the downstream shell environment is missing its lint toolchain, which is detected when `scripts/smoke.sh` fails at the `eslint` step

## Validation

- `bash scripts/ci/youaskm3_real_shell_validation.sh`
- `bash scripts/ci/youaskm3_integration_validation.sh`
- `bash scripts/ci/repository_checks.sh`
