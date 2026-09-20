#!/usr/bin/env bash
# Delete every VM and cached artifact belonging to the current base generation.
# Shared Codex state under .smolvm/codex-home is intentionally preserved.
set -euo pipefail

if [ "$#" -ne 0 ]; then
  echo "Usage: $0" >&2
  exit 2
fi

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-$SCRIPT_DIR/artifacts}"
STATE_ROOT="${STATE_ROOT:-$SCRIPT_DIR/.smolvm}"

if ! command -v smolvm >/dev/null 2>&1; then
  echo "ERROR: smolvm is not installed or is not on PATH." >&2
  exit 1
fi

if [ ! -s "$STATE_ROOT/base-id" ]; then
  echo "ERROR: no current base generation was found at $STATE_ROOT/base-id." >&2
  exit 1
fi

BASE_ID="$(sed -n '1p' "$STATE_ROOT/base-id")"
if [ "${#BASE_ID}" -ne 12 ]; then
  echo "ERROR: refusing to clean invalid base generation: $BASE_ID" >&2
  exit 1
fi
case "$BASE_ID" in
  *[!0-9a-f]*)
    echo "ERROR: refusing to clean invalid base generation: $BASE_ID" >&2
    exit 1
    ;;
esac

echo "Cleaning Codex base generation $BASE_ID..."

# Capture the list first. If SmolVM cannot enumerate its machines, leave the
# artifact and generation pointers intact so the cleanup can be retried.
MACHINE_LIST="$(smolvm machine ls --quiet)"
DELETED_MACHINES=0

while IFS= read -r machine; do
  [ -n "$machine" ] || continue

  case "$machine" in
    "codex-builder-$BASE_ID" | \
    "codex-login-$BASE_ID"-r* | \
    "codex-workspace-$BASE_ID"-r*)
      echo "  deleting VM: $machine"
      smolvm machine stop --name "$machine" >/dev/null 2>&1 || true
      smolvm machine delete --name "$machine" --force
      DELETED_MACHINES=$((DELETED_MACHINES + 1))
      ;;
  esac
done <<< "$MACHINE_LIST"

ARTIFACT_BASE="$ARTIFACTS_DIR/codex-devtools-$BASE_ID"
rm -f -- "$ARTIFACT_BASE" "$ARTIFACT_BASE.smolmachine"
rm -f -- "$STATE_ROOT/base-id" "$STATE_ROOT/machine-revision"

echo "Removed $DELETED_MACHINES VM(s) and the cached artifact."
echo "Preserved Codex state: $STATE_ROOT/codex-home"
echo "Next: run $SCRIPT_DIR/build-sandbox.sh [workspace]"
