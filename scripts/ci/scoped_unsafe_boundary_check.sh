#!/usr/bin/env bash

set -euo pipefail

readonly swift_boundary="crates/traverse-swift-host/src/lib.rs"
readonly expedition_boundary="crates/traverse-expedition-wasm/src/wasi_stdio.rs"
readonly expedition_root="crates/traverse-expedition-wasm/src/main.rs"

if ! grep -Fqx 'unsafe_code = "deny"' Cargo.toml; then
  echo "Workspace unsafe-code lint must remain set to deny." >&2
  exit 1
fi

opt_outs=()
while IFS= read -r path; do
  opt_outs+=("${path}")
done < <(grep -RIlF --include='*.rs' '#![allow(unsafe_code)]' crates || true)
if [[ "${#opt_outs[@]}" -ne 1 || "${opt_outs[0]:-}" != "${swift_boundary}" ]]; then
  echo "Only ${swift_boundary} may use a crate-level unsafe-code opt-out." >&2
  exit 1
fi

unsafe_files=()
while IFS= read -r path; do
  unsafe_files+=("${path}")
done < <(grep -RIl --include='*.rs' -E '#\[unsafe\(|unsafe[[:space:]]*(\{|fn|impl|trait|extern)' crates || true)
for path in "${unsafe_files[@]}"; do
  if [[ "${path}" != "${swift_boundary}" && "${path}" != "${expedition_boundary}" ]]; then
    echo "Unsafe syntax is permitted only in ${swift_boundary} or ${expedition_boundary}." >&2
    exit 1
  fi
done

if [[ -f "${expedition_boundary}" ]]; then
  if ! grep -Fqx '#[allow(unsafe_code)]' "${expedition_root}"; then
    echo "The expedition guest must scope its unsafe-code allowance to wasi_stdio." >&2
    exit 1
  fi
  if [[ "$(grep -Fc 'mod wasi_stdio;' "${expedition_root}")" -ne 1 ]]; then
    echo "The expedition guest must expose exactly one wasi_stdio module." >&2
    exit 1
  fi
  if [[ "$(grep -Fc 'wasi_snapshot_preview1' "${expedition_boundary}")" -ne 1 ]]; then
    echo "The expedition boundary must import exactly one WASI Preview 1 module." >&2
    exit 1
  fi
  for symbol in fd_read fd_write proc_exit; do
    if [[ "$(grep -Ec "fn ${symbol}\\(" "${expedition_boundary}")" -ne 1 ]]; then
      echo "Missing or duplicate audited WASI symbol: ${symbol}" >&2
      exit 1
    fi
  done
  if [[ "$(grep -Ec '^[[:space:]]*(pub[[:space:]]+)?unsafe[[:space:]]+extern[[:space:]]+"C"' "${expedition_boundary}")" -ne 1 ]]; then
    echo "The expedition boundary must contain exactly one audited unsafe extern block." >&2
    exit 1
  fi
  if grep -Eq 'environ_get|path_|fd_(open|close|seek|sync)|random_get|clock_|sock_|proc_raise' "${expedition_boundary}"; then
    echo "The expedition boundary imports a forbidden WASI capability." >&2
    exit 1
  fi
fi

exports=(
  traverse_swift_host_abi_version
  traverse_swift_host_create
  traverse_swift_host_invoke
  traverse_swift_host_destroy
  traverse_swift_host_status_message
)
for symbol in "${exports[@]}"; do
  if [[ "$(grep -Fc "fn ${symbol}" "${swift_boundary}")" -ne 1 ]]; then
    echo "Missing or duplicate audited C-ABI symbol: ${symbol}" >&2
    exit 1
  fi
done
if [[ "$(grep -Fc '#[unsafe(no_mangle)]' "${swift_boundary}")" -ne 5 ]]; then
  echo "The audited Swift host must expose exactly five production C-ABI symbols." >&2
  exit 1
fi

echo "Scoped unsafe C-ABI boundary check passed."
