#!/usr/bin/env bash
# Build the desired base and create/reuse the login and workspace machines.
set -euo pipefail

show_help() {
  printf '%s\n' \
    "Usage: ${0##*/} [--help] [WORKSPACE]" \
    "" \
    "Description:" \
    "  Build or reuse the sandbox base, then prepare the login and workspace" \
    "  machines." \
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

if [ "$#" -gt 1 ]; then
  echo "Usage: $0 [workspace]" >&2
  exit 2
fi

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=sandbox-common.sh
source "$SCRIPT_DIR/sandbox-common.sh"

WORKSPACE_INPUT="${1:-$PWD}"
SANDBOX_ENSURE_LOGIN=1
reconcile_workspace "$WORKSPACE_INPUT" start

echo
SANDBOX_PURGE_AFTER_BUILD=1 "$SCRIPT_DIR/purge-sandbox.sh"

echo
echo "Ready:"
echo "  artifact:  $ARTIFACT_FILE"
echo "  login VM:  $LOGIN_MACHINE"
echo "  Codex VM:  $RECONCILED_MACHINE"
echo "  workspace: $WORKSPACE_DIR"
echo
echo "Next: run $SCRIPT_DIR/start-login.sh once, then $SCRIPT_DIR/start-codex.sh '$WORKSPACE_DIR'"
