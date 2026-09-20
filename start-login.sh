#!/usr/bin/env bash
# Authenticate Codex in a tightly allowlisted, persistent login machine.
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
STATE_ROOT="${STATE_ROOT:-$SCRIPT_DIR/.smolvm}"

if ! command -v smolvm >/dev/null 2>&1; then
  echo "ERROR: smolvm is not installed or is not on PATH." >&2
  exit 1
fi

if [ ! -s "$STATE_ROOT/base-id" ]; then
  echo "ERROR: no built base generation was found." >&2
  echo "Run $SCRIPT_DIR/build-sandbox.sh first." >&2
  exit 1
fi

BASE_ID="$(sed -n '1p' "$STATE_ROOT/base-id")"
MACHINE_REVISION="$(sed -n '1p' "$STATE_ROOT/machine-revision" 2>/dev/null || printf '1\n')"
LOGIN_MACHINE="${LOGIN_MACHINE:-codex-login-$BASE_ID-r$MACHINE_REVISION}"

if ! smolvm machine status --name "$LOGIN_MACHINE" >/dev/null 2>&1; then
  echo "ERROR: login machine $LOGIN_MACHINE does not exist." >&2
  echo "Run $SCRIPT_DIR/build-sandbox.sh first." >&2
  exit 1
fi

stop_login_machine() {
  smolvm machine stop --name "$LOGIN_MACHINE" >/dev/null 2>&1 || true
}
trap stop_login_machine EXIT INT TERM

smolvm machine start --name "$LOGIN_MACHINE"
smolvm machine exec -it --name "$LOGIN_MACHINE" -- sh -lc \
  'if [ -d /opt/codex-home ]; then cp -a -n /opt/codex-home/. /root/.codex/; fi; exec codex login --device-auth'
