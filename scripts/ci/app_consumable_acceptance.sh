#!/usr/bin/env bash

set -euo pipefail

# scripts/ci/app_consumable_acceptance.sh is the authoritative app-consumable acceptance entrypoint.

bash scripts/ci/react_demo_live_adapter_smoke.sh

echo "App-consumable acceptance passed."
