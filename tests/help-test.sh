#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

mkdir -p "$TEST_ROOT/captures" "$TEST_ROOT/lifecycle/state" \
  "$TEST_ROOT/lifecycle/artifacts" "$TEST_ROOT/lifecycle/workspace"
printf 'preserve\n' > "$TEST_ROOT/lifecycle/state/sentinel"
printf 'preserve\n' > "$TEST_ROOT/lifecycle/artifacts/sentinel"

SCRIPTS=(
  build-sandbox.sh
  build-login.sh
  start-codex.sh
  codex-sandbox.sh
  start-login.sh
  repair-sandbox.sh
  clean-sandbox.sh
  purge-sandbox.sh
)

run_help() {
  local script="$1"
  shift
  local stdout="$TEST_ROOT/captures/$script.stdout"
  local stderr="$TEST_ROOT/captures/$script.stderr"
  local status

  set +e
  PATH="$TEST_ROOT/no-tools" \
  STATE_ROOT="$TEST_ROOT/lifecycle/state" \
  ARTIFACTS_DIR="$TEST_ROOT/lifecycle/artifacts" \
  BASE_SMOLFILE="$TEST_ROOT/lifecycle/missing.smolfile" \
  SMOLVM_TEST_LOG="$TEST_ROOT/lifecycle/smolvm.log" \
    /bin/bash "$REPO_ROOT/$script" "$@" > "$stdout" 2> "$stderr"
  status=$?
  set -e

  if [ "$status" -ne 0 ]; then
    printf 'FAIL: %s help exited %s\n' "$script" "$status" >&2
    cat "$stderr" >&2
    exit 1
  fi
  if [ -s "$stderr" ]; then
    printf 'FAIL: %s help wrote to stderr\n' "$script" >&2
    cat "$stderr" >&2
    exit 1
  fi
  if [ ! -s "$stdout" ]; then
    printf 'FAIL: %s help produced no output\n' "$script" >&2
    exit 1
  fi
  if [ "$(sed -n '1p' "$stdout")" != "Usage: $script${EXPECTED_USAGE_SUFFIX[$script]}" ]; then
    printf 'FAIL: %s help has the wrong synopsis\n' "$script" >&2
    sed -n '1p' "$stdout" >&2
    exit 1
  fi
  grep -Fq 'Description:' "$stdout"
  grep -Fq 'Arguments:' "$stdout"
  grep -Fq 'Options:' "$stdout"
  grep -Fq -- '--help' "$stdout"
}

declare -A EXPECTED_USAGE_SUFFIX=(
  [build-sandbox.sh]=' [--help] [WORKSPACE]'
  [build-login.sh]=' [--help] [WORKSPACE]'
  [start-codex.sh]=' [--help] [--allow-host HOST]... [--port PORT]... [WORKSPACE] [-- CODEX_ARG...]'
  [codex-sandbox.sh]=' [--help] [--allow-host HOST]... [--port PORT]... [WORKSPACE] [-- CODEX_ARG...]'
  [start-login.sh]=' [--help]'
  [repair-sandbox.sh]=' [--help] (--check | --replace-stale [--force]) [--allow-host HOST]... [WORKSPACE]'
  [clean-sandbox.sh]=' [--help]'
  [purge-sandbox.sh]=' [--help] [--force | --force-all]'
)

for script in "${SCRIPTS[@]}"; do
  run_help "$script" --help
done

# Help takes precedence over other launcher arguments while they are being parsed.
run_help build-sandbox.sh "$TEST_ROOT/does-not-exist" extra --help
run_help build-login.sh "$TEST_ROOT/does-not-exist" extra --help
run_help start-codex.sh --allow-host --help "$TEST_ROOT/does-not-exist" extra
run_help codex-sandbox.sh --allow-host --help "$TEST_ROOT/does-not-exist" extra
run_help start-login.sh ignored --help ignored
run_help repair-sandbox.sh --check --help "$TEST_ROOT/does-not-exist" extra
run_help clean-sandbox.sh unexpected --help
run_help purge-sandbox.sh --force --help

# Each page describes its complete command-specific interface.
grep -Fq 'current directory' "$TEST_ROOT/captures/build-sandbox.sh.stdout"
grep -Fq 'Compatibility entry point' "$TEST_ROOT/captures/build-login.sh.stdout"
for script in start-codex.sh codex-sandbox.sh; do
  output="$TEST_ROOT/captures/$script.stdout"
  grep -Fq -- '--allow-host=HOST' "$output"
  grep -Fq 'Repeatable' "$output"
  grep -Fq 'before or after WORKSPACE' "$output"
  grep -Fq 'must be non-empty' "$output"
  grep -Fq -- '--port=PORT' "$output"
  grep -Fq 'same guest and host port' "$output"
  grep -Fq 'host loopback by default' "$output"
  grep -Fq 'omitting --port clears every earlier mapping' "$output"
  grep -Fq 'forwarded to Codex without interpretation' "$output"
  grep -Fq -- '-- --help requests Codex help' "$output"
done
grep -Fq 'Compatibility entry point' "$TEST_ROOT/captures/codex-sandbox.sh.stdout"
grep -Fq 'None.' "$TEST_ROOT/captures/start-login.sh.stdout"
grep -Fq 'device authentication' "$TEST_ROOT/captures/start-login.sh.stdout"
for option in --check --replace-stale --force '--allow-host=HOST'; do
  grep -Fq -- "$option" "$TEST_ROOT/captures/repair-sandbox.sh.stdout"
done
grep -Fq 'Exactly one' "$TEST_ROOT/captures/repair-sandbox.sh.stdout"
grep -Fq 'With --replace-stale only' "$TEST_ROOT/captures/repair-sandbox.sh.stdout"
grep -Fq 'workspace network allowlist used by' "$TEST_ROOT/captures/repair-sandbox.sh.stdout"
grep -Fq 'start-codex.sh' "$TEST_ROOT/captures/repair-sandbox.sh.stdout"
grep -Fq 'must be non-empty' "$TEST_ROOT/captures/repair-sandbox.sh.stdout"
grep -Fq 'shared Codex state' "$TEST_ROOT/captures/clean-sandbox.sh.stdout"
for text in --force --force-all conservative 'mutually exclusive' Running 'protected artifacts'; do
  grep -Fq -- "$text" "$TEST_ROOT/captures/purge-sandbox.sh.stdout"
done

# No help request may inspect tools or mutate lifecycle state.
test ! -e "$TEST_ROOT/lifecycle/smolvm.log"
test "$(cat "$TEST_ROOT/lifecycle/state/sentinel")" = preserve
test "$(cat "$TEST_ROOT/lifecycle/artifacts/sentinel")" = preserve
test "$(find "$TEST_ROOT/lifecycle" -mindepth 1 -printf '%P\n' | LC_ALL=C sort)" = "$(printf '%s\n' artifacts artifacts/sentinel state state/sentinel workspace | LC_ALL=C sort)"

# Invalid invocations retain their status-2 stderr path rather than becoming help.
assert_usage_error() {
  local script="$1"
  shift
  local status
  set +e
  /bin/bash "$REPO_ROOT/$script" "$@" \
    > "$TEST_ROOT/captures/invalid.stdout" \
    2> "$TEST_ROOT/captures/invalid.stderr"
  status=$?
  set -e
  if [ "$status" -ne 2 ] || [ ! -s "$TEST_ROOT/captures/invalid.stderr" ]; then
    printf 'FAIL: %s invalid invocation did not retain its status-2 stderr path\n' "$script" >&2
    exit 1
  fi
}

assert_usage_error build-sandbox.sh one two
assert_usage_error start-codex.sh --unknown
assert_usage_error repair-sandbox.sh
assert_usage_error clean-sandbox.sh unexpected
assert_usage_error purge-sandbox.sh --force --force-all

echo 'PASS: lifecycle command help is complete, side-effect-free, and alias-aware'
