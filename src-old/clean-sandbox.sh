#!/usr/bin/env bash
# Delete one workspace's VMs or all state for the authoritative current generation.
set -euo pipefail

show_help() {
  printf '%s\n' \
    "Usage: ${0##*/} [--help] [--workspace [WORKSPACE]]" \
    "" \
    "Description:" \
    "  Remove machines and cached artifacts belonging to the authoritative current" \
    "  generation, or remove only the machines belonging to one workspace." \
    "  The shared Codex state is always preserved." \
    "" \
    "Arguments:" \
    "  WORKSPACE    Workspace whose machines should be removed (default: current" \
    "               directory; valid only with --workspace)." \
    "" \
    "Options:" \
    "  --workspace  Remove every managed machine for WORKSPACE across generations" \
    "               and configurations, preserving shared machines and artifacts." \
    "  --help       Display this help and exit."
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

MODE=generation
WORKSPACE_INPUT=""
usage() {
  echo "Usage: $0 [--workspace [WORKSPACE]]" >&2
}

if [ "$#" -gt 0 ]; then
  if [ "$1" != --workspace ] || [ "$#" -gt 2 ] || \
     { [ "$#" -eq 2 ] && [[ "$2" == -* ]]; }
  then
    usage
    exit 2
  fi
  MODE=workspace
  WORKSPACE_INPUT="${2:-$PWD}"
fi

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
if [ "$MODE" = generation ]; then
  sandbox_calculate_base_identity
else
  if [ ! -d "$WORKSPACE_INPUT" ]; then
    echo "ERROR: workspace is not a directory: $WORKSPACE_INPUT" >&2
    exit 1
  fi
  WORKSPACE_DIR="$(CDPATH= cd -- "$WORKSPACE_INPUT" && pwd -P)"
  WORKSPACE_PATH_SHA256="$(printf '%s' "$WORKSPACE_DIR" | sandbox_hash_stream)"
fi
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

if [ "$MODE" = generation ]; then
  echo "Cleaning Codex base generation $BASE_ID..."
else
  echo "Cleaning Codex workspace $WORKSPACE_DIR..."
fi
DELETED_MACHINES=0
SELECTED_MACHINES=()
while IFS=$'\x1f' read -r machine state kind label_base label_revision \
  label_workspace label_workspace_path configuration any_managed
do
  [ -n "$machine" ] || continue
  selected=""
  if [ "$MODE" = generation ]; then
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
  elif [ "$label_workspace_path" = "$WORKSPACE_PATH_SHA256" ]; then
    expected_name="codex-workspace-$label_base-r$label_revision-${label_workspace:0:12}"
    if [ "$kind" != workspace ] || [[ ! "$label_base" =~ ^[0-9a-f]{12}$ ]] || \
       [[ ! "$label_revision" =~ ^[0-9]+$ ]] || \
       [[ ! "$label_workspace" =~ ^[0-9a-f]{64}$ ]] || \
       { [ "$configuration" != default ] && [ "$configuration" != custom ]; } || \
       [ "$machine" != "$expected_name" ]
    then
      echo "ERROR: refusing to clean $machine because its managed labels are contradictory." >&2
      exit 1
    fi
    selected=yes
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

if [ "$MODE" = generation ]; then
  rm -f -- "$ARTIFACT_BASE" "$ARTIFACT_FILE"
  rm -f -- "$STATE_ROOT/base-id" "$STATE_ROOT/machine-revision"
  echo "Removed $DELETED_MACHINES VM(s) and the cached artifact."
else
  echo "Removed $DELETED_MACHINES workspace VM(s) for $WORKSPACE_DIR."
fi
echo "Preserved Codex state: $STATE_ROOT/codex-home"
if [ "$MODE" = generation ]; then
  echo "Next: run $SCRIPT_DIR/build-sandbox.sh [workspace]"
else
  echo "Next: run $SCRIPT_DIR/start-codex.sh $WORKSPACE_DIR"
fi
