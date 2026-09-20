# Feature 0001: Reconcile workspace sandbox state

## Status

Proposed

## Summary

Starting Codex must reconcile the desired sandbox definition with the machines
already registered in SmolVM. The launcher must not rely on `.smolvm/base-id`
or `.smolvm/machine-revision` to discover the correct workspace machine.

If those files are missing, corrupt, or stale, the launcher shall derive the
current base, runtime revision, and workspace identity from authoritative
inputs, inspect the SmolVM inventory, reuse an exact matching machine when one
exists, and repair the state files. It shall not silently create a replacement
when stale machines for the same workspace make the situation ambiguous.

An explicit repair command shall allow the user to inspect and replace stale
workspace machines safely.

## Motivation

`start-codex.sh` currently reads `base-id` and `machine-revision` before it can
calculate the expected machine name. This creates several failure modes:

- If `base-id` is missing, startup stops even when the correct machine and base
  artifact still exist.
- If `machine-revision` is missing, startup silently assumes revision `1`.
- If either file contains stale but valid data, startup may select an obsolete
  machine or conclude that the desired machine is absent.
- If the calculated machine is not found, startup calls `build-sandbox.sh`
  without first identifying older machines for the same workspace.
- A newly created machine can leave the older machine dangling indefinitely.
- A failure of `smolvm machine status` is treated like a missing machine, even
  when the failure may mean that SmolVM inventory is unavailable or corrupt.

The state files are reconstructable metadata. They must not be authoritative
for machine identity.

## Goals

- Start an existing, correctly configured workspace machine even if either
  state file has been removed.
- Detect stale state files and repair them automatically after reconciliation.
- Distinguish a missing machine from a failure to query SmolVM.
- Discover all SmolVM machines associated with the requested workspace.
- Avoid silently deleting a persistent machine or its writable overlay.
- Avoid silently creating another machine when stale workspace machines make
  the intended action ambiguous.
- Provide an explicit, inspectable way to replace stale workspace machines.
- Support existing machines created before managed labels were introduced.
- Share identity and reconciliation logic among build, start, login, and clean
  workflows instead of duplicating it in each script.

## Non-goals

- Moving artifacts or Codex state to a host-global directory.
- Changing how the prepared `.smolmachine` base is built.
- Migrating files from one machine's writable overlay into another machine.
- Automatically deleting old base artifacts or unrelated workspace machines.
- Detecting arbitrary manual changes made directly to SmolVM's internal data.
- Replacing the current manual machine revision with a runtime-policy digest.

## Terminology

- **Desired base ID:** The first 12 hexadecimal characters of the hash of the
  current `smolvm --version` output and the complete base Smolfile contents.
- **Desired revision:** The machine revision declared by the installed sandbox
  scripts. It has one authoritative definition shared by every workflow.
- **Workspace hash:** The full SHA-256 hash of the canonical absolute workspace
  path.
- **Workspace ID:** The first 12 hexadecimal characters of the workspace hash.
- **Desired machine:** The workspace machine whose base ID, revision, and
  workspace identity all match the desired values.
- **Stale machine:** A managed machine for the same workspace whose base ID or
  revision differs from the desired values.
- **Legacy machine:** A machine created before managed SmolVM labels were added.
- **Dangling machine:** A stale machine that is no longer selected by the
  launcher. A dangling machine is not assumed to be disposable.

## Authoritative inputs

The launcher shall derive desired state from:

1. The canonical absolute workspace path.
2. The base Smolfile contents.
3. The installed SmolVM version.
4. The single machine revision declared by the sandbox implementation.
5. An explicit `CODEX_MACHINE` override, when supplied.

The following files are compatibility caches only:

- `.smolvm/base-id`
- `.smolvm/machine-revision`

Missing, empty, malformed, or stale cache files shall not prevent inventory or
reconciliation. Their values shall never override values derived from the
authoritative inputs.

## Machine identity

Unless `CODEX_MACHINE` explicitly overrides the name, the desired workspace
machine name remains:

```text
codex-workspace-<base-id>-r<revision>-<workspace-id>
```

All newly created workspace machines shall also have these SmolVM labels:

```text
codex-sandbox.kind=workspace
codex-sandbox.base-id=<base-id>
codex-sandbox.revision=<revision>
codex-sandbox.workspace-sha256=<full-workspace-hash>
```

Labels are discovery metadata. The deterministic machine name remains useful
for operators and backward compatibility.

The full workspace hash prevents reconciliation from depending solely on the
truncated 12-character workspace ID. The canonical workspace path itself need
not be stored as a label.

## Reconciliation interface

Workspace lifecycle decisions shall live behind one internal interface:

```text
reconcile_workspace(workspace, mode) -> selected machine name
```

Supported modes are:

- `start`: non-destructively select or create the desired machine.
- `check`: report the classification without creating, stopping, deleting, or
  rewriting anything.
- `replace-stale`: explicitly remove stale machines for this workspace and
  ensure the desired machine exists.

The implementation may be a sourced Bash module or a dedicated executable,
but callers and tests shall use the same interface and behavior.

## Inventory requirements

Reconciliation shall obtain the SmolVM inventory once per attempt using a
scriptable inventory command such as:

```bash
smolvm machine ls --json
```

If inventory cannot be obtained or parsed, reconciliation shall fail without
calling build, create, stop, or delete operations. An unknown inventory must
never be treated as an empty inventory.

For newly labelled machines, workspace ownership shall be established using
the full workspace-hash label. A machine with missing or contradictory managed
labels shall not be deleted automatically.

For legacy machines, reconciliation may recognize only names matching this
strict shape:

```text
^codex-workspace-[0-9a-f]{12}-r[0-9]+-<workspace-id>$
```

A legacy machine whose complete name equals the desired name may be reused.
Other legacy matches may be reported as stale, but require explicit replacement
before deletion.

## Reconciliation behavior

| Desired machine | Stale machines for workspace | `start` behavior |
| --- | --- | --- |
| Exists | None | Select it and repair compatibility caches. |
| Exists | One or more | Select it, repair caches, and warn about stale machines. |
| Missing | None | Ensure the base exists, create the desired machine, and repair caches. |
| Missing | One or more | Fail with an actionable report; do not create or delete a machine. |
| Unknown because inventory failed | Unknown | Fail without mutation. |

The actionable stale-machine report shall include:

- Each candidate machine name.
- Its SmolVM state when available.
- Its base ID and revision when discoverable.
- Whether it was identified by labels or legacy-name matching.
- The exact repair command required to replace it.

If the desired machine exists in a `created`, `stopped`, `failed`, or
`unreachable` state, it is still an existing machine. Reconciliation shall not
create another machine with a different identity. Normal startup shall attempt
to start the selected machine and expose the resulting SmolVM error if recovery
fails.

## Explicit stale replacement

Provide a repair entry point with these behaviors:

```bash
./repair-sandbox.sh --check [workspace]
./repair-sandbox.sh --replace-stale [workspace]
./repair-sandbox.sh --replace-stale --force [workspace]
```

`--check` is read-only and reports the result of reconciliation mode `check`.

`--replace-stale` shall:

1. Calculate desired state from authoritative inputs.
2. Obtain and validate the complete SmolVM inventory.
3. Display every stale machine selected for removal.
4. Refuse to remove a running stale machine.
5. Ensure the desired base and desired workspace machine exist.
6. Verify the desired machine from a fresh inventory.
7. Stop and delete the selected stale machines.
8. Repair the compatibility cache files.

Feature 0002 defines the readiness gate and failure semantics for this
create-before-delete ordering.

`--force` permits the repair operation to stop a running stale machine during
post-verification cleanup before deleting it. It does not broaden workspace
matching and must not affect an unrelated machine.

The repair command shall require positive managed-label ownership or strict
legacy-name matching before deleting a machine. Ambiguous or contradictory
metadata shall cause the repair to stop and report the affected machine.

## `CODEX_MACHINE` override

An explicit `CODEX_MACHINE` value is authoritative for selection but not for
automatic cleanup.

- If the named machine exists, startup shall use it.
- If it does not exist, normal build behavior may create it using the desired
  base and workspace configuration.
- Reconciliation shall not infer that similarly named machines belong to the
  workspace.
- `replace-stale` shall not delete override-named machines unless their managed
  labels positively match the requested workspace.

## Compatibility cache repair

After, and only after, the desired artifact and selected machine have been
confirmed, reconciliation shall write:

```text
.smolvm/base-id
.smolvm/machine-revision
```

Writes shall be atomic: write a temporary file in the same directory, flush or
close it successfully, and rename it over the destination. A failed
reconciliation shall leave existing cache files untouched.

The caches remain available to older scripts during migration, but updated
`start-codex.sh`, `start-login.sh`, and `clean-sandbox.sh` shall not require
them for discovery.

## Concurrency

Reconciliation shall serialize mutation for the desired workspace identity.
Two concurrent starts must not both create or replace the same workspace
machine. Read-only `check` operations need not acquire the mutation lock.

After acquiring the lock, a mutating operation shall refresh the inventory
before deciding whether creation or deletion is necessary.

## Safety invariants

- Inventory failure never implies machine absence.
- State cache failure never implies machine absence.
- Normal startup never deletes a machine.
- Normal startup never creates a new machine when stale candidates for the
  same workspace exist and the desired machine is missing.
- A machine belonging to another workspace is never stopped or deleted.
- A running stale machine is never deleted without explicit `--force`.
- Compatibility caches are written only after successful reconciliation.
- The exact desired machine is preferred regardless of stale cache contents.

## User-visible examples

Missing cache files with an existing desired machine:

```text
Recovered workspace machine codex-workspace-a1b2c3d4e5f6-r1-0123456789ab.
Repaired .smolvm/base-id and .smolvm/machine-revision.
```

Only a stale machine exists:

```text
No machine matches the current sandbox definition for this workspace.
Found stale workspace machine:
  codex-workspace-111111111111-r1-0123456789ab (stopped)
  base: 111111111111; desired: a1b2c3d4e5f6

No machines were created or deleted.
Inspect with:
  ./repair-sandbox.sh --check /absolute/workspace
Replace with:
  ./repair-sandbox.sh --replace-stale /absolute/workspace
```

Inventory failure:

```text
ERROR: could not read the SmolVM machine inventory; refusing to build or alter machines.
```

## Acceptance criteria

1. Given a missing `base-id` file and an existing desired machine, starting
   Codex reuses that machine, does not invoke a build or create operation, and
   restores the cache file.
2. Given a missing `machine-revision` file and an existing desired machine,
   starting Codex reuses that machine and restores the cache file.
3. Given stale cache values and an existing desired machine, the desired
   machine is selected and both caches are corrected.
4. Given an existing stale machine but no desired machine, normal startup exits
   non-zero, reports the stale machine, and performs no build, create, stop, or
   delete operation.
5. Given no desired or stale workspace machine, normal startup ensures the base
   and creates exactly one desired machine.
6. Given multiple stale candidates, all are reported and normal startup makes
   no machine changes.
7. Given a failed inventory command or invalid inventory JSON, no machine or
   artifact mutation occurs.
8. Given an exact legacy machine name without labels, normal startup reuses it.
9. Given a non-exact legacy machine for the workspace, normal startup reports it
   as stale and does not delete it.
10. Given `--replace-stale`, stopped stale machines for the requested workspace
    are deleted and exactly one desired machine is ensured.
11. Given a running stale machine, `--replace-stale` refuses to delete it unless
    `--force` is present.
12. Given a machine for another workspace, no reconciliation or repair mode
    stops, deletes, or modifies it.
13. Given an existing desired machine in a failed or unreachable state, normal
    startup does not create a replacement; it attempts startup and reports the
    SmolVM failure.
14. Given two concurrent normal starts for a new workspace, at most one machine
    create operation occurs.
15. Newly created workspace machines contain all managed labels defined by this
    feature.
16. A failed reconciliation does not rewrite either compatibility cache file.

## Implementation outline

1. Extract base-ID, revision, workspace hashing, inventory, and machine-name
   logic into the shared reconciliation module.
2. Add managed labels to workspace-machine creation in `build-sandbox.sh`.
3. Change `start-codex.sh` to call reconciliation instead of reading cache files
   and performing a single-name status check.
4. Add `repair-sandbox.sh` as the explicit check and replacement interface.
5. Update `start-login.sh` and `clean-sandbox.sh` to derive current identity and
   inventory rather than requiring `base-id`.
6. Extend the SmolVM test fixture to return complete inventories, labels, and
   machine states and to record mutation attempts.
7. Add acceptance tests for every state transition and safety invariant above.
8. Document the recovery and repair commands in `README.md`.
