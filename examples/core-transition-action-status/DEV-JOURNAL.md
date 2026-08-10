# core.transition-action-status — dev journal

- Source contract: Loop capability package v2 `core.transition-action-status@1.0.0`, adapted for Traverse deserializer (`provenance.source`, `evidence` shape) and WF4 `snoozed` alignment.
- Guest: `#![no_std]` evaluator over inlined `transition_config.allowed_transitions` + `owner_only`.
- Smoke: `bash scripts/ci/core_transition_action_status_example_smoke.sh`.
- Ticket: traverse-framework/traverse#1026.

- 1.2.0 honesty bump (#1046): declare `emits`/`event_emission` for governed event `core.action-item.status-transitioned@1.0.0` (already published in registry). Declaration-only — WASM guest still pure memory_only evaluation.
