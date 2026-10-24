#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

SMOLVM_TEST_VERSION='smolvm test 1.0.0'
BASE_ID="$({ printf '%s\n' "$SMOLVM_TEST_VERSION"; cat "$REPO_ROOT/devtools-base.smolfile"; } | sha256sum | awk '{print substr($1, 1, 12)}')"
OTHER_BASE_ID=fedcba987654

mkdir -p \
  "$TEST_ROOT/bin" \
  "$TEST_ROOT/state/codex-home" \
  "$TEST_ROOT/artifacts"
cp "$REPO_ROOT/tests/fixtures/smolvm" "$TEST_ROOT/bin/smolvm"
chmod +x "$TEST_ROOT/bin/smolvm"

printf '%s\n' "$OTHER_BASE_ID" > "$TEST_ROOT/state/base-id"
printf '99\n' > "$TEST_ROOT/state/machine-revision"
printf 'keep\n' > "$TEST_ROOT/state/codex-home/config.toml"
printf 'artifact\n' > "$TEST_ROOT/artifacts/codex-devtools-$BASE_ID"
printf 'artifact\n' > "$TEST_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"
printf 'keep\n' > "$TEST_ROOT/artifacts/codex-devtools-$OTHER_BASE_ID.smolmachine"

MACHINES="$(printf '%s\n' \
  "codex-builder-$BASE_ID" \
  "codex-login-$BASE_ID-r1" \
  "codex-login-$BASE_ID-r2" \
  "codex-workspace-$BASE_ID-r1-111111111111" \
  "codex-workspace-$BASE_ID-r2-222222222222" \
  "codex-builder-$OTHER_BASE_ID" \
  "unrelated-machine")"

OUTPUT="$(
  PATH="$TEST_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$TEST_ROOT/smolvm.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_MACHINES="$MACHINES" \
  SMOLVM_TEST_ALLOW_CLEANUP=1 \
  STATE_ROOT="$TEST_ROOT/state" \
  ARTIFACTS_DIR="$TEST_ROOT/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/clean-sandbox.sh"
)"

for machine in \
  "codex-builder-$BASE_ID" \
  "codex-login-$BASE_ID-r1" \
  "codex-login-$BASE_ID-r2" \
  "codex-workspace-$BASE_ID-r1-111111111111" \
  "codex-workspace-$BASE_ID-r2-222222222222"
do
  if ! grep -Fqx "machine delete --name $machine --force" "$TEST_ROOT/smolvm.log"; then
    echo "FAIL: current-generation machine was not deleted: $machine" >&2
    exit 1
  fi
done

if grep -Fq "$OTHER_BASE_ID" "$TEST_ROOT/smolvm.log"; then
  echo 'FAIL: a machine from another generation was modified' >&2
  exit 1
fi
if grep -Fq 'unrelated-machine' "$TEST_ROOT/smolvm.log"; then
  echo 'FAIL: an unrelated machine was modified' >&2
  exit 1
fi

if [ -e "$TEST_ROOT/artifacts/codex-devtools-$BASE_ID" ] || \
   [ -e "$TEST_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine" ]; then
  echo 'FAIL: current-generation artifacts were not removed' >&2
  exit 1
fi
if [ ! -e "$TEST_ROOT/artifacts/codex-devtools-$OTHER_BASE_ID.smolmachine" ]; then
  echo 'FAIL: an artifact from another generation was removed' >&2
  exit 1
fi
if [ -e "$TEST_ROOT/state/base-id" ] || [ -e "$TEST_ROOT/state/machine-revision" ]; then
  echo 'FAIL: generation pointers were not removed' >&2
  exit 1
fi
if [ ! -e "$TEST_ROOT/state/codex-home/config.toml" ]; then
  echo 'FAIL: shared Codex state was removed' >&2
  exit 1
fi

if ! grep -Fq 'Removed 5 VM(s) and the cached artifact.' <<< "$OUTPUT"; then
  printf 'FAIL: cleanup summary was incorrect\n%s\n' "$OUTPUT" >&2
  exit 1
fi

WORKSPACE_CASE="$TEST_ROOT/workspace-clean"
TARGET_WORKSPACE="$WORKSPACE_CASE/target"
OTHER_WORKSPACE="$WORKSPACE_CASE/other"
mkdir -p "$WORKSPACE_CASE/state/codex-home" "$WORKSPACE_CASE/state/ports" \
  "$WORKSPACE_CASE/artifacts" "$TARGET_WORKSPACE" "$OTHER_WORKSPACE"
TARGET_PATH_HASH="$(printf '%s' "$TARGET_WORKSPACE" | sha256sum | awk '{print $1}')"
OTHER_PATH_HASH="$(printf '%s' "$OTHER_WORKSPACE" | sha256sum | awk '{print $1}')"
TARGET_DEFAULT_HASH=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
TARGET_CUSTOM_HASH=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
OTHER_HASH=cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
TARGET_DEFAULT_NAME="codex-workspace-$BASE_ID-r4-${TARGET_DEFAULT_HASH:0:12}"
TARGET_CUSTOM_NAME="codex-workspace-$OTHER_BASE_ID-r3-${TARGET_CUSTOM_HASH:0:12}"
OTHER_NAME="codex-workspace-$BASE_ID-r4-${OTHER_HASH:0:12}"

workspace_json() {
  local name="$1" state="$2" base="$3" revision="$4" workspace_hash="$5" path_hash="$6" configuration="$7"
  printf '{"name":"%s","state":"%s","labels":{"codex-sandbox.kind":"workspace","codex-sandbox.base-id":"%s","codex-sandbox.revision":"%s","codex-sandbox.workspace-sha256":"%s","codex-sandbox.workspace-path-sha256":"%s","codex-sandbox.configuration":"%s"}}' \
    "$name" "$state" "$base" "$revision" "$workspace_hash" "$path_hash" "$configuration"
}

TARGET_DEFAULT_JSON="$(workspace_json "$TARGET_DEFAULT_NAME" running "$BASE_ID" 4 "$TARGET_DEFAULT_HASH" "$TARGET_PATH_HASH" default)"
TARGET_CUSTOM_JSON="$(workspace_json "$TARGET_CUSTOM_NAME" stopped "$OTHER_BASE_ID" 3 "$TARGET_CUSTOM_HASH" "$TARGET_PATH_HASH" custom)"
OTHER_JSON="$(workspace_json "$OTHER_NAME" stopped "$BASE_ID" 4 "$OTHER_HASH" "$OTHER_PATH_HASH" default)"
printf '[%s,%s,%s,{"name":"codex-login-%s-r4","state":"stopped","labels":{}}]\n' \
  "$TARGET_DEFAULT_JSON" "$TARGET_CUSTOM_JSON" "$OTHER_JSON" "$BASE_ID" \
  > "$WORKSPACE_CASE/inventory.json"
printf 'keep\n' > "$WORKSPACE_CASE/artifacts/codex-devtools-$BASE_ID.smolmachine"
printf '%s\n' "$BASE_ID" > "$WORKSPACE_CASE/state/base-id"
printf '4\n' > "$WORKSPACE_CASE/state/machine-revision"
printf 'keep\n' > "$WORKSPACE_CASE/state/codex-home/config.toml"
printf '{}\n' > "$WORKSPACE_CASE/state/ports/$TARGET_DEFAULT_NAME.json"
printf '{}\n' > "$WORKSPACE_CASE/state/ports/$TARGET_CUSTOM_NAME.pending.json"
: > "$WORKSPACE_CASE/smolvm.log"

WORKSPACE_OUTPUT="$(
  cd "$TARGET_WORKSPACE"
  PATH="$TEST_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$WORKSPACE_CASE/smolvm.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY_FILE="$WORKSPACE_CASE/inventory.json" \
  SMOLVM_TEST_ALLOW_CLEANUP=1 \
  STATE_ROOT="$WORKSPACE_CASE/state" \
  ARTIFACTS_DIR="$WORKSPACE_CASE/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/clean-sandbox.sh" --workspace
)"

for machine in "$TARGET_DEFAULT_NAME" "$TARGET_CUSTOM_NAME"; do
  grep -Fqx "machine stop --name $machine" "$WORKSPACE_CASE/smolvm.log"
  grep -Fqx "machine delete --name $machine --force" "$WORKSPACE_CASE/smolvm.log"
done
if grep -Fq "$OTHER_NAME" "$WORKSPACE_CASE/smolvm.log" || \
   grep -Fq "codex-login-$BASE_ID-r4" "$WORKSPACE_CASE/smolvm.log"; then
  echo 'FAIL: workspace cleanup modified a shared or unrelated machine' >&2
  exit 1
fi
test -e "$WORKSPACE_CASE/artifacts/codex-devtools-$BASE_ID.smolmachine"
test -e "$WORKSPACE_CASE/state/base-id"
test -e "$WORKSPACE_CASE/state/machine-revision"
test -e "$WORKSPACE_CASE/state/codex-home/config.toml"
test ! -e "$WORKSPACE_CASE/state/ports/$TARGET_DEFAULT_NAME.json"
test ! -e "$WORKSPACE_CASE/state/ports/$TARGET_CUSTOM_NAME.pending.json"
grep -Fq "Removed 2 workspace VM(s) for $TARGET_WORKSPACE." <<< "$WORKSPACE_OUTPUT"

MALFORMED_NAME="codex-workspace-$BASE_ID-r4-dddddddddddd"
MALFORMED_JSON="$(workspace_json "$MALFORMED_NAME" stopped "$BASE_ID" 4 "$TARGET_DEFAULT_HASH" "$TARGET_PATH_HASH" default)"
printf '[%s,%s]\n' "$TARGET_DEFAULT_JSON" "$MALFORMED_JSON" \
  > "$WORKSPACE_CASE/inventory.json"
: > "$WORKSPACE_CASE/smolvm.log"
if PATH="$TEST_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$WORKSPACE_CASE/smolvm.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY_FILE="$WORKSPACE_CASE/inventory.json" \
  SMOLVM_TEST_ALLOW_CLEANUP=1 \
  STATE_ROOT="$WORKSPACE_CASE/state" \
  ARTIFACTS_DIR="$WORKSPACE_CASE/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/clean-sandbox.sh" --workspace "$TARGET_WORKSPACE" \
    > "$WORKSPACE_CASE/malformed.output" 2>&1
then
  echo 'FAIL: workspace cleanup accepted contradictory matching metadata' >&2
  exit 1
fi
if grep -Eq '^machine (stop|delete) ' "$WORKSPACE_CASE/smolvm.log"; then
  echo 'FAIL: workspace cleanup modified a machine before rejecting contradictory metadata' >&2
  exit 1
fi
grep -Fq 'managed labels are contradictory' "$WORKSPACE_CASE/malformed.output"

echo 'PASS: generation and workspace cleanup remove only their owned sandbox state'
