#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=../sandbox-common.sh
source "$REPO_ROOT/sandbox-common.sh"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

SMOLVM_TEST_VERSION='smolvm test 1.0.0'
BASE_ID="$({ printf '%s\n' "$SMOLVM_TEST_VERSION"; cat "$REPO_ROOT/devtools-base.smolfile"; } | sha256sum | awk '{print substr($1, 1, 12)}')"
OLD_BASE=111111111111
REVISION="$SANDBOX_MACHINE_REVISION"
DEFAULT_HASH="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
CUSTOM_HASH="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
PATH_HASH="cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
DEFAULT_NAME="codex-workspace-$BASE_ID-r$REVISION-${DEFAULT_HASH:0:12}"
CUSTOM_NAME="codex-workspace-$BASE_ID-r$REVISION-${CUSTOM_HASH:0:12}"
OLD_NAME="codex-workspace-$OLD_BASE-r1-dddddddddddd"

workspace_json() {
  local name="$1" state="$2" base="$3" revision="$4" workspace_hash="$5" configuration="$6"
  printf '{"name":"%s","state":"%s","labels":{"codex-sandbox.kind":"workspace","codex-sandbox.base-id":"%s","codex-sandbox.revision":"%s","codex-sandbox.workspace-sha256":"%s","codex-sandbox.workspace-path-sha256":"%s","codex-sandbox.configuration":"%s"}}' \
    "$name" "$state" "$base" "$revision" "$workspace_hash" "$PATH_HASH" "$configuration"
}

prepare_case() {
  CASE_ROOT="$TEST_ROOT/$1"
  mkdir -p "$CASE_ROOT/bin" "$CASE_ROOT/state" "$CASE_ROOT/artifacts"
  cp "$REPO_ROOT/tests/fixtures/smolvm" "$CASE_ROOT/bin/smolvm"
  chmod +x "$CASE_ROOT/bin/smolvm"
  printf 'current\n' > "$CASE_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"
  : > "$CASE_ROOT/smolvm.log"
}

run_purge() {
  PATH="$CASE_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY_FILE="${INVENTORY_FILE:-}" \
  SMOLVM_TEST_INVENTORY_SEQUENCE_FILE="${INVENTORY_SEQUENCE_FILE:-}" \
  SMOLVM_TEST_INVENTORY_COUNT_FILE="${INVENTORY_COUNT_FILE:-}" \
  SMOLVM_TEST_DELETE_FAIL_NAME="${DELETE_FAIL_NAME:-}" \
  SMOLVM_TEST_ALLOW_CLEANUP=1 \
  STATE_ROOT="$CASE_ROOT/state" \
  ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/purge-sandbox.sh" "$@"
}

prepare_case conservative-legacy
DEFAULT_JSON="$(workspace_json "$DEFAULT_NAME" stopped "$BASE_ID" "$REVISION" "$DEFAULT_HASH" default)"
printf '[%s,{"name":"%s","state":"stopped","labels":{}}]\n' \
  "$DEFAULT_JSON" "$OLD_NAME" > "$CASE_ROOT/inventory.json"
INVENTORY_FILE="$CASE_ROOT/inventory.json"
printf 'old\n' > "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"
printf 'old companion\n' > "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE"
printf 'keep\n' > "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.tmp"
mkdir -p "$CASE_ROOT/artifacts/codex-devtools-fedcba987654.smolmachine/subdirectory"
printf 'keep\n' > "$CASE_ROOT/artifacts/codex-devtools-fedcba987654.smolmachine/subdirectory/file"
run_purge > "$CASE_ROOT/output" 2>&1
grep -Fqx "machine delete --name $OLD_NAME --force" "$CASE_ROOT/smolvm.log"
test -e "$CASE_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"
test ! -e "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"
test ! -e "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE"
test -e "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.tmp"
test -e "$CASE_ROOT/artifacts/codex-devtools-fedcba987654.smolmachine/subdirectory/file"
grep -Fq 'Removed 1 machine(s), 1 generation(s), and 2 artifact file(s).' "$CASE_ROOT/output"

prepare_case conservative-refusal
DEFAULT_JSON="$(workspace_json "$DEFAULT_NAME" stopped "$BASE_ID" "$REVISION" "$DEFAULT_HASH" default)"
CUSTOM_JSON="$(workspace_json "$CUSTOM_NAME" stopped "$BASE_ID" "$REVISION" "$CUSTOM_HASH" custom)"
printf '[%s,%s,{"name":"%s","state":"stopped","labels":{}}]\n' \
  "$DEFAULT_JSON" "$CUSTOM_JSON" "$OLD_NAME" > "$CASE_ROOT/inventory.json"
INVENTORY_FILE="$CASE_ROOT/inventory.json"
printf 'old\n' > "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"
if run_purge > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: conservative purge accepted a current custom configuration' >&2
  exit 1
else
  status=$?
fi
test "$status" -eq 3
test -e "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"
if grep -Fq 'machine delete' "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: conservative refusal deleted a machine' >&2
  exit 1
fi
grep -Fq 'Additional --allow-host options' "$CASE_ROOT/output"

prepare_case force
DEFAULT_JSON="$(workspace_json "$DEFAULT_NAME" stopped "$BASE_ID" "$REVISION" "$DEFAULT_HASH" default)"
CUSTOM_JSON="$(workspace_json "$CUSTOM_NAME" stopped "$BASE_ID" "$REVISION" "$CUSTOM_HASH" custom)"
printf '[%s,%s]\n' "$DEFAULT_JSON" "$CUSTOM_JSON" > "$CASE_ROOT/inventory.json"
INVENTORY_FILE="$CASE_ROOT/inventory.json"
run_purge --force > "$CASE_ROOT/output" 2>&1
grep -Fqx "machine delete --name $CUSTOM_NAME --force" "$CASE_ROOT/smolvm.log"
if grep -Fq "machine delete --name $DEFAULT_NAME" "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: --force deleted a default configuration' >&2
  exit 1
fi

prepare_case build-refusal
mkdir -p "$CASE_ROOT/workspace"
ACTUAL_PATH_HASH="$(printf '%s' "$CASE_ROOT/workspace" | sha256sum | awk '{print $1}')"
ACTUAL_DEFAULT_HASH="$(sandbox_hash_set \
  "--from=$CASE_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine" \
  "--mem=$SANDBOX_WORKSPACE_MEMORY" \
  "--cpus=$SANDBOX_WORKSPACE_CPUS" \
  "--volume=$CASE_ROOT/state/codex-home:/root/.codex" \
  "--volume=$CASE_ROOT/workspace:/workspace/myrepo" \
  "${SANDBOX_DEFAULT_ALLOW_HOSTS[@]/#/--allow-host=}")"
ACTUAL_DEFAULT_NAME="codex-workspace-$BASE_ID-r$REVISION-${ACTUAL_DEFAULT_HASH:0:12}"
ACTUAL_CUSTOM_HASH="eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
ACTUAL_CUSTOM_NAME="codex-workspace-$BASE_ID-r$REVISION-${ACTUAL_CUSTOM_HASH:0:12}"
printf '[{"name":"%s","state":"stopped","labels":{"codex-sandbox.kind":"workspace","codex-sandbox.base-id":"%s","codex-sandbox.revision":"%s","codex-sandbox.workspace-sha256":"%s","codex-sandbox.workspace-path-sha256":"%s","codex-sandbox.configuration":"default"}},{"name":"%s","state":"stopped","labels":{"codex-sandbox.kind":"workspace","codex-sandbox.base-id":"%s","codex-sandbox.revision":"%s","codex-sandbox.workspace-sha256":"%s","codex-sandbox.workspace-path-sha256":"%s","codex-sandbox.configuration":"custom"}}]\n' \
  "$ACTUAL_DEFAULT_NAME" "$BASE_ID" "$REVISION" "$ACTUAL_DEFAULT_HASH" "$ACTUAL_PATH_HASH" \
  "$ACTUAL_CUSTOM_NAME" "$BASE_ID" "$REVISION" "$ACTUAL_CUSTOM_HASH" "$ACTUAL_PATH_HASH" \
  > "$CASE_ROOT/inventory.json"
printf 'old\n' > "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"
PATH="$CASE_ROOT/bin:$PATH" \
SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
SMOLVM_TEST_INVENTORY_FILE="$CASE_ROOT/inventory.json" \
SMOLVM_TEST_ALLOW_CREATE=1 \
SMOLVM_TEST_ALLOW_CLEANUP=1 \
STATE_ROOT="$CASE_ROOT/state" \
ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
  "$REPO_ROOT/build-sandbox.sh" "$CASE_ROOT/workspace" > "$CASE_ROOT/output" 2>&1
grep -Fq 'Build completed; conservative purge did not run.' "$CASE_ROOT/output"
test -e "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"

prepare_case force-all-running
DEFAULT_JSON="$(workspace_json "$DEFAULT_NAME" stopped "$BASE_ID" "$REVISION" "$DEFAULT_HASH" default)"
RUNNING_OLD="codex-workspace-$OLD_BASE-r1-eeeeeeeeeeee"
printf '[%s,{"name":"codex-login-%s-r%s","state":"stopped","labels":{}},{"name":"%s","state":"running","labels":{}}]\n' \
  "$DEFAULT_JSON" "$BASE_ID" "$REVISION" "$RUNNING_OLD" > "$CASE_ROOT/inventory.json"
INVENTORY_FILE="$CASE_ROOT/inventory.json"
printf 'protected\n' > "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"
run_purge --force-all > "$CASE_ROOT/output" 2>&1
grep -Fqx "machine delete --name $DEFAULT_NAME --force" "$CASE_ROOT/smolvm.log"
grep -Fqx "machine delete --name codex-login-$BASE_ID-r$REVISION --force" "$CASE_ROOT/smolvm.log"
if grep -Fq "$RUNNING_OLD" "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: --force-all touched a running machine' >&2
  exit 1
fi
test -e "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"

prepare_case ambiguous
printf '[{"name":"%s","state":"stopped","labels":{"codex-sandbox.kind":"workspace","codex-sandbox.base-id":"%s","codex-sandbox.revision":"%s","codex-sandbox.workspace-sha256":"%s"}}]\n' \
  "$DEFAULT_NAME" "$BASE_ID" "$REVISION" "$DEFAULT_HASH" > "$CASE_ROOT/inventory.json"
INVENTORY_FILE="$CASE_ROOT/inventory.json"
printf 'old\n' > "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"
if run_purge --force-all > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: ambiguous current metadata was accepted' >&2
  exit 1
fi
test -e "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"
grep -Fq 'workspace labels are incomplete or contradictory' "$CASE_ROOT/output"

prepare_case fresh-change
printf '[{"name":"%s","state":"stopped","labels":{}}]\n[{"name":"%s","state":"running","labels":{}}]\n' \
  "$OLD_NAME" "$OLD_NAME" > "$CASE_ROOT/inventory-sequence"
INVENTORY_FILE=""
INVENTORY_SEQUENCE_FILE="$CASE_ROOT/inventory-sequence"
INVENTORY_COUNT_FILE="$CASE_ROOT/inventory-count"
printf 'old\n' > "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"
if run_purge > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: purge ignored a fresh running-state change' >&2
  exit 1
fi
if grep -Fq 'machine delete' "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: changed inventory caused a deletion' >&2
  exit 1
fi
test -e "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE.smolmachine"

echo 'PASS: purge plans conservatively, honors force policies, and protects running and ambiguous state'
