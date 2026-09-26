#!/usr/bin/env python3
"""Validate and classify the scriptable SmolVM machine inventory."""

from __future__ import annotations

import argparse
import json
import os
import re
import stat
import sys
from pathlib import Path
from typing import NoReturn

SEPARATOR = "\x1f"
MANAGED_KEYS = (
    "codex-sandbox.kind",
    "codex-sandbox.base-id",
    "codex-sandbox.revision",
    "codex-sandbox.workspace-sha256",
    "codex-sandbox.workspace-path-sha256",
    "codex-sandbox.configuration",
)
KIND_KEY, BASE_KEY, REVISION_KEY, WORKSPACE_KEY, PATH_KEY, CONFIGURATION_KEY = MANAGED_KEYS
BASE_PATTERN = re.compile(r"^[0-9a-f]{12}$")
REVISION_PATTERN = re.compile(r"^[0-9]+$")
HASH_PATTERN = re.compile(r"^[0-9a-f]{64}$")
WORKSPACE_NAME_PATTERN = re.compile(
    r"^codex-workspace-([0-9a-f]{12})-r([0-9]+)-([0-9a-f]{12})$"
)
LOGIN_NAME_PATTERN = re.compile(r"^codex-login-([0-9a-f]{12})-r([0-9]+)$")
BUILDER_NAME_PATTERN = re.compile(r"^codex-builder-([0-9a-f]{12})$")
ARTIFACT_PATTERN = re.compile(r"^codex-devtools-([0-9a-f]{12})(\.smolmachine)?$")


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


def normalized_state(value: str) -> str | None:
    state = value.casefold()
    if state in {"running", "stopped", "created"}:
        return state
    return None


def classify_purge_inventory(
    inventory: list[dict[str, object]], current_base: str
) -> list[dict[str, str]]:
    if not BASE_PATTERN.fullmatch(current_base):
        fail("current base ID is malformed")

    managed: list[dict[str, str]] = []
    ambiguous: list[tuple[str, str]] = []
    for machine in inventory:
        name = str(machine["name"])
        raw_state = str(machine["state"])
        state = normalized_state(raw_state)
        labels = machine["labels"]
        assert isinstance(labels, dict)
        has_managed_labels = any(key.startswith("codex-sandbox.") for key in labels)
        builder_match = BUILDER_NAME_PATTERN.fullmatch(name)
        login_match = LOGIN_NAME_PATTERN.fullmatch(name)
        workspace_match = WORKSPACE_NAME_PATTERN.fullmatch(name)

        if not (builder_match or login_match or workspace_match or has_managed_labels):
            continue
        if state is None:
            ambiguous.append((name, f"managed machine state {raw_state!r} is unsupported"))
            continue

        if builder_match:
            base_id = builder_match.group(1)
            if has_managed_labels and not (
                labels.get(KIND_KEY) == "builder"
                and labels.get(BASE_KEY) == base_id
                and not any(key in labels for key in MANAGED_KEYS[2:])
            ):
                ambiguous.append((name, "builder labels are incomplete or contradictory"))
                continue
            managed.append(
                {
                    "name": name,
                    "state": state,
                    "kind": "builder",
                    "base_id": base_id,
                    "revision": "",
                    "workspace_sha256": "",
                    "workspace_path_sha256": "",
                    "configuration": "",
                    "source": "labels" if has_managed_labels else "legacy-name",
                }
            )
            continue

        if login_match:
            base_id, revision = login_match.groups()
            if has_managed_labels and not (
                labels.get(KIND_KEY) == "login"
                and labels.get(BASE_KEY) == base_id
                and labels.get(REVISION_KEY) == revision
                and not any(key in labels for key in MANAGED_KEYS[3:])
            ):
                ambiguous.append((name, "login labels are incomplete or contradictory"))
                continue
            managed.append(
                {
                    "name": name,
                    "state": state,
                    "kind": "login",
                    "base_id": base_id,
                    "revision": revision,
                    "workspace_sha256": "",
                    "workspace_path_sha256": "",
                    "configuration": "",
                    "source": "labels" if has_managed_labels else "legacy-name",
                }
            )
            continue

        if not workspace_match:
            ambiguous.append((name, "managed labels are attached to an unrecognized machine name"))
            continue

        name_base, name_revision, name_workspace_id = workspace_match.groups()
        if not has_managed_labels:
            if name_base == current_base:
                ambiguous.append((name, "current-generation workspace metadata is missing"))
                continue
            managed.append(
                {
                    "name": name,
                    "state": state,
                    "kind": "workspace",
                    "base_id": name_base,
                    "revision": name_revision,
                    "workspace_sha256": "",
                    "workspace_path_sha256": "",
                    "configuration": "",
                    "source": "legacy-name",
                }
            )
            continue

        kind = labels.get(KIND_KEY, "")
        base_id = labels.get(BASE_KEY, "")
        revision = labels.get(REVISION_KEY, "")
        workspace_hash = labels.get(WORKSPACE_KEY, "")
        path_hash = labels.get(PATH_KEY, "")
        configuration = labels.get(CONFIGURATION_KEY, "")
        old_labels_valid = (
            kind == "workspace"
            and bool(BASE_PATTERN.fullmatch(base_id))
            and bool(REVISION_PATTERN.fullmatch(revision))
            and bool(HASH_PATTERN.fullmatch(workspace_hash))
            and base_id == name_base
            and revision == name_revision
            and workspace_hash[:12] == name_workspace_id
        )
        complete_labels = (
            old_labels_valid
            and bool(HASH_PATTERN.fullmatch(path_hash))
            and configuration in {"default", "custom"}
        )
        if complete_labels:
            source = "labels"
        elif old_labels_valid and base_id != current_base and not path_hash and not configuration:
            source = "legacy-labels"
        else:
            ambiguous.append((name, "workspace labels are incomplete or contradictory"))
            continue
        managed.append(
            {
                "name": name,
                "state": state,
                "kind": "workspace",
                "base_id": base_id,
                "revision": revision,
                "workspace_sha256": workspace_hash,
                "workspace_path_sha256": path_hash,
                "configuration": configuration,
                "source": source,
            }
        )

    if ambiguous:
        details = "; ".join(f"{name}: {reason}" for name, reason in sorted(ambiguous))
        fail(f"ambiguous managed machine inventory: {details}")
    return sorted(managed, key=lambda item: item["name"])


def regular_file_size(path: Path) -> int | None:
    try:
        metadata = path.lstat()
    except OSError:
        return None
    if not stat.S_ISREG(metadata.st_mode):
        return None
    return metadata.st_size


def artifact_inventory(artifacts_dir: Path) -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    try:
        entries = list(os.scandir(artifacts_dir))
    except OSError as error:
        fail(f"could not enumerate artifact directory: {error}")
    for entry in entries:
        match = ARTIFACT_PATTERN.fullmatch(entry.name)
        if not match or not entry.is_file(follow_symlinks=False):
            continue
        result.append(
            {
                "base_id": match.group(1),
                "path": str((artifacts_dir / entry.name).absolute()),
            }
        )
    return sorted(result, key=lambda item: (item["base_id"], item["path"]))


def build_purge_plan(args: argparse.Namespace, inventory: list[dict[str, object]]) -> dict[str, object]:
    artifacts_dir = args.artifacts_dir.resolve()
    current_artifact = artifacts_dir / f"codex-devtools-{args.current_base}.smolmachine"
    size = regular_file_size(current_artifact)
    if size is None or size == 0:
        fail(f"current artifact is missing, empty, or not a regular file: {current_artifact}")

    machines = classify_purge_inventory(inventory, args.current_base)
    artifacts = artifact_inventory(artifacts_dir)
    running = [machine for machine in machines if machine["state"] == "running"]
    protected = sorted({args.current_base, *(machine["base_id"] for machine in running)})
    current_workspaces = [
        machine
        for machine in machines
        if machine["kind"] == "workspace" and machine["base_id"] == args.current_base
    ]
    has_alternate = any(
        machine["configuration"] == "custom" for machine in current_workspaces
    )

    selected_machines: list[dict[str, str]] = []
    for machine in machines:
        if machine["state"] == "running":
            continue
        if args.policy == "conservative":
            selected = machine["base_id"] != args.current_base
        elif args.policy == "force":
            selected = machine["base_id"] != args.current_base or (
                machine["kind"] == "workspace"
                and machine["base_id"] == args.current_base
                and machine["configuration"] == "custom"
            )
        else:
            selected = True
        if selected:
            selected_machines.append(machine)

    selected_artifacts = [
        artifact
        for artifact in artifacts
        if artifact["base_id"] != args.current_base
        and artifact["base_id"] not in protected
    ]
    selected_names = {machine["name"] for machine in selected_machines}
    retained_machines: list[dict[str, str]] = []
    for machine in machines:
        if machine["name"] in selected_names or machine["state"] == "running":
            continue
        retained = dict(machine)
        if machine["kind"] == "workspace" and machine["configuration"] == "default":
            retained["retention_reason"] = "current default configuration"
        elif machine["kind"] == "workspace":
            retained["retention_reason"] = "current workspace configuration"
        else:
            retained["retention_reason"] = f"current {machine['kind']} machine"
        retained_machines.append(retained)
    plan: dict[str, object] = {
        "current_base": args.current_base,
        "policy": args.policy,
        "has_alternate": has_alternate,
        "managed_machines": machines,
        "current_workspaces": current_workspaces,
        "running_machines": running,
        "retained_machines": retained_machines,
        "protected_generations": protected,
        "artifacts": artifacts,
        "selected_machines": selected_machines,
        "selected_artifacts": selected_artifacts,
        "selected_generations": sorted(
            {artifact["base_id"] for artifact in selected_artifacts}
        ),
    }
    return plan


def load_plan(path: Path) -> dict[str, object]:
    try:
        plan = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(f"invalid purge plan: {error}")
    if not isinstance(plan, dict):
        fail("purge plan root must be an object")
    return plan


def render_plan(args: argparse.Namespace) -> None:
    plan = load_plan(args.plan)
    rows = plan.get(args.section.replace("-", "_"), [])
    if args.section == "summary":
        emit(
            len(plan.get("selected_machines", [])),
            len(plan.get("selected_generations", [])),
            len(plan.get("selected_artifacts", [])),
            len(plan.get("running_machines", [])),
        )
        return
    if not isinstance(rows, list):
        fail(f"purge plan section {args.section!r} is invalid")
    for row in rows:
        if not isinstance(row, dict):
            fail(f"purge plan section {args.section!r} contains an invalid row")
        if args.section == "selected-artifacts":
            emit(row.get("path", ""), row.get("base_id", ""))
        elif args.section == "current-workspaces":
            emit(
                row.get("name", ""), row.get("state", ""),
                row.get("workspace_path_sha256", ""), row.get("configuration", ""),
            )
        elif args.section == "retained-machines":
            emit(
                row.get("name", ""), row.get("state", ""),
                row.get("retention_reason", ""),
            )
        else:
            emit(
                row.get("name", ""), row.get("state", ""), row.get("kind", ""),
                row.get("base_id", ""), row.get("configuration", ""),
            )


def verify_artifact_protection(
    args: argparse.Namespace, inventory: list[dict[str, object]]
) -> None:
    plan = load_plan(args.plan)
    current_base = plan.get("current_base")
    if not isinstance(current_base, str):
        fail("purge plan has no current base")
    selected = plan.get("selected_generations", [])
    if not isinstance(selected, list) or not all(isinstance(value, str) for value in selected):
        fail("purge plan has invalid selected generations")
    selected_set = set(selected)
    machines = classify_purge_inventory(inventory, current_base)
    conflicts = [
        machine["name"]
        for machine in machines
        if machine["state"] == "running" and machine["base_id"] in selected_set
    ]
    if conflicts:
        fail(
            "artifact deletion is no longer safe; running machine(s) protect a "
            f"selected generation: {', '.join(sorted(conflicts))}"
        )


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

    purge_parser = subparsers.add_parser("purge-plan")
    purge_parser.add_argument("inventory", type=Path)
    purge_parser.add_argument("current_base")
    purge_parser.add_argument("artifacts_dir", type=Path)
    purge_parser.add_argument(
        "policy", choices=("conservative", "force", "force-all")
    )

    render_parser = subparsers.add_parser("render-plan")
    render_parser.add_argument("plan", type=Path)
    render_parser.add_argument(
        "section",
        choices=(
            "selected-machines",
            "selected-artifacts",
            "current-workspaces",
            "running-machines",
            "retained-machines",
            "summary",
        ),
    )

    protection_parser = subparsers.add_parser("verify-artifact-protection")
    protection_parser.add_argument("inventory", type=Path)
    protection_parser.add_argument("plan", type=Path)

    args = parser.parse_args()
    try:
        if args.command == "render-plan":
            render_plan(args)
            return 0

        inventory = load_inventory(args.inventory)
        if args.command == "normalize":
            normalize(inventory)
        elif args.command == "classify":
            classify(args, inventory)
        elif args.command == "verify-readiness":
            verify_readiness(args, inventory)
        elif args.command == "purge-plan":
            print(
                json.dumps(
                    build_purge_plan(args, inventory),
                    ensure_ascii=False,
                    sort_keys=True,
                    separators=(",", ":"),
                )
            )
        else:
            verify_artifact_protection(args, inventory)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
