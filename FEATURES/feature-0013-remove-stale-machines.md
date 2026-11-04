# Feature 0013: Remove stale managed machines by workspace or globally

## Status

Proposed

This specification defines implementation work. Adding this document does not
implement `smolsandbox remove` or authorize execution of a removal command.

## Summary

Add `smolsandbox remove` to delete managed workspace machines associated with
the user's canonical current directory whose machine policy revision is older
than the executable's current policy revision.

Provide three independent options:

- `--all` includes current and newer revisions as well as stale revisions.
- `--force` permits stopping and removing selected running machines.
- `--global` includes managed machines across all workspaces, including shared
  login and builder machines when the revision filter permits them.

Add `--dry-run` to report the selection without stopping machines, deleting
machines, changing application state, or taking lifecycle locks. Removal
deletes guest writable state. It preserves base artifacts, definitions,
generation caches, and persistent host harness state.

## Baseline and product decisions

Specification preparation observed package version `0.6.0` in the uncommitted
Feature 0012 working tree based on commit
`e092975222c645a129de7460707b18d884e205cc`. That commit alone does not contain
the listing implementation. Existing changes are outside this specification's
file edits.

Relevant source and specifications:

- `src/lifecycle.rs`: machine policy `REVISION`, creation labels, locks,
  workspace cleanup, and purge execution.
- `src/domain.rs`: inventory parsing, managed metadata, and purge selection.
- `src/listing.rs` and [Feature 0012](feature-0012-list-workspace-machines.md):
  workspace association and inspection assessments.
- `src/platform.rs`: canonical workspace resolution.
- `src/ports.rs`: machine-bound manifest and pending-journal removal.
- `src/config.rs`: application home and state-directory overrides.

The following decisions make the requested behavior precise. These are
proposed command semantics, not claims about existing removal behavior.

1. A project is a canonical workspace directory, using the same association
   as `list`. Repository discovery does not widen deletion to the Git root.
2. Stale means an older machine policy revision. A different base generation,
   network allowlist, configuration hash, or selected name alone does not
   establish staleness for this command.
3. The package version, currently `0.6.0`, is separate from the machine policy
   revision, currently `5`. Recompilation or a package version bump alone does
   not make a machine stale.
4. Revision comparison is independent of definitions and base artifacts, so
   the same rule applies across profiles and globally. A machine that uses a
   different definition or runtime may remain useful with those inputs.
5. `--all` widens revision eligibility within the chosen scope. It never
   establishes ownership, widens workspace scope, or permits stopping a
   running machine by itself.

## Command interface

```text
smolsandbox remove
smolsandbox remove /path/to/project
smolsandbox remove --dry-run
smolsandbox remove --all
smolsandbox remove --force
smolsandbox remove --all --force
smolsandbox remove --global
smolsandbox remove --global --all
smolsandbox remove --global --force
smolsandbox remove --global --all --force
smolsandbox remove --global --all --force --dry-run
smolsandbox remove --smolvm /path/to/smolvm /path/to/project
```

Accept an optional WORKSPACE and `--smolvm` using existing path and runtime
selection semantics, including `SMOLVM_BIN`. Permit options before or after
WORKSPACE. Reject WORKSPACE together with `--global`; global selection has no
reference workspace. Reject excess positionals and guest command arguments.

Reject `--smolfile`, `--allow-host`, `--port`, `--replace-stale`, `--check`,
`--workspace`, and `--force-all`. A definition, launch allowlist, or port set
does not participate in removal selection. `--all` and `--force` compose;
neither implies the other. `--global` composes with both options.

The command executes the requested removal without an interactive confirmation
prompt. `--dry-run` provides an optional preview. Application help and version
remain offline and independent of definitions, state, and SmolVM.

## Scope and ownership

### Workspace scope

When WORKSPACE is omitted, canonicalize the invocation directory. Otherwise,
canonicalize WORKSPACE exactly as lifecycle commands do, including symlinks
and native Windows paths. Reject missing paths and paths that are not
directories. A Git repository root and its subdirectory remain separate
workspaces; users select the root explicitly or invoke the command there.
Directories without Git metadata are valid workspaces.

Associate workspace machines through a valid
`codex-sandbox.workspace-path-sha256` label equal to the full SHA-256 of the
canonical workspace path's UTF-8 bytes. Include matching workspace machines
across every profile, base generation, revision, and configuration. Do not use
the immutable configuration hash as the association filter.

Preserve shared builders, shared login machines, and other workspace paths,
including with `--all --force`. A matching path label on a row with a
contradictory kind or other ownership metadata is an error, not permission to
delete a shared or unrelated machine.

Count workspace candidates with missing or malformed path labels and report
that association is unavailable. A workspace candidate has managed kind
`workspace` or matches the complete supported workspace naming pattern. Do
not claim that unassociated candidates belong to the selected workspace.
Never infer a path from a name suffix, base ID, configuration hash, Git
remote, or mount inspection. Unassociated candidates are excluded from
workspace removal, including with `--all --force`.

### Global scope

`--global` searches the selected SmolVM's inventory for managed workspace,
login, and builder machines, across all profiles and base generations. It
does not require a reference workspace or an available project definition.
Global means machines visible to that runtime and user registry; the command
does not scan other users, hosts, or independent runtime registries.

Recognize labelled custom machine names when their complete kind-specific
metadata establishes ownership. Also recognize unlabelled legacy machines
whose names match these complete patterns:

```text
codex-builder-<12 lowercase hexadecimal characters>
codex-login-<12 lowercase hexadecimal characters>-r<decimal revision>
codex-workspace-<12 lowercase hexadecimal characters>-r<decimal revision>-<12 lowercase hexadecimal characters>
```

Legacy names establish supported managed identity for global removal; they
do not establish a workspace path. A legacy workspace row reported as
`unknown` by listing because its path is unavailable requires this explicit
global ownership rule before it can be removed.

Any `codex-sandbox.` label makes a row a managed-metadata candidate. If managed
labels are present, validate the labels rather than falling back to the name.
Recognized deterministic names must agree with their labels. Complete
labelled workspace metadata requires kind, base ID, policy revision, full
configuration hash, full workspace path hash, and `default` or `custom`
configuration classification. Login metadata requires kind, base ID, and
revision without workspace fields. Builder metadata requires kind and base ID
without revision or workspace fields. Validate existing hexadecimal and
decimal formats. Do not require a deterministic name for a valid labelled
custom machine.

Unrelated machines remain excluded. Missing or contradictory managed metadata
within the requested scope causes refusal before any stop or deletion;
`--all` and `--force` do not bypass ownership validation. Preserve the current
strict validators used by cleanup, purge, and repair. Do not use a listing
assessment or arbitrary name prefix as deletion authorization.

## Revision and runtime-state filters

For a verified workspace or login machine with revision R and the executable's
policy revision C, stale means `R < C`. Compare decimal revisions numerically,
including leading-zero normalization, without integer overflow. Treat
malformed revisions as ownership errors. Preserve `R == C` and `R > C` unless
`--all` is present; a future revision must not be silently treated as older.

Builders have no policy revision and cannot be classified as stale by this
command. Global stale removal preserves builders. `--global --all` makes
verified builders eligible.

Apply runtime-state eligibility after scope, ownership, and revision:

- Without `--force`, delete eligible `stopped` or `created` machines. Report
  eligible `running` machines as skipped without stopping or deleting them.
- With `--force`, stop eligible `running` machines, verify they are stopped
  from fresh inventory, and then delete them. The stop operation must
  succeed; force does not permit deletion after a failed stop.
- Preserve unfamiliar runtime states and refuse the removal before mutations
  if a machine otherwise selected for deletion has such a state. Force does
  not override an unknown state.

With current policy revision `5`, the expected selection is:

| Scope and identity | No options | `--force` | `--all` | `--all --force` |
| --- | --- | --- | --- | --- |
| In-scope workspace or login, r4, stopped | Delete | Delete | Delete | Delete |
| In-scope workspace or login, r4, running | Skip | Stop, verify, delete | Skip | Stop, verify, delete |
| In-scope workspace or login, r5 or r6, stopped | Preserve | Preserve | Delete | Delete |
| In-scope workspace or login, r5 or r6, running | Preserve | Preserve | Skip | Stop, verify, delete |
| Global builder, stopped | Preserve | Preserve | Delete | Delete |
| Global builder, running | Preserve | Preserve | Skip | Stop, verify, delete |
| Shared machine without `--global` | Preserve | Preserve | Preserve | Preserve |
| Other workspace without `--global` | Preserve | Preserve | Preserve | Preserve |
| Unrelated machine | Preserve | Preserve | Preserve | Preserve |

Login rows in the table are in scope only with `--global`. Every deletion
also requires validated ownership and the execution checks below.

## Planning, execution, and failure preservation

Build a deterministic plan from one parsed inventory. Sort targets by full
machine name. Validate every relevant ownership claim, selected runtime
state, and selected machine's creation timestamp before the first mutation.
A selected machine needs a valid runtime creation timestamp to distinguish
its incarnation from another machine registered under the same name. Refuse
selected names that cannot safely address local machine-bound state.

Reject malformed inventory JSON, duplicate names, missing names, and invalid
inventory field types through the existing parser. An inventory or validation
error before execution leaves all machines and application files untouched.

For execution, acquire the existing application mutation lock used to
coordinate creation and artifact cleanup. Preserve existing lock acquisition
order when workspace locks are also needed. Re-read inventory under the lock
and refuse before mutations if the relevant plan, ownership, incarnation, or
state changed. Never silently add newly discovered targets to an authorized
plan. Missing application directories may be created only for execution's
existing lock mechanism; dry-run creates none.

Revalidate each target from fresh inventory immediately before its stop or
delete operation. Match its name, kind, managed identity, path association
when required, creation timestamp, runtime state, and reported port metadata.
If a running target was skipped in the plan, leave that target skipped even
if it later stops. If a planned stopped target starts, refuse rather than
silently expanding the plan's stop operations, including with `--force`.
Only transitions caused by this command's successful forced stop may update
the planned state. External SmolVM clients do not honor application locks;
report that runtime queries and deletion are not an atomic transaction.

After a successful forced stop, verify the same machine incarnation is now
stopped and still has the expected ownership. If stop or verification fails,
do not delete that machine. Stop processing later targets and exit nonzero.
An earlier successfully stopped machine may remain stopped after failure;
do not restart it automatically.

After successful deletion, verify that the selected machine name is absent
from fresh inventory before removing its port manifest and pending journal
from the configured state root. If the name is present again, inventory fails,
or deletion fails, preserve local port state and stop processing. Use the
existing machine-name and state-path checks. Missing port files are harmless;
do not recover or reconcile ports before removing a machine.

If local port-state removal fails after confirmed VM deletion, report the
partial result, stop processing later targets, and exit nonzero. Never claim
rollback of completed stops or deletions. Report successful deletions before
the failure and identify the failed target and unprocessed targets.

Do not require an existing definition, base artifact, generation cache, desired
replacement machine, or successful guest boot. Do not build a replacement.
Inspect only runtime capabilities required for inventory, deletion, and stop
when applicable. Skip virtualization, creation, packing, and guest-execution
preflight; removing a VM must not require the ability to boot a guest.

## Preserved resources and compatibility

Delete only selected machine registrations and their guest writable state,
plus local machine-bound port files after verified deletion. Preserve:

- Every cached base artifact, including artifacts no longer referenced by a VM.
- User definitions and embedded template bytes.
- Persistent Codex, Claude, OpenCode, and Hermes host state and credentials.
- Generation caches, compatibility caches, unrelated state, and unrelated VMs.

Use native application home and existing `STATE_ROOT` selection semantics.
Do not read generation caches to authorize deletion or rewrite those caches
to describe a partial removal. Existing startup recovery handles absent
machines. Keep `purge` responsible for artifact cleanup.

Do not change machine names, creation labels, immutable hash inputs,
`REVISION`, `list` assessments, or existing clean, purge, repair, and start
behavior. The new removal policy is separate from launch comparison and
replacement eligibility. This command does not require a machine policy
revision bump because it changes no machine-creation policy.

## Output and exit status

Print the canonical workspace or explicit global scope, current policy
revision, stale-only or all-revisions mode, running-machine policy, and whether
the invocation is a dry-run. Print full selected machine names, kinds, base
IDs, revisions when available, raw states, path hashes or shared/unknown
association, and the selection reason. Do not invent paths from hashes.

Distinguish planned `delete`, `stop-and-delete`, and `skip-running` actions
from completed operations. In dry-run, use future actions and never print
that a VM was removed. Include totals for removed machines, planned removals,
skipped running machines, protected current/newer machines, and unassociated
workspace candidates as applicable. The unassociated count does not assert
ownership by the selected workspace.

An empty selection prints an explicit no-eligible-machines message. Successful
execution, dry-run, and an empty selection exit `0`. Skipping eligible running
machines without `--force` is a successful bounded operation; state the skip
count and the option required to include them. Argument errors exit `2`.
Runtime, ownership, unknown-state, concurrency, or local-state failures exit
nonzero and identify any partial results on stderr. Do not continue after a
failed mutation.

Dry-run performs inventory and required version discovery only, with no
lifecycle locks, permission changes, cache updates, or application directory
creation. A successful preview does not promise a later execution will find
the same inventory. The external SmolVM process may maintain its own registry
while servicing queries. JSON output and interactive confirmation are outside
this feature's scope.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | Default removal deletes only stopped/created older-policy workspace machines for the canonical invocation directory across profiles, bases, and configurations. | Independent mixed-inventory integration fixture with fixed revision expectations. |
| AC-02 | Explicit paths, relative paths, symlinks, directories without Git, repository subdirectories, and native Windows paths follow existing workspace semantics without widening to the repository root. | Native path and CLI integration tests. |
| AC-03 | `--all` includes equal/newer revisions within the same scope and preserves running machines without force. | Selection matrix tests including older/current/newer, equal revisions with leading zeros, and different bases/configurations. |
| AC-04 | `--force` stops and verifies only selected running targets; stop failure or failed verification never authorizes deletion. | Exact runtime call ordering and failure fixtures. |
| AC-05 | Global scope includes every supported managed workspace and shared login across revisions; builders are included only with global all-revisions removal. | Mixed labelled/custom/legacy inventory and all eight global/all/force combinations. |
| AC-06 | Unrelated and out-of-scope VMs remain untouched; unassociated workspaces are counted and never guessed into local scope. | Missing/malformed path labels, name-suffix collisions, other workspaces, and unrelated names. |
| AC-07 | Contradictory or incomplete in-scope ownership refuses before all mutations; all/force cannot bypass validation. | Deterministic-name contradictions, namespace-only labels, partial metadata, custom-name ownership, and strict legacy cases. |
| AC-08 | Base, configuration, package version, and explicit name differences alone do not establish stale revision; future revisions and revisionless builders remain protected by default. | Independent fixed policy fixtures and decimal comparison overflow tests. |
| AC-09 | Dry-run matches selection without application writes, locks, stop/delete calls, virtualization preflight, or a definition/artifact/cache requirement. | State and permission snapshots, absent-directory fixtures, and runtime call logs with virtualization bypass disabled. |
| AC-10 | Inventory failure, malformed JSON, duplicate/missing names, invalid fields, unknown selected states, and missing incarnation information refuse before mutation. | Failure fixtures asserting no stops, deletions, or state changes. |
| AC-11 | Relevant changes after planning, name reuse, and changes between targets refuse further mutations without expanding scope. | Controlled inventory sequences and concurrent application-lock tests, including a running VM appearing under a deleted name. |
| AC-12 | Successful deletion precedes port-file removal; failed deletion or failed absence verification preserves port files; partial failures preserve later targets and all artifacts. | Fixed failure-preservation and call-order tests. |
| AC-13 | All flag combinations, invalid arguments, offline help/version, no-op behavior, skip counts, and partial-result exit statuses follow the contract. | CLI parser, help, output, and process exit-code assertions. |
| AC-14 | Artifacts, definitions, credentials, generation caches, creation identity, and existing lifecycle behavior remain unchanged. | Resource snapshots, creation-policy diff review, and all required Rust regression checks. |
| AC-15 | Active documentation explains stale-policy scope, canonical-directory association, the complete flag matrix, running-state handling, preserved artifacts, and destructive guest-state effects. | Help and documentation review against tested behavior. |

## Implementation ownership and evidence

The implementer owns CLI parsing, definition-independent removal context,
validated ownership and revision selection, deterministic planning, locking,
runtime revalidation, port-file cleanup, output, independent tests, and active
documentation updates. Reuse canonical workspace resolution and established
identity formats. Keep inspection and deletion authorization distinct.

Implementation is a feature and requires a MINOR version bump in `Cargo.toml`
and the matching package entry in `Cargo.lock`. From the observed `0.6.0`
working tree, the next feature version is `0.7.0` if no intervening release
changes that baseline. This specification-only change leaves the software
version unchanged.

Run every independent Rust check listed in `AGENTS.md` after implementation.
Create `FEATURES/feature-0013-validation.md` with the tested revision, native
target, commands, exit codes, and `passed`, `failed`, `not_run`, or
`inconclusive` for each acceptance criterion. Record new results and pending
native or real runtime checks in Feature 0010 validation without changing
historical observations. Exercise real deletion only against explicitly
disposable managed VMs on compatible hosts; record the exact flag combination,
selected identities, retained resources, and observed results. Mocked removal
tests do not establish real guest behavior or physical baseline removal.

## Scope limits

This feature does not add base-generation cleanup, configuration-relative
staleness, Git-root expansion, workspace-path recovery, migration of legacy
metadata, artifact garbage collection, guest health checks, scheduling, or
automatic removal after a build. It does not stop or delete a machine merely
because `list` reports a difference from the current launch configuration.
