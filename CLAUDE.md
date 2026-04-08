# Traverse Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-07

## Active Technologies

- Rust 1.94+
- Cargo workspace
- serde (JSON serialization)
- semver
- WASM (target)

## Project Structure

```text
crates/
  traverse-runtime/      # Core execution engine
  traverse-contracts/    # Contract definitions and validation
  traverse-registry/     # Capability and event registries
  traverse-cli/          # Command-line interface
  traverse-mcp/          # Model Context Protocol (stub)
specs/                   # Versioned, immutable governing specs
contracts/               # Capability and event contracts
docs/                    # ADRs, quality standards, policies
.specify/                # Speckit: constitution, scripts, templates
scripts/ci/              # Deterministic spec-alignment gate
```

## Commands

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo run -p traverse-cli
bash scripts/ci/spec_alignment_check.sh
```

## Code Style

- No `unsafe`, no `unwrap()`, no `panic!()`, no TODO comments
- 100% test coverage for core business and runtime logic
- Deterministic: same inputs must produce same outputs

## Recent Changes

- 183-add-claude-code-agent: Added Claude Code as parallel AI agent (CLAUDE.md, init-options.json, speckit-plan skill)

<!-- MANUAL ADDITIONS START -->
## Governance

Read `.specify/memory/constitution.md` before any implementation work.

### Key rules

1. **Spec-first**: every feature needs an approved spec in `specs/` before code — no spec, no merge
2. **Contract-first**: contracts are source of truth; code conforms to contracts, not vice versa
3. **Spec-alignment gate**: CI blocks PRs that drift from `specs/governance/approved-specs.json`
4. **Traceability**: all work must have a GitHub issue + Project 1 item + PR

### Approved specs

| ID  | Name |
|-----|------|
| 001 | foundation-v0-1 |
| 002 | capability-contracts |
| 003 | event-contracts |
| 004 | spec-alignment-gate |
| 005 | capability-registry |
| 006 | runtime-request-execution |
| 007 | workflow-registry-traversal |
| 008 | expedition-example-domain |
| 009 | expedition-example-artifacts |

### Feature branch convention

Feature branches must follow `NNN-feature-name` or `YYYYMMDD-HHMMSS-feature-name`.
Each feature gets a directory at `specs/<branch-name>/` with `spec.md` and `plan.md`.

### Development workflow

1. Clarify capability boundary
2. Define or amend governing spec in `specs/`
3. Define contracts in `contracts/`
4. Write tests
5. Implement smallest change satisfying spec + contract
6. Verify CI gate passes before opening PR
<!-- MANUAL ADDITIONS END -->
