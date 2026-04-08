#!/usr/bin/env bash

set -euo pipefail

cargo test -p traverse-mcp validates_youaskm3_mcp_consumption_path

echo "MCP consumption validation passed."
