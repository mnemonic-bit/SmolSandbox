#!/usr/bin/env bash
# codex-sandbox — provision (once) and start the Codex smolVM (with Podman +
# Buildah + Playwright inside), then drop straight into `codex` in the guest.
#
# Usage:
#   OPENAI_API_KEY=sk-... ./codex-sandbox.sh [repo-dir]
#
# Requirements:
#   - smolvm installed (curl -sSL https://smolmachines.com/install.sh | bash)
#   - codex-sandbox.smolfile reachable via $SMOLFILE (default below)
#   - OPENAI_API_KEY exported in your host shell (the Smolfile's
#     [secrets] from_env entry resolves it into the guest at machine start);
#     OR sign in once inside the guest with: codex login --device-auth
#
# Codex session state (sessions/, history, skills, config) is persisted on the
# HOST at <repo-dir>/.smolvm/codex-home and mounted at /root/.codex in the
# guest, so it survives deleting and re-creating the machine. Make sure
# `.smolvm/` is in the repo's .gitignore — the script reminds you below.
set -euo pipefail

MACHINE="codex-sandbox"
SMOLFILE="${SMOLFILE:-$HOME/.config/smolvm/codex-sandbox.smolfile}"
REPO_DIR="${1:-$PWD}"
# Size (GiB) of the ext4 storage disk. Podman/Buildah image layers live here
# (/storage/containers), so size it for your biggest builds.
STORAGE_GIB="${STORAGE_GIB:-20}"
STATE_DIR="$REPO_DIR/.smolvm/codex-home"

# 0. Prepare the host-side Codex state dir (mounted at /root/.codex).
mkdir -p "$STATE_DIR"
if [ -f "$REPO_DIR/.gitignore" ]; then
  grep -qxF '.smolvm/' "$REPO_DIR/.gitignore" || \
    echo "NOTE: '.smolvm/' is not in $REPO_DIR/.gitignore — Codex session state may get committed." >&2
fi

# 1. Fail fast if the API key is missing — [secrets] from_env resolves it
#    from the HOST environment at machine start, so it must be exported.
#    (Skip this check if you authenticate with `codex login --device-auth`
#    inside the guest instead — adjust to taste.)
if [ -z "${OPENAI_API_KEY:-}" ]; then
  echo "ERROR: OPENAI_API_KEY is not set. Export it first, e.g.:" >&2
  echo "  export OPENAI_API_KEY=sk-..." >&2
  exit 1
fi

# 2. One-time: create the machine. Run from the repo dir so the Smolfile's
#    volumes = ["./:/workspace/myrepo"] binds the right project (mounted one
#    level below /workspace so codebase-memory-mcp doesn't treat the mount
#    root as "too broad" to index).
#    --storage sizes the ext4 disk that backs nested container storage.
if ! smolvm machine ls | grep -q "$MACHINE"; then
  (cd "$REPO_DIR" && smolvm machine create --name "$MACHINE" --smolfile "$SMOLFILE" --storage "$STORAGE_GIB")
fi

# 3. Boot (OPENAI_API_KEY is resolved into the guest from the host env here)
#    and run Codex.
smolvm machine start --name "$MACHINE"
exec smolvm machine exec --name "$MACHINE" -it -- codex

# --- Fallback if your smolvm build has no [secrets] support at all ---------
# Replace step 3 with (key appears in the host process list while attached):
#   smolvm machine start --name "$MACHINE"
#   exec smolvm machine exec --name "$MACHINE" -it \
#     --env OPENAI_API_KEY="$OPENAI_API_KEY" -- codex
#
# Or just open a shell and start Codex yourself:
#   smolvm machine start --name "$MACHINE"
#   smolvm machine shell --name "$MACHINE"
#   $ codex                      # sessions live in /root/.codex/sessions
#   $ codex resume               # pick up where a previous session left off
#   $ codex login --device-auth  # ChatGPT subscription sign-in (once, persists)
#   $ podman run --rm docker.io/library/alpine echo hi   # nested containers
#   $ buildah build -t myapp docker.io/library/node:22   # no daemon needed
