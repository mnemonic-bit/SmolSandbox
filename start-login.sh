#!/usr/bin/env bash

LOGIN_MACHINE="codex-login"
LOGIN_SMOLFILE="codex-login.smolfile"
REPO_DIR="${1:-$PWD}"
CODEX_HOME_DIR="~/.codex"

if ! smolvm machine ls | grep -q "$LOGIN_MACHINE"; then
  echo "Machine $LOGIN_MACHINE does not exist. Please call build-login.sh first to create the login machine."
  exit 1
fi

mkdir -p "$CODEX_HOME_DIR"

# Just start and stop the login machine, login will happen within this..
smolvm machine start --name "$LOGIN_MACHINE"
smolvm machine exec -it --name "$LOGIN_MACHINE" -- codex login --device-auth
smolvm machine stop --name "$LOGIN_MACHINE"
