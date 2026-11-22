# Feature 0019: Readable workspace paths and machine identity inputs

## Status

In progress

Creation metadata, readable listing, compatibility handling, and independent
regressions are implemented. [Validation](feature-0019-validation.md) records
local checks and pending native and real SmolVM execution. Acceptance remains
pending while mandatory execution evidence is unavailable.

## Outcome and baseline

Users can identify the workspace associated with a machine in `smolsbx list
--all` and inspect the creation inputs that explain its identity. Machines
retain readable metadata in SmolVM labels, so users can inspect and filter
the inventory without the original repository or a separate application
index. Shared builder and login machines remain explicitly shared.

Specification preparation observed commit
`c2ed7d1bde477e837e1fb720d42044e5e4a15069`, application version `0.12.0`,
and machine policy `REVISION = "5"` on 2026-11-22. The working tree was
clean before specification preparation. The implementer owns label creation,
listing output, compatibility handling, tests, help, operating guidance,
versioning, and validation evidence.

This feature extends [Feature 0012](feature-0012-list-workspace-machines.md).
It supersedes that feature's prohibition on new creation labels and its
hash-only association display. Its inventory scope, reference selection,
read-only behavior, and separation of inspection from mutation remain in
force.

## Source analysis: hashes and existing labels

The following findings describe the observed Rust baseline. Historical
shell specifications do not define current implementation behavior.

### Base generation

[`base_id`](../src/lifecycle.rs) computes:

```text
normalized_version = SmolVM --version stdout with trailing CR/LF removed
base_digest = SHA256(UTF8(normalized_version) + b"\n" + selected_definition_bytes)
base_id = first 12 lowercase hexadecimal characters of base_digest
```

The complete selected file participates, including comments, whitespace,
line endings, harness marker, provisioning commands, resource settings, and
network settings. There is no semantic TOML normalization before hashing.
The definition's filename and location do not themselves enter this hash.
The selected SmolVM executable's path does not enter the hash; its version
output does.

### Workspace configuration and path association

[`Lifecycle::new`](../src/lifecycle.rs) forms this immutable option set:

```text
--from=<artifacts-dir>/codex-devtools-<base-id>.smolmachine
--mem=12288
--cpus=4
--volume=<resolved-harness-home>:<profile-guest-home>
--volume=<canonical-workspace>:/workspace/myrepo
--allow-host=<host>  # one entry per effective host
```

[`domain::hash_set`](../src/domain.rs) sorts these strings, removes exact
duplicates, serializes the array as compact UTF-8 JSON using the existing
serializer, and computes its full SHA-256. The first 12 hexadecimal
characters form the workspace ID in the default machine name. Option order
and duplicates do not affect this hash.

The effective hosts are the exact-string union of the selected definition's
`[network].allow_hosts` and CLI `--allow-host` additions. Case and other
textual differences remain significant. Changing definition bytes can
change the base ID even when the parsed effective host set stays the same.

The independent association hash is SHA-256 of the canonical workspace
path's UTF-8 bytes. It excludes the base generation and creation options.
[`canonical_workspace`](../src/platform.rs) resolves the directory and
normalizes Windows extended path prefixes. A repository subdirectory is a
distinct workspace; repository discovery only selects a definition.

### Existing coverage and required additions

At the observed baseline, all existing keys below use the `codex-sandbox.`
prefix. Bug 0007 changes new creation labels to `SmolSandbox.` while retaining
old-key reads. The six-key ownership contract is declared by `MANAGED_KEYS` in
[`src/domain.rs`](../src/domain.rs). Creation paths are `ensure_base`,
`ensure_login`, and `create_workspace` in
[`src/lifecycle.rs`](../src/lifecycle.rs).

| Value | Effect on identity | Existing label coverage | Required addition |
| --- | --- | --- | --- |
| Kind | Distinguishes builder, login, and workspace ownership. | `kind` on all three kinds. | Preserve. |
| Base ID | Names all three kinds; enters workspace identity through the artifact path. | `base-id` on all three kinds. | Preserve. |
| Machine policy revision | Names login and workspace machines and participates in ownership; not an input to the workspace hash. | `revision` on login and workspace machines; deliberately absent on builders. | Preserve kind-specific semantics. |
| Full immutable workspace hash | Identifies the effective workspace creation option set. | `workspace-sha256` on workspace machines. | Preserve. |
| Canonical workspace path | Enters both the path association hash and workspace volume option. | Only `workspace-path-sha256` on workspace machines; no readable path. | Store the exact canonical path. |
| Effective allow-host set | Each host is a direct workspace hash input. | No host values; `configuration` records only `default` or `custom`. | Store every effective host option, including defaults. |
| Base artifact path | Direct workspace hash input; includes the base ID and resolved artifacts directory. | No path label. | Store the exact `--from` value. |
| Runtime workspace memory and CPUs | Direct workspace hash inputs; currently `12288` MiB and `4`. | No resource labels. | Store the exact `--mem` and `--cpus` values. |
| Harness-home host path and guest target | Direct workspace hash input through the complete volume option. | No mount labels. | Store the complete mapping. |
| Workspace host path and guest target | Direct workspace hash input through the complete volume option. | Only the host path's association hash; no complete mapping. | Store the complete mapping. |
| SmolVM version output | Direct base-generation hash input. | No version label. | Store the exact normalized version string. |
| Selected definition bytes | Direct base-generation hash input, covering all provisioning and definition settings. | Only the resulting short base ID. | Store a full SHA-256 fingerprint of the exact bytes and the selected source path; do not copy the file contents. |
| Harness profile | Encoded in definition bytes and selects the guest-home target and default host state directory. | No explicit profile label. | Store `codex`, `claude`, `opencode`, or `hermes`. |
| Default/custom classification | Derived from whether CLI additions extend definition hosts; not a separate hash input. | `configuration` on workspace machines. | Preserve; do not treat this value as a substitute for the hosts. |

[`Config::resolve_inner`](../src/config.rs) supplies resolved inputs.
`ARTIFACTS_DIR` affects the hashed artifact path. The native application home
and, for Codex, `CODEX_HOME_DIR` affect the hashed harness-home mount.
Relative overrides resolve against the invocation directory; those paths
are not all filesystem-canonicalized. Record the exact strings used by
identity calculation, rather than applying new path normalization.
`--smolfile` and `BASE_SMOLFILE` select the bytes and profile. Store resolved
values rather than an environment dump or the original argument spelling.

Ports are mutable and excluded from immutable identity. `STATE_ROOT`,
verbosity, lifecycle flags, harness arguments after `--`, credentials, and
the application package version do not enter these hashes. Name overrides
`CODEX_MACHINE`, `LOGIN_MACHINE`, and `BUILDER_MACHINE` select actual machine
names without changing these hash algorithms; inventory already exposes
the actual name. Do not claim that every runtime setting is hashed.

Builders have a base ID and no independent creation-options hash. Login
machines have a base ID and revision, but no workspace hash. Login hosts
come from `Profile::login_hosts`, separately from workspace hosts. Record
their creation options without inventing a new identity hash for them.

## Metadata labels

Preserve the six ownership suffixes and their kind-specific values. Use
`SmolSandbox.` for new ownership and descriptive labels, as specified by
[Bug 0007](../BUGS/bug-0007-machine-label-prefix.md). Read the old namespace
for compatibility. Conflicting duplicate ownership values refuse mutation;
conflicting duplicate descriptive values invalidate the descriptive component
without authorizing or denying ownership. Add two descriptive labels. Each
value is a compact JSON object serialized as a single label string, passed
directly through process arguments without a shell. JSON encoding preserves arrays, embedded equals signs, Unicode,
Windows paths, and escaped control characters without comma-delimited or
colon-delimited reconstruction rules.

### `SmolSandbox.base-inputs`

Attach this label to every newly created builder, login, and workspace
machine. The object contains:

| Field | Type and meaning |
| --- | --- |
| `version` | Integer `1`, the label schema version. |
| `smolvm_version` | Exact normalized string used in the base hash, including any prefix or build text; not a parsed semantic version. |
| `definition_sha256` | Full lowercase SHA-256 of the selected definition bytes before parsing or normalization. |
| `definition_path` | Absolute resolved selected-definition path used for this creation request. This is provenance, not an extra hash input. |
| `harness` | Selected profile identifier. |

Use the bytes and version already selected and checked by the lifecycle.
Do not reread a potentially edited definition solely to populate labels.
The definition fingerprint identifies every byte-level input, including
settings that are not separately parsed by smolsbx. Do not flatten arbitrary
Smolfile contents into labels: a user-owned definition may contain secrets.

The fingerprint is not a replacement input to `base_id`. Reproducing the
base ID still requires the original definition bytes and recorded version;
hashing `definition_sha256` with the version produces a different result.
The source path may later be moved, deleted, or contain different bytes.
Listing must not read that path or claim the file still matches.

### `SmolSandbox.machine-inputs`

Attach this label to every newly created machine. The object contains:

| Field | Type and meaning |
| --- | --- |
| `version` | Integer `1`, the label schema version. |
| `workspace_path` | Exact canonical workspace string for workspace machines; `null` for builders and login machines. |
| `options` | Sorted, exact-deduplicated array of creation options, with each entry formatted as `--option=value`. |

For workspace machines, `options` is exactly the canonical array hashed by
`domain::hash_set`: artifact path, memory, CPUs, both complete volume
mappings, and every effective allow-host option. Serializing that array
with the existing hash serializer must reproduce `workspace-sha256`.
Generate hashing, these labels, and create arguments from the same resolved
creation inputs. Future hashed creation options must also enter this array.
Do not maintain a second manually synchronized description of the policy.

For login machines, record the actual `--from`, harness-home `--volume`,
and login-only `--allow-host` options. For builders, record the actual
`--smolfile` option. Do not copy workspace options onto shared machines or
invent explicit CPU/memory options where the command inherits defaults.
Builder policy inside the definition remains represented by its fingerprint.

Exclude `--name` and all `--label` arguments from `options`: names are
already inventory fields and including labels would be circular. Exclude
mutable port updates, guest execution arguments, and secret references.
Metadata records the application's creation request, not a verified dump
of current guest state or subsequent external SmolVM edits.

Both labels are required when this implementation creates a machine.
Creation failure must retain existing lifecycle failure-preservation
behavior. Never retry a failed create by silently omitting metadata.
An existing machine reused through any selection path, including an
explicit name override, retains its original labels; do not overwrite
creation metadata with the current invocation's requested policy.

## Listing and filtering

Preserve the current command interface:

```text
smolsbx list
smolsbx list /path/to/project
smolsbx list --all
smolsbx list --all --json
```

Add `WORKSPACE` and `HARNESS` columns to human output. Display full readable
workspace paths when available, `shared` for valid builder/login rows, and
`unknown` when no path can be established. Retain machine names, identity
hashes, runtime state, assessments, reasons, and the reference header.
For all-workspace output, retain the full association hash as well as the
new readable path. Escape tabs, line breaks, and terminal control characters
in human output so a label cannot introduce extra rows or terminal commands.

For rows carrying supported metadata, print labeled detail lines with the
recorded SmolVM version, definition path and fingerprint, and every creation
option. Users must be able to see the full host set, resources, artifact
path, and mounts without decoding hashes. Do not truncate these values.
Shared rows expose only their own recorded inputs.

Keep JSON report schema `version: 1` and all existing fields and meanings.
Add these fields to every machine row, including legacy rows:

| Field | Contract |
| --- | --- |
| `workspace_path` | Readable canonical workspace string when available, otherwise `null`. |
| `workspace_path_source` | `label`, `reference`, or `unknown`. |
| `harness` | Supported recorded profile identifier, otherwise `null`. |
| `base_inputs` | Decoded supported `base-inputs` object, otherwise `null`. |
| `machine_inputs` | Decoded supported `machine-inputs` object, otherwise `null`. |
| `metadata_status` | `complete`, `partial`, `unavailable`, `invalid`, or `unsupported`, as defined below. |
| `metadata_reasons` | Array of diagnostic strings, empty when metadata is complete. |

These are additive schema changes. Label schema versions are independent
of the report schema. Keep stdout as one JSON object with native strings
and arrays; do not embed the label's JSON as another string in the report.

Retain existing path-based filtering through the optional workspace argument.
Existing `list --allow-host` options extend the reference identity; they
must not become inventory filters. No generic label-filter flag is required
by this feature. Document JSON filtering examples for a recorded path and
an effective host, with exact-string equality and array membership:

```bash
smolsbx list --all --json |
  jq --arg path '/home/me/projects/myrepo' \
    '.machines[] | select(.workspace_path == $path)'

smolsbx list --all --json |
  jq --arg option '--allow-host=packages.example.com' \
    '.machines[] | select((.machine_inputs.options // []) | index($option))'
```

The same metadata is available through SmolVM's raw inventory, independently
of smolsbx's listing formatter:

```bash
smolvm machine ls --json |
  jq --arg path '/home/me/projects/myrepo' '
    .[] |
    (.labels["SmolSandbox.machine-inputs"]? | fromjson?) as $inputs |
    select($inputs.workspace_path == $path)
  '
```

These commands describe the implemented feature, not current baseline output.
`jq` is an optional user tool; do not add it as an application dependency.

## Validation of descriptive metadata and older machines

Keep descriptive metadata separate from `MANAGED_KEYS`. Appending the new
keys to that positional array would incorrectly make workspace fields
mandatory on older machines or forbidden on shared machines. Review
`classify`, `managed_inventory`, removal ownership checks, listing
validation, and lifecycle cleanup paths for namespace assumptions.

Validate each supported label's field types, required fields, fingerprint
shape, and kind-specific semantics. For a workspace snapshot, verify that
the recorded path hashes to the existing valid `workspace-path-sha256`,
that the workspace volume contains that exact path and the existing guest
target, and that the canonical option array hashes to the existing valid
`workspace-sha256`. Compare the recorded `--from` value's base-generation
filename with the existing `base-id`. Use complete strings; do not parse
Windows drive letters by splitting every colon. A hash match establishes
consistency between labels, not authenticity or actual guest configuration.

Define metadata status with this precedence:

1. `invalid`: a supported label has malformed JSON, missing/wrongly typed
   fields, invalid option structure, or a detectable contradiction.
2. `unsupported`: no supported label is invalid, but a label declares an
   unknown schema version.
3. `complete`: both labels are present, supported, and valid for the kind.
4. `partial`: exactly one label is supported and valid and the other is absent.
5. `unavailable`: neither label is present.

Return `null` for an invalid or unsupported component while preserving any
independently valid component. Report the machine name and affected component
in `metadata_reasons`. Do not echo arbitrary malformed payloads into errors.
Unknown extra object fields do not invalidate a supported version.

If a valid workspace snapshot supplies a consistent path, set
`workspace_path_source` to `label`. Otherwise, when the existing valid path
hash equals the canonical reference workspace hash, the formatter can
display the reference path with source `reference`. This fallback is an
explicit inference from the existing association hash, not recovered
creation metadata. It does not fill in other missing creation inputs or
change `metadata_status`. Otherwise return `null` with source `unknown`.
Shared rows always use a null workspace path and source `unknown` in JSON;
the human table displays `shared` only when existing ownership permits it.

Do not inspect another workspace's filesystem, query guest mounts, search
the host for hash candidates, or read historical Smolfiles to infer missing
metadata. An old machine for another workspace may remain unreadable until
its known path is supplied as the reference or a new machine is created.
Do not add automatic backfilling or a migration command in this feature.

Missing, malformed, unsupported, or contradictory descriptive metadata
must not change existing ownership, association scope, assessments,
reconciliation, replacement eligibility, or deletion authorization.
In particular, descriptive labels alone cannot establish positive ownership
or repair contradictory authoritative labels. Keep malformed inventory
JSON, non-string label values, duplicate names, and existing authoritative
field validation fatal under their current contracts. A string containing
invalid metadata JSON is a per-row metadata issue, not a fatal inventory
failure. Keep old machines usable according to their existing policy.

Listing still calls only version and inventory interfaces, takes no
application lifecycle locks, and performs no application writes or machine
updates. It needs no virtualization preflight or guest boot. It must work
when application directories, referenced definitions, artifacts, and other
workspace paths are missing, subject to existing reference-selection rules.

## Identity compatibility and versioning

Preserve definition bytes, templates, base-ID hashing, workspace option
serialization, path hashing, mount spelling, host normalization, and name
override semantics. The descriptive labels do not enter an identity hash.
No persistent application registry or secondary allowlist is introduced.

Under [AGENTS.md](../AGENTS.md), changing Rust machine-creation policy for
unchanged definition bytes requires a `REVISION` increment. Adding the new
creation labels is such a change. Increment `5` to `6` from the observed
baseline, or use the next revision if the implementation baseline advances.
Update the independent revision expectations in
`startup_has_independent_names_labels_order_arguments_and_exit_status` and
`creation_uses_selected_default_hosts_plus_normalized_additions_and_revision`.

The revision increment changes default login/workspace names and ownership
revision labels; it does not change the workspace hash for an otherwise
identical option set. Builders remain revisionless. Existing machines keep
their creation-time policy and metadata. Existing stale-machine checks and
explicit replacement flows still apply; do not silently replace machines
to populate labels. Reused cached artifacts and existing builders need not
be rebuilt solely to attach metadata.

Implementation requires a MINOR version bump in `Cargo.toml` and the
matching `smolsandbox` entry in `Cargo.lock`: `0.13.0` from this baseline.
If the baseline advances, derive the bump from that version. This
specification-only change leaves software version `0.12.0` unchanged.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | Newly created builders, login machines, and workspace machines carry both supported metadata labels with their actual selected profile, normalized runtime version, source path, and byte fingerprint. | Independent creation-argument and inventory expectations for all four profiles and all three machine kinds. |
| AC-02 | Workspace metadata contains every direct hash input: `--from`, `--mem`, `--cpus`, both volumes, and every effective `--allow-host`; hashing the recorded array reproduces the existing full hash. | Fixed independently constructed canonical arrays and digests; compare metadata against actual create arguments without calling production helpers to construct expectations. |
| AC-03 | Host order and exact duplicates leave the hash and metadata option array unchanged; a new host appears once and changes the hash. Login hosts remain separate. | Extend `allowlist_is_a_set_and_ports_do_not_change_immutable_hash` and the independent selected-host creation regression. |
| AC-04 | Artifact-directory, harness-home, workspace, resource, profile-target, definition-byte, and runtime-version differences are represented by the recorded values or definition fingerprint and affect identity exactly as before, apart from the required revision increment. | Fixed input-change cases, existing hash fixtures, and a byte-only Smolfile edit with unchanged parsed settings; exercise resource variants at the identity boundary without adding new CLI flags. |
| AC-05 | Ports, state-directory overrides, verbosity, credentials, and guest arguments do not enter the metadata hash array; name overrides affect the actual name without relabeling an existing machine. | Existing port/override regressions plus retained-label assertions and definition-content secret sentinel checks. |
| AC-06 | Human listing shows full workspace paths, profiles, recorded base inputs, hosts, resources, and mappings; shared rows are marked shared and older unknown paths stay unknown. | Mixed old/new inventory command fixtures, deterministic ordering, empty results, and terminal-control escaping checks. |
| AC-07 | JSON retains schema version 1 and all old fields; every row exposes the new typed fields with specified nulls, status, provenance, and diagnostics. The documented path/host selection expressions select the intended rows. | Parsed output contract assertions and independent predicate checks against mixed fixtures; validate the published jq examples when jq is available. |
| AC-08 | Path/hash, options/hash, and base-filename contradictions are reported as metadata errors without changing authoritative assessments or mutation decisions. Malformed and future metadata remain inspectable. | Alter each descriptive field independently; compare listing scope and lifecycle/removal plans with and without the optional labels. |
| AC-09 | Missing metadata does not make legacy machines unusable. Matching reference hashes permit only the documented path fallback; other-workspace hashes never produce guessed paths. | Old inventory fixtures across generations, profiles, custom names, partial metadata, and missing historical paths. |
| AC-10 | Listing does not write, lock, update, or require virtualization, and does not read historical definition paths or other workspace directories. | Existing listing no-write/call-log tests, missing-file fixtures, and a snapshot of application and machine state. |
| AC-11 | Metadata round-trips exact strings containing spaces, Unicode, equals signs, backslashes, and JSON-escaped controls; paths retain existing Unix symlink and Windows normalization semantics. | Serializer/inventory/output fixtures and native path checks on supported targets. |
| AC-12 | Creation policy uses the next revision, versions receive the required matching minor bump, and old machine preservation and create-before-delete behavior remain intact. | Independent revision/name expectations, manifest/lockfile review, replacement failure-preservation checks, removal/purge regressions, and all seven repository Rust checks. |
| AC-13 | Supported real SmolVM preserves both JSON-valued labels through create, list, stop/start, and deletion of a disposable fixture, with no truncated values. | Record runtime version, native target, profile, exact commands, and observed raw and smolsbx JSON, including a long host set and native path quoting. Record native execution evidence separately from mocked checks. |
| AC-14 | Help and active operating guidance explain readable paths, all-workspace inspection, recorded versus current state, old-machine limits, structured labels, and JSON filtering. | Help contract checks and documentation review against the final implementation. |

## Implementation and evidence

Keep the creation inputs in one representation shared by hashing, create
arguments, and metadata serialization. Preserve existing fixed hash
expectations when adding this representation. Test the relationship between
the three outputs with independent expected values rather than copying the
production hash calculation into every assertion.

Review the network and identity regressions named in AGENTS.md and the
[Feature 0009 coverage map](feature-0009-coverage.md). Review
`replacement_prepares_and_verifies_before_deleting_stale_machine` and its
failure-preservation checks for the revision transition. Update exact-label
expectations deliberately; do not weaken them to subset-only assertions
that would miss absent metadata.

During implementation, create
`FEATURES/feature-0019-validation.md`. Record the tested revision, application
version, revision policy, host target, runtime version, commands, exit codes,
and AC-01 through AC-14 results using `passed`, `failed`, `not_run`, or
`inconclusive`. Run all seven independent Rust checks listed in AGENTS.md,
including both feature configurations, the production release build without
`test-adapter`, and baseline independence. Append new results and pending
native or real SmolVM checks to
[Feature 0010 validation](feature-0010-validation.md), preserving historical
observations and source revisions. Keep acceptance pending while any
mandatory criterion lacks passing evidence.

SmolVM's current upstream
[`machine` CLI source](https://github.com/smol-machines/smolvm/blob/main/src/cli/machine.rs)
documents create labels and JSON inventory; its
[`parse_labels` implementation](https://github.com/smol-machines/smolvm/blob/main/src/util.rs)
treats label values as opaque strings split only at the first equals sign.
These sources were inspected during specification preparation. They do not
establish behavior of an installed runtime. Implementation must verify the
supported SmolVM's round-trip behavior rather than infer it from current
upstream alone. This feature uses create labels and inventory already
required by preflight; it does not require runtime label-update support.

## Scope limits

This feature adds creation-time descriptive metadata and readable inspection.
It does not add generic user tags, new resource flags, a label-filter CLI,
automatic metadata backfills, a reverse-hash filesystem index, Git remote or
branch discovery, guest-state attestation, machine renaming, mutable network
policy, automatic deletion, or expanded cleanup authorization. Full user-owned
Smolfile contents and credentials are never copied into labels or reports.
