#!/usr/bin/env bash
# Compatibility entry point: the unified build creates both runtime machines.
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
exec "$SCRIPT_DIR/build-sandbox.sh" "$@"
