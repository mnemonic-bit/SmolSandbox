#!/usr/bin/env bash
# codex-sandbox — provision (once) and start the Codex smolVM (with Podman +
# Buildah + Playwright inside), then drop straight into `codex` in the guest.
#
# Usage:
#   ./codex-sandbox.sh [repo-dir]     # defaults to the current directory
#
# Requirements:
#   - smolvm installed (curl -sSL https://smolmachines.com/install.sh | bash)
#   - codex-sandbox.smolfile reachable via $SMOLFILE (default below)
#   - OPENAI_API_KEY exported in your host shell
#
# Codex session state (sessions/, history, skills, config) is persisted on the
# HOST at <repo-dir>/.smolvm/codex-home and mounted at /root/.codex in the
# guest, so it survives deleting and re-creating the machine. Make sure
# `.smolvm/` is in the repo's .gitignore — the script reminds you below.
set -euo pipefail

# to be removed after we got this to work for the first time:
OPENAI_API_KEY="${OPENAI_API_KEY:-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}"

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

# 1. One-time: register a *reference* to the host env var in smolvm's
#    secret store (the value is resolved at machine start, never stored).
if ! smolvm secret list 2>/dev/null | grep -q 'OPENAI_API_KEY'; then
  smolvm secret set OPENAI_API_KEY --from-env OPENAI_API_KEY
fi

# 2. One-time: create the machine. Run from the repo dir so the Smolfile's
#    volumes = ["./:/workspace"] binds the right project.
#    --storage sizes the ext4 disk that backs nested container storage.
if ! smolvm machine ls | grep -q "$MACHINE"; then
  (cd "$REPO_DIR" && smolvm machine create --name "$MACHINE" --smolfile "$SMOLFILE" --storage "$STORAGE_GIB")
fi

# 3. Boot (the secret is injected into the guest here) and run Codex.
smolvm machine start --name "$MACHINE"
exec smolvm machine exec --name "$MACHINE" -it -- codex

# --- Fallback if your smolvm build has no [secrets] / secret store ----------
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
#   $ podman run --rm docker.io/library/alpine echo hi   # nested containers
#   $ buildah build -t myapp docker.io/library/node:22   # no daemon needed
