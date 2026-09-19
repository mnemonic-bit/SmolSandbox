#!/usr/bin/env bash

# Builds the codex-sandbox.smolfile into a smolVM image.

LOGIN_MACHINE="codex-login"
LOGIN_SMOLFILE="codex-login.smolfile"
REPO_DIR="${1:-$PWD}"
CODEX_HOME_DIR="~/.codex"

if smolvm machine ls | grep -q "$LOGIN_MACHINE"; then
  (cd "$REPO_DIR" && smolvm machine delete --name "$LOGIN_MACHINE" --force)
fi

mkdir -p "$CODEX_HOME_DIR"

# Create a base image with the development tools installed.
smolvm machine create --name "$LOGIN_MACHINE" --smolfile "$LOGIN_SMOLFILE" --storage 10
smolvm machine start --name "$LOGIN_MACHINE"
smolvm machine stop --name "$LOGIN_MACHINE"
