#!/usr/bin/env bash

CODEX_MACHINE="codex-sandbox"
REPO_DIR="${1:-$PWD}"
CODEX_HOME_DIR="~/.codex"
SANDBOX_SMOLFILE="codex-sandbox.smolfile"
REPO_DIR="${1:-$PWD}"
SANDBOX_DIR="${SANDBOX_DIR:-/home/mruether/projects/ai/codex-sandbox}"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-$SANDBOX_DIR/artifacts}"
CODEX_HOME_DIR="~/.codex"


# One way to start the sandbox is to create a new machine from
# the base image, and then just execute coded inside the running
# machine.
# if ! smolvm machine ls | grep -q "$CODEX_MACHINE"; then
#   smolvm machine create --name "$CODEX_MACHINE" --from "$ARTIFACTS_DIR/devtools_base.smolmachine" --volume ~/.codex:/root/.codex --volume "$REPO_DIR:/workspace/myrepo" --allow-host chatgpt.com --allow-host openai.com --allow-host api.openai.com --allow-host registry-1.docker.io --allow-host auth.docker.io --allow-host production.cloudfront.docker.com --allow-host deb.debian.org --allow-host pypi.org --allow-host files.pythonhosted.org --allow-host archive.ubuntu.com --allow-host security.ubuntu.com --allow-host googlevideo.com --allow-host youtube.com --port 5000:5000
# fi


# mkdir -p "$CODEX_HOME_DIR"

# # Just start and stop the sandbox machine, login will happen within this..
# smolvm machine start --name "$CODEX_MACHINE"
# smolvm machine exec -it --name "$CODEX_MACHINE" -- codex
# #smolvm machine stop --name "$CODEX_MACHINE"


# An alternative way to start the sandbox is to use an ephemeral machine
# that will be deleted after it is stopped.
smolvm pack run --sidecar "$ARTIFACTS_DIR/devtools_base.smolmachine" --net -it --volume ~/.codex:/root/.codex --volume "$REPO_DIR:/workspace/myrepo" -p 5000:5000 -- codex
