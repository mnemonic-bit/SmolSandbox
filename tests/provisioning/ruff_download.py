"""Exercise template Ruff commands with real pipx and a local fixture wheel.

Run explicitly on Linux with Python 3.11+, pipx, python3-venv, and GNU timeout.
The Rust suite does not invoke this check. The fixture is not a Ruff binary.
Baseline mode supports healthy, recover, and persistent cases. Baseline recover
passes when the original bare command fails on the 18-second payload stall.
Deadline cases require the template's GNU timeout wrapper.
"""

import argparse
import io
import os
import re
import signal
import subprocess
import sys
import tempfile
import threading
import time
import tomllib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from zipfile import ZipFile


def fixture_wheel(version):
    archive = io.BytesIO()
    metadata = f"ruff-{version}.dist-info"
    with ZipFile(archive, "w") as wheel:
        wheel.writestr("ruff.py", f'def main(): print("fixture ruff {version}")\n')
        wheel.writestr(
            f"{metadata}/METADATA",
            f"Metadata-Version: 2.1\nName: ruff\nVersion: {version}\n",
        )
        wheel.writestr(
            f"{metadata}/WHEEL",
            "Wheel-Version: 1.0\nGenerator: fixture\nRoot-Is-Purelib: true\nTag: py3-none-any\n",
        )
        wheel.writestr(
            f"{metadata}/entry_points.txt",
            "[console_scripts]\nruff = ruff:main\n",
        )
        wheel.writestr(f"{metadata}/RECORD", "")
    return archive.getvalue()


def check(case, profile, baseline):
    if baseline and case in ("deadline", "full-deadline"):
        raise ValueError("baseline mode has no GNU timeout deadline")
    templates = Path(__file__).resolve().parents[2] / "templates"
    with (templates / f"{profile}.smlbx").open("rb") as source:
        command = next(
            command
            for command in tomllib.load(source)["dev"]["init"]
            if "pipx install ruff==" in command
        )
    if baseline:
        command = "pipx install ruff==0.16.8"
    if case == "deadline":
        # Only the wall-clock bound is shortened; pip retains the template policy.
        assert "--kill-after=10s 300s" in command
        command = command.replace("--kill-after=10s 300s", "--kill-after=2s 8s")

    version = re.search(r"\bpipx install ruff==(\d+\.\d+\.\d+)(?:\s|$)", command)
    assert version is not None, "Ruff command does not select a fixture version"
    version = version.group(1)
    wheel_name = f"ruff-{version}-py3-none-any.whl"
    payload = fixture_wheel(version)
    requests = []
    stopped = threading.Event()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            requests.append(self.path)
            if self.path == "/simple/ruff/":
                body = f'<a href="/packages/{wheel_name}">{wheel_name}</a>'.encode()
            elif self.path == f"/packages/{wheel_name}":
                body = payload
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header(
                "Content-Type",
                "text/html"
                if self.path.startswith("/simple")
                else "application/octet-stream",
            )
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            try:
                if self.path.startswith("/packages/") and case != "healthy":
                    self.wfile.write(body[:1])
                    self.wfile.flush()
                    body = body[1:]
                    if case in ("deadline", "full-deadline"):
                        # Keep reads active so only the overall deadline can stop pipx.
                        while not stopped.wait(30):
                            self.wfile.write(body[:1])
                            self.wfile.flush()
                            body = body[1:]
                        return
                    if stopped.wait(18 if case == "recover" else 360):
                        return
                self.wfile.write(body)
            except (BrokenPipeError, ConnectionResetError):
                pass

    with tempfile.TemporaryDirectory(prefix="ruff-download-") as temporary:
        root = Path(temporary)
        env = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith(("PIP_", "PIPX_"))
        }
        env.update(
            PIPX_HOME=str(root / "pipx"),
            PIPX_SHARED_LIBS=str(root / "shared"),
            PIPX_BIN_DIR=str(root / "bin"),
            PIPX_MAN_DIR=str(root / "man"),
            PIPX_DEFAULT_PYTHON=sys.executable,
            PIP_CONFIG_FILE=os.devnull,
            PIP_NO_CACHE_DIR="1",
            PIP_DISABLE_PIP_VERSION_CHECK="1",
            NO_PROXY="127.0.0.1",
            no_proxy="127.0.0.1",
        )
        # Seed pipx's shared pip offline, as preceding installs do in provisioning.
        subprocess.run(
            [sys.executable, "-m", "venv", str(root / "shared")],
            check=True,
            capture_output=True,
        )
        with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
            threading.Thread(target=server.serve_forever, daemon=True).start()
            env["PIP_INDEX_URL"] = f"http://127.0.0.1:{server.server_port}/simple/"
            started = time.monotonic()
            try:
                with subprocess.Popen(
                    ["/bin/sh", "-c", command],
                    env=env,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    start_new_session=True,
                ) as process:
                    try:
                        output, _ = process.communicate(timeout=315)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.communicate()
                        raise
                elapsed = time.monotonic() - started
            finally:
                stopped.set()
                server.shutdown()

        mode = "baseline" if baseline else "template"
        print(
            f"{profile} {case} {mode}: status={process.returncode}, elapsed={elapsed:.1f}s"
        )
        print(f"Command: {command}\nRequests: {requests}\n{output}", flush=True)
        assert f"/packages/{wheel_name}" in requests, (
            "pipx did not request the wheel payload"
        )
        if case == "healthy" or (case == "recover" and not baseline):
            assert process.returncode == 0, (
                "Ruff installation aborted on a recoverable payload stall"
            )
            result = subprocess.run(
                [str(root / "bin/ruff")], check=True, capture_output=True, text=True
            )
            assert result.stdout.strip() == f"fixture ruff {version}", (
                "installed fixture reports a different Ruff version"
            )
        else:
            assert process.returncode != 0, "persistent failure unexpectedly succeeded"
            assert not (root / "bin/ruff").exists(), (
                "failed installation exposed a usable Ruff app"
            )
            assert elapsed < (15 if case == "deadline" else 315), (
                "installation exceeded its failure bound"
            )
            if case == "recover" and baseline:
                assert "ReadTimeoutError" in output, (
                    "baseline did not reproduce the payload read timeout"
                )
            if case in ("deadline", "full-deadline"):
                assert process.returncode in (124, 137), (
                    "installation did not reach GNU timeout"
                )
        print(f"passed: {profile} {case} {mode}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "case",
        choices=["healthy", "recover", "persistent", "deadline", "full-deadline"],
    )
    parser.add_argument("profile", choices=["codex", "claude", "opencode", "hermes"])
    parser.add_argument(
        "--baseline",
        action="store_true",
        help=(
            "use original Ruff 0.16.8 command; recovery expects a read timeout; "
            "deadline cases are unsupported"
        ),
    )
    arguments = parser.parse_args()
    if arguments.baseline and arguments.case in ("deadline", "full-deadline"):
        parser.error("--baseline supports healthy, recover, and persistent cases")
    check(arguments.case, arguments.profile, arguments.baseline)
