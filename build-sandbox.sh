#!/usr/bin/env bash

# Builds the codex-sandbox.smolfile into a smolVM image.

BASE_MACHINE="devtools-base"
LOGIN_MACHINE="codex-login"
SANDBOX_MACHINE="codex-sandbox"
BASE_SMOLFILE="devtools-base.smolfile"
SANDBOX_SMOLFILE="codex-sandbox.smolfile"
REPO_DIR="${1:-$PWD}"
TMP_DIR="/tmp/smolvm-build-sandbox"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-$REPO_DIR/artifacts}"
CODEX_HOME_DIR="~/.codex"

if smolvm machine ls | grep -q "$BASE_MACHINE"; then
  (cd "$REPO_DIR" && smolvm machine delete --name "$BASE_MACHINE" --force)
fi

mkdir -p "$ARTIFACTS_DIR"
mkdir -p "$TMP_DIR"
mkdir -p "$CODEX_HOME_DIR"

# Create a base image with the development tools installed.
smolvm machine create --name "$BASE_MACHINE" --smolfile "$BASE_SMOLFILE" --storage 20
smolvm machine start --name "$BASE_MACHINE"
smolvm machine stop --name "$BASE_MACHINE"

smolvm pack create --from-vm "$BASE_MACHINE" -o "$ARTIFACTS_DIR/devtools_base"

# One of these is what I need:
#smolvm pack run --sidecar "$ARTIFACTS_DIR/devtools_base.smolmachine" --net --allow-host chatgpt.com --allow-host openai.com --allow-host api.openai.com -it --volume ~/.codex:/root/.codex -- codex login --device-auth
#smolvm machine create "$LOGIN_MACHINE" --from "$ARTIFACTS_DIR/devtools_base.smolmachine" --allow-host chatgpt.com --allow-host openai.com --allow-host api.openai.com && smolvm machine start --name "$LOGIN_MACHINE" && smolvm machine exec -it --name "$LOGIN_MACHINE" -- codex login && mkdir -p ~/.codex && smolvm machine exec --name "$LOGIN_MACHINE" -- cat /root/.codex/auth.json > ~/.codex/auth.json && smolvm machine rm --name "$LOGIN_MACHINE"
# This actually works:
#smolvm machine create --name "$LOGIN_MACHINE" --from "$ARTIFACTS_DIR/devtools_base.smolmachine" --volume ~/.codex:/root/.codex --volume /tmp/smolvm-build-sandbox:/workspace/myrepo --allow-host chatgpt.com --allow-host openai.com --allow-host api.openai.com && smolvm machine start --name "$LOGIN_MACHINE" && smolvm machine exec -it --name "$LOGIN_MACHINE" -- codex login --device-auth && smolvm machine rm --name "$LOGIN_MACHINE"

# Create the sandbox machine from the base image
#smolvm machine create --name "$SANDBOX_MACHINE" --smolfile "$SANDBOX_SMOLFILE" --storage 20
#smolvm machine create --name "$SANDBOX_MACHINE" --from "$ARTIFACTS_DIR/devtools_base.smolmachine" --volume ~/.codex:/root/.codex --volume "$REPO_DIR:/workspace/myrepo" --allow-host chatgpt.com --allow-host openai.com --allow-host api.openai.com && smolvm machine start --name "$SANDBOX_MACHINE" && smolvm machine exec -it --name "$SANDBOX_MACHINE" -- codex login --device-auth && smolvm machine rm --name "$SANDBOX_MACHINE"
