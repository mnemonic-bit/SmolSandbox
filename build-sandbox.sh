#!/usr/bin/env bash
# Build the prepared Codex base once, then create persistent machines from it.
#
# Usage:
#   ./build-sandbox.sh [workspace]
#
# The workspace defaults to the directory from which this script is invoked.
# Nothing is deleted on rebuild: the Smolfile and smolvm version form a cache
# key, so a changed base creates a new artifact and a new generation of VMs.
set -euo pipefail

if [ "$#" -gt 1 ]; then
  echo "Usage: $0 [workspace]" >&2
  exit 2
fi

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
BASE_SMOLFILE="${BASE_SMOLFILE:-$SCRIPT_DIR/devtools-base.smolfile}"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-$SCRIPT_DIR/artifacts}"
STATE_ROOT="${STATE_ROOT:-$SCRIPT_DIR/.smolvm}"
CODEX_HOME_DIR="${CODEX_HOME_DIR:-$STATE_ROOT/codex-home}"
WORKSPACE_INPUT="${1:-$PWD}"
# Bump this when changing mounts, ports, or runtime egress policy. The prepared
# base artifact remains reusable, while derived machines get a new identity.
MACHINE_REVISION=1

if ! command -v smolvm >/dev/null 2>&1; then
  echo "ERROR: smolvm is not installed or is not on PATH." >&2
  echo "Install it with: curl -sSL https://smolmachines.com/install.sh | bash" >&2
  exit 1
fi

if [ ! -f "$BASE_SMOLFILE" ]; then
  echo "ERROR: base Smolfile not found: $BASE_SMOLFILE" >&2
  exit 1
fi

if [ ! -d "$WORKSPACE_INPUT" ]; then
  echo "ERROR: workspace is not a directory: $WORKSPACE_INPUT" >&2
  exit 1
fi

WORKSPACE_DIR="$(CDPATH= cd -- "$WORKSPACE_INPUT" && pwd -P)"
SMOLVM_VERSION="$(smolvm --version)"

hash_stream() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  else
    shasum -a 256 | awk '{print $1}'
  fi
}

machine_exists() {
  smolvm machine status --name "$1" >/dev/null 2>&1
}

# Including the runtime version prevents an artifact made by an incompatible
# smolvm/guest-agent pair from being silently reused after an upgrade.
BASE_ID="$({ printf '%s\n' "$SMOLVM_VERSION"; command cat "$BASE_SMOLFILE"; } | hash_stream | cut -c1-12)"
WORKSPACE_ID="$(printf '%s' "$WORKSPACE_DIR" | hash_stream | cut -c1-12)"

BUILDER_MACHINE="${BUILDER_MACHINE:-codex-builder-$BASE_ID}"
LOGIN_MACHINE="${LOGIN_MACHINE:-codex-login-$BASE_ID-r$MACHINE_REVISION}"
CODEX_MACHINE="${CODEX_MACHINE:-codex-workspace-$BASE_ID-r$MACHINE_REVISION-$WORKSPACE_ID}"
ARTIFACT_BASE="$ARTIFACTS_DIR/codex-devtools-$BASE_ID"
ARTIFACT_FILE="$ARTIFACT_BASE.smolmachine"

mkdir -p "$ARTIFACTS_DIR" "$CODEX_HOME_DIR"

if [ ! -s "$ARTIFACT_FILE" ]; then
  echo "Building Codex base generation $BASE_ID ($SMOLVM_VERSION)..."

  if ! machine_exists "$BUILDER_MACHINE"; then
    # SmolVM caches the OCI image and the Smolfile init layer. Keeping this
    # named builder also makes repacking a missing artifact inexpensive.
    smolvm machine create \
      --name "$BUILDER_MACHINE" \
      --smolfile "$BASE_SMOLFILE"
  fi

  # A previous interrupted run may have left the builder running.
  smolvm machine stop --name "$BUILDER_MACHINE" >/dev/null 2>&1 || true
  smolvm machine start --name "$BUILDER_MACHINE"
  smolvm machine stop --name "$BUILDER_MACHINE"

  # Pack to a temporary basename so an interrupted build is never mistaken
  # for a complete cached artifact.
  TMP_ARTIFACT_BASE="$ARTIFACTS_DIR/.codex-devtools-$BASE_ID-$$"
  cleanup_tmp_artifact() {
    rm -f "$TMP_ARTIFACT_BASE" "$TMP_ARTIFACT_BASE.smolmachine"
  }
  trap cleanup_tmp_artifact EXIT
  smolvm pack create --from-vm "$BUILDER_MACHINE" -o "$TMP_ARTIFACT_BASE"
  mv "$TMP_ARTIFACT_BASE.smolmachine" "$ARTIFACT_FILE"
  if [ -f "$TMP_ARTIFACT_BASE" ]; then
    mv "$TMP_ARTIFACT_BASE" "$ARTIFACT_BASE"
  fi
  trap - EXIT
else
  echo "Reusing cached Codex base generation $BASE_ID."
fi

if ! machine_exists "$LOGIN_MACHINE"; then
  echo "Creating login-only machine $LOGIN_MACHINE..."
  smolvm machine create \
    --name "$LOGIN_MACHINE" \
    --from "$ARTIFACT_FILE" \
    --volume "$CODEX_HOME_DIR:/root/.codex" \
    --allow-host auth.openai.com \
    --allow-host chatgpt.com \
    --allow-host api.openai.com
fi

if ! machine_exists "$CODEX_MACHINE"; then
  echo "Creating workspace machine $CODEX_MACHINE..."
  smolvm machine create \
    --name "$CODEX_MACHINE" \
    --from "$ARTIFACT_FILE" \
    --volume "$CODEX_HOME_DIR:/root/.codex" \
    --volume "$WORKSPACE_DIR:/workspace/myrepo" \
    --allow-host api.openai.com \
    --allow-host auth.openai.com \
    --allow-host chatgpt.com \
    --allow-host github.com \
    --allow-host api.github.com \
    --allow-host raw.githubusercontent.com \
    --allow-host codeload.github.com \
    --allow-host objects.githubusercontent.com \
    --allow-host release-assets.githubusercontent.com \
    --allow-host registry.npmjs.org \
    --allow-host api.nuget.org \
    --allow-host globalcdn.nuget.org \
    --allow-host pypi.org \
    --allow-host files.pythonhosted.org \
    --allow-host registry-1.docker.io \
    --allow-host auth.docker.io \
    --allow-host production.cloudflare.docker.com \
    --allow-host mcr.microsoft.com \
    --port 3000:3000 \
    --port 5000:5000 \
    --port 5173:5173 \
    --port 8000:8000 \
    --port 8080:8080
fi

# Launchers read this pointer instead of guessing which definition-addressed
# generation is current.
printf '%s\n' "$BASE_ID" > "$STATE_ROOT/base-id"
printf '%s\n' "$MACHINE_REVISION" > "$STATE_ROOT/machine-revision"

echo
echo "Ready:"
echo "  artifact:  $ARTIFACT_FILE"
echo "  login VM:  $LOGIN_MACHINE"
echo "  Codex VM:  $CODEX_MACHINE"
echo "  workspace: $WORKSPACE_DIR"
echo
echo "Next: run $SCRIPT_DIR/start-login.sh once, then $SCRIPT_DIR/start-codex.sh '$WORKSPACE_DIR'"
