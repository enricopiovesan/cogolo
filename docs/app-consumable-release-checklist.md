# Traverse v0.1 App-Consumable Release Checklist

This checklist is the release decision aid for the first app-consumable Traverse release.

It does not redefine the downstream contract or the validation specs. It only states what must be true before Traverse can claim `app-consumable v0.1`, and what can safely wait until after release.

## Governing Specs

- `specs/019-downstream-consumer-contract/spec.md`
- `specs/020-downstream-integration-validation/spec.md`
- `specs/021-app-facing-operational-constraints/spec.md`

## Release Blockers

Traverse MUST NOT claim `app-consumable v0.1` unless all of the following are satisfied:

- [ ] The governed browser consumer path exists and is documented in [quickstart.md](/Users/piovese/Documents/cogolo/quickstart.md).
- [ ] The live local browser adapter path passes [scripts/ci/react_demo_live_adapter_smoke.sh](/Users/piovese/Documents/cogolo/scripts/ci/react_demo_live_adapter_smoke.sh).
- [ ] The browser demo path is documented as a real live adapter consumer in [apps/react-demo/README.md](/Users/piovese/Documents/cogolo/apps/react-demo/README.md).
- [ ] The downstream MCP consumption path exists and passes [scripts/ci/mcp_consumption_validation.sh](/Users/piovese/Documents/cogolo/scripts/ci/mcp_consumption_validation.sh).
- [ ] The first real `youaskm3` integration path exists and passes [scripts/ci/youaskm3_integration_validation.sh](/Users/piovese/Documents/cogolo/scripts/ci/youaskm3_integration_validation.sh).
- [ ] The end-to-end acceptance path exists and passes [scripts/ci/app_consumable_acceptance.sh](/Users/piovese/Documents/cogolo/scripts/ci/app_consumable_acceptance.sh).
- [ ] The operational constraints for app-facing browser and MCP surfaces are documented in [docs/adapter-boundaries.md](/Users/piovese/Documents/cogolo/docs/adapter-boundaries.md) and [docs/compatibility-policy.md](/Users/piovese/Documents/cogolo/docs/compatibility-policy.md).
- [ ] The consumer contract and integration-validation model remain aligned with approved governing specs.

If any item above is unchecked, `app-consumable v0.1` is blocked.

## Required Evidence

The release decision should be backed by:

- the first app-consumable quickstart
- the browser live-adapter smoke path
- the MCP consumption validation path
- the first real `youaskm3` integration validation path
- the end-to-end app-consumable acceptance path
- reviewable PR checks on the release-related documentation and validation artifacts

## Post-Release Follow-Up

The following are valid follow-up items after `app-consumable v0.1` and do not block the first release:

- release automation and packaging polish
- broader app-consumer templates for future downstream apps
- stronger production deployment hardening
- full auth and multi-tenant policy work
- broader performance baselines and load testing
- additional downstream validation paths beyond `youaskm3`

## Reviewer Shortcut

A reviewer can answer the release question by checking:

1. the governed consumer contract
2. the downstream integration-validation spec
3. the operational-constraints spec
4. the quickstart
5. the live browser adapter smoke path
6. the MCP validation path
7. the first real `youaskm3` integration validation path
8. the end-to-end acceptance path

If those artifacts and checks exist and are passing, the first app-consumable release can be evaluated on evidence rather than interpretation.
