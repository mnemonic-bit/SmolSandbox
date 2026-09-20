#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

SMOLVM_TEST_VERSION='smolvm test 1.0.0'
BASE_ID="$({ printf '%s\n' "$SMOLVM_TEST_VERSION"; cat "$REPO_ROOT/devtools-base.smolfile"; } | sha256sum | awk '{print substr($1, 1, 12)}')"

prepare_case() {
  CASE_ROOT="$TEST_ROOT/$1"
  mkdir -p "$CASE_ROOT/bin" "$CASE_ROOT/state" "$CASE_ROOT/artifacts" "$CASE_ROOT/workspace"
  cp "$REPO_ROOT/tests/fixtures/smolvm" "$CASE_ROOT/bin/smolvm"
  chmod +x "$CASE_ROOT/bin/smolvm"
  WORKSPACE_HASH="$(printf '%s' "$CASE_ROOT/workspace" | sha256sum | awk '{print $1}')"
  WORKSPACE_ID="$(printf '%s' "$WORKSPACE_HASH" | cut -c1-12)"
  DESIRED="codex-workspace-$BASE_ID-r1-$WORKSPACE_ID"
  printf 'artifact\n' > "$CASE_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"
}

run_start() {
  PATH="$CASE_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY="$INVENTORY" \
  SMOLVM_TEST_ALLOW_LIFECYCLE=1 \
  STATE_ROOT="$CASE_ROOT/state" \
  ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/start-codex.sh" "$CASE_ROOT/workspace" -- --help
}

run_replace() {
  local force="${1:-}"
  local -a force_args=()
  if [ "$force" = force ]; then
    force_args=(--force)
  fi
  PATH="$CASE_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY="${INVENTORY:-[]}" \
  SMOLVM_TEST_INVENTORY_FILE="${SMOLVM_TEST_INVENTORY_FILE:-}" \
  SMOLVM_TEST_INVENTORY_SEQUENCE_FILE="${SMOLVM_TEST_INVENTORY_SEQUENCE_FILE:-}" \
  SMOLVM_TEST_INVENTORY_COUNT_FILE="${SMOLVM_TEST_INVENTORY_COUNT_FILE:-}" \
  SMOLVM_TEST_INVENTORY_FAIL_AT="${SMOLVM_TEST_INVENTORY_FAIL_AT:-}" \
  SMOLVM_TEST_CREATE_FAIL="${SMOLVM_TEST_CREATE_FAIL:-}" \
  SMOLVM_TEST_PACK_FAIL="${SMOLVM_TEST_PACK_FAIL:-}" \
  SMOLVM_TEST_STOP_FAIL_NAME="${SMOLVM_TEST_STOP_FAIL_NAME:-}" \
  SMOLVM_TEST_DELETE_FAIL_NAME="${SMOLVM_TEST_DELETE_FAIL_NAME:-}" \
  SMOLVM_TEST_ALLOW_CREATE=1 \
  SMOLVM_TEST_ALLOW_CLEANUP=1 \
  CODEX_MACHINE="${CODEX_MACHINE:-}" \
  STATE_ROOT="$CASE_ROOT/state" \
  ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/repair-sandbox.sh" --replace-stale "${force_args[@]}" \
      "$CASE_ROOT/workspace"
}

set_preserved_caches() {
  printf 'keep-base\n' > "$CASE_ROOT/state/base-id"
  printf 'keep-revision\n' > "$CASE_ROOT/state/machine-revision"
}

assert_preserved_caches() {
  test "$(cat "$CASE_ROOT/state/base-id")" = keep-base
  test "$(cat "$CASE_ROOT/state/machine-revision")" = keep-revision
}

assert_no_stale_cleanup() {
  local stale_name="$1"
  if grep -Eq "machine (stop|delete) --name $stale_name( |$)" "$CASE_ROOT/smolvm.log"; then
    echo "FAIL: stale machine $stale_name was touched before a successful handoff" >&2
    exit 1
  fi
}

prepare_case recover
INVENTORY="[{\"name\":\"$DESIRED\",\"state\":\"stopped\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}]"
run_start > "$CASE_ROOT/output" 2>&1
test "$(cat "$CASE_ROOT/state/base-id")" = "$BASE_ID"
test "$(cat "$CASE_ROOT/state/machine-revision")" = 1
if grep -Fq 'machine create' "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: cache recovery created a machine' >&2
  exit 1
fi

prepare_case stale
STALE="codex-workspace-111111111111-r7-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
printf 'keep-base\n' > "$CASE_ROOT/state/base-id"
printf 'keep-revision\n' > "$CASE_ROOT/state/machine-revision"
if run_start > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: normal startup accepted a stale-only inventory' >&2
  exit 1
fi
test "$(cat "$CASE_ROOT/state/base-id")" = keep-base
test "$(cat "$CASE_ROOT/state/machine-revision")" = keep-revision
if grep -Eq 'machine (create|start|stop|delete)' "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: stale-only startup mutated a machine' >&2
  exit 1
fi
grep -Fq "$STALE" "$CASE_ROOT/output"
grep -Fq -- '--replace-stale' "$CASE_ROOT/output"

prepare_case invalid
INVENTORY='{not-json'
if run_start > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: invalid inventory was accepted' >&2
  exit 1
fi
if grep -Eq 'machine (create|start|stop|delete)' "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: invalid inventory caused a machine mutation' >&2
  exit 1
fi

prepare_case inventory-failure
INVENTORY='[]'
if SMOLVM_TEST_INVENTORY_FAIL=1 run_start > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: failed inventory command was treated as an empty inventory' >&2
  exit 1
fi
if grep -Eq 'machine (create|start|stop|delete)' "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: failed inventory command caused a machine mutation' >&2
  exit 1
fi

prepare_case create
INVENTORY='[]'
SMOLVM_TEST_ALLOW_CREATE=1 run_start > "$CASE_ROOT/output" 2>&1
if [ "$(grep -Fc "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log")" -ne 1 ]; then
  echo 'FAIL: missing desired machine was not created exactly once' >&2
  exit 1
fi
for label in \
  'codex-sandbox.kind=workspace' \
  "codex-sandbox.base-id=$BASE_ID" \
  'codex-sandbox.revision=1' \
  "codex-sandbox.workspace-sha256=$WORKSPACE_HASH"
do
  grep -Fq -- "--label $label" "$CASE_ROOT/smolvm.log"
done

prepare_case build-base
rm -f -- "$CASE_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"
INVENTORY='[]'
SMOLVM_TEST_ALLOW_CREATE=1 SMOLVM_TEST_ALLOW_CLEANUP=1 run_start > "$CASE_ROOT/output" 2>&1
test -s "$CASE_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"
grep -Fq "machine create --name codex-builder-$BASE_ID" "$CASE_ROOT/smolvm.log"
grep -Fq "pack create --from-vm codex-builder-$BASE_ID" "$CASE_ROOT/smolvm.log"
grep -Fqx "machine delete --name codex-builder-$BASE_ID --force" "$CASE_ROOT/smolvm.log"
grep -Fq "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log"

prepare_case clean-existing-builder
OLD_BASE_ID=111111111111
printf 'old artifact\n' > "$CASE_ROOT/artifacts/codex-devtools-$OLD_BASE_ID.smolmachine"
INVENTORY="[{\"name\":\"$DESIRED\",\"state\":\"stopped\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}},{\"name\":\"codex-builder-$BASE_ID\",\"state\":\"stopped\",\"labels\":{}},{\"name\":\"codex-builder-$OLD_BASE_ID\",\"state\":\"stopped\",\"labels\":{}}]"
SMOLVM_TEST_ALLOW_CLEANUP=1 run_start > "$CASE_ROOT/output" 2>&1
grep -Fqx "machine delete --name codex-builder-$BASE_ID --force" "$CASE_ROOT/smolvm.log"
grep -Fqx "machine delete --name codex-builder-$OLD_BASE_ID --force" "$CASE_ROOT/smolvm.log"

prepare_case replace
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
printf '%s\n' "$INVENTORY" > "$CASE_ROOT/inventory.json"
PATH="$CASE_ROOT/bin:$PATH" \
SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
SMOLVM_TEST_INVENTORY_FILE="$CASE_ROOT/inventory.json" \
SMOLVM_TEST_ALLOW_CREATE=1 \
SMOLVM_TEST_ALLOW_CLEANUP=1 \
STATE_ROOT="$CASE_ROOT/state" \
ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
  "$REPO_ROOT/repair-sandbox.sh" --replace-stale "$CASE_ROOT/workspace" > "$CASE_ROOT/output" 2>&1
grep -Fqx "machine delete --name $STALE --force" "$CASE_ROOT/smolvm.log"
grep -Fq "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log"
create_line="$(grep -n -m1 -F "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log" | cut -d: -f1)"
delete_line="$(grep -n -m1 -F "machine delete --name $STALE --force" "$CASE_ROOT/smolvm.log" | cut -d: -f1)"
test "$create_line" -lt "$delete_line"
grep -Fq "Verified desired workspace machine $DESIRED (created)." "$CASE_ROOT/output"

prepare_case running
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"Running\",\"labels\":{}}]"
if PATH="$CASE_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY="$INVENTORY" \
  SMOLVM_TEST_ALLOW_CREATE=1 \
  SMOLVM_TEST_ALLOW_CLEANUP=1 \
  STATE_ROOT="$CASE_ROOT/state" \
  ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/repair-sandbox.sh" --replace-stale "$CASE_ROOT/workspace" > "$CASE_ROOT/output" 2>&1
then
  echo 'FAIL: repair removed a running stale machine without --force' >&2
  exit 1
fi
if grep -Eq 'machine (create|start|stop|delete)' "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: refused running replacement still mutated a machine' >&2
  exit 1
fi
: > "$CASE_ROOT/smolvm.log"
printf '%s\n' "$INVENTORY" > "$CASE_ROOT/inventory.json"
PATH="$CASE_ROOT/bin:$PATH" \
SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
SMOLVM_TEST_INVENTORY_FILE="$CASE_ROOT/inventory.json" \
SMOLVM_TEST_ALLOW_CREATE=1 \
SMOLVM_TEST_ALLOW_CLEANUP=1 \
STATE_ROOT="$CASE_ROOT/state" \
ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
  "$REPO_ROOT/repair-sandbox.sh" --replace-stale --force "$CASE_ROOT/workspace" > "$CASE_ROOT/output" 2>&1
grep -Fqx "machine stop --name $STALE" "$CASE_ROOT/smolvm.log"
grep -Fqx "machine delete --name $STALE --force" "$CASE_ROOT/smolvm.log"
grep -Fq "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log"
create_line="$(grep -n -m1 -F "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log" | cut -d: -f1)"
stop_line="$(grep -n -m1 -F "machine stop --name $STALE" "$CASE_ROOT/smolvm.log" | cut -d: -f1)"
delete_line="$(grep -n -m1 -F "machine delete --name $STALE --force" "$CASE_ROOT/smolvm.log" | cut -d: -f1)"
test "$create_line" -lt "$stop_line"
test "$stop_line" -lt "$delete_line"

prepare_case replace-build-failure
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
rm -f -- "$CASE_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"
set_preserved_caches
if SMOLVM_TEST_PACK_FAIL=1 run_replace > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: replacement accepted a failed desired-base build' >&2
  exit 1
fi
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-create-failure
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
set_preserved_caches
if SMOLVM_TEST_CREATE_FAIL=1 run_replace > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: replacement accepted a failed desired-machine create' >&2
  exit 1
fi
grep -Fq "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log"
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-partial-create-failure
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
printf '%s\n' \
  "$INVENTORY" \
  "[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}},{\"name\":\"$DESIRED\",\"state\":\"created\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}]" \
  > "$CASE_ROOT/inventory-sequence"
set_preserved_caches
if SMOLVM_TEST_CREATE_FAIL=1 \
   SMOLVM_TEST_INVENTORY_SEQUENCE_FILE="$CASE_ROOT/inventory-sequence" \
   SMOLVM_TEST_INVENTORY_COUNT_FILE="$CASE_ROOT/inventory-count" \
   run_replace > "$CASE_ROOT/output" 2>&1
then
  echo 'FAIL: partial registration after create failure permitted cleanup' >&2
  exit 1
fi
grep -Fq 'preserved for inspection' "$CASE_ROOT/output"
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-refresh-failure
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
printf '%s\n%s\n' "$INVENTORY" "$INVENTORY" > "$CASE_ROOT/inventory-sequence"
set_preserved_caches
if SMOLVM_TEST_INVENTORY_SEQUENCE_FILE="$CASE_ROOT/inventory-sequence" \
   SMOLVM_TEST_INVENTORY_COUNT_FILE="$CASE_ROOT/inventory-count" \
   SMOLVM_TEST_INVENTORY_FAIL_AT=2 \
   run_replace > "$CASE_ROOT/output" 2>&1
then
  echo 'FAIL: replacement cleaned stale state after refresh failure' >&2
  exit 1
fi
grep -Fq "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log"
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-refresh-invalid
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
printf '%s\n%s\n' "$INVENTORY" '{not-json' > "$CASE_ROOT/inventory-sequence"
set_preserved_caches
if SMOLVM_TEST_INVENTORY_SEQUENCE_FILE="$CASE_ROOT/inventory-sequence" \
   SMOLVM_TEST_INVENTORY_COUNT_FILE="$CASE_ROOT/inventory-count" \
   run_replace > "$CASE_ROOT/output" 2>&1
then
  echo 'FAIL: replacement cleaned stale state after invalid refreshed inventory' >&2
  exit 1
fi
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-missing-desired
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
printf '%s\n%s\n' "$INVENTORY" "$INVENTORY" > "$CASE_ROOT/inventory-sequence"
set_preserved_caches
if SMOLVM_TEST_INVENTORY_SEQUENCE_FILE="$CASE_ROOT/inventory-sequence" \
   SMOLVM_TEST_INVENTORY_COUNT_FILE="$CASE_ROOT/inventory-count" \
   run_replace > "$CASE_ROOT/output" 2>&1
then
  echo 'FAIL: replacement cleaned stale state while desired machine was missing' >&2
  exit 1
fi
grep -Fq 'desired machine is missing' "$CASE_ROOT/output"
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-duplicate-desired
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
DESIRED_JSON="{\"name\":\"$DESIRED\",\"state\":\"created\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}"
printf '%s\n%s\n' "$INVENTORY" "[$DESIRED_JSON,$DESIRED_JSON,{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]" > "$CASE_ROOT/inventory-sequence"
set_preserved_caches
if SMOLVM_TEST_INVENTORY_SEQUENCE_FILE="$CASE_ROOT/inventory-sequence" \
   SMOLVM_TEST_INVENTORY_COUNT_FILE="$CASE_ROOT/inventory-count" \
   run_replace > "$CASE_ROOT/output" 2>&1
then
  echo 'FAIL: duplicate desired inventory entries permitted cleanup' >&2
  exit 1
fi
grep -Fq 'duplicate machine name' "$CASE_ROOT/output"
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-contradictory-desired
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
INVENTORY="[{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
CONTRADICTORY_DESIRED="{\"name\":\"$DESIRED\",\"state\":\"created\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"another-workspace\"}}"
printf '%s\n%s\n' "$INVENTORY" "[$CONTRADICTORY_DESIRED,{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]" > "$CASE_ROOT/inventory-sequence"
set_preserved_caches
if SMOLVM_TEST_INVENTORY_SEQUENCE_FILE="$CASE_ROOT/inventory-sequence" \
   SMOLVM_TEST_INVENTORY_COUNT_FILE="$CASE_ROOT/inventory-count" \
   run_replace > "$CASE_ROOT/output" 2>&1
then
  echo 'FAIL: contradictory desired labels permitted cleanup' >&2
  exit 1
fi
grep -Fq 'refusing cleanup because refreshed workspace ownership is ambiguous' "$CASE_ROOT/output"
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-failed-desired
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
DESIRED_JSON="{\"name\":\"$DESIRED\",\"state\":\"failed\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}"
INVENTORY="[$DESIRED_JSON,{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
set_preserved_caches
if run_replace > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: failed desired machine passed replacement readiness' >&2
  exit 1
fi
if grep -Fq "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: existing failed desired machine was recreated' >&2
  exit 1
fi
grep -Fq "state is 'failed'" "$CASE_ROOT/output"
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-existing-desired
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
DESIRED_JSON="{\"name\":\"$DESIRED\",\"state\":\"Stopped\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}"
INVENTORY="[$DESIRED_JSON,{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}}]"
run_replace > "$CASE_ROOT/output" 2>&1
if grep -Fq "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: existing ready desired machine was recreated' >&2
  exit 1
fi
grep -Fqx "machine delete --name $STALE --force" "$CASE_ROOT/smolvm.log"
test "$(cat "$CASE_ROOT/state/base-id")" = "$BASE_ID"
test "$(cat "$CASE_ROOT/state/machine-revision")" = 1

prepare_case replace-cleanup-failure
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
SECOND_STALE="codex-workspace-222222222222-r2-$WORKSPACE_ID"
DESIRED_JSON="{\"name\":\"$DESIRED\",\"state\":\"created\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}"
INVENTORY="[$DESIRED_JSON,{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}},{\"name\":\"$SECOND_STALE\",\"state\":\"stopped\",\"labels\":{}}]"
set_preserved_caches
if SMOLVM_TEST_DELETE_FAIL_NAME="$STALE" run_replace > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: replacement accepted stale cleanup failure' >&2
  exit 1
fi
grep -Fq "could not delete stale workspace machine $STALE" "$CASE_ROOT/output"
if grep -Fq "machine delete --name $SECOND_STALE" "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: cleanup continued after a stale deletion failure' >&2
  exit 1
fi
if grep -Fq "machine delete --name $DESIRED" "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: desired machine was deleted as cleanup rollback' >&2
  exit 1
fi
assert_preserved_caches

prepare_case replace-stop-failure
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
DESIRED_JSON="{\"name\":\"$DESIRED\",\"state\":\"created\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}"
INVENTORY="[$DESIRED_JSON,{\"name\":\"$STALE\",\"state\":\"running\",\"labels\":{}}]"
set_preserved_caches
if SMOLVM_TEST_STOP_FAIL_NAME="$STALE" run_replace force > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: replacement accepted stale stop failure' >&2
  exit 1
fi
grep -Fq "could not stop stale workspace machine $STALE" "$CASE_ROOT/output"
if grep -Fq "machine delete --name $STALE" "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: cleanup deleted a stale machine after its stop failed' >&2
  exit 1
fi
assert_preserved_caches

prepare_case replace-override
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
OVERRIDE_MACHINE='my-explicit-codex-machine'
INVENTORY="[{\"name\":\"$OVERRIDE_MACHINE\",\"state\":\"created\",\"labels\":{}},{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"111111111111\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}]"
CODEX_MACHINE="$OVERRIDE_MACHINE" run_replace > "$CASE_ROOT/output" 2>&1
grep -Fq "Verified desired workspace machine $OVERRIDE_MACHINE (created)." "$CASE_ROOT/output"
grep -Fqx "machine delete --name $STALE --force" "$CASE_ROOT/smolvm.log"

prepare_case replace-override-contradictory
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
OVERRIDE_MACHINE='my-explicit-codex-machine'
INVENTORY="[{\"name\":\"$OVERRIDE_MACHINE\",\"state\":\"created\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"not-this-workspace\"}},{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"111111111111\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}]"
set_preserved_caches
if CODEX_MACHINE="$OVERRIDE_MACHINE" run_replace > "$CASE_ROOT/output" 2>&1; then
  echo 'FAIL: contradictory override labels passed replacement readiness' >&2
  exit 1
fi
grep -Fq 'incomplete or contradictory managed labels' "$CASE_ROOT/output"
assert_no_stale_cleanup "$STALE"
assert_preserved_caches

prepare_case replace-unrelated
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
UNRELATED='another-workspace-machine'
DESIRED_JSON="{\"name\":\"$DESIRED\",\"state\":\"created\",\"labels\":{\"codex-sandbox.kind\":\"workspace\",\"codex-sandbox.base-id\":\"$BASE_ID\",\"codex-sandbox.revision\":\"1\",\"codex-sandbox.workspace-sha256\":\"$WORKSPACE_HASH\"}}"
INVENTORY="[$DESIRED_JSON,{\"name\":\"$STALE\",\"state\":\"stopped\",\"labels\":{}},{\"name\":\"$UNRELATED\",\"state\":\"running\",\"labels\":{}}]"
run_replace > "$CASE_ROOT/output" 2>&1
if grep -Eq "machine (stop|delete) --name $UNRELATED( |$)" "$CASE_ROOT/smolvm.log"; then
  echo 'FAIL: replacement touched an unrelated machine' >&2
  exit 1
fi

prepare_case concurrent
printf '[]\n' > "$CASE_ROOT/inventory.json"
run_concurrent_start() {
  PATH="$CASE_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY_FILE="$CASE_ROOT/inventory.json" \
  SMOLVM_TEST_ALLOW_CREATE=1 \
  SMOLVM_TEST_ALLOW_LIFECYCLE=1 \
  STATE_ROOT="$CASE_ROOT/state" \
  ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/start-codex.sh" "$CASE_ROOT/workspace" -- --help >/dev/null 2>&1
}
run_concurrent_start & first_pid=$!
run_concurrent_start & second_pid=$!
wait "$first_pid"
wait "$second_pid"
if [ "$(grep -Fc "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log")" -ne 1 ]; then
  echo 'FAIL: concurrent starts created the workspace machine more than once' >&2
  exit 1
fi

prepare_case concurrent-replace
STALE="codex-workspace-111111111111-r1-$WORKSPACE_ID"
printf '[{"name":"%s","state":"stopped","labels":{}}]\n' "$STALE" > "$CASE_ROOT/inventory.json"
run_concurrent_replace() {
  SMOLVM_TEST_INVENTORY_FILE="$CASE_ROOT/inventory.json" \
    run_replace > "$CASE_ROOT/replace-$BASHPID.output" 2>&1
}
run_concurrent_replace & first_pid=$!
run_concurrent_replace & second_pid=$!
wait "$first_pid"
wait "$second_pid"
if [ "$(grep -Fc "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log")" -ne 1 ]; then
  echo 'FAIL: concurrent replacements created the desired machine more than once' >&2
  exit 1
fi
if [ "$(grep -Fc "machine delete --name $STALE --force" "$CASE_ROOT/smolvm.log")" -ne 1 ]; then
  echo 'FAIL: concurrent replacements deleted the stale machine more than once' >&2
  exit 1
fi

echo 'PASS: workspace reconciliation recovers, verifies before cleanup, preserves failures, and serializes handoff'
