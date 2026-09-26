# Feature 0003: Purge superseded sandbox artifacts and machines

## Status

Proposed

## Summary

The sandbox shall provide a `purge-sandbox.sh` command that removes superseded
managed artifacts and, when explicitly requested, the stopped managed machines
that keep an older or alternate sandbox configuration alive.

Purging is inventory-aware. Before removing anything, the command shall
calculate the current base generation, validate the complete SmolVM machine
inventory, correlate managed machines with artifact generations, and identify
workspace machines created with the default configuration separately from
machines created with additional `--allow-host` options.

The conservative invocation, with no force option, shall refuse all mutation
when it finds a supported alternate configuration for the current base
generation. It shall warn the user and print the machines that caused the
refusal. This prevents a temporary wider network configuration from being
mistaken for an obsolete sandbox. Superseded legacy generations do not require
this warning and may be removed when none of their machines is running.

Two explicit policies cover intentional cleanup:

- `--force` removes stopped superseded and alternate-configuration machines,
  but preserves machines that represent the current base's default
  configuration.
- `--force-all` removes every stopped managed sandbox machine in scope,
  including stopped default-configuration machines. Running machines are
  always preserved.

The exact current artifact and any superseded artifact protected by a retained
running machine shall be preserved in every mode. Files outside the managed
artifact naming scheme shall remain untouched.

## Motivation

Every change to `devtools-base.smolfile` or the installed SmolVM version creates
a new definition-addressed artifact under `artifacts/`. The existing workflow
intentionally retains prior generations, so these large files and their
machines accumulate after newer sandboxes are built.

Artifact age alone is no longer enough to make a safe cleanup decision.
Feature 0005 made the immutable workspace-machine specification part of machine
identity. In particular, an invocation such as:

```bash
./start-codex.sh --allow-host packages.example.com /path/to/project
```

legitimately creates a machine alongside the same workspace's default machine.
The user may use the wider configuration temporarily and later switch back to
the default configuration. Neither machine is stale merely because the other
was used more recently.

Conversely, machines and artifacts from an older base generation are legacy
sandbox state. When they are stopped and unambiguous, the conservative purge
may remove the generation without presenting the alternate-configuration
warning. Explicit modes are still needed to prune supported parallel variants
or to perform a deep cleanup of all stopped managed machines.

## Goals

- Inspect artifacts and the complete SmolVM machine inventory before deletion.
- Keep the exact current managed artifact generation.
- Preserve every running machine in every purge mode.
- Preserve the artifact generation associated with a retained running machine.
- Treat default and additional-`--allow-host` workspace configurations as
  legitimate parallel variants.
- Make conservative purge an all-or-nothing no-op when supported current-base
  variants require a user decision.
- Permit stopped legacy generations to be removed without a variant warning.
- Let `--force` prune stopped variants while retaining current-base default
  machines.
- Let `--force-all` remove all stopped managed machines, including stopped
  default machines.
- Remove both forms of output that `smolvm pack create` may produce for an
  eligible superseded artifact generation.
- Restrict machine and artifact deletion to positively identified managed
  sandbox state.
- Make the complete plan, retained objects, warnings, and failures visible.

## Non-goals

- Stopping a running machine during purge, even with `--force-all`.
- Deleting the current `.smolmachine` artifact.
- Deleting an artifact generation associated with a retained running machine.
- Inferring configuration age from file timestamps, machine creation times, or
  most-recent use.
- Choosing one additional-`--allow-host` configuration as newer or better than
  another.
- Deleting unrelated SmolVM machines or manually named packs.
- Purging Codex state under `.smolvm/codex-home`.
- Deleting arbitrary files, directories, temporary files, or paths outside
  `ARTIFACTS_DIR`.
- Restoring a machine or artifact after it has been deleted successfully.
- Discovering a newer upstream SmolVM version.

## Relationship to earlier features

Feature 0001 lists automatic deletion of old base artifacts as a non-goal.
This feature supersedes that non-goal only for the bounded, inventory-aware
purge described here. Feature 0001's identity, inventory validation, managed
ownership, and machine-safety requirements otherwise remain in force.

Feature 0002's create-before-delete replacement remains separate. Repair
addresses one requested workspace configuration; purge performs
repository-wide retention and cleanup. A failed purge shall not roll back a
successfully reconciled sandbox.

Feature 0005 intentionally permits machines for different immutable creation
policies to coexist. This feature preserves that behavior during ordinary
startup and conservative purge. To make global purge classification possible,
new workspace machines shall carry enough managed metadata to distinguish the
canonical workspace independently from its immutable configuration and to say
whether the configuration contains any user-supplied `--allow-host` value.

## Terminology

- **Current base ID:** The first 12 hexadecimal characters of the hash of the
  current `smolvm --version` output and complete base Smolfile contents.
- **Current artifact:** The non-empty regular file
  `artifacts/codex-devtools-<current-base-id>.smolmachine`.
- **Managed artifact generation:** Files named
  `codex-devtools-<generation>.smolmachine` and, when present, its companion
  `codex-devtools-<generation>`, where `<generation>` is exactly 12 lowercase
  hexadecimal characters.
- **Superseded artifact generation:** A managed artifact generation whose ID is
  not the current base ID.
- **Default configuration:** The canonical immutable workspace-machine
  specification formed with the repository's default allowlist and no
  user-supplied `--allow-host` values.
- **Alternate configuration:** A supported immutable workspace-machine
  specification for the current base that is not the default configuration.
  Additional `--allow-host` values are the principal example.
- **Supported managed machine:** A builder, login, or workspace machine whose
  current managed name and labels are complete, internally consistent, and
  recognized by this feature's inventory schema.
- **Legacy generation:** A generation whose base ID differs from the current
  base ID and whose artifacts and machines use a recognized earlier sandbox
  naming or metadata format. A legacy generation is not an alternate
  configuration of the current base.
- **Protected generation:** The current generation or a generation referenced
  by at least one retained running managed machine.
- **Purge plan:** The complete, precomputed set of machines and artifact files
  selected for removal, plus retained objects and the reason for each decision.

The words *old*, *stale*, or *variant* shall not be used as deletion criteria
without mapping the object to one of the classifications above.

## Managed metadata for configuration classification

The inventory format shall add managed labels equivalent to:

```text
codex-sandbox.workspace-path-sha256=<sha256-of-canonical-workspace-path>
codex-sandbox.configuration=default|custom
```

The path-only digest groups configurations for the same canonical workspace
without exposing the path in inventory output. It is distinct from the
existing `codex-sandbox.workspace-sha256`, whose value identifies the complete
immutable creation specification.

`codex-sandbox.configuration=default` is valid only when no additional
`--allow-host` was requested. `custom` means that at least one effective
additional host remains after canonicalization and removal of duplicates of
default hosts. The label describes how the configuration was requested; the
effective default-plus-additional allowlist remains part of
`codex-sandbox.workspace-sha256` as specified by Feature 0005.

The purge command shall not guess that a current-base workspace machine is a
default configuration merely from its name. A current-base machine with
missing, malformed, or contradictory managed metadata is ambiguous and blocks
all deletion. `--force` and `--force-all` do not override ambiguous ownership.

A machine from a superseded base may be classified as legacy using a strictly
recognized earlier name or label format. This compatibility classification is
why a stopped legacy generation does not need the current-base variant
warning. Unrecognized or contradictory machines remain unrelated or ambiguous;
they shall never be deleted merely because their names share a prefix.

## User interface

The command accepts these mutually exclusive policies:

```bash
./purge-sandbox.sh
./purge-sandbox.sh --force
./purge-sandbox.sh --force-all
```

The no-option invocation is the conservative policy. `--force` acknowledges
and permits removal of stopped alternate configurations while retaining
current-base default-configuration machines. `--force-all` requests deep
cleanup and may also remove stopped default-configuration machines.

`--force` and `--force-all` are mutually exclusive. Unknown options or
positional arguments are usage errors and shall cause no mutation.

`build-sandbox.sh` shall invoke the conservative policy after successful
reconciliation. It shall never imply either force policy. After a conservative
refusal, it shall leave the usable sandbox intact, report the standalone force
commands, and return success for the completed build while clearly reporting
that no purge occurred. A direct `purge-sandbox.sh` conservative refusal exits
non-zero so automation can distinguish it from a completed purge.

## Common eligibility gates

Before any purge mode mutates state, it shall:

1. Acquire the repository-wide purge lock.
2. Recalculate the base ID from the current authoritative inputs.
3. Confirm that the recalculated ID equals the ID used to construct the plan.
4. Confirm that the exact current `.smolmachine` path is a non-empty regular
   file.
5. Read and strictly validate `smolvm machine ls --json`.
6. Enumerate and strictly classify direct managed artifact children.
7. Classify every potentially managed machine as supported, legacy,
   unrelated, or ambiguous.
8. Build the complete purge plan before performing its first deletion.

If any gate fails, the command shall remove nothing, report the reason, and
exit non-zero. Force options do not bypass these gates.

## Conservative policy

The conservative invocation shall first look for supported alternate
configurations of the current base. If any exists, whether running or stopped,
the command shall:

1. Print a warning that legitimate parallel sandbox configurations exist.
2. List each machine, its state, its workspace-path digest, and whether it is
   default or custom.
3. Explain that additional `--allow-host` options can intentionally create
   these variants.
4. Print the `--force` and `--force-all` alternatives.
5. Remove no machine, artifact, cache, or state file.

The no-mutation rule applies to the whole plan: it shall not partially clean a
legacy generation before discovering or reporting a supported current-base
variant.

If there is no supported current-base alternate configuration, the
conservative policy may remove:

- stopped managed machines belonging only to superseded legacy generations;
  and
- superseded managed artifact generations that are not protected by a running
  machine.

This legacy cleanup is reported as normal purge output, not as the
alternate-configuration warning. Running legacy machines and their artifact
generations are retained and reported.

The conservative policy always preserves current-base default workspace
machines, the current login machine, and the current artifact. Temporary
stopped builder cleanup may continue under its existing positive-artifact
gate.

## `--force` policy: preserve default configurations

`--force` overrides only the supported alternate-configuration refusal. It
shall remove:

- stopped managed machines from superseded generations;
- stopped current-base workspace machines labelled `custom`; and
- superseded artifact generations not protected by a retained running
  machine.

It shall preserve:

- every running machine;
- every artifact generation associated with a retained running machine;
- every current-base workspace machine labelled `default`, even when stopped;
- the current login machine; and
- the current artifact and its optional companion file.

Thus, a user who temporarily starts a workspace with extra `--allow-host`
options can return to the standard sandbox and run `--force` to discard stopped
custom variants without losing stopped default-config overlays.

## `--force-all` policy: preserve only running machines

`--force-all` requests the aggressive machine-retention policy. It shall remove
every stopped, positively identified managed builder, login, and workspace
machine in scope, including stopped current-base default and custom workspace
machines.

It shall never stop or delete a running machine. A running machine protects its
artifact generation from deletion. Superseded artifact generations with no
retained running machine may be removed. The current artifact and its optional
companion remain preserved so the current sandbox can be created again without
a rebuild.

The phrase "purge everything" in command output and documentation always means
all eligible stopped managed machines and superseded artifacts; it never
includes running machines, the current artifact, ambiguous state, unrelated
machines, or unmanaged files.

## Artifact scope

Artifact candidates are direct children of `ARTIFACTS_DIR`. A path is eligible
only when it is a regular file and its basename exactly matches one of:

```text
codex-devtools-[0-9a-f]{12}.smolmachine
codex-devtools-[0-9a-f]{12}
```

For an eligible superseded generation, both matching forms shall be removed
when present. The current `.smolmachine` and current companion shall always be
preserved.

The implementation shall not use an unvalidated recursive glob. It shall not
follow subdirectories, derive deletion targets from artifact contents, or
remove a path merely because its name begins with `codex-devtools-`.

Temporary publication paths, malformed generation names, directories,
subdirectory contents, unrelated files, and every path outside
`ARTIFACTS_DIR` are outside the candidate set.

## Purge protocol

While holding the purge lock, the command shall:

1. Apply every common eligibility gate.
2. Compute the complete machine and artifact retention plan.
3. Apply the selected policy and, for conservative mode, stop before mutation
   if the alternate-configuration warning is required.
4. Print retained running machines and the generations they protect.
5. Print every stopped machine and artifact generation selected for removal.
6. Re-read the machine inventory immediately before the first deletion.
7. Abort without deletion if any selected machine changed identity or state,
   any new ambiguous machine appeared, or the protection set changed.
8. Delete selected stopped machines before deleting their artifacts.
9. Re-read inventory after machine deletion and confirm that no retained or new
   running machine protects an artifact selected for removal.
10. Remove only the validated artifact files in eligible generations.
11. Report the number of machines, generations, and files removed and retained.

An empty selected set is a successful no-op.

## Failure behavior

- A failed build or reconciliation shall not enter automatic conservative
  purge.
- Failure to read or parse the complete machine inventory shall prevent all
  purge deletions.
- A missing, empty, non-regular, or mismatched current artifact shall prevent
  all purge deletions.
- Ambiguous managed ownership or configuration metadata shall prevent all
  purge deletions in every mode.
- A conservative current-base variant warning shall prevent all deletion.
- A machine that becomes running is immediately retained and protects its
  artifact generation.
- A machine deletion failure shall stop further deletion and preserve every
  artifact not already removed.
- An artifact deletion failure shall preserve the current generation, stop
  further deletion, report the failed path, and exit non-zero.
- Successfully deleted machines or superseded files are not recreated after a
  later failure.
- Purge failure shall not remove Codex state or compatibility cache files.

## Concurrency

Artifact publication, machine creation, machine replacement, and purge shall
participate in a repository-wide purge lock so the plan cannot race with a new
machine or artifact. Existing workspace reconciliation locks remain responsible
for serializing operations on one deterministic workspace configuration.

When both locks are needed, every command shall acquire the purge lock before a
workspace reconciliation lock. The common lock order prevents deadlock.

The purge lock covers identity revalidation, inventory reads, artifact
enumeration, plan construction, machine deletions, and artifact deletions. A
lock acquisition failure causes no mutation.

## Safety invariants

- No mutation occurs until a complete, validated purge plan exists.
- No mutation occurs after a conservative alternate-configuration warning.
- No running machine is stopped or deleted in any mode.
- Every running managed machine protects its artifact generation.
- The current artifact generation is never deleted.
- `--force` never deletes a current-base default-configuration workspace
  machine.
- A current-base machine with ambiguous configuration metadata is never
  deleted and blocks all other deletion.
- A machine is deleted only after positive managed ownership and a fresh
  stopped-state check.
- A path outside the exact managed artifact naming scheme is never deleted.
- Purging never recursively deletes a directory.
- Purging never removes Codex state or compatibility cache files.

## User-visible examples

Conservative refusal because a temporary additional-host configuration exists:

```text
WARNING: parallel configurations exist for the current sandbox generation.
  codex-workspace-a1b2c3d4e5f6-r4-111111111111 (stopped, default)
  codex-workspace-a1b2c3d4e5f6-r4-222222222222 (stopped, custom)
Additional --allow-host options can create legitimate parallel machines.
No machines or artifacts were removed.
To remove stopped custom and legacy machines while preserving defaults:
  ./purge-sandbox.sh --force
To remove every stopped managed machine:
  ./purge-sandbox.sh --force-all
```

Conservative legacy cleanup with no current-base variant:

```text
Purging superseded legacy sandbox generation 111111111111:
  deleting stopped VM: codex-workspace-111111111111-r3-333333333333
  deleting artifact: artifacts/codex-devtools-111111111111.smolmachine
Removed 1 machine(s), 1 generation(s), and 1 artifact file(s).
```

Forced cleanup that preserves the default configuration:

```text
Preserving current default configuration:
  codex-workspace-a1b2c3d4e5f6-r4-111111111111 (stopped)
Removing stopped custom configuration:
  codex-workspace-a1b2c3d4e5f6-r4-222222222222
Removed 1 machine(s); preserved 1 default machine(s).
```

Aggressive cleanup with a running machine:

```text
Preserving running machine and artifact generation 999999999999:
  codex-workspace-999999999999-r2-444444444444
Removing all other stopped managed machines.
```

## Acceptance criteria

1. Given only the current artifact and current default machines, conservative
   purge succeeds as a no-op.
2. Given a current-base default workspace machine and a custom machine created
   with an additional `--allow-host`, conservative purge warns, removes
   nothing, and exits non-zero when invoked directly.
3. The same conservative refusal reached after a successful build reports that
   purge did not run but does not turn the completed build into a failure.
4. Given a stopped, unambiguous legacy machine and matching superseded
   artifact, conservative purge removes both without the current-base variant
   warning.
5. Given a running legacy machine, every mode preserves that machine and its
   matching artifact generation.
6. Given current-base default and custom machines, `--force` deletes stopped
   custom machines and preserves stopped default machines.
7. Given the same machines, `--force-all` deletes every stopped managed machine,
   including stopped default machines.
8. Neither force option stops or deletes a running machine.
9. `--force` and `--force-all` are mutually exclusive and invalid command lines
   cause no mutation.
10. New workspace machines record a stable path-only workspace digest and a
    validated `default` or `custom` configuration label.
11. Reordered or duplicate additional hosts retain Feature 0005's canonical
    machine identity and configuration classification.
12. A duplicate of a default host does not turn a default configuration into a
    custom configuration.
13. Missing, malformed, or contradictory current-base managed labels block all
    deletion even with `--force-all`.
14. Given a non-empty current artifact and two unreferenced superseded managed
    artifacts, eligible older files are removed and both current-generation
    files are preserved.
15. Unrelated files, malformed `codex-devtools-*` names, temporary publication
    files, directories, and subdirectory contents remain unchanged.
16. A fresh inventory state change before deletion aborts without deleting a
    machine or artifact.
17. A machine deletion failure prevents deletion of its artifact generation and
    stops subsequent deletion.
18. An artifact deletion failure preserves the current artifact, stops later
    deletion, and reports the failed path.
19. Two concurrent build, start, repair, and purge operations obey the global
    lock order and cannot publish, create, or delete from a stale purge plan.
20. Existing reconciliation, replacement, startup, allow-host, and cleanup
    tests continue to pass.

## Implementation outline

1. Add stable workspace-path and default/custom configuration labels to new
   workspace machines without changing Feature 0005's immutable-specification
   identity.
2. Extend inventory normalization and validation to classify supported,
   legacy, unrelated, and ambiguous machines and to group machines by base
   generation and canonical workspace identity.
3. Add a strict parser for managed artifact basenames and a correlator from
   machine base IDs to artifact generations.
4. Add the repository-wide purge lock and enforce one lock order across build,
   start, repair, clean, and purge workflows.
5. Implement a planner that completes all validation and produces explicit
   retained and removal sets before mutation.
6. Implement conservative, `--force`, and `--force-all` policies over the same
   immutable plan.
7. Add `purge-sandbox.sh`; call its conservative policy after a successful
   `build-sandbox.sh` reconciliation.
8. Delete selected stopped machines before their artifact files and revalidate
   inventory at both deletion boundaries.
9. Add fixture coverage for parallel allow-host configurations, default
   retention, aggressive cleanup, running-machine protection, legacy cleanup,
   malformed metadata, failures, and concurrency.
10. Update `README.md` when the feature is implemented.
