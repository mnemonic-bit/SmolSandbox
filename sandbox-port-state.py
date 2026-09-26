#!/usr/bin/env python3
"""Validate and atomically maintain launcher-owned SmolVM port state."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import tempfile
from pathlib import Path
from typing import NoReturn

FORMAT_VERSION = 1
MACHINE_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]*$")
HASH_PATTERN = re.compile(r"^[0-9a-f]{64}$")


def fail(message: str) -> NoReturn:
    raise ValueError(message)


def validate_binding(machine: str, created_at: int, workspace_sha256: str) -> None:
    if not MACHINE_PATTERN.fullmatch(machine):
        fail(f"machine name {machine!r} cannot be used for local port state")
    if created_at < 0:
        fail("machine creation timestamp is invalid")
    if not HASH_PATTERN.fullmatch(workspace_sha256):
        fail("workspace hash is invalid")


def validate_ports(value: object, field: str) -> list[int]:
    if not isinstance(value, list):
        fail(f"{field} must be an array")
    ports: list[int] = []
    for port in value:
        if isinstance(port, bool) or not isinstance(port, int) or not 1 <= port <= 65535:
            fail(f"{field} contains an invalid port")
        ports.append(port)
    if ports != sorted(set(ports)) or len(ports) > 64:
        fail(f"{field} must contain at most 64 sorted unique ports")
    return ports


def read_json(path: Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot read {path.name}: {error}")
    if not isinstance(value, dict):
        fail(f"{path.name} must contain a JSON object")
    return value


def load_state(
    path: Path,
    kind: str,
    machine: str,
    created_at: int,
    workspace_sha256: str,
) -> tuple[str, dict[str, object] | None, str]:
    if not path.exists():
        return "absent", None, "missing"
    try:
        value = read_json(path)
        if value.get("version") != FORMAT_VERSION:
            fail("unsupported format version")
        if kind == "manifest":
            value["ports"] = validate_ports(value.get("ports"), "ports")
        else:
            value["removable_ports"] = validate_ports(
                value.get("removable_ports"), "removable_ports"
            )
            value["target_ports"] = validate_ports(
                value.get("target_ports"), "target_ports"
            )
        stored_created_at = value.get("created_at")
        if isinstance(stored_created_at, bool) or not isinstance(stored_created_at, int):
            fail("created_at must be an integer")
        stored_machine = value.get("machine")
        stored_hash = value.get("workspace_sha256")
        if not isinstance(stored_machine, str) or not isinstance(stored_hash, str):
            fail("machine and workspace_sha256 must be strings")
    except ValueError as error:
        return "invalid", None, str(error)

    if (
        stored_machine != machine
        or stored_created_at != created_at
        or stored_hash != workspace_sha256
    ):
        return "mismatch", value, "state belongs to another machine incarnation"
    return "valid", value, ""


def atomic_write(path: Path, value: dict[str, object]) -> None:
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        os.fchmod(descriptor, 0o600)
        with os.fdopen(descriptor, "w", encoding="utf-8") as output:
            json.dump(value, output, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
        directory_descriptor = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory_descriptor)
        finally:
            os.close(directory_descriptor)
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def remove_file(path: Path) -> None:
    try:
        path.unlink()
    except FileNotFoundError:
        return


def paths(state_root: Path, machine: str) -> tuple[Path, Path]:
    directory = state_root / "ports"
    return directory / f"{machine}.json", directory / f"{machine}.pending.json"


def binding(args: argparse.Namespace) -> dict[str, object]:
    return {
        "version": FORMAT_VERSION,
        "machine": args.machine,
        "created_at": args.created_at,
        "workspace_sha256": args.workspace_sha256,
    }


def requested_ports(args: argparse.Namespace) -> list[int]:
    return validate_ports(args.ports, "requested ports")


def prepare(args: argparse.Namespace) -> None:
    validate_binding(args.machine, args.created_at, args.workspace_sha256)
    requested = requested_ports(args)
    manifest_path, pending_path = paths(args.state_root, args.machine)
    manifest_status, manifest, manifest_reason = load_state(
        manifest_path, "manifest", args.machine, args.created_at, args.workspace_sha256
    )
    pending_status, pending, pending_reason = load_state(
        pending_path, "pending", args.machine, args.created_at, args.workspace_sha256
    )
    manifest_ports = list(manifest["ports"]) if manifest_status == "valid" else []

    if args.state == "running":
        valid = (
            manifest_status == "valid"
            and pending_status == "absent"
            and args.count == len(manifest_ports)
            and manifest_ports == requested
        )
        if not valid:
            fail(
                f"running machine {args.machine} has {args.count} stored mapping(s), "
                f"manifest ports {manifest_ports if manifest_status == 'valid' else manifest_status}, "
                f"pending journal {pending_status}, and requested ports {requested}; "
                "stop the active session before retrying"
            )
        print("running")
        return

    if args.state not in {"created", "stopped"}:
        fail(f"machine {args.machine} is in unsupported state {args.state!r}")

    if pending_status == "absent" and manifest_status == "valid":
        if args.count != len(manifest_ports):
            fail(
                f"machine {args.machine} reports {args.count} mapping(s), but its port "
                f"manifest records {len(manifest_ports)}; refusing to overwrite unowned drift"
            )
    elif pending_status != "valid" and args.count > 0:
        state_problem = (
            f"manifest {manifest_status} ({manifest_reason}); pending journal "
            f"{pending_status} ({pending_reason})"
        )
        fail(
            f"machine {args.machine} has {args.count} mapping(s) but no safe removable "
            f"port set: {state_problem}. Inspect `smolvm machine ls --verbose`, remove "
            "the displayed mappings from the stopped machine, and retry"
        )

    known = set(requested)
    if manifest_status == "valid":
        known.update(manifest_ports)
    if pending_status == "valid":
        assert pending is not None
        known.update(pending["removable_ports"])
        known.update(pending["target_ports"])
    removable = sorted(known)

    if not removable and args.count > 0:
        fail(
            f"machine {args.machine} has {args.count} mapping(s), but the recovery "
            "journal contains no removable candidates. Inspect `smolvm machine ls "
            "--verbose`, remove the displayed mappings from the stopped machine, and retry"
        )

    if not removable and args.count == 0:
        if not args.dry_run:
            atomic_write(manifest_path, {**binding(args), "ports": requested})
            remove_file(pending_path)
        print("skip")
        return

    if not args.dry_run:
        atomic_write(
            pending_path,
            {
                **binding(args),
                "removable_ports": removable,
                "target_ports": requested,
            },
        )
    print("update")
    for port in removable:
        print(port)


def commit(args: argparse.Namespace) -> None:
    validate_binding(args.machine, args.created_at, args.workspace_sha256)
    requested = requested_ports(args)
    manifest_path, pending_path = paths(args.state_root, args.machine)
    pending_status, pending, reason = load_state(
        pending_path, "pending", args.machine, args.created_at, args.workspace_sha256
    )
    if pending_status != "valid" or pending is None:
        fail(f"cannot commit port state: pending journal is {pending_status} ({reason})")
    if pending["target_ports"] != requested:
        fail("cannot commit port state: pending target changed")
    atomic_write(manifest_path, {**binding(args), "ports": requested})
    remove_file(pending_path)


def remove(args: argparse.Namespace) -> None:
    if not MACHINE_PATTERN.fullmatch(args.machine):
        fail(f"machine name {args.machine!r} cannot be used for local port state")
    manifest_path, pending_path = paths(args.state_root, args.machine)
    remove_file(manifest_path)
    remove_file(pending_path)


def add_binding_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("state_root", type=Path)
    parser.add_argument("machine")
    parser.add_argument("created_at", type=int)
    parser.add_argument("workspace_sha256")
    parser.add_argument("ports", type=int, nargs="*")


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare_parser = subparsers.add_parser("prepare")
    add_binding_arguments(prepare_parser)
    prepare_parser.add_argument("--state", required=True)
    prepare_parser.add_argument("--count", required=True, type=int)
    prepare_parser.add_argument("--dry-run", action="store_true")

    commit_parser = subparsers.add_parser("commit")
    add_binding_arguments(commit_parser)

    remove_parser = subparsers.add_parser("remove")
    remove_parser.add_argument("state_root", type=Path)
    remove_parser.add_argument("machine")

    args = parser.parse_args()
    try:
        if args.command == "prepare":
            if not 0 <= args.count <= 64:
                fail("mapping count must be from 0 through 64")
            prepare(args)
        elif args.command == "commit":
            commit(args)
        else:
            remove(args)
    except (OSError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
