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
PATH="$CASE_ROOT/bin:$PATH" \
SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
SMOLVM_TEST_INVENTORY="$INVENTORY" \
SMOLVM_TEST_ALLOW_CREATE=1 \
SMOLVM_TEST_ALLOW_CLEANUP=1 \
STATE_ROOT="$CASE_ROOT/state" \
ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
  "$REPO_ROOT/repair-sandbox.sh" --replace-stale "$CASE_ROOT/workspace" > "$CASE_ROOT/output" 2>&1
grep -Fqx "machine delete --name $STALE --force" "$CASE_ROOT/smolvm.log"
grep -Fq "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log"

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
PATH="$CASE_ROOT/bin:$PATH" \
SMOLVM_TEST_LOG="$CASE_ROOT/smolvm.log" \
SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
SMOLVM_TEST_INVENTORY="$INVENTORY" \
SMOLVM_TEST_ALLOW_CREATE=1 \
SMOLVM_TEST_ALLOW_CLEANUP=1 \
STATE_ROOT="$CASE_ROOT/state" \
ARTIFACTS_DIR="$CASE_ROOT/artifacts" \
BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
  "$REPO_ROOT/repair-sandbox.sh" --replace-stale --force "$CASE_ROOT/workspace" > "$CASE_ROOT/output" 2>&1
grep -Fqx "machine stop --name $STALE" "$CASE_ROOT/smolvm.log"
grep -Fqx "machine delete --name $STALE --force" "$CASE_ROOT/smolvm.log"
grep -Fq "machine create --name $DESIRED" "$CASE_ROOT/smolvm.log"

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

echo 'PASS: workspace reconciliation recovers, blocks stale state, repairs explicitly, and serializes creation'
