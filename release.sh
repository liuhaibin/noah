#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

ARGS=()
for arg in "$@"; do
  if [[ "$arg" == "--build" ]]; then
    ARGS+=("--build")
  elif [[ "$arg" == "--upload" ]]; then
    ARGS+=("--upload")
  else
    ARGS+=("$arg")
  fi
done

node ./scripts/release.mjs "${ARGS[@]}"
