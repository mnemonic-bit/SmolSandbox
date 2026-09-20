#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

python3 - "$REPO_ROOT/devtools-base.smolfile" <<'PY'
import re
import sys
import tomllib

smolfile_path = sys.argv[1]
with open(smolfile_path, "rb") as smolfile:
    definition = tomllib.load(smolfile)

assert definition["image"] == "debian:bookworm-slim"

init_commands = definition["dev"]["init"]
assert not any(
    "codebase-memory-mcp/main/scripts/setup.sh" in command
    for command in init_commands
)
cbm_install_command = next(
    command
    for command in init_commands
    if "codebase-memory-mcp/v0.11.0/install.sh" in command
)
assert (
    "CBM_DOWNLOAD_URL=https://github.com/DeusData/codebase-memory-mcp/"
    "releases/download/v0.11.0"
) in cbm_install_command
assert cbm_install_command.endswith("bash -s -- --clients=codex")
assert "/root/.local/bin/codebase-memory-mcp --version" in init_commands

package_command = next(
    command
    for command in init_commands
    if "apt-get install -y --no-install-recommends" in command
    and "podman" in command
)
installed_packages = package_command.split()
assert "podman-docker" in installed_packages
for installer_dependency in (
    "bash",
    "ca-certificates",
    "coreutils",
    "curl",
    "grep",
    "gzip",
    "mawk",
    "tar",
):
    assert installer_dependency in installed_packages
assert any(
    command.startswith("docker run --rm ")
    for command in init_commands
)

config_command = next(
    command
    for command in init_commands
    if "> /root/.codex/config.toml" in command
)
match = re.search(r"printf '([^']*)' > /root/\.codex/config\.toml", config_command)
if match is None:
    raise AssertionError("could not extract the Codex seed configuration")

config = tomllib.loads(match.group(1).replace(r"\n", "\n"))

assert config["model"] == "gpt-5.6-sol"
assert config["model_reasoning_effort"] == "high"
assert config["model_context_window"] == 1_000_000
assert config["model_auto_compact_token_limit"] == 900_000
assert config["tui"]["status_line"] == [
    "model-with-reasoning",
    "current-dir",
    "context-remaining",
    "used-tokens",
    "five-hour-limit",
    "weekly-limit",
]
assert config["tui"]["status_line_use_colors"] is True
assert config["projects"]["/workspace/myrepo"]["trust_level"] == "trusted"
PY

echo 'PASS: base image and Codex seed configuration match the requested defaults'
