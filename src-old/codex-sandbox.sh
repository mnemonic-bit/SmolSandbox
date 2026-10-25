#!/usr/bin/env bash
# Compatibility entry point for the original launcher name.
set -euo pipefail

show_help() {
  printf '%s\n' \
    "Usage: ${0##*/} [--help] [--allow-host HOST]... [--port PORT]... [WORKSPACE] [-- CODEX_ARG...]" \
    "" \
    "Description:" \
    "  Compatibility entry point for start-codex.sh. It starts Codex in the" \
    "  persistent sandbox machine for a workspace." \
    "" \
    "Arguments:" \
    "  WORKSPACE     Workspace mounted in the sandbox (default: current directory)." \
    "  CODEX_ARG...  Arguments after -- are forwarded to Codex without interpretation." \
    "" \
    "Options:" \
    "  --allow-host HOST" \
    "  --allow-host=HOST  Add HOST to the default network allowlist. Repeatable; may" \
    "                     appear before or after WORKSPACE, but not after --. HOST" \
    "                     must be non-empty, contain no line breaks, and not start with -." \
    "  --port PORT" \
    "  --port=PORT        Publish repeatable TCP PORT on the same guest and host port." \
    "                     Ports are 1-65535, bind to host loopback by default, and may" \
    "                     appear before or after WORKSPACE. The exact set remains on the" \
    "                     selected machine until a later launch reconciles another set;" \
    "                     omitting --port clears every earlier mapping on this launch." \
    "  --help             Display launcher help and exit." \
    "  --                 End sandbox option parsing. All remaining options belong" \
    "                     to Codex; for example, -- --help requests Codex help."
}

for argument in "$@"; do
  if [ "$argument" = -- ]; then
    break
  fi
  if [ "$argument" = --help ]; then
    show_help
    exit 0
  fi
done

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
exec "$SCRIPT_DIR/start-codex.sh" "$@"
