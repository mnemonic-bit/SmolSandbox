# Feature 0003: Purge superseded sandbox artifacts

## Status

Proposed

## Summary

After `build-sandbox.sh` successfully reconciles the current sandbox, it shall
remove cached sandbox artifact generations whose generation hash differs from
the current base ID.

Purging is allowed only when the artifacts directory contains a non-empty
`.smolmachine` whose filename exactly matches the base ID calculated from the
current Smolfile and `smolvm --version`. If that current artifact cannot be
confirmed, the build shall not remove any artifact.

The current generation, Codex state, SmolVM machines, and files outside the
managed artifact naming scheme shall remain untouched.

## Motivation

Every change to `devtools-base.smolfile` or the installed SmolVM version creates
a new definition-addressed artifact under `artifacts/`. The existing build
workflow intentionally retains prior generations, so these large files
accumulate even after a newer sandbox has been built successfully.

Once a build has confirmed the artifact for the currently calculated base ID,
older cached artifacts are no longer needed by this repository's normal build
path. Removing them automatically bounds disk usage while the current-artifact
gate prevents a failed or incomplete build from destroying the last known
artifact cache.

## Goals

- Keep exactly the current managed artifact generation after a successful
  `build-sandbox.sh` reconciliation.
- Require positive confirmation of the current `.smolmachine` before deleting
  any other generation.
- Remove both forms of output that `smolvm pack create` may produce for each
  superseded generation.
- Restrict deletion to files owned by this repository's artifact naming
  convention.
- Preserve the current artifact even when stale cleanup fails partway through.
- Make purge decisions and failures visible in command output and tests.

## Non-goals

- Deleting, stopping, or otherwise changing builder, login, workspace, or
  unrelated SmolVM machines.
- Purging Codex state under `.smolvm/codex-home`.
- Deleting arbitrary files, directories, temporary files, or manually named
  packs from `artifacts/`.
- Retaining a configurable number of historical generations.
- Restoring a superseded artifact after it has been deleted successfully.
- Discovering a newer upstream version when the local authoritative inputs are
  unchanged.
- Running automatic artifact purging from `start-codex.sh`, `start-login.sh`,
  `repair-sandbox.sh`, or `clean-sandbox.sh`.

## Relationship to earlier features

Feature 0001 lists automatic deletion of old base artifacts as a non-goal.
This feature supersedes only that non-goal for a successful
`build-sandbox.sh` invocation. Feature 0001's identity, reconciliation, and
machine-safety requirements remain unchanged.

Feature 0002 excludes deletion of old base artifacts from stale-machine
replacement. This feature does not change replacement behavior: artifact
purging remains a post-success responsibility of `build-sandbox.sh`, not of
`repair-sandbox.sh --replace-stale`.

## Terminology

- **Current base ID:** The first 12 hexadecimal characters of the hash of the
  current `smolvm --version` output and complete base Smolfile contents, as
  defined by Feature 0001.
- **Current artifact:** The non-empty regular file
  `artifacts/codex-devtools-<current-base-id>.smolmachine`.
- **Managed artifact generation:** Files named
  `codex-devtools-<generation>.smolmachine` and, when present, its companion
  `codex-devtools-<generation>`, where `<generation>` is exactly 12 lowercase
  hexadecimal characters.
- **Superseded generation:** A managed artifact generation whose generation
  component does not equal the current base ID.
- **Purge:** Removal of every file belonging to a superseded managed artifact
  generation.

## Trigger and eligibility gate

Purging shall run automatically only after `build-sandbox.sh` has completed
successful workspace reconciliation, including confirmation or creation of the
current artifact and the requested runtime machines.

Immediately before purging, the script shall:

1. Recalculate the base ID from the current authoritative inputs.
2. Confirm that the recalculated ID equals the ID used by the successful build.
3. Confirm that the exact current `.smolmachine` path exists, is a regular file,
   and is non-empty.

If any check fails, the script shall remove nothing, report that the purge was
refused, and exit non-zero. Reusing an already existing current artifact is
eligible; the artifact need not have been created by the same invocation.

## Purge scope

The purge candidate set shall be derived from direct children of
`ARTIFACTS_DIR`. A file is eligible only when its basename exactly matches one
of these forms:

```text
codex-devtools-[0-9a-f]{12}.smolmachine
codex-devtools-[0-9a-f]{12}
```

For every generation other than the current base ID, both matching forms shall
be removed when present. The current `.smolmachine` and current companion file
shall always be preserved.

The implementation shall not use an unvalidated recursive glob. It shall not
follow subdirectories, derive deletion targets from artifact contents, or
remove a file merely because its name begins with `codex-devtools-`.

In particular, the following are outside the candidate set:

- temporary publication paths such as `.codex-devtools-*`;
- names with a missing, short, long, uppercase, or non-hexadecimal generation;
- directories and files below subdirectories;
- unrelated files in `artifacts/`; and
- every path outside `ARTIFACTS_DIR`.

## Purge protocol

Artifact publication and purging shall share a repository-wide artifact lock
so concurrent build invocations cannot publish or inspect a partially changing
candidate set.

While holding that lock, the successful build shall:

1. Revalidate the current base ID and current-artifact eligibility gate.
2. Enumerate direct children of `ARTIFACTS_DIR` without following directories.
3. Parse and validate each candidate basename.
4. Separate the current generation from superseded generations.
5. Report the superseded generations selected for removal.
6. Remove only the validated files for those generations.
7. Report the number of generations and files removed.

An empty superseded set is a successful no-op.

## Failure behavior

- A failed sandbox build or reconciliation shall not enter the purge protocol.
- A missing, empty, non-regular, or mismatched current artifact shall prevent
  all purge deletions.
- Failure to enumerate or validate the artifact directory shall prevent all
  purge deletions.
- A deletion failure shall preserve the current generation, stop further
  deletion, report the failed path, and make `build-sandbox.sh` exit non-zero.
- Superseded files deleted before a later deletion failure are not restored.
- Purge failure shall not delete the successfully created current artifact or
  roll back successfully created runtime machines.

## Concurrency

The artifact lock shall be distinct from the workspace reconciliation lock
because `ARTIFACTS_DIR` is shared by every workspace using this repository.
The lock shall cover current-artifact revalidation, candidate enumeration, and
all purge deletions.

The current base ID shall be recalculated after acquiring the artifact lock. If
the Smolfile or reported SmolVM version changed since the build calculated its
base ID, purging shall be refused without deleting anything.

## Safety invariants

- No artifact is deleted unless the exact current `.smolmachine` exists and is
  non-empty.
- No artifact is deleted after an unsuccessful `build-sandbox.sh`
  reconciliation.
- The artifact whose generation equals the current base ID is never deleted.
- A path that does not exactly match the managed naming scheme is never
  deleted.
- Purging never recursively deletes a directory.
- Purging never stops or deletes a SmolVM machine.
- Purging never removes Codex state or compatibility cache files.
- A purge failure never rolls back the usable current sandbox.

## User-visible examples

Successful purge after creating a new generation:

```text
Building Codex base generation a1b2c3d4e5f6 (...)
Ready:
  artifact:  .../artifacts/codex-devtools-a1b2c3d4e5f6.smolmachine
Purging superseded sandbox artifact generations:
  111111111111
  222222222222
Removed 2 generation(s), 3 file(s).
```

Successful no-op with only the current generation:

```text
No superseded sandbox artifact generations to purge.
```

Refusal when the current artifact cannot be confirmed:

```text
ERROR: refusing artifact purge: current artifact is missing or empty:
  .../artifacts/codex-devtools-a1b2c3d4e5f6.smolmachine
No artifacts were removed.
```

## Acceptance criteria

1. Given a successful build, a non-empty current `.smolmachine`, and two older
   managed generations, both older generations' managed files are removed and
   both current-generation files are preserved.
2. Given a successful build that reuses an existing current artifact, old
   managed generations are still purged.
3. Given no superseded generation, the build succeeds and reports a no-op.
4. Given a failed build or failed workspace reconciliation, no artifact is
   removed.
5. Given no current `.smolmachine`, an empty current `.smolmachine`, or a
   current path that is not a regular file, no artifact is removed and the
   command exits non-zero.
6. Given a Smolfile or SmolVM-version change between build identity calculation
   and purge revalidation, no artifact is removed.
7. Given an older `.smolmachine` with an optional extensionless companion, all
   present managed files for that older generation are removed.
8. Given unrelated files, malformed `codex-devtools-*` names, temporary
   publication files, and subdirectories in `ARTIFACTS_DIR`, all remain
   unchanged.
9. Given a deletion failure, the current artifact remains available, later
   candidates are not deleted, and the command exits non-zero with the failed
   path in its output.
10. Given existing login or workspace machines for a superseded generation,
    artifact purging does not stop, delete, or modify those machines.
11. Given two concurrent builds, artifact publication and purge operations do
    not overlap inside the repository-wide artifact lock.
12. Existing reconciliation, replacement, startup, and cleanup tests continue
    to pass.

## Implementation outline

1. Add a shared helper that strictly parses managed artifact basenames and
   returns their generation IDs.
2. Add a repository-wide artifact lock used by artifact publication and purge.
3. Add a purge helper that recalculates the base ID, enforces the
   current-artifact gate, enumerates direct children, and removes only validated
   superseded files.
4. Call the purge helper from `build-sandbox.sh` only after successful
   reconciliation.
5. Keep machine cleanup and compatibility-cache behavior unchanged.
6. Add fixture coverage for successful purge, reuse, no-op, failed build,
   missing or empty current artifacts, malformed filenames, deletion failure,
   identity changes, and concurrent builds.
7. Update `README.md` to describe automatic retention of only the current
   artifact generation when implementation lands.
