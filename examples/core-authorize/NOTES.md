# core.authorize

Governed example for `core.authorize@1.1.0` — deterministic hybrid authorization evaluator.

Policy is supplied at invocation (RBAC / ABAC / hybrid). Guest implements the nine published use cases: allow/deny match, tenant isolation, ownership, suspended override, obligations, fail-closed default, empty policy, break-glass, and invalid principal.

`policy_hash` is an honest `fnv1a64:` content fingerprint of the supplied
policy. It is for deterministic correlation in evaluation evidence, not a
cryptographic SHA-256 digest.

## Run

```bash
bash scripts/ci/core_authorize_example_smoke.sh
```

## Coverage

| ID | Fixture | Expected `reason_code` |
|----|---------|------------------------|
| UC-01 | `uc01-admin-delete-allow.json` | `matched_allow_rule` |
| UC-02 | `uc02-tenant-isolation-deny.json` | `matched_deny_rule` |
| UC-03 | `uc03-owner-update-allow.json` | `matched_allow_rule` |
| UC-04 | `uc04-suspended-deny.json` | `matched_deny_rule` |
| UC-05 | `uc05-obligations-allow.json` | `matched_allow_rule` (+ obligations) |
| UC-06 | `uc06-no-match-deny.json` | `no_matching_rule` |
| UC-07 | `uc07-empty-policy-deny.json` | `empty_or_invalid_policy` |
| UC-08 | `uc08-break-glass-allow.json` | `break_glass_override` |
| UC-09 | `uc09-invalid-principal-deny.json` | `invalid_principal` |

## Guest-enforced fields

`principal.id` is intentionally omitted from `inputs.schema.properties.principal.required`
so UC-09 can reach the WASM guest. Host JSON Schema would otherwise reject the
omission before execution. The guest returns `reason_code: invalid_principal`.
This follows the guest-enforced fields convention in
`docs/capability-contract-authoring-guide.md` — it is not a silent weakening of
default fail-closed host validation for other required fields.

Registry: publish `capabilities/core/core.authorize/1.1.0/` with a real artifact release after this package lands.
