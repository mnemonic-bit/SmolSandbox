#!/usr/bin/env bash
# Authenticate Codex in a tightly allowlisted, persistent login machine.
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=sandbox-common.sh
source "$SCRIPT_DIR/sandbox-common.sh"

sandbox_prepare_identity "$PWD"
sandbox_read_inventory
if ! sandbox_inventory_has_name "$LOGIN_MACHINE"; then
  sandbox_discard_inventory
  echo "ERROR: login machine $LOGIN_MACHINE does not exist." >&2
  echo "Run $SCRIPT_DIR/build-sandbox.sh first." >&2
  exit 1
fi
sandbox_discard_inventory

stop_login_machine() {
  smolvm machine stop --name "$LOGIN_MACHINE" >/dev/null 2>&1 || true
}
trap stop_login_machine EXIT INT TERM

smolvm machine start --name "$LOGIN_MACHINE"
smolvm machine exec -it --name "$LOGIN_MACHINE" -- sh -lc \
  'if [ -d /opt/codex-home ]; then cp -a -n /opt/codex-home/. /root/.codex/; fi; exec codex login --device-auth'
