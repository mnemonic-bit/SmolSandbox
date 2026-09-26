#!/usr/bin/env bash
# Compatibility entry point: the unified build creates both runtime machines.
set -euo pipefail

show_help() {
  printf '%s\n' \
    "Usage: ${0##*/} [--help] [WORKSPACE]" \
    "" \
    "Description:" \
    "  Compatibility entry point for build-sandbox.sh. The unified build creates" \
    "  or reuses the sandbox base and prepares both login and workspace machines." \
    "" \
    "Arguments:" \
    "  WORKSPACE  Workspace whose machine is prepared (default: current directory)." \
    "" \
    "Options:" \
    "  --help     Display this help and exit."
}

for argument in "$@"; do
  if [ "$argument" = --help ]; then
    show_help
    exit 0
  fi
done

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
exec "$SCRIPT_DIR/build-sandbox.sh" "$@"
