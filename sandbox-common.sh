#!/usr/bin/env bash
# Shared authoritative identity, inventory, creation, and reconciliation logic.

SANDBOX_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
SANDBOX_MACHINE_REVISION=3
SANDBOX_WORKSPACE_MEMORY=12288
SANDBOX_WORKSPACE_CPUS=4
SANDBOX_REQUESTED_ALLOW_HOSTS=()
SANDBOX_DEFAULT_ALLOW_HOSTS=(
  api.openai.com
  auth.openai.com
  chatgpt.com
  github.com
  api.github.com
  raw.githubusercontent.com
  codeload.github.com
  objects.githubusercontent.com
  release-assets.githubusercontent.com
  registry.npmjs.org
  api.nuget.org
  globalcdn.nuget.org
  pypi.org
  files.pythonhosted.org
  deb.debian.org
  dot.net
  aka.ms
  builds.dotnet.microsoft.com
  ci.dot.net
  cdn.playwright.dev
  storage.googleapis.com
  playwright.download.prss.microsoft.com
  deb.nodesource.com
  registry-1.docker.io
  auth.docker.io
  production.cloudflare.docker.com
  production.cloudfront.docker.com
  ghcr.io
  pkg-containers.githubusercontent.com
  mcr.microsoft.com
)

sandbox_hash_stream() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  else
    shasum -a 256 | awk '{print $1}'
  fi
}

sandbox_require_tools() {
  if ! command -v smolvm >/dev/null 2>&1; then
    echo "ERROR: smolvm is not installed or is not on PATH." >&2
    echo "Install it with: curl -sSL https://smolmachines.com/install.sh | bash" >&2
    return 1
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required to validate the SmolVM JSON inventory." >&2
    return 1
  fi
}

sandbox_hash_set() {
  python3 - "$@" <<'PY'
import hashlib
import json
import sys

values = sorted(set(sys.argv[1:]))
payload = json.dumps(values, ensure_ascii=False, separators=(",", ":")).encode()
print(hashlib.sha256(payload).hexdigest())
PY
}

sandbox_prepare_workspace_options() {
  local host
  local -a requested_hosts=("$@") machine_options=()

  for host in "${requested_hosts[@]}"; do
    if [ -z "$host" ] || [[ "$host" == -* ]] || \
       [[ "$host" == *$'\n'* ]] || [[ "$host" == *$'\r'* ]]; then
      echo "ERROR: --allow-host requires a non-empty host without line breaks or a leading dash." >&2
      return 2
    fi
  done

  SANDBOX_ADDITIONAL_ALLOW_HOSTS=()
  while IFS= read -r host; do
    [ -n "$host" ] || continue
    SANDBOX_ADDITIONAL_ALLOW_HOSTS+=("$host")
  done < <(printf '%s\n' "${requested_hosts[@]}" | LC_ALL=C sort -u)

  SANDBOX_ALLOW_HOSTS=()
  while IFS= read -r host; do
    [ -n "$host" ] || continue
    SANDBOX_ALLOW_HOSTS+=("$host")
  done < <(
    printf '%s\n' "${SANDBOX_DEFAULT_ALLOW_HOSTS[@]}" \
      "${SANDBOX_ADDITIONAL_ALLOW_HOSTS[@]}" | LC_ALL=C sort -u
  )

  machine_options=(
    "--from=$ARTIFACT_FILE"
    "--mem=$SANDBOX_WORKSPACE_MEMORY"
    "--cpus=$SANDBOX_WORKSPACE_CPUS"
    "--volume=$CODEX_HOME_DIR:/root/.codex"
    "--volume=$WORKSPACE_DIR:/workspace/myrepo"
  )
  for host in "${SANDBOX_ALLOW_HOSTS[@]}"; do
    machine_options+=("--allow-host=$host")
  done
  WORKSPACE_SHA256="$(sandbox_hash_set "${machine_options[@]}")" || return 1
  WORKSPACE_ID="${WORKSPACE_SHA256:0:12}"
}

sandbox_prepare_identity() {
  local workspace_input="$1"
  shift

  BASE_SMOLFILE="${BASE_SMOLFILE:-$SANDBOX_ROOT/devtools-base.smolfile}"
  ARTIFACTS_DIR="${ARTIFACTS_DIR:-$SANDBOX_ROOT/artifacts}"
  STATE_ROOT="${STATE_ROOT:-$SANDBOX_ROOT/.smolvm}"
  CODEX_HOME_DIR="${CODEX_HOME_DIR:-$STATE_ROOT/codex-home}"

  sandbox_require_tools || return 1
  if [ ! -f "$BASE_SMOLFILE" ]; then
    echo "ERROR: base Smolfile not found: $BASE_SMOLFILE" >&2
    return 1
  fi
  if [ ! -d "$workspace_input" ]; then
    echo "ERROR: workspace is not a directory: $workspace_input" >&2
    return 1
  fi

  WORKSPACE_DIR="$(CDPATH= cd -- "$workspace_input" && pwd -P)"
  SMOLVM_VERSION="$(smolvm --version)" || return 1
  BASE_ID="$({ printf '%s\n' "$SMOLVM_VERSION"; command cat "$BASE_SMOLFILE"; } | sandbox_hash_stream | cut -c1-12)"
  MACHINE_REVISION="$SANDBOX_MACHINE_REVISION"
  BUILDER_MACHINE="${BUILDER_MACHINE:-codex-builder-$BASE_ID}"
  LOGIN_MACHINE="${LOGIN_MACHINE:-codex-login-$BASE_ID-r$MACHINE_REVISION}"
  ARTIFACT_BASE="$ARTIFACTS_DIR/codex-devtools-$BASE_ID"
  ARTIFACT_FILE="$ARTIFACT_BASE.smolmachine"
  sandbox_prepare_workspace_options "$@" || return $?
  DEFAULT_CODEX_MACHINE="codex-workspace-$BASE_ID-r$MACHINE_REVISION-$WORKSPACE_ID"
  CODEX_MACHINE_OVERRIDE="${CODEX_MACHINE:-}"
  DESIRED_CODEX_MACHINE="${CODEX_MACHINE_OVERRIDE:-$DEFAULT_CODEX_MACHINE}"
}

sandbox_make_temp() {
  mktemp "${TMPDIR:-/tmp}/codex-sandbox.XXXXXX"
}

sandbox_read_inventory() {
  SANDBOX_INVENTORY_FILE="$(sandbox_make_temp)" || return 1
  SANDBOX_CLASSIFICATION_FILE="$(sandbox_make_temp)" || {
    rm -f -- "$SANDBOX_INVENTORY_FILE"
    return 1
  }

  if ! smolvm machine ls --json > "$SANDBOX_INVENTORY_FILE"; then
    echo "ERROR: could not read the SmolVM machine inventory; refusing to build or alter machines." >&2
    sandbox_discard_inventory
    return 1
  fi

  local override_flag=()
  if [ -n "$CODEX_MACHINE_OVERRIDE" ]; then
    override_flag=(--override)
  fi
  if ! python3 "$SANDBOX_ROOT/sandbox-inventory.py" classify \
    "$SANDBOX_INVENTORY_FILE" "$BASE_ID" "$MACHINE_REVISION" \
    "$WORKSPACE_SHA256" "$WORKSPACE_ID" "$DESIRED_CODEX_MACHINE" \
    "${override_flag[@]}" > "$SANDBOX_CLASSIFICATION_FILE"
  then
    echo "ERROR: could not parse the SmolVM machine inventory; refusing to build or alter machines." >&2
    sandbox_discard_inventory
    return 1
  fi

  SANDBOX_ALL_NAMES=()
  SANDBOX_ALL_STATES=()
  SANDBOX_STALE_NAMES=()
  SANDBOX_STALE_STATES=()
  SANDBOX_STALE_BASES=()
  SANDBOX_STALE_REVISIONS=()
  SANDBOX_STALE_SOURCES=()
  SANDBOX_AMBIGUOUS_NAMES=()
  SANDBOX_AMBIGUOUS_REASONS=()
  SANDBOX_DESIRED_FOUND=""
  SANDBOX_DESIRED_STATE=""

  local classification name state base revision source deletable reason index
  while IFS=$'\x1f' read -r classification name state base revision source deletable reason; do
    [ -n "$name" ] || continue
    index="${#SANDBOX_ALL_NAMES[@]}"
    SANDBOX_ALL_NAMES[$index]="$name"
    SANDBOX_ALL_STATES[$index]="$state"
    case "$classification" in
      desired)
        if [ -n "$SANDBOX_DESIRED_FOUND" ]; then
          SANDBOX_AMBIGUOUS_NAMES[${#SANDBOX_AMBIGUOUS_NAMES[@]}]="$name"
          SANDBOX_AMBIGUOUS_REASONS[${#SANDBOX_AMBIGUOUS_REASONS[@]}]="multiple machines match the desired identity"
        else
          SANDBOX_DESIRED_FOUND="$name"
          SANDBOX_DESIRED_STATE="$state"
        fi
        ;;
      stale)
        index="${#SANDBOX_STALE_NAMES[@]}"
        SANDBOX_STALE_NAMES[$index]="$name"
        SANDBOX_STALE_STATES[$index]="$state"
        SANDBOX_STALE_BASES[$index]="$base"
        SANDBOX_STALE_REVISIONS[$index]="$revision"
        SANDBOX_STALE_SOURCES[$index]="$source"
        ;;
      ambiguous)
        index="${#SANDBOX_AMBIGUOUS_NAMES[@]}"
        SANDBOX_AMBIGUOUS_NAMES[$index]="$name"
        SANDBOX_AMBIGUOUS_REASONS[$index]="$reason"
        ;;
    esac
  done < "$SANDBOX_CLASSIFICATION_FILE"
}

sandbox_discard_inventory() {
  if [ -n "${SANDBOX_INVENTORY_FILE:-}" ]; then
    rm -f -- "$SANDBOX_INVENTORY_FILE"
  fi
  if [ -n "${SANDBOX_CLASSIFICATION_FILE:-}" ]; then
    rm -f -- "$SANDBOX_CLASSIFICATION_FILE"
  fi
  SANDBOX_INVENTORY_FILE=""
  SANDBOX_CLASSIFICATION_FILE=""
}

sandbox_inventory_has_name() {
  local wanted="$1" index
  for ((index = 0; index < ${#SANDBOX_ALL_NAMES[@]}; index++)); do
    if [ "${SANDBOX_ALL_NAMES[$index]}" = "$wanted" ]; then
      return 0
    fi
  done
  return 1
}

sandbox_report_candidates() {
  local index
  if [ "${#SANDBOX_STALE_NAMES[@]}" -gt 0 ]; then
    echo "Found stale workspace machine(s):" >&2
    for ((index = 0; index < ${#SANDBOX_STALE_NAMES[@]}; index++)); do
      echo "  ${SANDBOX_STALE_NAMES[$index]} (${SANDBOX_STALE_STATES[$index]})" >&2
      echo "    base: ${SANDBOX_STALE_BASES[$index]:-unknown}; desired: $BASE_ID" >&2
      echo "    revision: ${SANDBOX_STALE_REVISIONS[$index]:-unknown}; desired: $MACHINE_REVISION" >&2
      echo "    identified by: ${SANDBOX_STALE_SOURCES[$index]}" >&2
    done
  fi
  if [ "${#SANDBOX_AMBIGUOUS_NAMES[@]}" -gt 0 ]; then
    echo "Found machine(s) with ambiguous workspace metadata:" >&2
    for ((index = 0; index < ${#SANDBOX_AMBIGUOUS_NAMES[@]}; index++)); do
      echo "  ${SANDBOX_AMBIGUOUS_NAMES[$index]}: ${SANDBOX_AMBIGUOUS_REASONS[$index]}" >&2
    done
  fi
}

sandbox_report_repair_commands() {
  local host
  printf 'Inspect with:\n  %q --check' "$SANDBOX_ROOT/repair-sandbox.sh" >&2
  for host in "${SANDBOX_ADDITIONAL_ALLOW_HOSTS[@]}"; do
    printf ' --allow-host %q' "$host" >&2
  done
  printf ' %q\n' "$WORKSPACE_DIR" >&2
  printf 'Replace with:\n  %q --replace-stale' "$SANDBOX_ROOT/repair-sandbox.sh" >&2
  for host in "${SANDBOX_ADDITIONAL_ALLOW_HOSTS[@]}"; do
    printf ' --allow-host %q' "$host" >&2
  done
  printf ' %q\n' "$WORKSPACE_DIR" >&2
}

sandbox_ensure_base() {
  mkdir -p "$ARTIFACTS_DIR" "$CODEX_HOME_DIR"
  if [ -s "$ARTIFACT_FILE" ]; then
    echo "Reusing cached Codex base generation $BASE_ID."
    return 0
  fi

  echo "Building Codex base generation $BASE_ID ($SMOLVM_VERSION)..."
  if ! sandbox_inventory_has_name "$BUILDER_MACHINE"; then
    smolvm machine create --name "$BUILDER_MACHINE" --smolfile "$BASE_SMOLFILE" || return 1
    SANDBOX_ALL_NAMES[${#SANDBOX_ALL_NAMES[@]}]="$BUILDER_MACHINE"
    SANDBOX_ALL_STATES[${#SANDBOX_ALL_STATES[@]}]="created"
  fi

  smolvm machine stop --name "$BUILDER_MACHINE" >/dev/null 2>&1 || true
  smolvm machine start --name "$BUILDER_MACHINE" || return 1
  smolvm machine stop --name "$BUILDER_MACHINE" || return 1

  local temporary_base="$ARTIFACTS_DIR/.codex-devtools-$BASE_ID-$$"
  if ! smolvm pack create --from-vm "$BUILDER_MACHINE" -o "$temporary_base"; then
    rm -f -- "$temporary_base" "$temporary_base.smolmachine"
    return 1
  fi
  mv -- "$temporary_base.smolmachine" "$ARTIFACT_FILE" || return 1
  if [ -f "$temporary_base" ]; then
    mv -- "$temporary_base" "$ARTIFACT_BASE" || return 1
  fi
}

sandbox_remove_builder_machines() {
  local index builder state builder_base artifact failed=0
  for ((index = 0; index < ${#SANDBOX_ALL_NAMES[@]}; index++)); do
    builder="${SANDBOX_ALL_NAMES[$index]}"
    state="${SANDBOX_ALL_STATES[$index]}"
    if [ "$builder" = "$BUILDER_MACHINE" ]; then
      artifact="$ARTIFACT_FILE"
    elif [[ "$builder" =~ ^codex-builder-([0-9a-f]{12})$ ]]; then
      builder_base="${BASH_REMATCH[1]}"
      artifact="$ARTIFACTS_DIR/codex-devtools-$builder_base.smolmachine"
    else
      continue
    fi
    # The artifact is the durable cache. Only remove builders backed by one
    # in this repository, and never interfere with an active build.
    [ -s "$artifact" ] || continue
    if [ "$state" = "running" ] || [ "$state" = "Running" ]; then
      echo "WARNING: leaving active builder machine $builder in place." >&2
      continue
    fi
    echo "Removing temporary builder machine $builder..."
    smolvm machine stop --name "$builder" >/dev/null 2>&1 || true
    if ! smolvm machine delete --name "$builder" --force; then
      failed=1
    fi
  done
  return "$failed"
}

sandbox_create_login_machine() {
  if sandbox_inventory_has_name "$LOGIN_MACHINE"; then
    return 0
  fi
  echo "Creating login-only machine $LOGIN_MACHINE..."
  if ! smolvm machine create \
    --name "$LOGIN_MACHINE" \
    --from "$ARTIFACT_FILE" \
    --volume "$CODEX_HOME_DIR:/root/.codex" \
    --allow-host auth.openai.com \
    --allow-host chatgpt.com \
    --allow-host api.openai.com
  then
    return 1
  fi
  SANDBOX_ALL_NAMES[${#SANDBOX_ALL_NAMES[@]}]="$LOGIN_MACHINE"
  SANDBOX_ALL_STATES[${#SANDBOX_ALL_STATES[@]}]="created"
}

sandbox_create_workspace_machine() {
  local host
  local -a allow_host_args=()
  for host in "${SANDBOX_ALLOW_HOSTS[@]}"; do
    allow_host_args+=(--allow-host "$host")
  done
  echo "Creating workspace machine $DESIRED_CODEX_MACHINE..."
  smolvm machine create \
    --name "$DESIRED_CODEX_MACHINE" \
    --label "codex-sandbox.kind=workspace" \
    --label "codex-sandbox.base-id=$BASE_ID" \
    --label "codex-sandbox.revision=$MACHINE_REVISION" \
    --label "codex-sandbox.workspace-sha256=$WORKSPACE_SHA256" \
    --from "$ARTIFACT_FILE" \
    --mem "$SANDBOX_WORKSPACE_MEMORY" \
    --cpus "$SANDBOX_WORKSPACE_CPUS" \
    --volume "$CODEX_HOME_DIR:/root/.codex" \
    --volume "$WORKSPACE_DIR:/workspace/myrepo" \
    "${allow_host_args[@]}"
}

sandbox_cache_value() {
  if [ -s "$1" ]; then
    sed -n '1p' "$1" 2>/dev/null || true
  fi
}

sandbox_repair_caches() {
  local old_base old_revision base_tmp revision_tmp
  old_base="$(sandbox_cache_value "$STATE_ROOT/base-id")"
  old_revision="$(sandbox_cache_value "$STATE_ROOT/machine-revision")"
  mkdir -p "$STATE_ROOT"
  base_tmp="$(mktemp "$STATE_ROOT/.base-id.XXXXXX")" || return 1
  revision_tmp="$(mktemp "$STATE_ROOT/.machine-revision.XXXXXX")" || {
    rm -f -- "$base_tmp"
    return 1
  }
  if ! printf '%s\n' "$BASE_ID" > "$base_tmp" || \
     ! printf '%s\n' "$MACHINE_REVISION" > "$revision_tmp"
  then
    rm -f -- "$base_tmp" "$revision_tmp"
    return 1
  fi
  if ! mv -- "$base_tmp" "$STATE_ROOT/base-id" || \
     ! mv -- "$revision_tmp" "$STATE_ROOT/machine-revision"
  then
    rm -f -- "$base_tmp" "$revision_tmp"
    return 1
  fi
  if [ "$old_base" != "$BASE_ID" ] || [ "$old_revision" != "$MACHINE_REVISION" ]; then
    echo "Repaired .smolvm/base-id and .smolvm/machine-revision."
  fi
}

sandbox_acquire_lock() {
  local lock_parent="$STATE_ROOT/locks" attempts=0 owner
  mkdir -p "$lock_parent"
  SANDBOX_LOCK_DIR="$lock_parent/workspace-$WORKSPACE_SHA256.lock"
  while ! mkdir "$SANDBOX_LOCK_DIR" 2>/dev/null; do
    owner="$(sed -n '1p' "$SANDBOX_LOCK_DIR/pid" 2>/dev/null || true)"
    if [ -n "$owner" ] && ! kill -0 "$owner" 2>/dev/null; then
      rm -f -- "$SANDBOX_LOCK_DIR/pid"
      rmdir "$SANDBOX_LOCK_DIR" 2>/dev/null || true
      continue
    fi
    attempts=$((attempts + 1))
    if [ "$attempts" -ge 600 ]; then
      echo "ERROR: timed out waiting for workspace reconciliation lock." >&2
      return 1
    fi
    sleep 0.1
  done
  printf '%s\n' "$$" > "$SANDBOX_LOCK_DIR/pid"
}

sandbox_release_lock() {
  if [ -n "${SANDBOX_LOCK_DIR:-}" ]; then
    rm -f -- "$SANDBOX_LOCK_DIR/pid"
    rmdir "$SANDBOX_LOCK_DIR" 2>/dev/null || true
    SANDBOX_LOCK_DIR=""
  fi
}

sandbox_verify_replacement_readiness() {
  if [ ! -s "$ARTIFACT_FILE" ]; then
    echo "ERROR: desired base artifact is missing or empty; stale workspace machines were left unchanged." >&2
    return 1
  fi

  local override_flag=() verification name state
  if [ -n "$CODEX_MACHINE_OVERRIDE" ]; then
    override_flag=(--override)
  fi
  if ! verification="$(python3 "$SANDBOX_ROOT/sandbox-inventory.py" verify-readiness \
    "$SANDBOX_INVENTORY_FILE" "$BASE_ID" "$MACHINE_REVISION" \
    "$WORKSPACE_SHA256" "$WORKSPACE_ID" "$DESIRED_CODEX_MACHINE" \
    "${override_flag[@]}" 2>&1)"
  then
    echo "ERROR: desired workspace machine did not pass replacement readiness: $verification" >&2
    echo "Stale workspace machines were left unchanged." >&2
    sandbox_report_repair_commands
    return 1
  fi

  IFS=$'\x1f' read -r name state <<< "$verification"
  RECONCILED_MACHINE="$name"
  echo "Verified desired workspace machine $name ($state)."
}

sandbox_report_create_failure() {
  echo "ERROR: could not create desired workspace machine $DESIRED_CODEX_MACHINE." >&2
  sandbox_discard_inventory
  if sandbox_read_inventory; then
    if [ -n "$SANDBOX_DESIRED_FOUND" ]; then
      echo "The desired machine appears in the refreshed inventory and was preserved for inspection." >&2
    fi
    sandbox_discard_inventory
  fi
  echo "Stale workspace machines were left unchanged." >&2
}

sandbox_reconcile_attempt() {
  local mode="$1" force="$2" index state refreshed_index found
  local -a replacement_names=() replacement_states=()
  sandbox_read_inventory || return 1

  if [ "$mode" = "check" ]; then
    echo "Desired workspace machine: $DESIRED_CODEX_MACHINE"
    if [ -n "$SANDBOX_DESIRED_FOUND" ]; then
      echo "Selected: $SANDBOX_DESIRED_FOUND ($SANDBOX_DESIRED_STATE)"
    else
      echo "Selected: none"
    fi
    sandbox_report_candidates
    sandbox_discard_inventory
    return 0
  fi

  if [ "$mode" = "start" ]; then
    if [ -z "$SANDBOX_DESIRED_FOUND" ] && \
       { [ "${#SANDBOX_STALE_NAMES[@]}" -gt 0 ] || [ "${#SANDBOX_AMBIGUOUS_NAMES[@]}" -gt 0 ]; }
    then
      echo "No machine matches the current sandbox definition for this workspace." >&2
      sandbox_report_candidates
      echo "No machines were created or deleted." >&2
      sandbox_report_repair_commands
      sandbox_discard_inventory
      return 1
    fi

    if [ -n "$SANDBOX_DESIRED_FOUND" ]; then
      if [ "${#SANDBOX_STALE_NAMES[@]}" -gt 0 ] || [ "${#SANDBOX_AMBIGUOUS_NAMES[@]}" -gt 0 ]; then
        echo "WARNING: selected the desired machine, but other workspace candidates remain." >&2
        sandbox_report_candidates
      fi
      RECONCILED_MACHINE="$SANDBOX_DESIRED_FOUND"
      echo "Recovered workspace machine $RECONCILED_MACHINE."
    else
      sandbox_ensure_base || {
        sandbox_discard_inventory
        return 1
      }
      sandbox_create_workspace_machine || {
        sandbox_discard_inventory
        return 1
      }
      RECONCILED_MACHINE="$DESIRED_CODEX_MACHINE"
    fi

    # Existing machines can start without the artifact, but the compatibility
    # cache is repaired only once the desired artifact has also been confirmed.
    if [ ! -s "$ARTIFACT_FILE" ]; then
      sandbox_ensure_base || {
        sandbox_discard_inventory
        return 1
      }
    fi
    if [ "${SANDBOX_ENSURE_LOGIN:-0}" = "1" ]; then
      sandbox_create_login_machine || {
        sandbox_discard_inventory
        return 1
      }
    fi
    if ! sandbox_remove_builder_machines; then
      echo "WARNING: one or more temporary builder machines could not be removed." >&2
    fi
    sandbox_repair_caches || {
      sandbox_discard_inventory
      return 1
    }
    sandbox_discard_inventory
    return 0
  fi

  if [ "${#SANDBOX_AMBIGUOUS_NAMES[@]}" -gt 0 ]; then
    echo "ERROR: refusing replacement because workspace ownership is ambiguous." >&2
    sandbox_report_candidates
    sandbox_discard_inventory
    return 1
  fi

  echo "Stale machines selected for removal:"
  if [ "${#SANDBOX_STALE_NAMES[@]}" -eq 0 ]; then
    echo "  none"
  else
    for ((index = 0; index < ${#SANDBOX_STALE_NAMES[@]}; index++)); do
      echo "  ${SANDBOX_STALE_NAMES[$index]} (${SANDBOX_STALE_STATES[$index]}, ${SANDBOX_STALE_SOURCES[$index]})"
      state="${SANDBOX_STALE_STATES[$index]}"
      if [ "${state,,}" = "running" ] && [ "$force" != "force" ]; then
        echo "ERROR: stale machine ${SANDBOX_STALE_NAMES[$index]} is running; rerun with --force to stop it." >&2
        sandbox_discard_inventory
        return 1
      fi
    done
  fi

  replacement_names=("${SANDBOX_STALE_NAMES[@]}")

  sandbox_ensure_base || {
    echo "ERROR: could not prepare desired base generation $BASE_ID." >&2
    echo "Stale workspace machines were left unchanged." >&2
    sandbox_discard_inventory
    return 1
  }
  if [ "${SANDBOX_ENSURE_LOGIN:-0}" = "1" ]; then
    sandbox_create_login_machine || {
      sandbox_discard_inventory
      return 1
    }
  fi
  if [ -n "$SANDBOX_DESIRED_FOUND" ]; then
    RECONCILED_MACHINE="$SANDBOX_DESIRED_FOUND"
  else
    sandbox_create_workspace_machine || {
      sandbox_report_create_failure
      return 1
    }
    RECONCILED_MACHINE="$DESIRED_CODEX_MACHINE"
  fi

  # The readiness decision must use an inventory obtained after preparation,
  # even when the desired machine already existed before replacement began.
  sandbox_discard_inventory
  if ! sandbox_read_inventory; then
    echo "Stale workspace machines were left unchanged." >&2
    return 1
  fi
  if [ "${#SANDBOX_AMBIGUOUS_NAMES[@]}" -gt 0 ]; then
    echo "ERROR: refusing cleanup because refreshed workspace ownership is ambiguous." >&2
    sandbox_report_candidates
    echo "Stale workspace machines were left unchanged." >&2
    sandbox_discard_inventory
    return 1
  fi
  if ! sandbox_verify_replacement_readiness; then
    sandbox_discard_inventory
    return 1
  fi

  # Cleanup is limited to the stale machines selected during preflight. Each
  # one must still be positively classified for this workspace in the fresh
  # inventory, and its refreshed state controls whether --force is required.
  for ((index = 0; index < ${#replacement_names[@]}; index++)); do
    found=0
    for ((refreshed_index = 0; refreshed_index < ${#SANDBOX_STALE_NAMES[@]}; refreshed_index++)); do
      if [ "${replacement_names[$index]}" = "${SANDBOX_STALE_NAMES[$refreshed_index]}" ]; then
        replacement_states[$index]="${SANDBOX_STALE_STATES[$refreshed_index]}"
        found=1
        break
      fi
    done
    if [ "$found" -ne 1 ]; then
      echo "ERROR: stale workspace machine ${replacement_names[$index]} is no longer eligible for cleanup." >&2
      echo "Stale workspace machines were left unchanged." >&2
      sandbox_discard_inventory
      return 1
    fi
    state="${replacement_states[$index]}"
    if [ "${state,,}" = "running" ] && [ "$force" != "force" ]; then
      echo "ERROR: stale machine ${replacement_names[$index]} is now running; rerun with --force to stop it." >&2
      echo "Stale workspace machines were left unchanged." >&2
      sandbox_discard_inventory
      return 1
    fi
  done

  for ((index = 0; index < ${#replacement_names[@]}; index++)); do
    state="${replacement_states[$index]}"
    if [ "${state,,}" = "running" ]; then
      if ! smolvm machine stop --name "${replacement_names[$index]}"; then
        echo "ERROR: could not stop stale workspace machine ${replacement_names[$index]}." >&2
        echo "The desired workspace machine remains available; compatibility caches were not changed." >&2
        sandbox_discard_inventory
        return 1
      fi
    fi
    if ! smolvm machine delete --name "${replacement_names[$index]}" --force; then
      echo "ERROR: could not delete stale workspace machine ${replacement_names[$index]}." >&2
      echo "The desired workspace machine remains available; compatibility caches were not changed." >&2
      sandbox_discard_inventory
      return 1
    fi
  done

  if ! sandbox_remove_builder_machines; then
    echo "WARNING: one or more temporary builder machines could not be removed." >&2
  fi
  sandbox_repair_caches || {
    sandbox_discard_inventory
    return 1
  }
  sandbox_discard_inventory
}

# reconcile_workspace WORKSPACE {start|check|replace-stale} [force]
# Sets RECONCILED_MACHINE for successful mutating operations.
reconcile_workspace() {
  local workspace="$1" mode="$2" force="${3:-}" result=0
  case "$mode" in
    start | check | replace-stale) ;;
    *)
      echo "ERROR: unsupported reconciliation mode: $mode" >&2
      return 2
      ;;
  esac
  sandbox_prepare_identity "$workspace" "${SANDBOX_REQUESTED_ALLOW_HOSTS[@]}" || return 1
  if [ "$mode" = "check" ]; then
    sandbox_reconcile_attempt "$mode" "$force"
    return
  fi
  sandbox_acquire_lock || return 1
  sandbox_reconcile_attempt "$mode" "$force" || result=$?
  sandbox_release_lock
  return "$result"
}
