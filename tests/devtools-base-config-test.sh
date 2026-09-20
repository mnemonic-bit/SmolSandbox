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

config_command = next(
    command
    for command in definition["dev"]["init"]
    if "> /root/.codex/config.toml" in command
)
match = re.search(r"printf '([^']*)' > /root/\.codex/config\.toml", config_command)
if match is None:
    raise AssertionError("could not extract the Codex seed configuration")

config = tomllib.loads(match.group(1).replace(r"\n", "\n"))

assert config["model"] == "gpt-5.6-sol"
assert config["model_reasoning_effort"] == "high"
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

echo 'PASS: base image seeds the requested Codex model, TUI, and project trust configuration'
