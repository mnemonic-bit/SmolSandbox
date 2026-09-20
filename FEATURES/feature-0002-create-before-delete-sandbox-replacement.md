# Feature 0002: Create-before-delete sandbox replacement

## Status

Implemented

## Summary

Replacing a stale workspace sandbox shall be a create-before-delete handoff.
The replacement workflow shall build the desired base, create and verify the
desired workspace machine, and only then stop or delete stale machines for that
workspace.

If the base build, machine creation, or verification fails, every stale
workspace machine shall remain untouched and the compatibility caches shall
remain unchanged. A successfully created desired machine may coexist with stale
machines while cleanup is pending or fails.

This feature changes the ordering of explicit stale replacement defined by
Feature 0001. It does not make normal startup destructive.

## Motivation

The current `replace-stale` workflow deletes stale workspace machines before it
ensures that the desired base and workspace machine exist. A build or create
failure after deletion leaves the workspace without a usable sandbox and
without an automatic rollback path.

Generation-specific machine names already allow old and new workspace machines
to coexist. Replacement shall use that property to keep the stale machine as a
fallback until the desired machine has crossed a verifiable readiness gate.

## Goals

- Preserve every stale workspace machine until the desired machine is verified.
- Keep a running stale machine running throughout the build and creation phase.
- Define a checkable readiness gate for destructive cleanup.
- Preserve the explicit `--replace-stale` authorization boundary from Feature
  0001.
- Leave a verified desired machine available when stale-machine cleanup fails.
- Serialize the complete handoff for a workspace.
- Make failure behavior and command ordering observable in tests.

## Non-goals

- Automatically replacing stale machines during normal `build-sandbox.sh` or
  `start-codex.sh` reconciliation.
- Starting the newly created workspace machine as a health check. Workspace
  machines remain created but stopped until their normal launcher starts them.
- Migrating data from a stale machine's writable overlay.
- Rolling back to a stale machine after destructive cleanup has begun.
- Deleting old login machines or base artifacts. They may be shared by other
  workspaces and require a separate reference-aware garbage-collection policy.
- Discovering newer upstream packages when the Smolfile, pinned versions, and
  installed SmolVM version are unchanged.

## Relationship to Feature 0001

This feature supersedes only the operation ordering under **Explicit stale
replacement** and the corresponding replacement acceptance criterion in
Feature 0001.

The following Feature 0001 behavior remains authoritative:

- `start` is non-destructive and refuses to create a desired machine when only
  stale candidates exist.
- `check` is read-only.
- Replacement requires the explicit `--replace-stale` command.
- Ambiguous ownership prevents replacement.
- A running stale machine requires `--force`.
- Workspace ownership and `CODEX_MACHINE` override rules are unchanged.
- Compatibility caches are written only after successful reconciliation.

## Terminology

- **Desired generation:** The base ID and machine revision derived from the
  current authoritative inputs defined by Feature 0001. "New" and "newer" in
  this feature mean this desired generation; they do not imply a remote update
  check.
- **Desired machine:** The exact workspace machine for the desired generation
  and requested workspace.
- **Stale machine:** A positively identified machine for the same workspace
  whose base ID or revision differs from the desired values.
- **Readiness gate:** The checks that prove the desired artifact and desired
  machine are suitable for handoff before stale cleanup begins.
- **Handoff:** The ordered transition from a verified desired machine to stale
  cleanup and cache repair.

## User interface

The existing repair interface remains the destructive entry point:

```bash
./repair-sandbox.sh --replace-stale [workspace]
./repair-sandbox.sh --replace-stale --force [workspace]
```

No additional confirmation flag is required. Supplying `--replace-stale`
authorizes removal only after the readiness gate succeeds. `--force` authorizes
stopping a running stale machine during the cleanup phase; it does not authorize
stopping that machine earlier.

`build-sandbox.sh` and `start-codex.sh` shall continue to report stale-only
state and direct the user to the explicit repair command.

## Replacement protocol

The complete mutating protocol shall run while holding the workspace
reconciliation lock.

### 1. Preflight

1. Derive the desired identity from authoritative inputs.
2. Acquire the workspace reconciliation lock.
3. Obtain and validate a fresh complete SmolVM inventory.
4. Classify the desired, stale, ambiguous, and unrelated machines.
5. Display every stale machine eligible for cleanup.
6. Refuse replacement before building or creating anything when:
   - inventory is unavailable or invalid;
   - any candidate has ambiguous or contradictory ownership metadata; or
   - a stale machine is running and `--force` is absent.

Preflight shall not stop or delete any machine.

### 2. Prepare the desired generation

1. Ensure that the desired base artifact exists and is non-empty.
2. If the exact desired machine is absent, create it from the desired artifact
   with all managed labels required by Feature 0001.
3. Preserve all stale machines in their original state throughout this phase.

Artifact publication retains its existing temporary-file-and-rename behavior.
A failed artifact build or failed workspace-machine creation shall end the
operation without entering cleanup.

### 3. Verify readiness

After preparation, reconciliation shall obtain and parse a new complete SmolVM
inventory. The desired machine passes the readiness gate only when all of the
following are true:

- The desired base artifact exists and is non-empty.
- Exactly one machine matches the deterministic desired identity, or the exact
  authorized `CODEX_MACHINE` override.
- Its managed labels positively identify the requested workspace, desired base
  ID, and desired revision. An exact legacy machine that Feature 0001 permits
  for reuse may pass without labels. An existing exact `CODEX_MACHINE` override
  may also pass without labels because the name is an explicit user selection;
  any managed labels it does carry must be complete and non-contradictory.
- Its state is `created`, `stopped`, or `running`, compared
  case-insensitively.

A desired machine in a `failed`, `unreachable`, missing, duplicate, unknown, or
unparseable state fails the readiness gate for replacement even when Feature
0001 would treat it as an existing machine during ordinary startup.

Failure to refresh or parse inventory after creation also fails the readiness
gate. In every readiness failure:

- stale machines remain running or stopped exactly as they were;
- no stale machine is deleted;
- compatibility caches remain unchanged; and
- the desired machine is preserved for inspection if SmolVM registered it.

The command shall exit non-zero and report why handoff was withheld.

### 4. Commit the handoff

Only after the readiness gate succeeds shall reconciliation:

1. Stop each running stale machine when `--force` was supplied.
2. Delete each selected stale machine.
3. Repair the compatibility cache files to the desired base ID and revision.
4. Report the desired machine as ready.

Cleanup shall fail fast. If a stop or delete operation fails, reconciliation
shall stop processing further stale machines, leave the verified desired
machine intact, leave any unprocessed stale machines intact, avoid rewriting
the compatibility caches, and exit non-zero with the exact failed operation.

Because SmolVM machine deletion is not transactional, a cleanup failure after
one of several stale machines has been removed cannot restore that machine.
The readiness gate guarantees that the desired machine already exists before
the first destructive operation.

## Existing desired machine

When both the desired machine and stale machines already exist,
`--replace-stale` shall not recreate the desired machine. It shall apply the
same readiness gate to the existing desired machine before stale cleanup.

An existing desired machine in a failed, unreachable, or unknown state shall
keep stale machines intact. This replacement rule is intentionally stricter
than ordinary startup selection.

## Failed creation and retry

If `smolvm machine create` returns non-zero, reconciliation shall refresh
inventory for reporting only. A partially registered desired machine shall not
permit stale cleanup during that attempt, regardless of its reported state.

On a later invocation, a previously registered desired machine may pass the
normal readiness gate. This makes retries recoverable without assuming that a
failed create command was side-effect free.

## Concurrency

The workspace lock shall cover preflight, potentially long-running base
preparation, desired-machine creation, readiness verification, stale cleanup,
and cache repair.

Every mutating decision shall use inventory obtained after the lock is held.
The post-creation readiness inventory shall be distinct from the preflight
inventory. Two concurrent replacement attempts through these scripts shall not
create the desired machine twice or delete the same stale machine twice.

## Safety invariants

- No stale workspace machine is stopped before the desired machine passes the
  readiness gate.
- No stale workspace machine is deleted before the desired machine passes the
  readiness gate.
- A failed build, create command, or readiness check leaves every stale machine
  untouched.
- A running stale machine remains running during desired-generation
  preparation.
- `--force` changes only the cleanup permission for running stale machines.
- A machine for another workspace is never stopped or deleted.
- Ambiguous ownership blocks all replacement mutation.
- The desired machine is never deleted as rollback for stale-cleanup failure.
- Cache files are unchanged unless the desired machine is verified and all
  selected stale cleanup succeeds.
- Every destructive operation is preceded in the command log by successful
  desired-machine verification.

## User-visible examples

Successful stopped-machine replacement:

```text
Stale machines selected for removal:
  codex-workspace-111111111111-r1-0123456789ab (stopped, labels)
Building Codex base generation a1b2c3d4e5f6 (...)
Creating workspace machine codex-workspace-a1b2c3d4e5f6-r1-0123456789ab...
Verified desired workspace machine codex-workspace-a1b2c3d4e5f6-r1-0123456789ab (created).
Removing stale workspace machine codex-workspace-111111111111-r1-0123456789ab...
Workspace machine ready: codex-workspace-a1b2c3d4e5f6-r1-0123456789ab
```

Desired creation failure:

```text
ERROR: could not create desired workspace machine codex-workspace-a1b2c3d4e5f6-r1-0123456789ab.
Stale workspace machines were left unchanged.
```

Post-create verification failure:

```text
ERROR: desired workspace machine exists but did not pass replacement readiness: failed.
Stale workspace machines were left unchanged.
Inspect with:
  ./repair-sandbox.sh --check /absolute/workspace
```

Cleanup failure after successful verification:

```text
Verified desired workspace machine codex-workspace-a1b2c3d4e5f6-r1-0123456789ab (created).
ERROR: could not delete stale workspace machine codex-workspace-111111111111-r1-0123456789ab.
The desired workspace machine remains available; compatibility caches were not changed.
```

## Acceptance criteria

1. Given one stopped stale machine and no desired machine, `--replace-stale`
   ensures the desired artifact, creates the desired machine, verifies it from
   a fresh inventory, and only then deletes the stale machine.
2. In the preceding case, the recorded `machine create` operation occurs before
   every `machine stop` or `machine delete` operation targeting a stale machine.
3. Given a running stale machine and `--force`, that machine remains running
   until desired-machine verification succeeds; its recorded stop and delete
   operations occur afterward.
4. Given a running stale machine without `--force`, replacement performs no
   build, create, stop, delete, or cache mutation.
5. Given a desired-base build failure, replacement exits non-zero, performs no
   stale stop or delete, and leaves both cache files unchanged.
6. Given a non-zero desired-machine create result, replacement exits non-zero,
   performs no stale stop or delete, and leaves both cache files unchanged.
7. Given a successful create followed by inventory failure or invalid JSON,
   replacement preserves the new machine for inspection, performs no stale stop
   or delete, and leaves both cache files unchanged.
8. Given a post-create inventory in which the desired machine is missing, has
   contradictory labels, is duplicated, or is failed or unreachable,
   replacement performs no stale stop or delete.
9. Given an existing desired machine in a created, stopped, or running state and
   one or more stale machines, replacement verifies and reuses the desired
   machine without invoking `machine create`, then removes the stale machines.
10. Given an existing desired machine in a failed or unreachable state,
    replacement leaves all stale machines intact.
11. Given a stale cleanup failure after readiness succeeds, the desired machine
    remains intact, unprocessed stale machines remain intact, cache files remain
    unchanged, and the command exits non-zero.
12. Given successful cleanup of all selected stale machines, cache files are
    atomically updated and the command reports the desired machine as ready.
13. Given multiple concurrent replacement attempts, at most one desired-machine
    create and one deletion attempt per stale machine occur through the scripts.
14. Given an unrelated machine, no replacement outcome stops, deletes, or
    modifies it.
15. Given an explicit `CODEX_MACHINE` override, stale cleanup occurs only when
    the override target passes the readiness gate and stale ownership satisfies
    Feature 0001.
16. Existing non-destructive `start` and read-only `check` tests continue to
    pass unchanged.

## Implementation outline

1. Split replacement in `sandbox_reconcile_attempt` into preflight, desired
   preparation, readiness verification, and stale cleanup phases.
2. Add a fresh-inventory readiness helper that applies the stricter replacement
   state and identity rules without weakening ordinary startup behavior.
3. Move `sandbox_ensure_base` and `sandbox_create_workspace_machine` ahead of
   the stale stop/delete loop.
4. Keep `sandbox_repair_caches` after complete stale cleanup.
5. Preserve the workspace lock across every replacement phase.
6. Extend the SmolVM fixture to provide a sequence of inventory responses and
   inject build, create, inventory, stop, and delete failures independently.
7. Extend `tests/reconcile-workspace-test.sh` with ordering assertions and every
   failure boundary in the acceptance criteria.
8. Update the replacement documentation in `README.md` and the superseded
   ordering in Feature 0001 when implementation lands.
