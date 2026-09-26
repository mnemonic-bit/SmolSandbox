# Feature 0005: Configurable workspace network allowlist

## Status

Implemented

## Summary

`start-codex.sh` shall accept repeatable `--allow-host` options that extend the
workspace machine's default network allowlist. Additional hosts shall never
replace or remove a default host.

Workspace machine identity shall include the complete set of immutable
`smolvm machine create` options. This includes the effective allowlist, base
artifact, CPU and memory allocation, and host-to-guest volume mappings. The
identity calculation shall treat these options as a set: argument order and
exact duplicate values shall not affect the selected machine.

As a result, the same workspace and effective creation policy reuse the same
persistent machine, while a materially different creation policy selects a
different machine without mutating the old one.

## Motivation

The workspace machine previously had one hard-coded network policy. A project
that needed another package registry, API, or download host required editing
`sandbox-common.sh`, incrementing the machine revision, and rebuilding or
repairing machines for every user.

Passing extra hosts only to `smolvm machine start` cannot solve this problem.
SmolVM network policy, resource allocation, and volume mappings are fixed by
`smolvm machine create`. Reusing a machine whose name depends only on the
workspace path could therefore silently reuse a machine with the wrong baked
configuration.

A deterministic digest of the complete immutable creation policy makes the
machine name describe the effective configuration. Canonical set semantics
also prevent harmless differences in option order or duplication from
creating redundant machines.

## Goals

- Allow a user to extend the default workspace allowlist from
  `start-codex.sh` without editing repository files.
- Preserve every default allowlist entry when additional hosts are supplied.
- Accept more than one additional host in one invocation.
- Make host order irrelevant to machine identity and creation arguments.
- Make exact duplicate hosts, including duplicates of defaults, irrelevant.
- Include every behaviorally significant immutable workspace-machine creation
  option in the workspace identity hash.
- Reuse a machine for equivalent effective configurations.
- Select a different machine for different effective configurations.
- Keep normal startup non-destructive when another configuration already has a
  machine for the same workspace.
- Allow repair commands to address the same additional-host configuration.
- Keep the Smolfile and workspace-machine default allowlists synchronized.

## Non-goals

- Removing or overriding a default allowlist entry from the command line.
- Changing the tightly restricted login-machine allowlist.
- Applying network-policy changes to an already-created machine.
- Parsing URLs into schemes, ports, paths, or normalized DNS names. Values are
  passed to SmolVM's `--allow-host` option as supplied.
- Treating differently cased or otherwise textually different host values as
  duplicates. Deduplication uses exact string equality.
- Passing additional hosts through `build-sandbox.sh` or `start-login.sh`.
- Migrating a writable overlay from one effective configuration to another.
- Automatically deleting machines created for an earlier configuration.
- Changing the explicit `CODEX_MACHINE` override semantics defined by Feature
  0001.

## Relationship to earlier features

This feature supersedes Feature 0001's definition of the workspace hash and
workspace ID. They no longer hash only the canonical absolute workspace path.
They identify the canonical immutable workspace-machine creation policy, which
contains the canonical path through the workspace volume mapping.

Feature 0001's inventory validation, managed labels, compatibility caches,
normal-start safety, and explicit override rules otherwise remain in force.
Feature 0002's create-before-delete protocol remains unchanged. Repair must be
given the same additional hosts when inspecting or replacing a machine for a
non-default configuration.

Different creation policies intentionally produce different workspace hashes.
Machines for those policies are not stale candidates for one another during
normal reconciliation and may coexist. Generation cleanup may remove all
managed workspace machines for the selected base according to its existing
rules.

## User interface

The launcher accepts both separated and equals forms:

```bash
./start-codex.sh --allow-host packages.example.com /path/to/project
./start-codex.sh --allow-host=packages.example.com /path/to/project
```

The option is repeatable and may appear before or after the workspace:

```bash
./start-codex.sh \
  --allow-host packages.example.com \
  /path/to/project \
  --allow-host downloads.example.com \
  -- --help
```

The first `--` ends sandbox option parsing. Every following argument is passed
to Codex and shall not be interpreted as an allowlist option.

`repair-sandbox.sh` accepts the same repeatable forms with `--check` and
`--replace-stale`:

```bash
./repair-sandbox.sh --check \
  --allow-host packages.example.com \
  /path/to/project

./repair-sandbox.sh --replace-stale \
  --allow-host packages.example.com \
  /path/to/project
```

When startup reports repair commands, it shall include the canonical
additional-host options required to address the same configuration.

A missing or empty host, a value beginning with `-`, or a value containing a
line break is invalid and shall cause a usage failure before reconciliation.

## Effective allowlist

The effective workspace allowlist is the union of:

1. The default hosts declared by the sandbox implementation.
2. Every host supplied through `--allow-host` for the current invocation.

Before identity calculation or machine creation, the union shall be sorted
using bytewise ordering and deduplicated using exact equality. The canonical
list shall be used for both operations so identity and the actual SmolVM
command cannot disagree.

Supplying a default host again has no effect. These invocations are therefore
equivalent:

```bash
./start-codex.sh \
  --allow-host a.example \
  --allow-host z.example \
  /path/to/project

./start-codex.sh \
  --allow-host z.example \
  --allow-host api.openai.com \
  --allow-host a.example \
  --allow-host a.example \
  /path/to/project
```

The `[network].allow_hosts` values in `devtools-base.smolfile` and the default
workspace `--allow-host` values in `sandbox-common.sh` shall remain
synchronized. Any change to either default list requires:

- the corresponding change to the other list;
- an increment to `SANDBOX_MACHINE_REVISION`; and
- updated assertions in `tests/devtools-base-config-test.sh` and
  `tests/reconcile-workspace-test.sh`.

## Immutable machine specification

The workspace hash is the SHA-256 digest of the canonical immutable machine
specification. Each option is represented as one `--option=value` string. The
current specification contains:

```text
--from=<base-artifact-path>
--mem=<workspace-memory-in-MiB>
--cpus=<workspace-cpu-count>
--volume=<codex-home-host-path>:/root/.codex
--volume=<canonical-workspace-path>:/workspace/myrepo
--allow-host=<effective-host>  # one entry per host
```

The strings are deduplicated, sorted, serialized as a compact UTF-8 JSON
array, and hashed with SHA-256. This length-aware serialization prevents
ambiguous concatenations. The complete digest is the workspace hash; its first
12 hexadecimal characters are the workspace ID used in the deterministic
machine name:

```text
codex-workspace-<base-id>-r<revision>-<workspace-id>
```

The complete digest remains stored in the
`codex-sandbox.workspace-sha256` managed label. Although the historical label
name says "workspace," its value now identifies the workspace's full immutable
machine specification.

The machine name itself and derived managed labels are not inputs to the hash,
which avoids a circular definition. The artifact path incorporates the base
ID, while the machine name continues to carry the base ID and machine revision
as separate, operator-visible components.

## Reconciliation behavior

For ordinary startup:

1. Parse sandbox options and preserve Codex arguments after `--`.
2. Canonicalize the workspace directory.
3. Derive the base artifact path and other immutable creation options.
4. Form the default-plus-requested effective host set.
5. Canonicalize and hash the immutable machine specification.
6. Reconcile the resulting desired machine using the existing inventory and
   locking protocol.
7. If creation is required, use the same canonical host set in the
   `smolvm machine create` command.
8. Start Codex in the selected machine and stop the machine when Codex exits.

An existing machine for another effective policy shall not be edited in place.
Normal startup may create the newly desired machine and leave the earlier
machine intact. This preserves its writable overlay and avoids granting new
network access to an identity created for a narrower policy.

An explicit `CODEX_MACHINE` name continues to override the deterministic name.
If that named machine already exists, reconciliation may reuse it according to
Feature 0001 even when the requested immutable specification differs. Users of
the override are responsible for selecting a machine with the intended baked
configuration.

## Compatibility

Changing from the former path-only workspace hash to the immutable-specification
hash changes the deterministic machine name even when no additional host is
requested. On the first launch after this feature, the launcher may therefore
create a new machine and leave the older path-hash machine untouched.

The shared Codex home remains mounted from the same host directory, so login
state, configuration, skills, and sessions remain available. Data stored only
in the old machine's writable overlay is not migrated automatically. Existing
cleanup commands retain responsibility for removing old managed machines when
the user chooses to do so.

## Safety invariants

- Additional hosts can only widen the default workspace allowlist; they cannot
  remove defaults.
- Equivalent option sets always produce the same workspace hash and machine
  name.
- A changed immutable option always changes the hash except for the negligible
  cryptographic collision probability of SHA-256.
- The canonical hosts used for hashing are exactly those passed to workspace
  machine creation.
- No effective host appears more than once in the create command.
- Additional workspace hosts are never added to the login machine.
- Codex arguments after `--` cannot alter sandbox creation policy.
- Selecting a different effective configuration does not delete or mutate the
  machine for an earlier configuration.
- Inventory and creation failures retain the non-destructive behavior defined
  by Features 0001 and 0002.

## Acceptance criteria

- `start-codex.sh --allow-host extra.example [workspace]` creates a workspace
  machine with `--allow-host extra.example` in addition to every default host.
- More than one `--allow-host` option is accepted before or after the workspace.
- `--allow-host=value` is equivalent to `--allow-host value`.
- Reversing additional-host order produces the same desired machine name.
- Repeating an additional host produces the same desired machine name and one
  create argument for that host.
- Repeating a default host produces the same desired machine name and one
  create argument for that host.
- Changing the effective host set changes the desired machine name.
- Changing another baked option, such as the Codex-home volume source, changes
  the desired machine name.
- Missing and malformed host values fail before machine mutation.
- Repair accepts and reports the additional hosts for the selected policy.
- The default Smolfile and workspace allowlists are asserted to contain the
  same set.
- The complete `tests/*-test.sh` suite passes.

## Implementation outline

1. Centralize workspace resource values and default allow hosts in
   `sandbox-common.sh`.
2. Parse repeatable allow-host options in `start-codex.sh` and
   `repair-sandbox.sh`.
3. Canonicalize requested and default hosts into one sorted unique array.
4. Build a canonical set of immutable machine-option strings and hash its JSON
   serialization.
5. Use the digest for `WORKSPACE_SHA256`, `WORKSPACE_ID`, deterministic naming,
   locking, labels, and reconciliation.
6. Construct the create command's allow-host arguments from the same canonical
   array.
7. Preserve the requested host set in generated repair commands.
8. Synchronize default allowlists and increment the machine revision.
9. Add integration coverage for order independence, duplicate elimination,
   identity changes, validation, and the complete regression suite.
