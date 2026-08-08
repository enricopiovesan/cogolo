#!/usr/bin/env bash
set -euo pipefail

# Retired (spec 100-capability-package-authoring FR-009). This script used to
# emit a stale scaffold shape (flat "input_schema"/"output_schema" contract
# keys, no manifest.json, a std-linked cargo build) that cannot be loaded by
# `traverse-cli capability-package inspect`/`execute`. It now fails loudly
# and points authors at the real, governed create path instead of silently
# producing a broken scaffold.

cat >&2 <<'MSG'
scripts/scaffold/new-capability.sh has been retired (spec 100-capability-package-authoring FR-009).

It used to emit a scaffold shape that capability-package inspect/execute
cannot load (flat input_schema/output_schema contract keys, no manifest.json,
a std-linked build). Use the real create path instead:

  traverse-cli capability new <capability-id>

Run `traverse-cli capability new --help` for details.
MSG

exit 1
