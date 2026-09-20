#!/usr/bin/env bash
# Compatibility entry point for the original launcher name.
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
exec "$SCRIPT_DIR/start-codex.sh" "$@"
