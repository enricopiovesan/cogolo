# core.transition-action-status — dev journal

- Source contract: Loop capability package v2 `core.transition-action-status@1.0.0`, adapted for Traverse deserializer (`provenance.source`, `evidence` shape) and WF4 `snoozed` alignment.
- Guest: `#![no_std]` evaluator over inlined `transition_config.allowed_transitions` + `owner_only`.
- Smoke: `bash scripts/ci/core_transition_action_status_example_smoke.sh`.
- Ticket: traverse-framework/traverse#1026.
