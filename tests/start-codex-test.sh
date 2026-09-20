#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

mkdir -p "$TEST_ROOT/bin" "$TEST_ROOT/state" "$TEST_ROOT/artifacts" "$TEST_ROOT/workspace"
cp "$REPO_ROOT/tests/fixtures/smolvm" "$TEST_ROOT/bin/smolvm"
chmod +x "$TEST_ROOT/bin/smolvm"
printf 'test-base\n' > "$TEST_ROOT/state/base-id"
printf '1\n' > "$TEST_ROOT/state/machine-revision"
printf 'artifact\n' > "$TEST_ROOT/artifacts/codex-devtools-test-base.smolmachine"

set +e
OUTPUT="$(
  PATH="$TEST_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$TEST_ROOT/smolvm.log" \
  STATE_ROOT="$TEST_ROOT/state" \
  ARTIFACTS_DIR="$TEST_ROOT/artifacts" \
  CODEX_MACHINE="test-machine" \
    "$REPO_ROOT/start-codex.sh" "$TEST_ROOT/workspace" -- --help 2>&1
)"
STATUS=$?
set -e

if [ "$STATUS" -ne 0 ]; then
  printf 'FAIL: launcher exited %s after the Codex-home seed failed\n%s\n' "$STATUS" "$OUTPUT" >&2
  exit 1
fi

if ! grep -Fq 'machine exec --name test-machine -it -- codex --help' "$TEST_ROOT/smolvm.log"; then
  printf 'FAIL: launcher never invoked Codex\n%s\n' "$OUTPUT" >&2
  exit 1
fi

if ! grep -Fq 'machine stop --name test-machine' "$TEST_ROOT/smolvm.log"; then
  echo 'FAIL: launcher did not stop the machine after Codex exited' >&2
  exit 1
fi

echo 'PASS: a Codex-home seed failure does not prevent Codex from starting'
