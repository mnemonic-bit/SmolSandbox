#!/usr/bin/env bash
# Start Codex in a persistent VM dedicated to the requested workspace.
# Usage: ./start-codex.sh [--allow-host HOST]... [workspace] [-- codex arguments...]
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=sandbox-common.sh
source "$SCRIPT_DIR/sandbox-common.sh"

usage() {
  echo "Usage: $0 [--allow-host HOST]... [workspace] [-- codex arguments...]" >&2
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

SANDBOX_ENSURE_LOGIN=0
reconcile_workspace "$WORKSPACE_INPUT" start
CODEX_MACHINE="$RECONCILED_MACHINE"

stop_codex_machine() {
  smolvm machine stop --name "$CODEX_MACHINE" >/dev/null 2>&1 || true
}
trap stop_codex_machine EXIT INT TERM

smolvm machine start --name "$CODEX_MACHINE"

if ! smolvm machine exec --name "$CODEX_MACHINE" -- sh -lc \
  'if [ -d /opt/codex-home ]; then cp -r --update=none /opt/codex-home/. /root/.codex/; fi'
then
  echo "WARNING: could not seed the persistent Codex home; continuing with its existing contents." >&2
fi

CODEX_EXEC=(smolvm machine exec --name "$CODEX_MACHINE" -it)
if [ -n "${OPENAI_API_KEY:-}" ]; then
  CODEX_EXEC+=(--secret-env OPENAI_API_KEY=OPENAI_API_KEY)
fi
CODEX_EXEC+=(-- codex "${CODEX_ARGUMENTS[@]}")

echo "Starting Codex in $WORKSPACE_DIR (machine: $CODEX_MACHINE)..."
"${CODEX_EXEC[@]}"
