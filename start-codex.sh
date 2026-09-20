#!/usr/bin/env bash
# Start Codex in a persistent VM dedicated to the requested workspace.
# Usage: ./start-codex.sh [workspace] [-- codex arguments...]
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-$SCRIPT_DIR/artifacts}"
STATE_ROOT="${STATE_ROOT:-$SCRIPT_DIR/.smolvm}"
CODEX_MACHINE_OVERRIDE="${CODEX_MACHINE:-}"

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

if ! command -v smolvm >/dev/null 2>&1; then
  echo "ERROR: smolvm is not installed or is not on PATH." >&2
  exit 1
fi

if [ ! -d "$WORKSPACE_INPUT" ]; then
  echo "ERROR: workspace is not a directory: $WORKSPACE_INPUT" >&2
  exit 1
fi

if [ ! -s "$STATE_ROOT/base-id" ]; then
  echo "ERROR: no built base generation was found." >&2
  echo "Run $SCRIPT_DIR/build-sandbox.sh first." >&2
  exit 1
fi

hash_stream() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  else
    shasum -a 256 | awk '{print $1}'
  fi
}

machine_exists() {
  smolvm machine status --name "$1" >/dev/null 2>&1
}

WORKSPACE_DIR="$(CDPATH= cd -- "$WORKSPACE_INPUT" && pwd -P)"
BASE_ID="$(sed -n '1p' "$STATE_ROOT/base-id")"
MACHINE_REVISION="$(sed -n '1p' "$STATE_ROOT/machine-revision" 2>/dev/null || printf '1\n')"
WORKSPACE_ID="$(printf '%s' "$WORKSPACE_DIR" | hash_stream | cut -c1-12)"
ARTIFACT_FILE="$ARTIFACTS_DIR/codex-devtools-$BASE_ID.smolmachine"
CODEX_MACHINE="${CODEX_MACHINE_OVERRIDE:-codex-workspace-$BASE_ID-r$MACHINE_REVISION-$WORKSPACE_ID}"

if [ ! -s "$ARTIFACT_FILE" ]; then
  echo "ERROR: base artifact is missing: $ARTIFACT_FILE" >&2
  echo "Run $SCRIPT_DIR/build-sandbox.sh to repair it." >&2
  exit 1
fi

if ! machine_exists "$CODEX_MACHINE"; then
  # This is deliberately a named machine, not `pack run`: SmolVM extracts the
  # shared base layers once and this workspace keeps its own persistent overlay.
  "$SCRIPT_DIR/build-sandbox.sh" "$WORKSPACE_DIR"

  # The build may have advanced to a new definition-addressed generation because
  # the Smolfile or smolvm version changed since the previous run.
  BASE_ID="$(sed -n '1p' "$STATE_ROOT/base-id")"
  MACHINE_REVISION="$(sed -n '1p' "$STATE_ROOT/machine-revision" 2>/dev/null || printf '1\n')"
  CODEX_MACHINE="${CODEX_MACHINE_OVERRIDE:-codex-workspace-$BASE_ID-r$MACHINE_REVISION-$WORKSPACE_ID}"
fi

stop_codex_machine() {
  smolvm machine stop --name "$CODEX_MACHINE" >/dev/null 2>&1 || true
}
trap stop_codex_machine EXIT INT TERM

smolvm machine start --name "$CODEX_MACHINE"

# The mounted Codex home persists login and sessions outside the VM. Seed it
# once with the config, skills, and MCP registration baked into the artifact.
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
