"""Validate Codex skill copying and seeding with disposable fixtures.

Run explicitly with Python 3.11+, jq, Node, and GNU cp 9.3+ on Linux.
The seed command requires GNU cp's --update=none option.
The independent Rust checks do not invoke this script or access upstream.
Use --definition to inspect a freshly generated Codex definition.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import tempfile
import tomllib
from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
RESULTS = []


def run(command, root, env=None, expected=0):
    command = command.replace("/root/", f"{root}/root/")
    command = command.replace("/opt/", f"{root}/opt/")
    command = command.replace("/workspace/", f"{root}/workspace/")
    result = subprocess.run(
        ["sh", "-c", command],
        env=env,
        text=True,
        capture_output=True,
        cwd=root,
        timeout=30,
    )
    if expected == 0:
        assert result.returncode == 0, result.stderr
    else:
        assert result.returncode != 0, "command unexpectedly succeeded"
    return result


def write(path, content, mode=0o644):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)
    path.chmod(mode)


def inventory(path):
    result = {}
    for child in sorted(path.rglob("*")):
        assert not child.is_symlink(), f"unmaterialized symlink: {child}"
        relative = child.relative_to(path).as_posix()
        mode = stat.S_IMODE(child.stat().st_mode)
        if child.is_file():
            result[relative] = ["file", mode, hashlib.sha256(child.read_bytes()).hexdigest()]
        else:
            result[relative] = ["directory", mode]
    return result


def fixture(root):
    names = ["grill-me", "grilling", "third-skill"]
    installed = root / "root/.agents/skills"
    installed.mkdir(parents=True)
    checkout = root / "checkout"
    for name in names:
        source = checkout / name
        write(source / "SKILL.md", f"---\nname: {name}\n---\nFixture skill.\n".encode())
        (installed / name).symlink_to(source, target_is_directory=True)
    write(checkout / "third-skill/resources/nested/reference.txt", b"nested resource\n")
    write(checkout / "third-skill/scripts/run", b"#!/bin/sh\nprintf fixture\\n\n", 0o755)
    (checkout / "third-skill/resources/link.txt").symlink_to("nested/reference.txt")
    write(installed / "unrelated-source/SKILL.md", b"Unrelated installation.\n")
    entries = {name: {"source": "mattpocock/skills", "sourceType": "github"} for name in names}
    entries["THIRD Skill"] = entries.pop("third-skill")
    entries["unrelated-source"] = {"source": "example/other", "sourceType": "github"}
    lock = root / "root/.agents/.skill-lock.json"
    write(lock, json.dumps({"version": 3, "skills": entries}).encode())
    home = root / "root/.codex"
    write(home / "config.toml", b'model = "fixture"\n')
    write(home / "skills/custom/SKILL.md", b"Custom skill.\n")
    expected = root / "expected"
    expected.mkdir()
    for name in names:
        shutil.copytree(installed / name, expected / name, symlinks=False)
    return names, lock, expected


def seed_command():
    source = (REPO / "src/lifecycle.rs").read_text()
    body = source.split("    fn seed(&self, machine: &str)", 1)[1]
    literal = re.search(r'let command = format!\(\s*("(?:[^"\\]|\\.)*")', body)
    assert literal, "Lifecycle.seed command was not found"
    return json.loads(literal.group(1)).replace("{seed}", "/opt/codex-home").replace(
        "{}", "/root/.codex"
    )


def check_fixture(commands):
    install = next(command for command in commands if "npx -y skills@" in command)
    copy = next(command for command in commands if "cp -aL" in command)
    reset = next(command for command in commands if ".skills |= with_entries" in command)
    final = commands[-1]
    snapshot, cleanup = final.split(" && apt-get clean", 1)
    assert cleanup == " && rm -rf /var/lib/apt/lists/* /tmp/node-setup.sh"
    assert commands.index(install) < commands.index(copy) < len(commands) - 1

    with tempfile.TemporaryDirectory(prefix="codex-skills-") as temporary:
        root = Path(temporary)
        names, lock, expected = fixture(root)
        binary = root / "bin/npx"
        write(
            binary,
            b"#!/usr/bin/python3\nimport json, os, sys\n"
            b"open(os.environ['SKILLS_ARGUMENTS'], 'w').write(json.dumps(sys.argv[1:]))\n"
            b"print(os.environ.get('SKILLS_OUTPUT', ''))\n"
            b"sys.exit(int(os.environ.get('SKILLS_EXIT', '0')))\n",
            0o755,
        )
        env = dict(os.environ, PATH=f"{binary.parent}:{os.environ['PATH']}")
        env["SKILLS_ARGUMENTS"] = str(root / "arguments.json")
        run(install, root, env)
        assert json.loads((root / "arguments.json").read_text()) == [
            "-y", "skills@1.8.1", "add", "mattpocock/skills", "--skill", "*",
            "--agent", "codex", "--global", "--yes",
        ]
        run(install, root, dict(env, SKILLS_EXIT="29"), expected=29)
        RESULTS.append({"check": "literal wildcard and installation failure", "result": "passed"})
        for diagnostic in ["Failed to install 1", "SKILLS_CATEGORY_DIR_CONFLICT", "SKILLS_LOCK_WRITE_FAILED"]:
            partial = run(
                f"{install} && {copy} && {snapshot}",
                root,
                dict(env, SKILLS_OUTPUT=f"\x1b[31m{diagnostic}\x1b[0m"),
                expected=1,
            )
            assert not (root / "opt/codex-home").exists()
            RESULTS.append({"check": f"exit-zero diagnostic: {diagnostic}", "result": "passed", "observed_exit": partial.returncode})

        run(copy, root)
        home = root / "root/.codex"
        assert (home / "config.toml").read_bytes() == b'model = "fixture"\n'
        assert (home / "skills/custom/SKILL.md").read_bytes() == b"Custom skill.\n"
        assert not (home / "skills/unrelated-source").exists()
        for name in names:
            assert inventory(home / "skills" / name) == inventory(expected / name)
        run(snapshot, root)
        shutil.rmtree(root / "root/.agents")
        shutil.rmtree(root / "checkout")
        for base in [home, root / "opt/codex-home"]:
            for name in names:
                assert inventory(base / "skills" / name) == inventory(expected / name)
        RESULTS.append({"check": "complete materialized copy and snapshot", "result": "passed"})

        prepared = inventory(root / "opt/codex-home")
        shutil.rmtree(home)
        home.mkdir()
        seed = seed_command()
        run(seed, root)
        assert inventory(home) == prepared
        retained = {
            "skills/grill-me/SKILL.md": b"User edited skill.\n",
            "config.toml": b'model = "retained"\n',
            "auth.json": b'{"credential":"retained"}\n',
            "sessions/session.jsonl": b'{"session":"retained"}\n',
            "skills/custom/SKILL.md": b"User custom skill.\n",
        }
        for relative, content in retained.items():
            write(home / relative, content)
        missing = home / "skills/third-skill/resources/nested/reference.txt"
        missing.unlink()
        shutil.rmtree(home / "skills/grilling")
        run(seed, root)
        first = inventory(home)
        run(seed, root)
        assert inventory(home) == first
        for relative, content in retained.items():
            assert (home / relative).read_bytes() == content
        assert missing.read_bytes() == b"nested resource\n"
        assert inventory(home / "skills/grilling") == inventory(expected / "grilling")
        RESULTS.append({"check": "empty retained and repeated persistent-home seeding", "result": "passed"})

    for case in ["empty", "missing-lock", "malformed-lock", "missing-skill", "copy-failure", "stale-inventory"]:
        with tempfile.TemporaryDirectory(prefix=f"codex-skills-{case}-") as temporary:
            root = Path(temporary)
            _, lock, _ = fixture(root)
            if case == "empty":
                lock.write_text('{"version":3,"skills":{}}')
            elif case == "missing-lock":
                lock.unlink()
            elif case == "malformed-lock":
                lock.write_text("invalid JSON")
            elif case == "missing-skill":
                (root / "checkout/grill-me/SKILL.md").unlink()
            elif case == "copy-failure":
                binary = root / "bin/cp"
                write(binary, b"#!/bin/sh\nexit 31\n", 0o755)
            else:
                write(root / "bin/npx", b"#!/bin/sh\nexit 0\n", 0o755)
                unrelated = (root / "root/.agents/skills/unrelated-source/SKILL.md").read_bytes()
                run(reset, root)
                assert set(json.loads(lock.read_text())["skills"]) == {"unrelated-source"}
                assert (root / "root/.agents/skills/unrelated-source/SKILL.md").read_bytes() == unrelated
                assert (root / "root/.agents/skills/grill-me/SKILL.md").exists()
            env = dict(os.environ, PATH=f"{root / 'bin'}:{os.environ['PATH']}")
            if case == "stale-inventory":
                run(install, root, env)
            result = run(copy, root, env, expected=1)
            RESULTS.append({"check": case, "result": "passed", "observed_exit": result.returncode})


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--definition", type=Path, default=REPO / "templates/codex.smlbx")
    parser.add_argument("--report", type=Path)
    arguments = parser.parse_args()
    with arguments.definition.open("rb") as source:
        commands = tomllib.load(source)["dev"]["init"]
    check_fixture(commands)
    report = json.dumps({"result": "passed", "checks": RESULTS}, indent=2)
    print(report)
    if arguments.report:
        arguments.report.write_text(report + "\n")
