#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

mkdir -p "$TEST_ROOT/bin" "$TEST_ROOT/state" "$TEST_ROOT/artifacts" "$TEST_ROOT/workspace"
cp "$REPO_ROOT/tests/fixtures/smolvm" "$TEST_ROOT/bin/smolvm"
chmod +x "$TEST_ROOT/bin/smolvm"

SMOLVM_TEST_VERSION='smolvm test 1.0.0'
BASE_ID="$({ printf '%s\n' "$SMOLVM_TEST_VERSION"; cat "$REPO_ROOT/devtools-base.smolfile"; } | sha256sum | awk '{print substr($1, 1, 12)}')"
printf 'artifact\n' > "$TEST_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"

run_start() {
  local log_name="$1" codex_home="$2"
  shift 2
  : > "$TEST_ROOT/$log_name.log"
  PATH="$TEST_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$TEST_ROOT/$log_name.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY='[]' \
  SMOLVM_TEST_ALLOW_CREATE=1 \
  SMOLVM_TEST_ALLOW_LIFECYCLE=1 \
  STATE_ROOT="$TEST_ROOT/state" \
  CODEX_HOME_DIR="$codex_home" \
  ARTIFACTS_DIR="$TEST_ROOT/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/start-codex.sh" "$@" -- --help >/dev/null 2>&1
  sed -n 's/^machine create --name \([^ ]*\).*/\1/p' "$TEST_ROOT/$log_name.log"
}

FIRST_NAME="$(run_start first "$TEST_ROOT/state/codex-home" \
  --allow-host z.example \
  "$TEST_ROOT/workspace" \
  --allow-host a.example \
  --allow-host api.openai.com)"
SECOND_NAME="$(run_start second "$TEST_ROOT/state/codex-home" \
  --allow-host=a.example \
  --allow-host z.example \
  --allow-host a.example \
  --allow-host api.openai.com \
  "$TEST_ROOT/workspace")"

test -n "$FIRST_NAME"
test "$FIRST_NAME" = "$SECOND_NAME"
for host in a.example z.example api.openai.com; do
  if [ "$(grep -o -- "--allow-host $host" "$TEST_ROOT/second.log" | wc -l)" -ne 1 ]; then
    echo "FAIL: effective allowlist did not contain exactly one $host entry" >&2
    exit 1
  fi
done

DIFFERENT_HOST_NAME="$(run_start different-host "$TEST_ROOT/state/codex-home" \
  "$TEST_ROOT/workspace" \
  --allow-host different.example)"
if [ "$DIFFERENT_HOST_NAME" = "$FIRST_NAME" ]; then
  echo 'FAIL: changing the effective allowlist did not change machine identity' >&2
  exit 1
fi

DIFFERENT_VOLUME_NAME="$(run_start different-volume "$TEST_ROOT/another-codex-home" \
  --allow-host z.example \
  --allow-host a.example \
  "$TEST_ROOT/workspace")"
if [ "$DIFFERENT_VOLUME_NAME" = "$FIRST_NAME" ]; then
  echo 'FAIL: changing a baked volume did not change machine identity' >&2
  exit 1
fi

if PATH="$TEST_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$TEST_ROOT/invalid.log" \
    "$REPO_ROOT/start-codex.sh" --allow-host >/dev/null 2>&1
then
  echo 'FAIL: a missing --allow-host value was accepted' >&2
  exit 1
fi

echo 'PASS: additional allow hosts are canonicalized and baked options determine machine identity'
