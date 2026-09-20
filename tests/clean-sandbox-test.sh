#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

BASE_ID=0123456789ab
OTHER_BASE_ID=fedcba987654

mkdir -p \
  "$TEST_ROOT/bin" \
  "$TEST_ROOT/state/codex-home" \
  "$TEST_ROOT/artifacts"
cp "$REPO_ROOT/tests/fixtures/smolvm" "$TEST_ROOT/bin/smolvm"
chmod +x "$TEST_ROOT/bin/smolvm"

printf '%s\n' "$BASE_ID" > "$TEST_ROOT/state/base-id"
printf '1\n' > "$TEST_ROOT/state/machine-revision"
printf 'keep\n' > "$TEST_ROOT/state/codex-home/config.toml"
printf 'artifact\n' > "$TEST_ROOT/artifacts/codex-devtools-$BASE_ID"
printf 'artifact\n' > "$TEST_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"
printf 'keep\n' > "$TEST_ROOT/artifacts/codex-devtools-$OTHER_BASE_ID.smolmachine"

MACHINES="$(printf '%s\n' \
  "codex-builder-$BASE_ID" \
  "codex-login-$BASE_ID-r1" \
  "codex-login-$BASE_ID-r2" \
  "codex-workspace-$BASE_ID-r1-workspace-one" \
  "codex-workspace-$BASE_ID-r2-workspace-two" \
  "codex-builder-$OTHER_BASE_ID" \
  "unrelated-machine")"

OUTPUT="$(
  PATH="$TEST_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$TEST_ROOT/smolvm.log" \
  SMOLVM_TEST_MACHINES="$MACHINES" \
  SMOLVM_TEST_ALLOW_CLEANUP=1 \
  STATE_ROOT="$TEST_ROOT/state" \
  ARTIFACTS_DIR="$TEST_ROOT/artifacts" \
    "$REPO_ROOT/clean-sandbox.sh"
)"

for machine in \
  "codex-builder-$BASE_ID" \
  "codex-login-$BASE_ID-r1" \
  "codex-login-$BASE_ID-r2" \
  "codex-workspace-$BASE_ID-r1-workspace-one" \
  "codex-workspace-$BASE_ID-r2-workspace-two"
do
  if ! grep -Fqx "machine delete --name $machine" "$TEST_ROOT/smolvm.log"; then
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

echo 'PASS: current generation is cleaned without touching other state'
