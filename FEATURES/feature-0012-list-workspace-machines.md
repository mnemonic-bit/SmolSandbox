# Feature 0012: List workspace machines and explain selection

## Status

Implemented

Implemented in commit `16318af`. Verification of revision
`5f439cdbb5db8fbb2bb142533bf101be77de8da9` passed all seven required Rust
checks on native Linux x86-64. Two agents reviewed specification coverage
and reported no concrete implementation gaps.

Native Windows, macOS ARM64, Linux ARM64, and real SmolVM validation remain
pending. See [Feature 0012 validation](feature-0012-validation.md) for the
recorded results and remaining checks.

## Summary

Add a read-only `smolsandbox list` command. By default, show managed workspace
machines associated with the canonical current directory, across all harness
profiles, base generations, machine policy revisions, and configurations.
Accept an optional workspace argument. Add `--all` to show managed machines
for every workspace, including shared builders and login machines.

Show each machine's runtime state separately from whether its identity matches
the current launch configuration. Explain identity differences without
claiming that a machine will never be used again or is safe to delete.

## Problem and baseline

Users currently need `smolvm machine ls --json` and knowledge of managed labels
to find their sandbox instances. `repair --check` reports selection for one
configuration, but its candidate recognition uses the immutable workspace
configuration hash. That hash changes with the base artifact, mounts, and
network allowlist, so repair inspection does not enumerate all generations
for a project.

The observed baseline is commit
`e092975222c645a129de7460707b18d884e205cc`, package version `0.5.1`, with a
clean working tree before specification preparation. Relevant source:

- `src/lifecycle.rs`: `Lifecycle::new`, workspace creation labels, and repair
  inspection and replacement.
- `src/domain.rs`: inventory parsing, identity classification, and managed
  metadata validation.
- `src/platform.rs`: canonical workspace resolution.
- `src/config.rs`: definition selection and directory overrides.
- `src/cli.rs`: subcommands and option parsing.

Workspace creation already records
`codex-sandbox.workspace-path-sha256`, the full SHA-256 of the canonical
workspace path's UTF-8 bytes. Unlike `workspace-sha256`, this value does not
include the base image or creation options. Use this existing association;
do not add a new label or change machine identity.

## Command interface

```text
smolsandbox list
smolsandbox list /path/to/project
smolsandbox list --all
smolsandbox list --json
smolsandbox list --all --json
smolsandbox list --smolfile ./codex.smlbx --allow-host packages.example.com /path/to/project
```

Accept `--smolvm`, `--smolfile`, and repeatable `--allow-host` with the existing
selection and parsing semantics. Options may precede or follow the workspace
argument. `--all` changes the inventory scope; the workspace argument still
selects the reference workspace for comparison. Reject lifecycle mutation
flags, `--port`, and guest command arguments.

Use the invocation directory when WORKSPACE is omitted. Canonicalize the
workspace exactly as lifecycle commands do, including symlink and Windows
path handling. A repository root and one of its subdirectories remain
different workspaces. Repository discovery selects definitions; it does not
replace the workspace path with the repository root.

## Inventory scope and association

For the default scope, include every inventory row that claims workspace
ownership and has a valid `workspace-path-sha256` equal to the reference path
hash. Include rows from other base generations and other harness profiles.
Do not filter by the current base ID, policy revision, configuration hash,
artifact availability, or stopped/running state.

If a row has the matching path label but contradictory kind, name, or other
managed metadata, retain the row as `unknown` and explain the contradiction.
The row claims association; the listing must not imply verified ownership.

For `--all`, include rows with any `codex-sandbox.` label and rows whose names
match the existing complete builder, login, or workspace naming patterns.
Exclude unrelated SmolVM machines. Recognize labelled custom names without
requiring a deterministic name, while validating their metadata. Missing or
contradictory metadata remains visible with an `unknown` assessment.

Builders and login machines are shared by a base generation rather than owned
by an individual workspace. Include them only with `--all`, identified as
`shared` when their ownership metadata or supported legacy name is valid.

Legacy workspace names and configuration hashes cannot recover a canonical
workspace path. Do not guess an association from a truncated name suffix,
mount inspection, or matching base generation. Show unassociated legacy rows
with `--all`. A workspace candidate has either the managed kind `workspace`
or a name matching the complete legacy workspace pattern. In both scopes,
count workspace candidates whose missing or malformed path labels prevent
association. Display that count in scoped output; the count is not a claim
that those machines belong to the selected workspace.

## Reference configuration and assessment

Resolve the reference definition, effective allowlist, artifact path, harness
home, CPU and memory settings, revision, and name override using the same
inputs as `start`. Reuse the identity calculation so listing and lifecycle
selection cannot acquire separate hashing rules. Repeat the same
`--allow-host` additions to compare a custom configuration.

Do not require an existing artifact, generation cache, port manifest, or
desired machine to calculate the reference identity. An identity match does
not promise a successful start: artifact recovery, port reconciliation, guest
boot, and authentication remain separate checks.

If automatic definition lookup finds no default definition, still list the
inventory. Report the unavailable reference and use `unassessed` for rows
that require that reference. An explicitly selected missing definition,
including `BASE_SMOLFILE`, or an invalid/unreadable selected definition is an
operational error. Do not fall back past an invalid nearer definition. If
`--allow-host` is supplied without an available definition, fail with guidance
to select a definition. The absent-default exception is specific to listing.

Use these assessment values for valid inventory rows:

| Assessment | Meaning |
| --- | --- |
| `selected` | The workspace path, current base, policy revision, immutable configuration, and selected machine name match the reference identity. |
| `different-policy` | The row belongs to the reference workspace, but its policy revision differs from the executable's revision. |
| `different-base` | The row belongs to the reference workspace, but its base ID differs from the selected definition/runtime generation. |
| `different-configuration` | The workspace, base, and revision match, but the creation options or explicit name selection differ. |
| `other-workspace` | A valid path label associates the row with a different workspace; visible with `--all`. |
| `shared` | A builder or login machine has no individual workspace association. |
| `unassessed` | Ownership is valid but the reference definition is unavailable. |
| `unknown` | Metadata is incomplete, contradictory, or insufficient to establish workspace association. |

Apply `unknown` before other assessments. Apply `shared` and
`other-workspace` before comparing the reference launch configuration. For
the reference workspace, apply `unassessed` if the definition is unavailable;
otherwise compare policy, base, then configuration/name in that order.
Record all known differences in `reasons`, even when a higher-priority
assessment supplies the displayed value. For shared machines, reasons may
describe differing base IDs or login revisions without implying exclusive
ownership by the reference workspace.

Keep the raw runtime `state` separate. Preserve unfamiliar state strings in
the listing and explain that lifecycle compatibility is unknown; inspection
must remain useful when a lifecycle command would refuse that state.

Describe differences as relative to the current invocation. A different base
may remain useful with another definition or runtime. A custom configuration
may remain useful with different `--allow-host` options. No assessment grants
permission to delete a machine. Listing must not extend repair's candidate
recognition or replacement eligibility.

## Output contract

Sort rows by full machine name. Human output includes NAME, STATE, KIND,
BASE, REVISION, and ASSESSMENT, followed by reasons for differing or unknown
rows. In `--all` output, include a workspace path hash, `shared`, or `unknown`
as the association. Do not invent a filesystem path from its hash. Print the
canonical reference workspace, selected definition when available, desired
workspace machine, and current policy revision above the table. Do not
truncate names or hashes needed to identify machines.

For an empty scope, print an explicit no-matching-machines message and exit
successfully. Scoped output also reports any unassociated workspace count.

`--json` writes one object to stdout, without progress messages or a human
table. Define schema version `1` with these fields:

- `version`: `1`.
- `scope`: `workspace` or `all`.
- `workspace`: canonical reference path.
- `reference`: definition path, base ID, revision, workspace configuration
  hash, and desired machine name; unavailable comparison fields are null.
- `unassociated_workspace_count`: count described above.
- `warnings`: messages about unavailable reference configuration or metadata.
- `machines`: ordered rows containing `name`, `state`, `kind`, `base_id`,
  `revision`, `workspace_path_sha256`, `workspace_sha256`, `configuration`,
  `source`, `assessment`, and `reasons`. Unavailable scalar values are null;
  `reasons` is an array of strings. `source` is `labels`, `legacy-name`, or
  `unknown`.

Empty results use `machines: []`. Inventory discovery or JSON parsing failure
produces a nonzero exit, a diagnostic on stderr, and no partial JSON on
stdout. Contradictory per-machine metadata is a reportable row, not a fatal
inventory failure. Preserve the existing fatal rejection of malformed JSON,
duplicate names, invalid label value types, and missing machine names.

Help and version follow the existing offline contract. Argument errors exit
`2`; successful listings exit `0`; runtime, definition, workspace, and
inventory failures exit nonzero. Unknown rows and unavailable automatic
default configuration do not make a successful listing fail.

## Read-only behavior and compatibility

Use the selected SmolVM's version and machine inventory interfaces. Skip
virtualization and mutation capability preflight so listing works without
`/dev/kvm` access or guest boot capability when inventory is available.
Take no application lifecycle locks. Do not create directories, rewrite
caches, seed credentials, recover port journals, build artifacts, change
permissions, or start, stop, update, or delete machines. The command reports
one inventory snapshot; it does not promise the inventory remains unchanged.

Limit the no-write guarantee to smolsandbox's actions. The external SmolVM
executable may maintain its own registry while servicing inventory queries.

Preserve template bytes, all creation labels, machine names, hash inputs,
`REVISION`, cleanup rules, and repair behavior. Listing does not require a
machine policy revision bump. Keep existing mutation validators strict;
do not make cleanup tolerate ambiguous metadata merely to support inspection.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | Default listing includes all labelled machines for the canonical invocation directory across bases, revisions, profiles, and configurations, and excludes other workspaces and shared machines. | Fixed mixed-inventory integration fixture. |
| AC-02 | Explicit paths, relative paths, symlinks, repository subdirectories, and native Windows paths follow existing workspace semantics. | Workspace resolution and command integration checks on native targets. |
| AC-03 | `--all` includes managed workspace, builder, login, legacy, and custom-name rows; unrelated machines remain excluded. | Mixed-inventory fixture with labelled overrides and unrelated names. |
| AC-04 | Policy, base, configuration, and name differences produce the specified assessments and complete reasons; raw state remains separate. | Independent fixed identity expectations, including multiple simultaneous differences and unfamiliar states. |
| AC-05 | Reference identity agrees with existing start/repair calculations; host option order, duplicates, and ports do not create a second identity rule. | Compare existing fixed creation expectations with listing reference fields; assert `--port` is rejected by list. |
| AC-06 | Missing path metadata is never guessed into the selected workspace; matching-path contradictions remain visible as unknown. | Legacy, missing-label, malformed-label, and contradictory-name fixtures. |
| AC-07 | Missing automatic default permits an unassessed inventory; explicit missing or invalid selected definitions fail without fallback. | Offline configuration-selection fixtures with absent and invalid nearer files. |
| AC-08 | Human and JSON output preserve complete identities, deterministic ordering, reasons, and empty results. | CLI output assertions and parsed JSON schema checks. |
| AC-09 | Listing performs no application writes or lifecycle mutations, works with absent application directories, and succeeds without virtualization preflight. | Snapshot application state and test-double calls; assert only version/inventory operations, with virtualization bypass disabled. |
| AC-10 | Runtime failures, malformed JSON, duplicate names, and invalid inventory fields fail without partial JSON; ambiguous individual rows remain inspectable. | Failure fixtures with stdout/stderr and exit-code assertions. |
| AC-11 | Help documents scope, comparison context, legacy limits, and read-only behavior; active operating documentation includes current-workspace and all-machine examples. | Help tests and documentation review. |
| AC-12 | Existing lifecycle, replacement, purge, identity, and port behavior remains unchanged. | Required repository regression checks and creation-policy diff review. |

## Implementation and evidence

The implementer owns CLI parsing, inventory reporting, identity reuse, output
formatting, independent tests, and active documentation updates. Separate
reportable metadata problems from fatal inventory parse errors without
weakening the existing mutation paths. Prefer existing parsed inventory and
identity inputs over extra runtime calls or a new persistent index.

Implementation is a feature and requires a MINOR version bump in Cargo.toml
and the matching Cargo.lock package entry; from the observed baseline the
next feature version is `0.6.0`. This specification-only change leaves the
software version unchanged.

Run every independent Rust check listed in AGENTS.md after implementation.
Create `FEATURES/feature-0012-validation.md` during implementation with the
tested revision, native target, commands, exit codes, and `passed`, `failed`,
`not_run`, or `inconclusive` for each acceptance criterion. Record new results
and pending native or real guest checks in Feature 0010 validation as required
by the repository instructions. Preserve historical observations. Mocked
listing checks do not establish real guest behavior or baseline removal.

## Scope limits

This feature adds inspection only. It does not add cleanup previews, disk
usage, VM directory discovery, continuous monitoring, workspace path recovery
from hashes, new persistent metadata, guest health probes, automatic deletion,
or broader repair replacement. A future cleanup preview needs its own exact
policy and authorization contract.
