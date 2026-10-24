# Feature 0004: Replace shell scripts with a cross-platform Rust application

## Status

Proposed

The configuration-location decision is recorded below. Provisioning-download
integrity is out of scope and tracked separately in
[Feature 0008](feature-0008-provisioning-download-integrity.md); it does not
block this migration.

## Summary

Replace the repository's Bash and Python lifecycle implementation with one
cross-platform Rust executable named `codex-sandbox`. Users may copy the
executable anywhere and run it without an installation directory, companion
resources, Bash, Python, or a Rust toolchain on the host.

The executable uses the current user's `~/.smolsandbox` directory for persistent
state and user-wide sandbox definitions. Definitions are named Smolfiles such
as `~/.smolsandbox/sandbox-name.smlbx`; a repository may provide files with the
same names in its own `.smolsandbox` directory. Resolve definitions nearest
first: repository, then user home. Select one complete file without merging.
A new `create-config` command writes `~/.smolsandbox/default.smlbx` from a
built-in template for exactly one of four harnesses: Claude, Codex, OpenCode,
or Hermes Agent. Lifecycle commands accept `--smolfile <name-or-path>` to
select another definition.

The work is a replacement of the existing sandbox scripts, with configuration
generation and those four harness choices as the bounded additions. Preserve
reconciliation, create-before-delete replacement, network allowlists, mutable
port publishing, cleanup, and purge behavior. Do not turn this migration into
a new configuration framework, diagnostics product, or provisioning-integrity
project.

Users install SmolVM separately. The selected harness and development tools
run inside the prepared guest, not on the host. The guest may continue to use
shell scripts and Python during provisioning; removing host script dependencies
does not require rewriting guest tools.

There is no migration from repository-local shell-era state. The Rust cutover
starts with a fresh application home. The old production scripts and their
script-based tests and fixtures are removed only after Rust parity passes.

## Goals

- Replace the host Bash/Python lifecycle implementation and its tests with
  Rust on Linux, macOS, and Windows wherever SmolVM supports the host.
- Deliver one executable that works when copied to an arbitrary directory.
- Keep persistent state and user-wide definitions under `~/.smolsandbox`, with
  repository-local definitions taking precedence by filename.
- Generate a usable `.smlbx` Smolfile for Claude, Codex, OpenCode, or Hermes Agent.
- Preserve existing lifecycle outcomes, inventory validation, ownership,
  locking, secret forwarding, and interactive cleanup guarantees.
- Port every implemented feature and existing behavioral test.
- Keep SmolVM behind a subprocess adapter and platform behavior behind
  native filesystem, process, and locking interfaces.

## Non-goals

- Additional harnesses, third-party harness plugins, or arbitrary host command
  execution through a harness setting.
- Installing a harness on the host or implementing harness features such as
  conversations, model selection, or credential management.
- New provisioning-download verification rules; see Feature 0008.
- A `doctor` command, workspace TOML configuration, or a workspace trust store.
- A new release-signing or build-provenance system as a migration requirement.
- Bundling, redistributing, or directly linking SmolVM.
- Shipping or downloading prebuilt `.smolmachine` guest artifacts.
- Retaining the old `.sh` entry points or providing compatibility wrappers.
- Importing, merging, or deleting shell-era `artifacts/` or `.smolvm/` state.
- Self-update, native installers, package-manager integration, or telemetry.
- Synchronizing application state between different host operating systems.

## Recorded design decisions

### Portable executable and user-owned Smolfile

The runtime default filename is `default.smlbx`. Look for it first in the
repository's `.smolsandbox` directory, then in the native user's
`~/.smolsandbox` directory. Named selections use the same order. Explicit
paths bypass this lookup. The executable's location never affects discovery;
no definition or resource beside the executable is required. The full
selection rules are specified under Configuration and application home.

The executable includes the template data needed by `create-config`. These
are generation inputs, not an implicit runtime fallback. Lifecycle commands
read the actual user-owned file; they do not silently generate it, download a
replacement, or overwrite edits. A missing definition produces an error that
names the searched paths and gives a `create-config` example.

Template revisions are coupled to application releases. Updating or moving the
executable does not update an existing `.smlbx` file. Generating a configuration
requires an explicit command, and an existing file is never overwritten by
`create-config`.

The complete effective file bytes, including harness selection metadata,
participate in the base ID with the selected SmolVM version output. Changing
the definition therefore selects a new sandbox generation. The executable
path and unused built-in templates do not participate in identity.

### Provisioning-download integrity is deferred

Feature 0004 preserves the provisioning behavior and checks already present
in the source definition. It introduces no new global checksum, signature,
installer-verification, or dependency-locking policy. Existing verification
must not be removed as part of the port.

[Feature 0008: Provisioning-download integrity](feature-0008-provisioning-download-integrity.md)
contains that separate work. Its policy decisions and implementation are not
prerequisites for Feature 0004. Existing build failure handling, non-empty
artifact validation, atomic publication, and safe reuse remain required here.

## Relationship to earlier features

The implemented behavior at cutover is the parity baseline:

- Feature 0001: authoritative identity, inventory validation, reconciliation,
  compatibility caches, explicit repair, and non-destructive startup.
- Feature 0002: prepare and freshly verify the desired replacement before
  stopping or deleting an eligible stale machine.
- Feature 0003: inventory-aware purge, conservative/force/force-all policies,
  running-generation protection, and conservative purge after a build.
- Feature 0005: repeatable additional network hosts and identity derived from
  the complete immutable workspace-machine creation specification.
- Feature 0006: useful help that exits without lifecycle work.
- Feature 0007: exact-set port reconciliation on the same persistent machine,
  including manifests, recovery journals, and update-before-start locking.

This feature changes the host language, command spelling, default state
location, and supported harness selection. It does not weaken the existing
safety invariants. Later implemented shell changes, including workspace-only
cleanup, must also be included in the cutover parity baseline.

The subprocess integration boundary is supported by
[the SmolVM embedding assessment](../RESEARCH/2026-09-20-smolvm-embedding.md).
The recorded decision is to use an independently installed executable rather
than depend on SmolVM's internal engine modules.

## Terminology

- **Application home:** `~/.smolsandbox`, resolved using the current user's
  native home directory rather than a shell expansion.
- **Repository definition directory:** `.smolsandbox` at the root of the
  repository containing the selected workspace, or containing the current
  directory for commands without a workspace argument.
- **Harness:** The guest coding-agent CLI selected by the sandbox definition.
  This feature supports only Claude, Codex, OpenCode, and Hermes Agent.
- **Harness profile:** Built-in template and guest launch/state conventions
  for one supported harness. It is not a plugin interface.
- **Effective Smolfile:** The complete repository-local or user-owned definition
  selected by the filename lookup or an explicit path. Definitions are never
  merged, inherited, or supplemented from another candidate file.
- **Desired base ID:** The first 12 lowercase hexadecimal characters of the
  SHA-256 hash over the selected SmolVM version output, a newline, and the
  complete effective Smolfile contents.
- **Workspace hash:** The SHA-256 hash of the canonical set of immutable
  machine-creation options, as defined by Feature 0005. The canonical workspace
  path is included through its mount; requested ports are excluded.
- **Workspace path digest:** The separate path-only digest used to associate
  machines with a workspace across generations and configurations.
- **Workspace ID:** The first 12 lowercase hexadecimal characters of the
  workspace hash.
- **Semantic compatibility:** Equivalent lifecycle outcomes, safety checks,
  side effects, command ordering, and exit behavior, allowing native path and
  operating-system diagnostic differences.

## Supported hosts and distribution

Build a native executable for each documented Linux, macOS, and Windows
operating-system/architecture combination supported by the selected SmolVM.
Target x86-64 and ARM64 where both Rust and SmolVM support the combination.
Unsupported hosts fail capability preflight with an actionable error.

An executable may be distributed in a platform archive, but extracting and
copying that executable alone must be sufficient. No adjacent Smolfile,
resource directory, installer, fixed working directory, or checkout is needed.
The binary exposes `--version` and `--help`.

SmolVM is the only required host executable. Native Rust libraries implement
hashing, JSON, file operations, path handling, locking, and process lifecycle.
Do not invoke host Bash, Python, `sha256sum`, `shasum`, `cp`, or `mv`.
Provisioning the guest may require network access; offline first use is not a
goal. Release signing and provenance infrastructure are not acceptance gates
for this script replacement.

## Command-line interface

```text
codex-sandbox create-config --harness <claude|codex|opencode|hermes>
codex-sandbox build [workspace]
codex-sandbox start [--allow-host HOST]... [--port PORT]... [workspace] [-- HARNESS_ARG...]
codex-sandbox login
codex-sandbox repair (--check | --replace-stale [--force]) [--allow-host HOST]... [workspace]
codex-sandbox clean [--workspace [workspace]]
codex-sandbox purge [--force | --force-all]
codex-sandbox --version
codex-sandbox --help
```

Every lifecycle command (`build`, `start`, `login`, `repair`, `clean`, and
`purge`) accepts `--smolfile <name-or-path>` before the standalone `--`.
For example:

```text
codex-sandbox start --smolfile claude.smlbx /path/to/project
codex-sandbox build --smolfile ./configs/custom.smlbx /path/to/project
```

Each command accepts `--help` without requiring a configuration file, SmolVM,
or lifecycle work. Version output has the same independence. Workspace defaults
and the first standalone `--` retain their current meaning: everything after
the delimiter belongs to the selected guest harness.

| Rust command | Replaced shell behavior |
| --- | --- |
| `create-config` | New generation of the user-owned sandbox definition |
| `build` | `build-sandbox.sh` and its `build-login.sh` alias |
| `start` | `start-codex.sh` and its `codex-sandbox.sh` alias |
| `login` | `start-login.sh`, with harness-specific authentication |
| `repair` | `repair-sandbox.sh` |
| `clean` | `clean-sandbox.sh`, including workspace-only cleanup |
| `purge` | `purge-sandbox.sh` |

Keep the executable name `codex-sandbox` for this migration; selecting another
harness does not select a different executable. No `doctor` command is added.
Unknown options, unsupported harness identifiers, conflicting modes, and excess
arguments are usage errors with exit `2`. `login` rejects positional arguments
instead of preserving the old script's accidental argument ignoring.

## Configuration and application home

The default layout is:

```text
~/.smolsandbox/
  default.smlbx
  sandbox-name.smlbx
  artifacts/
  state/
    ports/
  locks/
  codex-home/
  harness-home/
    claude/
    opencode/
    hermes/
```

The definition files shown above are examples; additional named `.smlbx` files
may coexist. A repository may supply its own definitions:

```text
<repository>/.smolsandbox/
  default.smlbx
  sandbox-name.smlbx
```

Resolve the effective Smolfile as follows:

1. Use `--smolfile <name-or-path>` when supplied. Otherwise, preserve
   `BASE_SMOLFILE` as an explicit path override. If neither is supplied, select
   the filename `default.smlbx`.
2. A flag value that is only a filename is looked up exactly as given, first
   at `<repository>/.smolsandbox/<filename>`, then at
   `~/.smolsandbox/<filename>`. Do not append an extension. Resolve the
   repository from the selected workspace, including `clean --workspace`;
   when no workspace is selected, use the current directory. Outside a
   repository, search only the user's definition directory. Do not search
   definitions beside the executable or in unrelated repositories.
3. A flag value containing a path delimiter (such as `./custom.smlbx`,
   `../configs/custom.smlbx`, or an absolute path) is an explicit path. Honor
   native absolute-path and separator conventions, including Windows paths.
   Resolve relative paths against the invocation's current directory, without
   prepending either `.smolsandbox` directory or falling back to filename
   lookup. `BASE_SMOLFILE` retains these explicit-path semantics even when
   its value is a bare filename.
4. Select the first existing candidate as one whole file. Only absence allows
   continuing to the next directory. An unreadable, malformed, or unsupported
   nearer definition is an error, not a reason to fall back. Never merge
   contents, harness metadata, or settings between files. A missing named
   file does not fall back to `default.smlbx`.
5. Report the selected path in configuration diagnostics. If lookup fails,
   report the candidate paths; if an explicit path fails, report that path
   without trying repository or home alternatives.

`codex-home` preserves the Codex state convention. Other profiles use separate
persistent homes with their required guest mounts. Only the selected profile's
credential-bearing state is mounted or seeded. Clean and purge preserve these
homes and all user-owned or repository-local definition files.

Preserve existing applicable environment overrides: `BASE_SMOLFILE`,
`ARTIFACTS_DIR`, `STATE_ROOT`, `CODEX_HOME_DIR`, `BUILDER_MACHINE`,
`LOGIN_MACHINE`, `CODEX_MACHINE`, `TMPDIR`, and `OPENAI_API_KEY`.
`BASE_SMOLFILE` explicitly selects an alternative definition; `--smolfile`
takes precedence over it, and both take precedence over default filename
lookup. `create-config` always creates `default.smlbx` in the application home
and does not follow `BASE_SMOLFILE` or repository-local discovery. The
`--smolfile` flag selects definitions for lifecycle commands, not generation.
`SANDBOX_ENSURE_LOGIN` remains an internal shell detail and is not supported.

Do not add user/workspace TOML files or search for provisioning inputs outside
the definition lookup described above. The previous draft's `.codex-sandbox`
application home, `CODEX_SANDBOX_HOME` override, and workspace trust store are
superseded by this design. Explicit component overrides remain available.

Select SmolVM with `--smolvm <path>`, then `SMOLVM_BIN`, then `PATH`.
Canonicalize the selected executable; never silently substitute a different
executable after an explicit selection fails.

No generated state is written into the workspace by default. Application-owned
state and port manifests are versioned and written atomically. Unknown newer
formats fail without rewriting them. The application home is host-local and
is not imported from shell-era state.

## Configuration generation and harness selection

`create-config` requires one `--harness` choice. Accepted identifiers are:

| Identifier | Harness |
| --- | --- |
| `claude` | Claude |
| `codex` | Codex |
| `opencode` | OpenCode |
| `hermes` | Hermes Agent |

For example:

```text
codex-sandbox create-config --harness codex
codex-sandbox build /path/to/project
codex-sandbox start /path/to/project -- --help
```

The command shall:

1. Validate the choice before creating directories or files.
2. Render the complete definition from built-in template data, including the
   selected harness's guest installation and provisioning defaults.
3. Record the profile identifier in application-readable metadata compatible
   with the Smolfile format, so the one file is sufficient to identify the
   guest harness. No separate profile registry is required.
4. Create the application home if needed and publish `default.smlbx` atomically
   without replacing any existing file, including a concurrent creation.
5. Print the created path, selected harness, and next build/start commands.

Generation works without SmolVM, host harness installations, credentials, or
network access. It does not provision a guest, query inventory, create a
machine, authenticate, or download templates. Invalid choices exit `2`;
filesystem failures and an existing destination fail non-zero without changing
that destination.

The user may inspect and edit the generated file. Lifecycle commands validate
the selected definition and profile metadata before mutation. A missing file,
unreadable or malformed definition, or missing/unknown profile fails with an
actionable error; none silently defaults to Codex. An explicitly overridden
Smolfile has the same profile contract.

Each profile supplies the guest installation, launch command, persistent-state
mounts, seed locations, and any supported authentication invocation needed to
use that harness. Share lifecycle policy across profiles. Do not reuse Codex
flags, `OPENAI_API_KEY`, login commands, or state paths for another harness
without that profile's documented support. If a harness has no separate login
workflow, `login` reports that and its supported authentication approach before
starting or creating any machine. Credentials remain the harness's concern.

The fixed profile set is the entire harness scope. There is no plugin loader,
automatic agent discovery, model/provider configuration UI, or mechanism to
install arbitrary harnesses through this command.

## Identity, guest preparation, and state

Use native canonical workspace paths with deterministic UTF-8 encoding.
Equivalent spellings on one host must select the same identity. Windows drive
and filesystem case behavior require native handling. Cross-OS hash equality
is not required.

Preserve the current managed naming and labels, including:

```text
codex-builder-<base-id>
codex-login-<base-id>-r<revision>
codex-workspace-<base-id>-r<revision>-<workspace-id>
```

Preserve the path-only workspace label, full immutable-policy hash,
configuration classification, runtime revision, strict legacy-name recognition,
and explicit override rules. The workspace remains mounted at
`/workspace/myrepo` inside the Linux guest.

The base ID hashes the actual selected Smolfile bytes and SmolVM version.
The selected filename, source directory, and path do not contribute to the
base ID: identical complete definitions select the same base regardless of
where they were found. Unselected files do not contribute to identity.
Workspace identity includes canonical immutable creation options: artifact,
CPU/memory allocation, all mounts, and effective default-plus-user allowlist.
Different harness definitions therefore select separate generations and state
mounts. Port choices remain mutable and never change identity.

Build the base locally through SmolVM, retaining existing preparation,
temporary publication, non-empty validation, and reuse rules. Do not silently
update a user's definition when building or starting. Preserve existing
provisioning pins and verification behavior when transferring the current
Codex definition into its template. New profiles define their provisioning in
their templates without adding a migration-wide integrity policy.

Seed missing persistent harness state without overwriting existing files.
For Codex, preserve `/opt/codex-home` seeding and its warning-only failure
behavior. Never log or persist API-key values in application configuration or
command arguments. Preserve Codex secret forwarding through SmolVM by
environment-variable name. Apply native user-private permissions to
credential-bearing state and retain separation between harness homes.

## SmolVM adapter and internal boundaries

Invoke the independently installed SmolVM through one adapter. Before lifecycle
mutation, check its canonical path, version, required operations, JSON output,
and available host virtualization prerequisites. An incompatible executable
fails before creation, replacement, update, or deletion.

The adapter owns argument construction, inventory capture, TTY attachment,
secret forwarding, exit-status translation, and subprocess failures. Platform
interfaces own native paths, filesystem operations, permissions, signals, and
locks. Pure typed domain logic owns identity, inventory classification,
readiness, replacement plans, and cleanup eligibility. The lifecycle service
executes those plans in the required order. Harness profiles supply only the
harness-specific provisioning and guest process/state conventions.

Inventory remains untrusted input. Preserve all current validation, including
UTF-8/JSON shape, unique names, state and label types, complete and consistent
ownership, creation timestamps, and port counts. Invalid or unavailable
inventory must never become an empty inventory or authorize mutation.

## Lifecycle behavior

### `build`

Resolve the effective definition, selected profile, workspace, and compatible
SmolVM. Reconcile non-destructively, ensure the current base, workspace, and
supported login machine, and print next commands. Preserve stale-only refusal
and ambiguity rules. Run Feature 0003's conservative post-build purge; its
refusal does not turn a successful build into a failure.

### `start`

Preserve repeatable `--allow-host` and `--port` parsing, validation, and
delimiter behavior. Reconcile immutable identity first. Reconcile the exact
requested port set through Feature 0007's manifest/journal protocol while
holding the existing mutation guarantees, then start the same selected
machine. Omitting all ports clears earlier mappings. A running machine with a
different or unknown set is never stopped just to change ports.

Seed the selected profile's persistent state and run its interactive guest
command with arguments after `--` passed unchanged. Codex retains current
Codex-specific launch flags and secret forwarding. Return the harness exit
status unless startup fails or cleanup is the only failure. Best-effort stop
a machine started by this invocation on completion or interceptable
termination; preserve the existing handling of an already-running machine.

### `login`

For profiles with a separate authentication command, confirm an existing
current-generation login machine from fresh valid inventory, start it, seed
its state, and invoke the profile's interactive authentication command. Do not
create a missing login machine. For Codex retain `codex login --device-auth`.
Stop the machine best-effort after completion or interceptable termination.
For profiles without a separate workflow, use the earlier profile refusal
contract without lifecycle mutation.

### `repair`

`--check` is read-only and lock-free. `--replace-stale` preserves Feature 0002's
preflight, preparation, fresh readiness gate, eligible stale cleanup, and final
cache publication. `--force` permits stopping a running stale machine only
after the desired replacement passes readiness. Repeat the same additional
hosts to address the same immutable configuration.

### `clean`

Default cleanup preserves the current generation-wide scope and ownership
checks. `--workspace [workspace]` preserves the implemented workspace-only
scope across generations and configurations. Preserve unrelated resources,
harness state, and the user's definition. Remove local port state only after
the corresponding machine has been successfully deleted.

### `purge`

Port Feature 0003 completely: conservative refusal, mutually exclusive
`--force` and `--force-all`, inventory/plan revalidation, artifact eligibility,
protection of running machines and their generations, current artifact
retention, and interrupted-builder recovery. Never stop a running machine as
part of purge.

## Concurrency, failures, and cleanup

Preserve workspace serialization and the artifact/purge lock with native
exclusive file locks. Keep the observable 60-second timeout. Refresh inventory
after locking; hold replacement locks for the full handoff and port/start
locks through verification and machine start. Never continue unlocked after a
failure. Native lock release on process exit may replace stale-PID handling.

`repair --check`, help, and version output perform no lifecycle or state
mutation. `create-config` performs only its explicitly requested local file
creation and does not require lifecycle locks or SmolVM.

Support interactive I/O, terminal resize where available, Ctrl-C, child exit
status, and interceptable termination notifications. Shutdown is best-effort;
forced termination and host crashes cannot guarantee a machine stop. Preserve
the guest failure as primary when cleanup also fails, and report cleanup
separately. If only cleanup fails, return an operational failure.

Human-readable output distinguishes success, warning, usage, and operational
failure without requiring color. Partial cleanup reports completed and
unprocessed work without claiming rollback. Never print secrets.

## Testing and validation

Port all existing behavioral tests to Rust, using a cross-platform Rust SmolVM
test double instead of Bash/Python fixtures. Cover identity, malformed
inventory, reconciliation, replacement readiness and ordering, clean, purge,
allowlists, ports, help, API-key forwarding, guest seeding, locking,
interruption, and failure precedence.

Add direct tests for the bounded new behavior:

- Executable relocation and execution from an unrelated working directory.
- Native application-home discovery and explicit component overrides.
- Default and named `.smlbx` lookup: repository before home, home-only lookup
  outside a repository, and repository selection from the workspace rather
  than the executable location.
- Whole-file selection with no merging; invalid or unreadable nearer files
  fail instead of falling back, and missing names never select the default.
- `--smolfile` precedence over `BASE_SMOLFILE`, exact filename matching, and
  explicit absolute/relative paths bypassing repository/home lookup on each
  supported host, with relative paths based on the invocation directory.
- Identical bytes at different filenames/locations producing the same base
  ID, and edits to unselected definitions leaving identity unchanged.
- All four template choices, profile selection, and launch/state isolation.
- Offline configuration creation without SmolVM or host harness tools.
- No-clobber behavior, including concurrent creation and filesystem failures.
- Missing or invalid definitions and unsupported profiles failing before
  lifecycle mutation.
- User edits and existing state surviving executable updates and relocation.
- Base identity using the actual file bytes, not executable/template versions.
- Harness-specific argument forwarding and supported/unsupported login paths.

Before deleting old files, run Linux differential tests against the script
implementation for equivalent Codex configurations. Compare plans, operation
order, labels, names, exit categories, and normalized output. Normalize only
the explicit CLI, home-location, profile, and native-path changes specified
here; other differences require a fix or specification amendment.

Run the mocked suite on every supported release target. Before cutover, run
real SmolVM smoke tests for guest build, inventory, creation, interactive
start, stop, and cleanup, including each supported harness. Validate that a
copied binary runs without adjacent resources or host Bash/Python. New
provisioning-integrity tests belong to Feature 0008 and are not a parity gate.

## Documentation requirements

At cutover, update the README and operator instructions for copying/running
the executable, SmolVM installation, the supported host matrix, the
`~/.smolsandbox` layout, `create-config` choices and no-clobber behavior,
user-editable definitions, repository-before-home lookup without merging,
`--smolfile` filename/path selection, override precedence, and old-script/subcommand
mapping. Include examples for allowlists, port publishing, repair, clean,
purge, and each harness's launch/authentication conventions.

Explain that harnesses run in the guest, existing configuration is never
automatically refreshed, shell-era state is not imported, and provisioning
integrity improvements are separately tracked by Feature 0008. Do not require
host agent installations or claim that SmolVM is bundled.

## Acceptance criteria

1. The feature is `Proposed`; Feature 0008 is referenced as separate work and
   is not an implementation or release prerequisite.
2. A single native executable runs from any directory without companion
   resources, host Bash/Python, or an end-user Rust toolchain.
3. Default discovery selects `default.smlbx` from the repository's
   `.smolsandbox` before `~/.smolsandbox`, independently of binary location.
   `--smolfile` takes precedence over `BASE_SMOLFILE`; bare flag filenames use
   the same nearest-first lookup, while explicit paths bypass it. Exactly one
   whole file is used, without merging or fallback from an invalid nearer file.
4. `create-config --harness` accepts exactly `claude`, `codex`, `opencode`, and
   `hermes`, with the documented display names.
5. Every choice generates a complete valid Smolfile identifying its profile
   without network access, SmolVM, credentials, or host harness tools.
6. Missing/invalid choices fail before file creation; existing files and
   concurrent creations are never overwritten; failed writes publish no
   partial destination.
7. Missing, unreadable, malformed, or unsupported definitions fail before
   lifecycle mutation with actionable diagnostics.
8. Moving/updating the executable leaves user edits, selected profile, and
   persistent state intact. The actual selected definition determines base
   identity, and requested ports remain excluded from workspace identity.
9. All mapped lifecycle commands preserve the current implemented behavior of
   Features 0001, 0002, 0003, 0005, 0006, and 0007 and later shell changes in
   the parity baseline.
10. Normal reconciliation remains non-destructive; replacement verifies the
    desired machine before stale cleanup; inventory failures never authorize
    mutation; all relevant operations remain serialized.
11. Harness launch, argument forwarding, state mounts, and authentication use
    the selected profile. Another harness's credential state is not mounted
    and Codex-only flags/secrets are not applied by default to other profiles.
12. Help/version work without configuration or SmolVM. Usage errors exit `2`;
    interactive processes preserve guest status and cleanup failure precedence.
13. Clean and purge preserve the definition and harness homes, respect their
    current scopes, and remove port state only after successful machine deletion.
14. Existing provisioning pins and verification checks are preserved; no new
    download-integrity policy is required to complete the Rust replacement.
15. No doctor command, workspace TOML/trust system, additional harnesses, or
    new signing/provenance project is required or added in this feature.
16. Existing tests have Rust replacements, Linux differential parity passes,
    the supported-platform mocked suite passes, and real SmolVM smoke tests
    cover all four profiles before cutover.
17. Cutover documentation is complete and all old production Bash scripts,
    Python lifecycle helpers, and script-based tests/fixtures are removed.

## Implementation stages

1. Establish the Rust executable, typed domain model, SmolVM test double, and
   native platform interfaces while keeping current scripts operational.
2. Implement repository/home definition discovery, `--smolfile` filename/path
   selection, and offline `create-config` templates for the four profiles,
   with atomic no-clobber publication.
3. Port existing identity, inventory, reconciliation, replacement, allowlist,
   port, clean, purge, and interactive-process behavior.
4. Implement profile-specific guest launch/state/authentication conventions and
   port all tests. Preserve Codex parity throughout.
5. Prove Linux differential parity, cross-platform execution, binary relocation,
   and real SmolVM/harness smoke behavior.
6. Update documentation and cut over atomically to the Rust implementation.
   No intermediate Rust stage is a supported release. Feature 0008 may proceed
   independently and does not hold this cutover open.
