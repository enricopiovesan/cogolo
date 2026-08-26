# Browser Demo Governance Boundary

Tracks [#1157](https://github.com/traverse-framework/traverse/issues/1157).
This is an integration guide, not a new execution authority. Existing
approved specs fully govern the boundary: `108-governed-runtime-workflow-composition`
defines provider neutrality and the untrusted-proposer boundary;
`109-runtime-workflow-proposals` governs proposal validation, authorization,
execution, and redacted traces; and `113-declarative-workflow-planning`
governs deterministic candidate generation. ADR-0043 and ADR-0050 record the
same decisions. No spec or ADR amendment is required for this documentation
slice.

## Roles and trust boundaries

| Role | Trust and authority |
| --- | --- |
| Browser client | An untrusted consumer. It can read public registry metadata, request direct execution through an approved host entry point, and display redacted evidence. It cannot mutate the catalog, manifest, or a workflow. |
| Optional planner | An untrusted external or browser-local proposer. It may create candidate proposals only; it receives neither runtime authority nor a bypass of review. Traverse does not host its model, planner credentials, or prompts. |
| Registry metadata source | Supplies versioned contract and artifact metadata. A browser must treat it as discovery input, never as execution authorization. |
| Runtime | The sole validator and executor. It canonicalizes and validates pinned proposals, verifies artifacts and host state, decides whether approval is needed, and produces execution evidence. |
| Trace sink | Receives only the bounded redacted trace projection defined by spec 109. It must not receive raw inputs, outputs, secrets, planner prompts, or approval-token material. |

## Two separate journeys

### Direct single-capability execution

A browser may request one capability only through an approved, browser-reachable
runtime entry point. Before execution, the runtime must resolve the published
version and verify the artifact identity, then apply the capability's manifest,
placement, policy, schema, risk, and resource controls. The browser displays
the returned redacted trace evidence and must describe any unavailable transport
or verification path as unavailable rather than simulating a successful run.

### Multi-step workflow proposal

A browser or optional planner may submit a bounded proposal, but it cannot
execute merely by being submitted. The runtime canonicalizes the proposal and
binds it to manifest, registry, policy, and budget snapshots. It validates the
declared capability set, exact artifact digests, placement, connector and
field-level data-flow policy, mapping schemas, risk metadata, and resource
limits. Ambiguous candidates remain alternatives for a reviewer; neither the
planner nor the runtime resolves them automatically. Any proposal requiring
approval needs a verified, scoped, bounded-use approval token before execution.
The runtime then executes only the validated sequential proposal and emits a
bounded redacted trace.

## Explicit prohibitions

- Traverse must not host a model, retain planner prompts or credentials, or
  treat natural-language interpretation as runtime authority.
- A planner must not create catalog entries, mutate manifests, alter a proposal
  after review, resolve ambiguity automatically, or execute a capability.
- Schema compatibility, public registry visibility, or a rendered browser
  diagram must never be represented as execution authorization or evidence.

## Verification evidence

Review this guide against specs 108, 109, and 113 plus ADR-0043 and ADR-0050.
Any future browser demo implementation must add an end-to-end test covering:

1. verified direct execution with visible redacted trace evidence;
2. rejected invalid or unauthorized workflow proposals; and
3. the absence of planner-hosted authority, prompt storage, and automatic
   ambiguity resolution.
