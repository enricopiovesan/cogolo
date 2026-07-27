#!/usr/bin/env bash

set -euo pipefail

cargo test -p traverse-runtime \
  workflows::tests::executes_workflow_deterministically_and_supports_workflow_backed_capabilities \
  -- --exact

cargo test -p traverse-runtime \
  workflows::tests::event_driven_helpers_are_deterministic_and_prevent_duplicate_consumption \
  -- --exact

cargo test -p traverse-runtime \
  workflows::tests::event_driven_helpers_reject_non_matching_predicates \
  -- --exact
