#!/usr/bin/env bash
# Ensure the host's packaged WIT stays aligned with the scripting API contract.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_WIT="$ROOT/crates/gromnie-scripting-api/src/wit/gromnie-script.wit"
HOST_WIT="$ROOT/crates/gromnie-scripting-host/src/wit/gromnie-script.wit"

if ! cmp -s "$API_WIT" "$HOST_WIT"; then
  echo "error: scripting API and host WIT files differ:" >&2
  diff -u "$API_WIT" "$HOST_WIT" >&2 || true
  exit 1
fi
