#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=../sandbox-common.sh
source "$REPO_ROOT/sandbox-common.sh"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

mkdir -p "$TEST_ROOT/bin" "$TEST_ROOT/state" "$TEST_ROOT/artifacts" "$TEST_ROOT/workspace"
cp "$REPO_ROOT/tests/fixtures/smolvm" "$TEST_ROOT/bin/smolvm"
chmod +x "$TEST_ROOT/bin/smolvm"

SMOLVM_TEST_VERSION='smolvm test 1.19.2'
BASE_ID="$({ printf '%s\n' "$SMOLVM_TEST_VERSION"; cat "$REPO_ROOT/devtools-base.smolfile"; } | sha256sum | awk '{print substr($1, 1, 12)}')"
printf 'artifact\n' > "$TEST_ROOT/artifacts/codex-devtools-$BASE_ID.smolmachine"
printf '[]\n' > "$TEST_ROOT/inventory.json"

run_launcher() {
  local log="$1"
  shift
  : > "$TEST_ROOT/$log.log"
  PATH="$TEST_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$TEST_ROOT/$log.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY_FILE="$TEST_ROOT/inventory.json" \
  SMOLVM_TEST_ALLOW_CREATE=1 \
  SMOLVM_TEST_ALLOW_LIFECYCLE=1 \
  STATE_ROOT="$TEST_ROOT/state" \
  ARTIFACTS_DIR="$TEST_ROOT/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/start-codex.sh" "$@"
}

inventory_value() {
  python3 - "$TEST_ROOT/inventory.json" "$1" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as inventory_file:
    machine = json.load(inventory_file)[0]
print(json.dumps(machine[sys.argv[2]], separators=(",", ":")))
PY
}

assert_no_lifecycle_mutation() {
  local log="$1"
  if grep -Eq '^machine (create|start|stop|delete) |^machine update --name ' "$log"; then
    echo "FAIL: unexpected lifecycle mutation in $log" >&2
    exit 1
  fi
}

# Canonicalize, create without baked mappings, update, verify, and start.
run_launcher first --port 05173 "$TEST_ROOT/workspace" --port=3000 --port 3000 -- --help >/dev/null
MACHINE="$(python3 - "$TEST_ROOT/inventory.json" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as inventory_file:
    print(json.load(inventory_file)[0]["name"])
PY
)"
test "$(inventory_value _test_ports)" = '[3000,5173]'
grep -Fq "machine update --name $MACHINE --remove-port 3000:3000 --remove-port 5173:5173 --port 3000 --port 5173" "$TEST_ROOT/first.log"
create_line="$(grep -F "machine create --name $MACHINE" "$TEST_ROOT/first.log")"
if [[ "$create_line" == *'--port'* ]]; then
  echo 'FAIL: requested ports were baked into machine creation' >&2
  exit 1
fi
MANIFEST="$TEST_ROOT/state/ports/$MACHINE.json"
PENDING="$TEST_ROOT/state/ports/$MACHINE.pending.json"
test -f "$MANIFEST"
test ! -e "$PENDING"
python3 - "$TEST_ROOT/inventory.json" <<'PY'
import json
import sys
path = sys.argv[1]
with open(path, encoding="utf-8") as inventory_file:
    inventory = json.load(inventory_file)
inventory[0]["overlay_marker"] = "persistent"
with open(path, "w", encoding="utf-8") as inventory_file:
    json.dump(inventory, inventory_file)
PY

# A changed set reuses the machine and resets every known candidate.
run_launcher changed --port 8000 "$TEST_ROOT/workspace" --port=5173 -- --help >/dev/null
test "$(inventory_value _test_ports)" = '[5173,8000]'
test "$(inventory_value overlay_marker)" = '"persistent"'
test "$(grep -Fc "machine create --name $MACHINE" "$TEST_ROOT/changed.log")" -eq 0
grep -Fq "machine update --name $MACHINE --remove-port 3000:3000 --remove-port 5173:5173 --remove-port 8000:8000 --port 5173 --port 8000" "$TEST_ROOT/changed.log"

# The empty request is authoritative and clears prior mappings.
run_launcher empty "$TEST_ROOT/workspace" -- --help >/dev/null
test "$(inventory_value _test_ports)" = '[]'
grep -Fq "machine update --name $MACHINE --remove-port 5173:5173 --remove-port 8000:8000" "$TEST_ROOT/empty.log"

# Everything after -- belongs to Codex and does not publish a port.
run_launcher delimiter "$TEST_ROOT/workspace" -- --port 3000 >/dev/null
test "$(inventory_value _test_ports)" = '[]'
if grep -Fq 'machine update --name' "$TEST_ROOT/delimiter.log"; then
  echo 'FAIL: --port after the delimiter was interpreted by the launcher' >&2
  exit 1
fi
grep -Fq -- '-- codex --no-daemon --port 3000' "$TEST_ROOT/delimiter.log"

# The upstream maximum of 64 distinct ports is accepted.
sixty_four=()
for ((port = 10001; port <= 10064; port++)); do
  sixty_four+=(--port "$port")
done
run_launcher maximum "${sixty_four[@]}" "$TEST_ROOT/workspace" -- --help >/dev/null
test "$(inventory_value ports)" = '64'
run_launcher clear-maximum "$TEST_ROOT/workspace" -- --help >/dev/null

# Invalid values and the distinct mapping limit fail before touching SmolVM.
for value in '' +1 -1 ' 1' '1 ' 0 65536 1:1 1-2 1/tcp; do
  : > "$TEST_ROOT/invalid.log"
  set +e
  PATH="$TEST_ROOT/bin:$PATH" SMOLVM_TEST_LOG="$TEST_ROOT/invalid.log" \
    "$REPO_ROOT/start-codex.sh" --port="$value" > /dev/null 2>&1
  status=$?
  set -e
  if [ "$status" -ne 2 ] || [ -s "$TEST_ROOT/invalid.log" ]; then
    echo "FAIL: invalid port value '$value' did not fail before lifecycle work" >&2
    exit 1
  fi
done
too_many=()
for ((port = 1; port <= 65; port++)); do
  too_many+=(--port "$port")
done
: > "$TEST_ROOT/invalid.log"
set +e
PATH="$TEST_ROOT/bin:$PATH" SMOLVM_TEST_LOG="$TEST_ROOT/invalid.log" \
  "$REPO_ROOT/start-codex.sh" "${too_many[@]}" > /dev/null 2>&1
status=$?
set -e
test "$status" -eq 2
test ! -s "$TEST_ROOT/invalid.log"

# Capability refusal happens before inventory or lifecycle mutation.
: > "$TEST_ROOT/incompatible.log"
set +e
SMOLVM_TEST_INCOMPATIBLE_UPDATE=1 run_launcher incompatible "$TEST_ROOT/workspace" -- --help \
  > "$TEST_ROOT/incompatible.output" 2>&1
status=$?
set -e
test "$status" -ne 0
grep -Fq 'installed SmolVM is incompatible' "$TEST_ROOT/incompatible.output"
if grep -Fq 'machine ls --json' "$TEST_ROOT/incompatible.log"; then
  echo 'FAIL: incompatible SmolVM reached inventory inspection' >&2
  exit 1
fi

# Malformed public mapping metadata fails closed before any lifecycle mutation.
python3 - "$TEST_ROOT/inventory.json" <<'PY'
import json
import sys
path = sys.argv[1]
with open(path, encoding="utf-8") as inventory_file:
    inventory = json.load(inventory_file)
inventory[0].pop("_test_ports", None)
inventory[0]["ports"] = "not-a-count"
with open(path, "w", encoding="utf-8") as inventory_file:
    json.dump(inventory, inventory_file)
PY
set +e
run_launcher malformed "$TEST_ROOT/workspace" --port 4000 -- --help > /dev/null 2>&1
status=$?
set -e
test "$status" -ne 0
assert_no_lifecycle_mutation "$TEST_ROOT/malformed.log"
python3 - "$TEST_ROOT/inventory.json" <<'PY'
import json
import sys
path = sys.argv[1]
with open(path, encoding="utf-8") as inventory_file:
    inventory = json.load(inventory_file)
inventory[0]["_test_ports"] = []
inventory[0]["ports"] = 0
with open(path, "w", encoding="utf-8") as inventory_file:
    json.dump(inventory, inventory_file)
PY

# A positive unknown set cannot be adopted without a matching manifest.
python3 - "$TEST_ROOT/inventory.json" <<'PY'
import json
import sys
path = sys.argv[1]
with open(path, encoding="utf-8") as inventory_file:
    inventory = json.load(inventory_file)
inventory[0]["_test_ports"] = [4444]
inventory[0]["ports"] = 1
with open(path, "w", encoding="utf-8") as inventory_file:
    json.dump(inventory, inventory_file)
PY
rm -f -- "$MANIFEST" "$PENDING"
set +e
run_launcher unknown "$TEST_ROOT/workspace" --port 4444 -- --help > "$TEST_ROOT/unknown.output" 2>&1
status=$?
set -e
test "$status" -ne 0
grep -Fq 'smolvm machine ls --verbose' "$TEST_ROOT/unknown.output"
assert_no_lifecycle_mutation "$TEST_ROOT/unknown.log"

# A zero-port new incarnation safely replaces stale local state.
python3 - "$TEST_ROOT/inventory.json" <<'PY'
import json
import sys
path = sys.argv[1]
with open(path, encoding="utf-8") as inventory_file:
    inventory = json.load(inventory_file)
inventory[0]["_test_ports"] = []
inventory[0]["ports"] = 0
inventory[0]["created_at"] = 2
with open(path, "w", encoding="utf-8") as inventory_file:
    json.dump(inventory, inventory_file)
PY
printf '%s\n' '{"version":1,"machine":"wrong","created_at":1,"workspace_sha256":"bad","ports":[4444]}' > "$MANIFEST"
run_launcher reincarnated "$TEST_ROOT/workspace" --port 5000 -- --help >/dev/null
test "$(inventory_value _test_ports)" = '[5000]'

# An interrupted transactional update leaves a journal that the next run uses.
set +e
SMOLVM_TEST_UPDATE_FAIL_AFTER=1 run_launcher interrupted "$TEST_ROOT/workspace" --port 6000 -- --help \
  > "$TEST_ROOT/interrupted.output" 2>&1
status=$?
set -e
test "$status" -ne 0
test -f "$PENDING"
test "$(inventory_value _test_ports)" = '[6000]'
run_launcher recovered "$TEST_ROOT/workspace" --port 7000 -- --help >/dev/null
test "$(inventory_value _test_ports)" = '[7000]'
grep -Fq -- '--remove-port 5000:5000' "$TEST_ROOT/recovered.log"
grep -Fq -- '--remove-port 6000:6000' "$TEST_ROOT/recovered.log"
grep -Fq -- '--remove-port 7000:7000' "$TEST_ROOT/recovered.log"
test ! -e "$PENDING"

# Update and verification failures never start the machine or run Codex.
set +e
SMOLVM_TEST_UPDATE_FAIL=1 run_launcher update-fail "$TEST_ROOT/workspace" --port 7100 -- --help \
  > /dev/null 2>&1
status=$?
set -e
test "$status" -ne 0
test -f "$PENDING"
if grep -Eq '^machine (start|exec) ' "$TEST_ROOT/update-fail.log"; then
  echo 'FAIL: update failure proceeded to start or guest execution' >&2
  exit 1
fi
set +e
SMOLVM_TEST_UPDATE_WRONG_COUNT=1 run_launcher verify-fail "$TEST_ROOT/workspace" --port 7200 -- --help \
  > /dev/null 2>&1
status=$?
set -e
test "$status" -ne 0
if grep -Eq '^machine (start|exec) ' "$TEST_ROOT/verify-fail.log"; then
  echo 'FAIL: post-update verification failure proceeded to start or guest execution' >&2
  exit 1
fi
python3 - "$TEST_ROOT/inventory.json" <<'PY'
import json
import sys
path = sys.argv[1]
with open(path, encoding="utf-8") as inventory_file:
    inventory = json.load(inventory_file)
inventory[0]["_test_ports"] = [7200]
inventory[0]["ports"] = 1
with open(path, "w", encoding="utf-8") as inventory_file:
    json.dump(inventory, inventory_file)
PY

# Recover from the retained journal, then accept only an exact running set.
run_launcher settle "$TEST_ROOT/workspace" --port 7300 -- --help >/dev/null

# The workspace lock keeps an update and its following start together.
: > "$TEST_ROOT/concurrent.log"
run_concurrent() {
  PATH="$TEST_ROOT/bin:$PATH" \
  SMOLVM_TEST_LOG="$TEST_ROOT/concurrent.log" \
  SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
  SMOLVM_TEST_INVENTORY_FILE="$TEST_ROOT/inventory.json" \
  SMOLVM_TEST_ALLOW_CREATE=1 \
  SMOLVM_TEST_ALLOW_LIFECYCLE=1 \
  STATE_ROOT="$TEST_ROOT/state" \
  ARTIFACTS_DIR="$TEST_ROOT/artifacts" \
  BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
    "$REPO_ROOT/start-codex.sh" --port 7600 "$TEST_ROOT/workspace" -- --help \
      >/dev/null 2>&1
}
run_concurrent & first_pid=$!
run_concurrent & second_pid=$!
wait "$first_pid"
wait "$second_pid"
first_update="$(grep -n -m1 '^machine update --name ' "$TEST_ROOT/concurrent.log" | cut -d: -f1)"
first_start="$(grep -n -m1 '^machine start --name ' "$TEST_ROOT/concurrent.log" | cut -d: -f1)"
test -n "$first_update"
test -n "$first_start"
test "$first_update" -lt "$first_start"
run_launcher settle-after-concurrent "$TEST_ROOT/workspace" --port 7300 -- --help >/dev/null
python3 - "$TEST_ROOT/inventory.json" <<'PY'
import json
import sys
path = sys.argv[1]
with open(path, encoding="utf-8") as inventory_file:
    inventory = json.load(inventory_file)
inventory[0]["state"] = "running"
with open(path, "w", encoding="utf-8") as inventory_file:
    json.dump(inventory, inventory_file)
PY
run_launcher running-match "$TEST_ROOT/workspace" --port 7300 -- --help >/dev/null
if grep -Eq '^machine (start|stop) |^machine update --name ' "$TEST_ROOT/running-match.log"; then
  echo 'FAIL: matching running machine was mutated' >&2
  exit 1
fi
set +e
run_launcher running-mismatch "$TEST_ROOT/workspace" --port 7400 -- --help \
  > "$TEST_ROOT/running-mismatch.output" 2>&1
status=$?
set -e
test "$status" -ne 0
grep -Fq 'stop the active session before retrying' "$TEST_ROOT/running-mismatch.output"
if grep -Eq '^machine (start|stop) |^machine update --name ' "$TEST_ROOT/running-mismatch.log"; then
  echo 'FAIL: mismatching running machine was mutated' >&2
  exit 1
fi

# A bind/start failure prevents guest execution and invokes stopped-state cleanup.
python3 - "$TEST_ROOT/inventory.json" <<'PY'
import json
import sys
path = sys.argv[1]
with open(path, encoding="utf-8") as inventory_file:
    inventory = json.load(inventory_file)
inventory[0]["state"] = "stopped"
with open(path, "w", encoding="utf-8") as inventory_file:
    json.dump(inventory, inventory_file)
PY
set +e
SMOLVM_TEST_START_FAIL=1 run_launcher collision "$TEST_ROOT/workspace" --port 7300 -- --help \
  > /dev/null 2>&1
status=$?
set -e
test "$status" -ne 0
grep -Fq "machine stop --name $MACHINE" "$TEST_ROOT/collision.log"
if grep -Fq 'machine exec' "$TEST_ROOT/collision.log"; then
  echo 'FAIL: start failure proceeded to guest execution' >&2
  exit 1
fi

# Successful machine deletion also removes both forms of launcher-owned state.
printf 'pending\n' > "$PENDING"
PATH="$TEST_ROOT/bin:$PATH" \
SMOLVM_TEST_LOG="$TEST_ROOT/clean.log" \
SMOLVM_TEST_VERSION="$SMOLVM_TEST_VERSION" \
SMOLVM_TEST_INVENTORY_FILE="$TEST_ROOT/inventory.json" \
SMOLVM_TEST_ALLOW_CLEANUP=1 \
STATE_ROOT="$TEST_ROOT/state" \
ARTIFACTS_DIR="$TEST_ROOT/artifacts" \
BASE_SMOLFILE="$REPO_ROOT/devtools-base.smolfile" \
  "$REPO_ROOT/clean-sandbox.sh" >/dev/null
test ! -e "$MANIFEST"
test ! -e "$PENDING"

echo 'PASS: published ports are canonical, exact, recoverable, mutable, and fail closed'
