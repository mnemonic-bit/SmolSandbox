#!/usr/bin/env bash
# Remove superseded managed machines and artifacts using a validated purge plan.
set -euo pipefail

show_help() {
  printf '%s\n' \
    "Usage: ${0##*/} [--help] [--force | --force-all]" \
    "" \
    "Description:" \
    "  Remove eligible stopped managed machines and unprotected artifacts. Running" \
    "  machines and protected artifacts are preserved under every policy." \
    "" \
    "Arguments:" \
    "  None." \
    "" \
    "Options:" \
    "  --force      Remove eligible stopped custom and legacy machines while" \
    "               preserving the current base's stopped default configurations." \
    "  --force-all  Remove every eligible stopped managed machine, including stopped" \
    "               default configurations." \
    "  --help       Display this help and exit." \
    "" \
    "  With no force option, the conservative purge policy is used. --force and" \
    "  --force-all are mutually exclusive."
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

usage() {
  echo "Usage: $0 [--force | --force-all]" >&2
}

POLICY=conservative
if [ "$#" -gt 1 ]; then
  usage
  exit 2
fi
if [ "$#" -eq 1 ]; then
  case "$1" in
    --force) POLICY=force ;;
    --force-all) POLICY=force-all ;;
    *)
      usage
      exit 2
      ;;
  esac
fi

PLAN_FILE=""
FRESH_PLAN_FILE=""
INVENTORY_FILE=""
FRESH_INVENTORY_FILE=""
AFTER_INVENTORY_FILE=""
cleanup_purge() {
  local path
  for path in "$PLAN_FILE" "$FRESH_PLAN_FILE" "$INVENTORY_FILE" \
    "$FRESH_INVENTORY_FILE" "$AFTER_INVENTORY_FILE"
  do
    [ -n "$path" ] && rm -f -- "$path"
  done
  sandbox_release_purge_lock
}

read_inventory() {
  local target="$1"
  if ! smolvm machine ls --json > "$target"; then
    echo "ERROR: could not read the complete SmolVM inventory; nothing was removed." >&2
    return 1
  fi
}

sandbox_prepare_environment
sandbox_acquire_purge_lock
trap cleanup_purge EXIT INT TERM
sandbox_calculate_base_identity
PLANNED_BASE_ID="$BASE_ID"

PLAN_FILE="$(sandbox_make_temp)"
FRESH_PLAN_FILE="$(sandbox_make_temp)"
INVENTORY_FILE="$(sandbox_make_temp)"
FRESH_INVENTORY_FILE="$(sandbox_make_temp)"
AFTER_INVENTORY_FILE="$(sandbox_make_temp)"

read_inventory "$INVENTORY_FILE"
if ! python3 "$SANDBOX_ROOT/sandbox-inventory.py" purge-plan \
  "$INVENTORY_FILE" "$BASE_ID" "$ARTIFACTS_DIR" "$POLICY" > "$PLAN_FILE"
then
  echo "ERROR: purge eligibility validation failed; nothing was removed." >&2
  exit 1
fi

if [ "$POLICY" = conservative ] && python3 - "$PLAN_FILE" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as plan_file:
    plan = json.load(plan_file)
raise SystemExit(0 if plan["has_alternate"] else 1)
PY
then
  echo "WARNING: parallel configurations exist for the current sandbox generation." >&2
  while IFS=$'\x1f' read -r name state workspace_path configuration; do
    [ -n "$name" ] || continue
    echo "  $name ($state, $configuration, workspace $workspace_path)" >&2
  done < <(
    python3 "$SANDBOX_ROOT/sandbox-inventory.py" render-plan \
      "$PLAN_FILE" current-workspaces
  )
  echo "Additional --allow-host options can create legitimate parallel machines." >&2
  echo "No machines or artifacts were removed." >&2
  echo "To remove stopped custom and legacy machines while preserving defaults:" >&2
  echo "  $SANDBOX_ROOT/purge-sandbox.sh --force" >&2
  echo "To remove every stopped managed machine:" >&2
  echo "  $SANDBOX_ROOT/purge-sandbox.sh --force-all" >&2
  if [ "${SANDBOX_PURGE_AFTER_BUILD:-0}" = 1 ]; then
    echo "Build completed; conservative purge did not run." >&2
    exit 0
  fi
  exit 3
fi

IFS=$'\x1f' read -r RECOVERY_ONLY CURRENT_ARTIFACT_AVAILABLE < <(
  python3 "$SANDBOX_ROOT/sandbox-inventory.py" render-plan "$PLAN_FILE" status
)

while IFS=$'\x1f' read -r name state kind generation configuration; do
  [ -n "$name" ] || continue
  echo "Preserving running machine and artifact generation $generation:"
  echo "  $name ($kind, $state${configuration:+, $configuration})"
done < <(
  python3 "$SANDBOX_ROOT/sandbox-inventory.py" render-plan \
    "$PLAN_FILE" running-machines
)
while IFS=$'\x1f' read -r name state reason; do
  [ -n "$name" ] || continue
  echo "Preserving $reason:"
  echo "  $name ($state)"
done < <(
  python3 "$SANDBOX_ROOT/sandbox-inventory.py" render-plan \
    "$PLAN_FILE" retained-machines
)
if [ "$RECOVERY_ONLY" = yes ]; then
  echo "Current artifact $ARTIFACT_FILE is missing."
  echo "Recovery is limited to the stopped current-generation builder; other machines and artifacts are preserved."
else
  echo "Preserving current artifact generation $BASE_ID."
fi

while IFS=$'\x1f' read -r name state kind generation configuration; do
  [ -n "$name" ] || continue
  echo "Selected stopped machine for removal: $name ($kind, generation $generation${configuration:+, $configuration})"
done < <(
  python3 "$SANDBOX_ROOT/sandbox-inventory.py" render-plan \
    "$PLAN_FILE" selected-machines
)
while IFS=$'\x1f' read -r path generation; do
  [ -n "$path" ] || continue
  echo "Selected superseded artifact: $path (generation $generation)"
done < <(
  python3 "$SANDBOX_ROOT/sandbox-inventory.py" render-plan \
    "$PLAN_FILE" selected-artifacts
)

RECALCULATED_BASE_ID="$(sandbox_current_base_id)"
if [ "$RECALCULATED_BASE_ID" != "$PLANNED_BASE_ID" ]; then
  echo "ERROR: the current base generation changed while planning; nothing was removed." >&2
  exit 1
fi
read_inventory "$FRESH_INVENTORY_FILE"
if ! python3 "$SANDBOX_ROOT/sandbox-inventory.py" purge-plan \
  "$FRESH_INVENTORY_FILE" "$BASE_ID" "$ARTIFACTS_DIR" "$POLICY" > "$FRESH_PLAN_FILE"
then
  echo "ERROR: fresh purge validation failed; nothing was removed." >&2
  exit 1
fi
if ! cmp -s -- "$PLAN_FILE" "$FRESH_PLAN_FILE"; then
  echo "ERROR: managed inventory or artifacts changed before deletion; nothing was removed." >&2
  exit 1
fi

REMOVED_MACHINES=0
while IFS=$'\x1f' read -r name state kind generation configuration; do
  [ -n "$name" ] || continue
  echo "Deleting stopped VM: $name"
  if ! smolvm machine delete --name "$name" --force; then
    echo "ERROR: could not delete stopped managed machine $name; no artifacts were removed." >&2
    exit 1
  fi
  sandbox_remove_port_state "$name"
  REMOVED_MACHINES=$((REMOVED_MACHINES + 1))
done < <(
  python3 "$SANDBOX_ROOT/sandbox-inventory.py" render-plan \
    "$PLAN_FILE" selected-machines
)

read_inventory "$AFTER_INVENTORY_FILE"
if ! python3 "$SANDBOX_ROOT/sandbox-inventory.py" verify-artifact-protection \
  "$AFTER_INVENTORY_FILE" "$PLAN_FILE"
then
  echo "ERROR: artifact protection changed after machine deletion; artifacts were preserved." >&2
  exit 1
fi
RECALCULATED_BASE_ID="$(sandbox_current_base_id)"
if [ "$RECALCULATED_BASE_ID" != "$PLANNED_BASE_ID" ]
then
  echo "ERROR: current base identity changed; superseded artifacts were preserved." >&2
  exit 1
fi
if [ "$CURRENT_ARTIFACT_AVAILABLE" = yes ] && \
   { [ ! -f "$ARTIFACT_FILE" ] || [ -L "$ARTIFACT_FILE" ] || [ ! -s "$ARTIFACT_FILE" ]; }
then
  echo "ERROR: current artifact identity changed; superseded artifacts were preserved." >&2
  exit 1
fi

ARTIFACTS_ABSOLUTE="$(CDPATH= cd -- "$ARTIFACTS_DIR" && pwd -P)"
REMOVED_ARTIFACTS=0
while IFS=$'\x1f' read -r path generation; do
  [ -n "$path" ] || continue
  basename="${path##*/}"
  parent="${path%/*}"
  if [ "$parent" != "$ARTIFACTS_ABSOLUTE" ] || \
     [[ ! "$basename" =~ ^codex-devtools-([0-9a-f]{12})(\.smolmachine)?$ ]] || \
     [ "${BASH_REMATCH[1]:-}" != "$generation" ] || \
     [ "$generation" = "$BASE_ID" ] || [ ! -f "$path" ] || [ -L "$path" ]
  then
    echo "ERROR: artifact candidate changed or escaped the managed scope: $path" >&2
    exit 1
  fi
  echo "Deleting artifact: $path"
  if ! rm -f -- "$path"; then
    echo "ERROR: could not delete artifact $path; later artifacts were preserved." >&2
    exit 1
  fi
  REMOVED_ARTIFACTS=$((REMOVED_ARTIFACTS + 1))
done < <(
  python3 "$SANDBOX_ROOT/sandbox-inventory.py" render-plan \
    "$PLAN_FILE" selected-artifacts
)

IFS=$'\x1f' read -r _planned_machines REMOVED_GENERATIONS _planned_artifacts RETAINED_RUNNING < <(
  python3 "$SANDBOX_ROOT/sandbox-inventory.py" render-plan "$PLAN_FILE" summary
)
echo "Removed $REMOVED_MACHINES machine(s), $REMOVED_GENERATIONS generation(s), and $REMOVED_ARTIFACTS artifact file(s)."
if [ "$RECOVERY_ONLY" = yes ]; then
  echo "The current artifact remains absent; rebuild generation $BASE_ID before purging superseded state."
else
  echo "Retained $RETAINED_RUNNING running managed machine(s) and the current generation $BASE_ID."
fi
