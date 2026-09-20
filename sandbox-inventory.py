#!/usr/bin/env python3
"""Validate and classify the scriptable SmolVM machine inventory."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import NoReturn

SEPARATOR = "\x1f"
MANAGED_KEYS = (
    "codex-sandbox.kind",
    "codex-sandbox.base-id",
    "codex-sandbox.revision",
    "codex-sandbox.workspace-sha256",
)


def fail(message: str) -> NoReturn:
    raise ValueError(message)


def clean(value: object, field: str) -> str:
    if value is None:
        return ""
    if not isinstance(value, str):
        fail(f"machine {field} must be a string")
    if any(character in value for character in ("\n", "\r", SEPARATOR)):
        fail(f"machine {field} contains an unsupported control character")
    return value


def load_inventory(path: Path) -> list[dict[str, object]]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(f"invalid JSON: {error}")
    if not isinstance(value, list):
        fail("inventory root must be a JSON array")

    result: list[dict[str, object]] = []
    names: set[str] = set()
    for index, item in enumerate(value):
        if not isinstance(item, dict):
            fail(f"inventory entry {index} must be an object")
        name = clean(item.get("name"), "name")
        if not name:
            fail(f"inventory entry {index} has no name")
        if name in names:
            fail(f"inventory contains duplicate machine name {name!r}")
        names.add(name)
        state = clean(item.get("state", "unknown"), "state") or "unknown"
        labels = item.get("labels", {})
        if labels is None:
            labels = {}
        if not isinstance(labels, dict):
            fail(f"machine {name!r} labels must be an object")
        normalized_labels: dict[str, str] = {}
        for key, label_value in labels.items():
            normalized_key = clean(key, "label key")
            normalized_labels[normalized_key] = clean(label_value, "label value")
        result.append({"name": name, "state": state, "labels": normalized_labels})
    return result


def emit(*fields: object) -> None:
    print(SEPARATOR.join(str(field) for field in fields))


def normalize(inventory: list[dict[str, object]]) -> None:
    for machine in inventory:
        labels = machine["labels"]
        assert isinstance(labels, dict)
        emit(
            machine["name"],
            machine["state"],
            labels.get(MANAGED_KEYS[0], ""),
            labels.get(MANAGED_KEYS[1], ""),
            labels.get(MANAGED_KEYS[2], ""),
            labels.get(MANAGED_KEYS[3], ""),
            "yes" if any(key in labels for key in MANAGED_KEYS) else "no",
        )


def classify(args: argparse.Namespace, inventory: list[dict[str, object]]) -> None:
    base_pattern = re.compile(r"^[0-9a-f]{12}$")
    revision_pattern = re.compile(r"^[0-9]+$")
    legacy_pattern = re.compile(
        rf"^codex-workspace-([0-9a-f]{{12}})-r([0-9]+)-{re.escape(args.workspace_id)}$"
    )

    for machine in inventory:
        name = str(machine["name"])
        state = str(machine["state"])
        labels = machine["labels"]
        assert isinstance(labels, dict)
        kind = labels.get(MANAGED_KEYS[0], "")
        base_id = labels.get(MANAGED_KEYS[1], "")
        revision = labels.get(MANAGED_KEYS[2], "")
        workspace_hash = labels.get(MANAGED_KEYS[3], "")
        any_managed = any(key in labels for key in MANAGED_KEYS)
        legacy_match = legacy_pattern.fullmatch(name)
        if args.override and name == args.desired_name:
            emit("desired", name, state, base_id, revision, "override", "no", "")
            continue

        positively_owned = kind == "workspace" and workspace_hash == args.workspace_hash
        complete_labels = (
            positively_owned
            and bool(base_pattern.fullmatch(base_id))
            and bool(revision_pattern.fullmatch(revision))
        )

        if positively_owned:
            if not complete_labels:
                emit(
                    "ambiguous", name, state, base_id, revision, "labels", "no",
                    "managed workspace labels are incomplete or malformed",
                )
            elif args.override:
                emit("stale", name, state, base_id, revision, "labels", "yes", "")
            elif name == args.desired_name and base_id == args.base_id and revision == args.revision:
                emit("desired", name, state, base_id, revision, "labels", "no", "")
            elif base_id != args.base_id or revision != args.revision:
                emit("stale", name, state, base_id, revision, "labels", "yes", "")
            else:
                emit(
                    "ambiguous", name, state, base_id, revision, "labels", "no",
                    "managed identity matches but the deterministic name does not",
                )
            continue

        if any_managed:
            # Managed metadata takes precedence over a legacy-looking name. A
            # conflicting exact/suffix match must block creation, but must never
            # be treated as safe to delete for this workspace.
            if name == args.desired_name or legacy_match or workspace_hash == args.workspace_hash:
                emit(
                    "ambiguous", name, state, base_id, revision, "labels", "no",
                    "managed labels are missing or contradict the requested workspace",
                )
            else:
                emit("unrelated", name, state, base_id, revision, "labels", "no", "")
            continue

        if args.override:
            emit("unrelated", name, state, "", "", "legacy-name", "no", "")
        elif name == args.desired_name:
            match = legacy_pattern.fullmatch(name)
            assert match is not None
            emit("desired", name, state, match.group(1), match.group(2), "legacy-name", "no", "")
        elif legacy_match:
            emit(
                "stale", name, state, legacy_match.group(1), legacy_match.group(2),
                "legacy-name", "yes", "",
            )
        else:
            emit("unrelated", name, state, "", "", "legacy-name", "no", "")


def verify_readiness(args: argparse.Namespace, inventory: list[dict[str, object]]) -> None:
    matches = [machine for machine in inventory if machine["name"] == args.desired_name]
    if not matches:
        fail("the desired machine is missing from the refreshed inventory")
    if len(matches) != 1:
        fail("multiple machines match the desired identity")

    machine = matches[0]
    labels = machine["labels"]
    assert isinstance(labels, dict)
    any_managed = any(key in labels for key in MANAGED_KEYS)
    if any_managed:
        expected = {
            MANAGED_KEYS[0]: "workspace",
            MANAGED_KEYS[1]: args.base_id,
            MANAGED_KEYS[2]: args.revision,
            MANAGED_KEYS[3]: args.workspace_hash,
        }
        if any(labels.get(key) != value for key, value in expected.items()):
            fail("the desired machine has incomplete or contradictory managed labels")
    elif not args.override:
        legacy_pattern = re.compile(
            rf"^codex-workspace-{re.escape(args.base_id)}-"
            rf"r{re.escape(args.revision)}-{re.escape(args.workspace_id)}$"
        )
        if not legacy_pattern.fullmatch(args.desired_name):
            fail("the unlabelled desired machine is not an allowed legacy machine")

    state = str(machine["state"])
    if state.casefold() not in {"created", "stopped", "running"}:
        fail(f"the desired machine state is {state!r}")
    emit(machine["name"], state)


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    normalize_parser = subparsers.add_parser("normalize")
    normalize_parser.add_argument("inventory", type=Path)

    classify_parser = subparsers.add_parser("classify")
    classify_parser.add_argument("inventory", type=Path)
    classify_parser.add_argument("base_id")
    classify_parser.add_argument("revision")
    classify_parser.add_argument("workspace_hash")
    classify_parser.add_argument("workspace_id")
    classify_parser.add_argument("desired_name")
    classify_parser.add_argument("--override", action="store_true")

    readiness_parser = subparsers.add_parser("verify-readiness")
    readiness_parser.add_argument("inventory", type=Path)
    readiness_parser.add_argument("base_id")
    readiness_parser.add_argument("revision")
    readiness_parser.add_argument("workspace_hash")
    readiness_parser.add_argument("workspace_id")
    readiness_parser.add_argument("desired_name")
    readiness_parser.add_argument("--override", action="store_true")

    args = parser.parse_args()
    try:
        inventory = load_inventory(args.inventory)
        if args.command == "normalize":
            normalize(inventory)
        elif args.command == "classify":
            classify(args, inventory)
        else:
            verify_readiness(args, inventory)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
