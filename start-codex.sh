#!/usr/bin/env bash
# Start Codex in a persistent VM dedicated to the requested workspace.
# Usage: ./start-codex.sh [workspace] [-- codex arguments...]
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=sandbox-common.sh
source "$SCRIPT_DIR/sandbox-common.sh"

if [ "${1:-}" = "--" ]; then
  WORKSPACE_INPUT="$PWD"
  shift
elif [ -n "${1:-}" ]; then
  WORKSPACE_INPUT="$1"
  shift
  if [ "${1:-}" = "--" ]; then
    shift
  fi
else
  WORKSPACE_INPUT="$PWD"
fi

SANDBOX_ENSURE_LOGIN=0
reconcile_workspace "$WORKSPACE_INPUT" start
CODEX_MACHINE="$RECONCILED_MACHINE"

stop_codex_machine() {
  smolvm machine stop --name "$CODEX_MACHINE" >/dev/null 2>&1 || true
}
trap stop_codex_machine EXIT INT TERM

smolvm machine start --name "$CODEX_MACHINE"

if ! smolvm machine exec --name "$CODEX_MACHINE" -- sh -lc \
  'if [ -d /opt/codex-home ]; then cp -a --update=none /opt/codex-home/. /root/.codex/; fi'
then
  echo "WARNING: could not seed the persistent Codex home; continuing with its existing contents." >&2
fi

CODEX_EXEC=(smolvm machine exec --name "$CODEX_MACHINE" -it)
if [ -n "${OPENAI_API_KEY:-}" ]; then
  CODEX_EXEC+=(--secret-env OPENAI_API_KEY=OPENAI_API_KEY)
fi
CODEX_EXEC+=(-- codex "$@")

echo "Starting Codex in $WORKSPACE_DIR (machine: $CODEX_MACHINE)..."
"${CODEX_EXEC[@]}"
