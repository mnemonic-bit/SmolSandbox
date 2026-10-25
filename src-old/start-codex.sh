#!/usr/bin/env bash
# Start Codex in a persistent VM dedicated to the requested workspace.
# Usage: ./start-codex.sh [--allow-host HOST]... [--port PORT]... [workspace] [-- codex arguments...]
set -euo pipefail

show_help() {
  printf '%s\n' \
    "Usage: ${0##*/} [--help] [--allow-host HOST]... [--port PORT]... [WORKSPACE] [-- CODEX_ARG...]" \
    "" \
    "Description:" \
    "  Start Codex in the persistent sandbox machine for a workspace." \
    "" \
    "Arguments:" \
    "  WORKSPACE     Workspace mounted in the sandbox (default: current directory)." \
    "  CODEX_ARG...  Arguments after -- are forwarded to Codex without interpretation." \
    "" \
    "Options:" \
    "  --allow-host HOST" \
    "  --allow-host=HOST  Add HOST to the default network allowlist. Repeatable; may" \
    "                     appear before or after WORKSPACE, but not after --. HOST" \
    "                     must be non-empty, contain no line breaks, and not start with -." \
    "  --port PORT" \
    "  --port=PORT        Publish repeatable TCP PORT on the same guest and host port." \
    "                     Ports are 1-65535, bind to host loopback by default, and may" \
    "                     appear before or after WORKSPACE. The exact set remains on the" \
    "                     selected machine until a later launch reconciles another set;" \
    "                     omitting --port clears every earlier mapping on this launch." \
    "  --help             Display launcher help and exit." \
    "  --                 End sandbox option parsing. All remaining options belong" \
    "                     to Codex; for example, -- --help requests Codex help."
}

for argument in "$@"; do
  if [ "$argument" = -- ]; then
    break
  fi
  if [ "$argument" = --help ]; then
    show_help
    exit 0
  fi
done

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=sandbox-common.sh
source "$SCRIPT_DIR/sandbox-common.sh"

usage() {
  echo "Usage: $0 [--allow-host HOST]... [--port PORT]... [workspace] [-- codex arguments...]" >&2
}

append_requested_port() {
  local raw="$1" canonical
  if [[ ! "$raw" =~ ^[0123456789]+$ ]]; then
    echo "ERROR: --port requires an ASCII decimal integer from 1 through 65535." >&2
    usage
    exit 2
  fi
  canonical="${raw#"${raw%%[!0]*}"}"
  canonical="${canonical:-0}"
  if [ "$canonical" = 0 ] || [ "${#canonical}" -gt 5 ] || \
     { [ "${#canonical}" -eq 5 ] && ((canonical > 65535)); }
  then
    echo "ERROR: --port requires an ASCII decimal integer from 1 through 65535." >&2
    usage
    exit 2
  fi
  SANDBOX_REQUESTED_PORTS+=("$canonical")
}

WORKSPACE_INPUT=""
CODEX_ARGUMENTS=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --allow-host)
      if [ "$#" -lt 2 ] || [ -z "$2" ] || [[ "$2" == -* ]]; then
        echo "ERROR: --allow-host requires a host." >&2
        usage
        exit 2
      fi
      SANDBOX_REQUESTED_ALLOW_HOSTS+=("$2")
      shift 2
      ;;
    --allow-host=*)
      host="${1#--allow-host=}"
      if [ -z "$host" ] || [[ "$host" == -* ]]; then
        echo "ERROR: --allow-host requires a host." >&2
        usage
        exit 2
      fi
      SANDBOX_REQUESTED_ALLOW_HOSTS+=("$host")
      shift
      ;;
    --port)
      if [ "$#" -lt 2 ]; then
        echo "ERROR: --port requires a value." >&2
        usage
        exit 2
      fi
      append_requested_port "$2"
      shift 2
      ;;
    --port=*)
      append_requested_port "${1#--port=}"
      shift
      ;;
    --)
      shift
      CODEX_ARGUMENTS=("$@")
      break
      ;;
    -*)
      echo "ERROR: unknown sandbox option: $1" >&2
      usage
      exit 2
      ;;
    *)
      if [ -n "$WORKSPACE_INPUT" ]; then
        usage
        exit 2
      fi
      WORKSPACE_INPUT="$1"
      shift
      ;;
  esac
done
WORKSPACE_INPUT="${WORKSPACE_INPUT:-$PWD}"
if [ "${#SANDBOX_REQUESTED_PORTS[@]}" -gt 0 ]; then
  mapfile -t SANDBOX_REQUESTED_PORTS < <(
    printf '%s\n' "${SANDBOX_REQUESTED_PORTS[@]}" | LC_ALL=C sort -n -u
  )
fi
if [ "${#SANDBOX_REQUESTED_PORTS[@]}" -gt 64 ]; then
  echo "ERROR: at most 64 distinct --port values may be requested." >&2
  usage
  exit 2
fi

SANDBOX_ENSURE_LOGIN=0
SANDBOX_KEEP_PURGE_LOCK=1
SANDBOX_KEEP_WORKSPACE_LOCK=1
SANDBOX_REQUIRE_PORT_CAPABILITY=1
reconcile_workspace "$WORKSPACE_INPUT" start
CODEX_MACHINE="$RECONCILED_MACHINE"
CODEX_MACHINE_STOP_ON_EXIT=0

stop_codex_machine() {
  if [ "$CODEX_MACHINE_STOP_ON_EXIT" = 1 ]; then
    smolvm machine stop --name "$CODEX_MACHINE" >/dev/null 2>&1 || true
  fi
  sandbox_release_lock
  sandbox_release_purge_lock
}
trap stop_codex_machine EXIT INT TERM

sandbox_reconcile_workspace_ports "$CODEX_MACHINE"
if [ "${SANDBOX_SELECTED_MACHINE_WAS_RUNNING:-0}" != 1 ]; then
  CODEX_MACHINE_STOP_ON_EXIT=1
  smolvm machine start --name "$CODEX_MACHINE"
fi
sandbox_release_lock
sandbox_release_purge_lock

if ! smolvm machine exec --name "$CODEX_MACHINE" -- sh -lc \
  'if [ -d /opt/codex-home ]; then cp -r --update=none /opt/codex-home/. /root/.codex/; fi'
then
  echo "WARNING: could not seed the persistent Codex home; continuing with its existing contents." >&2
fi

CODEX_EXEC=(smolvm machine exec --name "$CODEX_MACHINE" -it)
if [ -n "${OPENAI_API_KEY:-}" ]; then
  CODEX_EXEC+=(--secret-env OPENAI_API_KEY=OPENAI_API_KEY)
fi
CODEX_EXEC+=(-- codex --no-daemon "${CODEX_ARGUMENTS[@]}")

echo "Starting Codex in $WORKSPACE_DIR (machine: $CODEX_MACHINE)..."
"${CODEX_EXEC[@]}"
