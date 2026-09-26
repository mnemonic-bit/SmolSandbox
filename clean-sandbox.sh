#!/usr/bin/env bash
# Delete VMs and artifacts belonging to the authoritative current generation.
set -euo pipefail

show_help() {
  printf '%s\n' \
    "Usage: ${0##*/} [--help]" \
    "" \
    "Description:" \
    "  Remove machines and cached artifacts belonging to the authoritative current" \
    "  generation while preserving shared Codex state." \
    "" \
    "Arguments:" \
    "  None." \
    "" \
    "Options:" \
    "  --help  Display this help and exit."
}

for argument in "$@"; do
  if [ "$argument" = --help ]; then
    show_help
    exit 0
  fi
done

if [ "$#" -ne 0 ]; then
  echo "Usage: $0" >&2
  exit 2
fi

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=sandbox-common.sh
source "$SCRIPT_DIR/sandbox-common.sh"

INVENTORY_FILE=""
NORMALIZED_FILE=""
cleanup_inventory_files() {
  if [ -n "$INVENTORY_FILE" ] || [ -n "$NORMALIZED_FILE" ]; then
    rm -f -- "$INVENTORY_FILE" "$NORMALIZED_FILE"
  fi
  sandbox_release_purge_lock
}
sandbox_prepare_environment
sandbox_acquire_purge_lock
trap cleanup_inventory_files EXIT
sandbox_calculate_base_identity
INVENTORY_FILE="$(sandbox_make_temp)"
NORMALIZED_FILE="$(sandbox_make_temp)"

if ! smolvm machine ls --json > "$INVENTORY_FILE"; then
  echo "ERROR: could not read the SmolVM machine inventory; refusing to clean." >&2
  exit 1
fi
if ! python3 "$SANDBOX_ROOT/sandbox-inventory.py" normalize "$INVENTORY_FILE" > "$NORMALIZED_FILE"; then
  echo "ERROR: could not parse the SmolVM machine inventory; refusing to clean." >&2
  exit 1
fi

echo "Cleaning Codex base generation $BASE_ID..."
DELETED_MACHINES=0
SELECTED_MACHINES=()
while IFS=$'\x1f' read -r machine state kind label_base label_revision label_workspace any_managed; do
  [ -n "$machine" ] || continue
  selected=""
  if [ "$machine" = "codex-builder-$BASE_ID" ] || \
     [[ "$machine" =~ ^codex-login-${BASE_ID}-r[0-9]+$ ]]; then
    selected=yes
  fi
  if [ "$kind" = workspace ] && [ "$label_base" = "$BASE_ID" ] && \
     [[ "$label_revision" =~ ^[0-9]+$ ]] && [[ "$label_workspace" =~ ^[0-9a-f]{64}$ ]]; then
    selected=yes
  elif [[ "$machine" =~ ^codex-workspace-${BASE_ID}-r[0-9]+-[0-9a-f]{12}$ ]]; then
    if [ "$any_managed" = yes ]; then
      echo "ERROR: refusing to clean $machine because its managed labels are contradictory." >&2
      exit 1
    fi
    selected=yes
  elif [[ "$machine" == codex-workspace-${BASE_ID}-r* ]] && [ "$any_managed" = yes ]; then
    echo "ERROR: refusing to clean $machine because its managed labels are contradictory." >&2
    exit 1
  fi
  [ -n "$selected" ] || continue
  SELECTED_MACHINES[${#SELECTED_MACHINES[@]}]="$machine"
done < "$NORMALIZED_FILE"

for machine in "${SELECTED_MACHINES[@]}"; do
  echo "  deleting VM: $machine"
  smolvm machine stop --name "$machine" >/dev/null 2>&1 || true
  smolvm machine delete --name "$machine" --force
  sandbox_remove_port_state "$machine"
  DELETED_MACHINES=$((DELETED_MACHINES + 1))
done

rm -f -- "$ARTIFACT_BASE" "$ARTIFACT_FILE"
rm -f -- "$STATE_ROOT/base-id" "$STATE_ROOT/machine-revision"

echo "Removed $DELETED_MACHINES VM(s) and the cached artifact."
echo "Preserved Codex state: $STATE_ROOT/codex-home"
echo "Next: run $SCRIPT_DIR/build-sandbox.sh [workspace]"
