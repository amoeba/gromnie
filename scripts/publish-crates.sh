#!/usr/bin/env bash
#
# Publish the publishable workspace crates to crates.io, in dependency order.
#
# crates.io rejects a crate whose dependencies are not yet published, and its
# index takes a moment to serve a new release, so each crate is published and
# then waited on before the next one is attempted. Re-running is safe: crates
# whose version is already live are skipped.
#
# Usage:
#   scripts/publish-crates.sh
#
# Set CARGO_REGISTRY_TOKEN to authenticate (the release workflow gets a
# short-lived token from crates.io trusted publishing).

set -euo pipefail

# Ordered so that every crate's internal dependencies appear before it.
# Keep in sync with the publication table in docs/releasing.md.
CRATES=(
  gromnie-events
  gromnie-scripting-api
  gromnie-client
  gromnie-scripting-host
  gromnie-runner
  gromnie-tui
  gromnie-cli
)

for arg in "$@"; do
  case "$arg" in
    -h | --help)
      sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

# Read the version from cargo metadata, so the script follows the workspace
# version without duplicating it.
version_of() {
  cargo metadata --no-deps --format-version 1 |
    jq -r --arg crate "$1" '.packages[] | select(.name == $crate) | .version'
}

is_published() {
  local crate="$1" version="$2"
  curl --silent --show-error --fail --output /dev/null \
    -H 'User-Agent: gromnie-release (https://github.com/amoeba/gromnie)' \
    "https://crates.io/api/v1/crates/${crate}/${version}" 2>/dev/null
}

wait_for_publish() {
  local crate="$1" version="$2" attempts=60
  for ((i = 1; i <= attempts; i++)); do
    if is_published "$crate" "$version"; then
      return 0
    fi
    sleep 5
  done
  echo "timed out waiting for ${crate} ${version} to appear on crates.io" >&2
  return 1
}

for crate in "${CRATES[@]}"; do
  version="$(version_of "$crate")"
  if [[ -z "$version" ]]; then
    echo "error: ${crate} is not a workspace package" >&2
    exit 1
  fi

  if is_published "$crate" "$version"; then
    echo "== ${crate} ${version}: already published, skipping"
    continue
  fi

  echo "== ${crate} ${version}: publishing"
  cargo publish -p "$crate"
  wait_for_publish "$crate" "$version"
  echo "== ${crate} ${version}: available on crates.io"
done
