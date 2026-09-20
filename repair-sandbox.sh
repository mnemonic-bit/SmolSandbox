#!/usr/bin/env bash
# Inspect or explicitly replace stale workspace machines.
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=sandbox-common.sh
source "$SCRIPT_DIR/sandbox-common.sh"

MODE=""
FORCE=""
WORKSPACE_INPUT=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --check)
      [ -z "$MODE" ] || { echo "Usage: $0 (--check | --replace-stale [--force]) [workspace]" >&2; exit 2; }
      MODE=check
      ;;
    --replace-stale)
      [ -z "$MODE" ] || { echo "Usage: $0 (--check | --replace-stale [--force]) [workspace]" >&2; exit 2; }
      MODE=replace-stale
      ;;
    --force)
      FORCE=force
      ;;
    -*)
      echo "ERROR: unknown option: $1" >&2
      exit 2
      ;;
    *)
      [ -z "$WORKSPACE_INPUT" ] || { echo "Usage: $0 (--check | --replace-stale [--force]) [workspace]" >&2; exit 2; }
      WORKSPACE_INPUT="$1"
      ;;
  esac
  shift
done

if [ -z "$MODE" ] || { [ "$MODE" = check ] && [ -n "$FORCE" ]; }; then
  echo "Usage: $0 (--check | --replace-stale [--force]) [workspace]" >&2
  exit 2
fi

SANDBOX_ENSURE_LOGIN=0
reconcile_workspace "${WORKSPACE_INPUT:-$PWD}" "$MODE" "$FORCE"
if [ "$MODE" = replace-stale ]; then
  echo "Workspace machine ready: $RECONCILED_MACHINE"
fi
