#!/usr/bin/env bash
# Inspect or explicitly replace stale workspace machines.
set -euo pipefail

show_help() {
  printf '%s\n' \
    "Usage: ${0##*/} [--help] (--check | --replace-stale [--force]) [--allow-host HOST]... [WORKSPACE]" \
    "" \
    "Description:" \
    "  Inspect reconciliation state or explicitly replace stale workspace machines." \
    "  Exactly one of --check and --replace-stale is required for an operation." \
    "" \
    "Arguments:" \
    "  WORKSPACE           Workspace to inspect or repair (default: current directory)." \
    "" \
    "Options:" \
    "  --check             Report reconciliation state without replacing machines." \
    "  --replace-stale     Prepare and verify the desired machine before removing" \
    "                      eligible stale machines." \
    "  --force             With --replace-stale only, allow a running stale machine" \
    "                      to be stopped during replacement." \
    "  --allow-host HOST" \
    "  --allow-host=HOST   Add HOST to the workspace network allowlist used by" \
    "                      start-codex.sh. Repeatable before or after WORKSPACE; HOST" \
    "                      must be non-empty, contain no line breaks, and not start with -." \
    "  --help              Display this help and exit; no operation mode is required."
}

for argument in "$@"; do
  if [ "$argument" = --help ]; then
    show_help
    exit 0
  fi
done

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=sandbox-common.sh
source "$SCRIPT_DIR/sandbox-common.sh"

MODE=""
FORCE=""
WORKSPACE_INPUT=""
usage() {
  echo "Usage: $0 (--check | --replace-stale [--force]) [--allow-host HOST]... [workspace]" >&2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --check)
      [ -z "$MODE" ] || { usage; exit 2; }
      MODE=check
      ;;
    --replace-stale)
      [ -z "$MODE" ] || { usage; exit 2; }
      MODE=replace-stale
      ;;
    --force)
      FORCE=force
      ;;
    --allow-host)
      if [ "$#" -lt 2 ] || [ -z "$2" ] || [[ "$2" == -* ]]; then
        echo "ERROR: --allow-host requires a host." >&2
        usage
        exit 2
      fi
      SANDBOX_REQUESTED_ALLOW_HOSTS+=("$2")
      shift
      ;;
    --allow-host=*)
      host="${1#--allow-host=}"
      if [ -z "$host" ] || [[ "$host" == -* ]]; then
        echo "ERROR: --allow-host requires a host." >&2
        usage
        exit 2
      fi
      SANDBOX_REQUESTED_ALLOW_HOSTS+=("$host")
      ;;
    -*)
      echo "ERROR: unknown option: $1" >&2
      exit 2
      ;;
    *)
      [ -z "$WORKSPACE_INPUT" ] || { usage; exit 2; }
      WORKSPACE_INPUT="$1"
      ;;
  esac
  shift
done

if [ -z "$MODE" ] || { [ "$MODE" = check ] && [ -n "$FORCE" ]; }; then
  usage
  exit 2
fi

SANDBOX_ENSURE_LOGIN=0
reconcile_workspace "${WORKSPACE_INPUT:-$PWD}" "$MODE" "$FORCE"
if [ "$MODE" = replace-stale ]; then
  echo "Workspace machine ready: $RECONCILED_MACHINE"
fi
