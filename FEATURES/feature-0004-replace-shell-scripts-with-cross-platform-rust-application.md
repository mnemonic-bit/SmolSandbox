# Feature 0004: Replace shell scripts with a cross-platform Rust application

## Status

Blocked

Implementation is blocked until both decisions in **Required pre-implementation
decisions** are resolved and recorded in this specification. Research and
throwaway prototypes may continue, but production migration work shall not
begin while either decision remains open.

## Summary

Replace the repository's Bash and Python lifecycle implementation with one
cross-platform Rust application named `codex-sandbox`.

The application shall preserve the current sandbox lifecycle semantics,
machine identity, reconciliation safety, create-before-delete replacement, and
Codex launch behavior on Linux, macOS, and Windows. It shall expose those
workflows as `build`, `start`, `login`, `repair`, `clean`, and `doctor`
subcommands rather than retaining the existing `.sh` entry points.

Users shall install SmolVM separately and make a compatible `smolvm`
executable available on the host. Codex is not a host dependency: the prepared
guest shall continue to install and execute the pinned Codex CLI. The released
`codex-sandbox` archives shall otherwise be ready to run without Bash, Python,
a Rust toolchain, or platform command-line utilities.

Mutable state and user configuration shall move to a host-global
`.codex-sandbox` directory in the user's home directory. There shall be no
migration from repository-local shell-era state because this feature will be
deployed on a clean slate.

The final cutover is atomic. No intermediate Rust implementation is a
supported release, and all production scripts, script-based tests, the Python
inventory helper, and script-based fixtures shall be removed when parity and
release acceptance have passed.

## Motivation

The current implementation relies on Bash, Python, SHA-256 command-line
utilities, and other Unix behavior. Its arrays, regular-expression syntax,
signal traps, path handling, delimiter protocol, and fixture implementation do
not run natively across Linux, macOS, and Windows.

Portability is the primary reason for this migration. A Rust application can
provide one compiled lifecycle implementation on all host platforms supported
by SmolVM while retaining the existing safety model:

- derive sandbox identity from authoritative inputs;
- treat SmolVM inventory as untrusted input;
- never interpret inventory failure as machine absence;
- keep normal startup non-destructive;
- create and verify a replacement before deleting stale machines;
- serialize mutating operations; and
- stop a started workspace or login machine when an interceptable termination
  path ends the interactive session.

Moving state out of the source checkout also allows the application to be
installed globally and used from any workspace. Shipping platform release
archives removes any requirement for users to install a Rust toolchain.

## Goals

- Run the complete sandbox lifecycle on Linux, macOS, and Windows wherever a
  compatible SmolVM release supports the host.
- Replace the production Bash and Python implementation with one Rust
  executable and Rust modules.
- Replace Bash/Python tests and fixtures with Rust unit tests, integration
  tests, and a Rust SmolVM test double.
- Preserve the implemented behavior and safety invariants of Features 0001
  and 0002.
- Preserve current Linux behavior exactly where practical and define semantic
  equivalents for platform-specific paths, signals, permissions, and process
  handling elsewhere.
- Provide one documented CLI with stable subcommands, usage errors, exit-code
  categories, and actionable diagnostics.
- Use a host-global application home for artifacts, state, locks, Codex state,
  and user configuration.
- Retain deterministic guest-tool version pins and definition-addressed base
  artifacts.
- Keep SmolVM behind a narrow adapter so a future supported library API can
  replace subprocess integration without rewriting lifecycle policy.
- Produce signed, checksummed, provenance-bearing release archives for the
  supported SmolVM host matrix.
- Complete one atomic cutover only after differential, cross-platform, real
  SmolVM, packaging, and documentation acceptance has passed.

## Non-goals

- Bundling, redistributing, or directly linking SmolVM in the first release.
- Requiring Codex to be installed on the host.
- Preserving the old `.sh` filenames or providing compatibility launchers.
- Preserving accidental shell quirks when they conflict with a clear,
  cross-platform interface. In particular, `login` shall reject unexpected
  arguments rather than silently ignoring them.
- Migrating, importing, or merging repository-local `artifacts/` or `.smolvm/`
  state.
- Shipping a prebuilt `.smolmachine` guest artifact with application releases.
- Supporting a platform or architecture that the selected SmolVM release does
  not support.
- Adding self-update, MSI, PKG, Homebrew, or other package-manager integration.
- Providing a stable JSON output protocol before an automation consumer
  requires one.
- Sending usage, diagnostic, or crash telemetry.
- Making host state portable or synchronizable between operating systems.
- Resolving the two blocked implementation decisions implicitly in code.

## Relationship to earlier features

Feature 0001's identity, inventory, reconciliation, explicit repair, and
non-destructive startup requirements remain authoritative. Feature 0002's
create-before-delete replacement protocol and readiness gate also remain
authoritative. This feature changes their implementation language, command
spelling, state location, locking mechanism, and platform abstractions; it does
not weaken their safety invariants.

Feature 0003 remains a separate proposed feature. This feature shall not
silently implement automatic superseded-artifact purging merely because the
code is being rewritten. If Feature 0003 is implemented before the Feature
0004 cutover, its then-current behavior becomes part of the differential parity
baseline and shall be ported to Rust. Otherwise it remains independently
proposed.

The SmolVM integration decision is supported by
[`RESEARCH/2026-09-20-smolvm-embedding.md`](../RESEARCH/2026-09-20-smolvm-embedding.md).
As of SmolVM v1.16.2, its public high-level embedded API does not cover every
pack, complete-inventory, create-from-pack, and interactive-TTY workflow this
application needs. SmolVM's first-party Rust SDK also uses an executable
boundary. Direct linking may be reconsidered later only after the required
operations and Windows integration are supported upstream.

## Required pre-implementation decisions

The feature is blocked on two deliberately unresolved questions.

### 1. Default Smolfile packaging and discovery

Before implementation begins, this specification shall be amended to choose
how the default Smolfile is supplied. The decision shall compare at least:

- embedding its bytes in `codex-sandbox`;
- shipping it as a versioned resource beside the executable; and
- another explicitly defined acquisition mechanism.

The recorded decision shall define resource discovery, override behavior,
release packaging, version coupling, failure behavior, and how the exact bytes
participate in the base ID. No implementation shall assume that the default
Smolfile is embedded merely because embedding was discussed during design.

### 2. Provisioning-download integrity

Before implementation begins, this specification shall be amended to define
the integrity policy for every download performed while SmolVM builds the
guest. The decision shall cover version pins, transport requirements, hashes or
ecosystem integrity metadata where applicable, installer scripts, cache
publication, partial failures, and the behavior when verification information
is unavailable.

Guest tools shall remain version-pinned, but this feature intentionally does
not select a digest or verification policy until this blocker is resolved.

### Resolving the block

Both answers shall be written into this file, with corresponding acceptance
criteria and test requirements. After that review, the status may move from
`Blocked` to `Proposed`. Code intended for the migration shall not be merged
before both gates are resolved. Source-reading, research notes, and disposable
prototypes that answer the gates are allowed.

## Terminology

- **Application home:** The `.codex-sandbox` directory under the current
  user's native home directory, or the directory selected by
  `CODEX_SANDBOX_HOME`.
- **Supported host:** An operating-system and architecture combination in the
  documented support matrix for which both `codex-sandbox` and the selected
  SmolVM release are supported and tested.
- **Selected SmolVM:** The canonical host executable selected by the discovery
  precedence in this feature and accepted by capability preflight.
- **Default Smolfile:** The reviewed Smolfile supplied by the application using
  the mechanism selected by the first implementation gate.
- **Effective Smolfile:** The complete Smolfile bytes selected after applying
  configuration and explicit overrides.
- **Desired base ID:** The first 12 lowercase hexadecimal characters of the
  SHA-256 hash over the selected SmolVM version output, a newline, and the
  complete effective Smolfile contents, preserving Feature 0001.
- **Workspace hash:** The full SHA-256 hash of the deterministic native
  representation of the canonical host workspace path.
- **Workspace ID:** The first 12 lowercase hexadecimal characters of the
  workspace hash.
- **Dangerous workspace setting:** A workspace-controlled value that can add a
  host mount, redirect an executable, expose a secret, replace provisioning
  inputs, or otherwise extend access outside ordinary workspace behavior.
- **Semantic compatibility:** Preservation of lifecycle outcomes, safety
  invariants, side effects, exit-code categories, command ordering, and
  actionable information while allowing native path formatting and
  platform-specific diagnostic wording.

## Supported hosts

The support matrix shall be aligned with the SmolVM release selected for an
application release. Each `codex-sandbox` release shall document the exact
operating-system, architecture, and SmolVM combinations it tests and supports.

The release process shall target x86-64 and ARM64 wherever both Rust and
SmolVM support that combination. A build existing for an unsupported
combination does not make that combination supported. Unsupported hosts shall
fail capability preflight with an actionable message rather than run on a
best-effort basis.

`codex-sandbox` shall use Rust libraries and native operating-system APIs for
hashing, copying, path handling, permissions, JSON, locking, and process
lifecycle. It shall not invoke Bash, Python, `sha256sum`, `shasum`, `cp`, `mv`,
or other host utilities.

SmolVM is the only required host executable. Codex and all guest development
tools are installed into the prepared guest by the effective Smolfile.

## Distribution

Release automation shall publish versioned:

- `.tar.gz` archives for supported Linux and macOS targets; and
- `.zip` archives for supported Windows targets.

Each archive shall contain the `codex-sandbox` executable and the application
resources selected by the Smolfile packaging decision. It shall not contain
SmolVM or a prebuilt guest artifact.

Every release archive shall have a SHA-256 checksum, a signature, and build
provenance. Release validation shall confirm that SmolVM has not been included
accidentally. The binary shall expose `codex-sandbox --version`.

Users need neither a Rust toolchain nor Bash or Python to use a release.
Initial base construction may access the network through SmolVM to download the
pinned guest packages and tools required by the effective Smolfile. Fully
offline first use is not a goal.

Native installers, package-manager manifests, and in-application self-update
are deferred to later features.

## Command-line interface

The supported interface shall be:

```text
codex-sandbox build [workspace]
codex-sandbox start [workspace] [-- <codex arguments>...]
codex-sandbox login
codex-sandbox repair (--check | --replace-stale [--force]) [workspace]
codex-sandbox clean
codex-sandbox doctor [workspace]
codex-sandbox --version
codex-sandbox --help
```

`workspace` defaults to the current directory where the corresponding shell
workflow currently defaults to `$PWD`. A leading `--` after `start` means the
current workspace and forwards all following arguments to the guest Codex CLI.

The subcommands map to the old entry points as follows:

| Rust command | Replaced shell behavior |
| --- | --- |
| `build` | `build-sandbox.sh` and its `build-login.sh` alias |
| `start` | `start-codex.sh` and its `codex-sandbox.sh` alias |
| `login` | `start-login.sh` |
| `repair` | `repair-sandbox.sh` |
| `clean` | `clean-sandbox.sh` |
| `doctor` | New non-mutating host and configuration diagnostics |

`sandbox-common.sh` becomes internal Rust lifecycle and domain code.
`sandbox-inventory.py` becomes internal Rust inventory parsing,
classification, and readiness validation. Neither shall remain as an
independently supported interface.

Unknown options, conflicting repair modes, excess workspace arguments, and
arguments supplied to `login` or `clean` shall produce usage and exit with
code `2`. Operational failures shall exit non-zero without being reported as
usage errors.

## Compatibility contract

The change from script filenames to one subcommand-based executable is the
intentional command-level compatibility break. No compatibility wrapper,
symlink, or multicall installation is required.

Within each mapped subcommand, the application shall preserve:

- default-workspace selection;
- argument forwarding to guest Codex;
- the distinction between usage and operational failure;
- machine and artifact naming unless this feature explicitly defines native
  path normalization;
- all managed labels and inventory ownership rules;
- normal-start, repair, replacement, and clean side effects;
- create-before-delete ordering and readiness checks;
- API-key forwarding by environment-variable name rather than value;
- non-clobbering Codex-state seeding;
- best-effort VM shutdown after interactive sessions; and
- the exit status of the guest Codex process unless cleanup is the only
  failure.

On Linux, output and command ordering shall match the current implementation
exactly where practical. Across platforms, tests shall assert semantic output
rather than byte-for-byte path separators or operating-system diagnostic text.
Messages that recommend a follow-up command shall use the new
`codex-sandbox` syntax.

The current `start-login.sh` behavior of ignoring arguments is considered an
accidental quirk and shall not be preserved. `codex-sandbox login` accepts no
positional arguments.

## Configuration

The application shall support:

```text
~/.codex-sandbox/config.toml
<workspace>/.codex-sandbox.toml
```

The user configuration applies across workspaces. Workspace configuration
contains project-specific declarative settings and is subject to the trust
model below.

For every setting that has equivalent sources, precedence is:

```text
command-line option
> environment variable
> trusted workspace configuration
> user configuration
> application default
```

The implementation shall preserve these existing public environment
overrides where they remain applicable:

- `BASE_SMOLFILE`
- `ARTIFACTS_DIR`
- `STATE_ROOT`
- `CODEX_HOME_DIR`
- `BUILDER_MACHINE`
- `LOGIN_MACHINE`
- `CODEX_MACHINE`
- `TMPDIR`
- `OPENAI_API_KEY`

`SANDBOX_ENSURE_LOGIN` is an internal shell implementation detail and shall
not be supported.

`CODEX_SANDBOX_HOME` is the new whole-tree application-home override.
SmolVM executable discovery has its own precedence:

```text
--smolvm <path> > SMOLVM_BIN > user configuration > PATH
```

Workspace configuration shall never select or redirect the SmolVM executable.
The selected executable path shall be canonicalized before use.

Unknown keys, invalid types, conflicting values, and unreadable configuration
shall fail with the file and field identified. A partially parsed
configuration shall never authorize mutation.

## Workspace configuration trust

Harmless declarative workspace settings may load automatically. A workspace
configuration shall not automatically:

- add host mounts outside the workspace;
- redirect SmolVM or another executable;
- expose host secrets;
- replace the provisioning input;
- relocate credential-bearing state; or
- broaden guest access to host resources.

Before honoring a dangerous workspace setting, the application shall require
an explicit user trust decision. Trust shall be stored in application-owned
state and keyed by both canonical workspace identity and the configuration
digest. Changing dangerous configuration invalidates the prior decision.

Trust prompts shall enumerate the sensitive effects being authorized. A
non-interactive invocation without an applicable trust decision shall fail
closed. Workspace configuration may never override the prohibition on
selecting a SmolVM executable.

## Application home and persisted state

The default layout shall be:

```text
~/.codex-sandbox/
  config.toml
  artifacts/
  state/
  locks/
  codex-home/
```

The native user-home API shall locate the parent directory on every platform;
the child directory name remains `.codex-sandbox`. `CODEX_SANDBOX_HOME` may
relocate the entire tree. Existing individual environment overrides may
relocate the corresponding component after normal precedence is applied.

No generated state shall be written into a workspace by default. The optional
workspace `.codex-sandbox.toml` is configuration, not mutable state.

Every application-owned persisted format shall carry a schema version from
its first release. Unknown newer versions shall fail safely without mutation.
Future features may introduce migrations from known older Rust-era formats,
but this feature provides no migration from the shell-era `artifacts/` or
`.smolvm/` directories. Those directories shall not be imported, merged, or
deleted by the Rust application.

The application home is host-local. Sharing or synchronizing it between
operating systems is unsupported.

## Permissions and secrets

The application shall apply the strongest practical native user-only
permissions to the application home, user configuration, and `codex-home`.
On platforms with Unix modes this means directory and file permissions that
exclude group and other access; on Windows it means a user-restricted ACL.

If private access cannot be verified or enforced, an operation that would
create, read, mount, or expose credential-bearing state shall refuse to
continue. Non-sensitive artifacts, locks, and caches may continue after an
explicit warning. The application shall never loosen permissions to make an
operation succeed.

`OPENAI_API_KEY` shall never be persisted or printed. When non-empty, it shall
continue to be forwarded to the guest through SmolVM by environment-variable
name rather than by embedding its value in arguments or configuration.
Verbose diagnostics shall redact secrets and credential-bearing paths where
appropriate.

The application shall send no telemetry.

## Workspace and machine identity

The workspace shall be resolved with native filesystem canonicalization. Path
representations shall be deterministic so alternate spellings of the same
native path select the same workspace identity. Windows normalization shall
account for drive-letter and case behavior. The normalized path shall be
encoded deterministically as UTF-8 before hashing.

Identity equivalence is required within one host operating system, not across
different operating systems. A workspace accessed on Linux and Windows may
have different workspace hashes and machines.

Unless explicitly overridden, the desired machine name remains:

```text
codex-workspace-<base-id>-r<revision>-<workspace-id>
```

New workspace machines shall retain the complete managed label set from
Feature 0001, including the full workspace hash. Builder and login naming,
runtime-policy revision, legacy-name recognition, and explicit override rules
shall remain compatible with the current implementation.

Regardless of host path syntax, the workspace shall remain mounted inside the
Linux guest at:

```text
/workspace/myrepo
```

## Guest and base artifact model

The selected SmolVM shall build the base artifact locally from the effective
Smolfile. Application releases shall not publish or download a prebuilt
`.smolmachine` artifact.

The effective Smolfile shall keep exact versions for Codex and other guest
tools. Reviewed version changes alter the effective Smolfile bytes and
therefore produce a new desired base ID and artifact. Ordinary startup shall
never resolve an unpinned `latest` version silently.

The mechanism by which the default Smolfile reaches the application and the
integrity rules for provisioning downloads remain blocked decisions. All
other artifact identity, temporary publication, non-empty validation, and
reuse behavior shall remain compatible with the current implementation.

Codex shall continue to be installed into the guest artifact and seeded from
`/opt/codex-home` into the host-mounted `codex-home` without overwriting files
that already exist. Failure to seed defaults remains a warning and does not by
itself prevent Codex launch.

## SmolVM integration

The first release shall invoke an independently installed `smolvm` executable
through a subprocess adapter. It shall neither bundle SmolVM nor depend on
SmolVM's internal Rust engine modules.

After applying discovery precedence, preflight shall:

1. Canonicalize the executable path.
2. Obtain and parse its version output.
3. Verify the commands and machine-readable output capabilities required by
   the requested operation.
4. Verify that SmolVM reports the host virtualization prerequisites needed on
   the current platform.
5. Reject an incompatible executable before making lifecycle mutations.

The reported version output remains part of the desired base ID. The
application shall use capability checks rather than pinning Codex or the host
application to one exact SmolVM release. Errors shall identify the missing
capability and provide an actionable installation or upgrade instruction.

No arbitrary `smolvm` from `PATH` may silently replace an explicitly selected
executable. Workspace configuration cannot influence selection.

The adapter shall own argument construction, JSON capture, TTY attachment,
secret-environment forwarding, exit-status translation, and subprocess error
classification. Domain reconciliation code shall not construct command lines
or parse free-form CLI output.

## Internal architecture

The implementation shall preserve these explicit seams:

```text
CLI
 └─ lifecycle application service
     ├─ pure identity, inventory, and reconciliation domain logic
     ├─ SmolVM adapter
     ├─ cross-platform filesystem, process, permission, and locking adapter
     └─ configuration and state store
```

The pure domain layer shall own machine classification, readiness, replacement
planning, clean selection, and safety decisions. It shall accept typed
inventory and configuration and return an explicit plan or refusal; it shall
not perform I/O.

The lifecycle service shall execute plans, enforce refresh boundaries, and
preserve operation ordering. All SmolVM access shall pass through the adapter.
All platform-specific behavior shall pass through narrow platform interfaces.

This separation is required for exhaustive tests and for a future replacement
of the SmolVM subprocess adapter. The specification does not mandate a
particular Rust crate count or directory layout.

## Inventory handling

SmolVM inventory is untrusted input. The Rust parser shall preserve or
strengthen every validation currently performed by `sandbox-inventory.py`:

- require valid UTF-8 and JSON;
- require an array of uniquely named machine objects;
- validate names, states, labels, and expected field types;
- reject duplicate, incomplete, or contradictory managed ownership metadata;
- classify desired, stale, ambiguous, unrelated, and legacy records using the
  rules in Features 0001 and 0002; and
- apply the stricter replacement readiness state set before destructive
  cleanup.

The Rust implementation shall use typed values rather than reproducing the
unit-separator text protocol used between Bash and Python.

An unavailable, invalid, partial, or unrecognized inventory shall fail closed.
It shall never be treated as an empty inventory or authorize build, create,
stop, delete, cache, or trust mutation.

## Lifecycle behavior

### `build`

`build [workspace]` shall preserve `build-sandbox.sh` behavior:

1. Resolve configuration, trust, workspace identity, the selected SmolVM, and
   the effective Smolfile.
2. Reconcile in non-destructive `start` mode.
3. Ensure the current base artifact, workspace machine, and login machine.
4. Preserve all Feature 0001 ambiguity and stale-only refusal rules.
5. Print the ready artifact, machine, workspace, and next commands using the
   new CLI syntax.

It accepts zero or one workspace argument. Extra arguments are usage errors.

### `start`

`start [workspace] [-- <codex arguments>...]` shall:

1. Perform the same non-destructive workspace reconciliation as `build`,
   without requiring a login machine.
2. Start the selected workspace machine.
3. Best-effort seed missing Codex state from `/opt/codex-home`.
4. Run interactive guest Codex with every forwarded argument.
5. Forward `OPENAI_API_KEY` only when the host value is non-empty.
6. Best-effort stop the selected machine after the interactive process ends or
   an interceptable termination request occurs.

The guest Codex exit status shall be the command's exit status unless guest
startup fails or cleanup is the only failure. A seed-copy failure remains a
warning.

### `login`

`login` shall derive current desired identity, confirm the current-generation
login machine from a valid fresh inventory, start it, seed Codex state, and run
interactive:

```text
codex login --device-auth
```

inside the guest through SmolVM. It shall best-effort stop the login machine
after completion or an interceptable termination request. It shall not create
a missing login machine and accepts no positional arguments.

### `repair`

`repair --check [workspace]` shall be read-only and lock-free. It shall report
the Feature 0001 classification without creating, stopping, deleting, or
rewriting state.

`repair --replace-stale [--force] [workspace]` shall preserve Feature 0002's
complete create-before-delete protocol. It shall hold the workspace mutation
lock across preflight, base preparation, desired creation, fresh readiness
verification, stale cleanup, and state publication. `--force` authorizes only
the post-readiness stop of running stale machines.

### `clean`

`clean` is generation-wide, not limited to one workspace. It shall preserve
the existing selection scope:

- the exact current builder;
- current-base login machines at any numeric revision;
- positively labelled current-base workspace machines; and
- strictly named legacy current-base workspace machines.

Before mutation it shall print every selected resource. Contradictory managed
metadata shall block cleanup. Selected machines shall be stopped best-effort
and force-deleted as today. Current managed artifacts and compatibility state
selected by existing behavior shall be removed, while `codex-home`, unrelated
machines, unmanaged files, and other generations remain untouched.

If Feature 0003 is implemented before cutover, its additional artifact-purge
behavior shall be ported according to **Relationship to earlier features**.

### `doctor`

`doctor [workspace]` is a new read-only command. It shall validate and report:

- user and workspace configuration;
- application-home existence, schema, writability, and permission posture;
- SmolVM discovery, canonical path, version, and required capabilities;
- host virtualization prerequisites exposed by SmolVM;
- workspace canonicalization and identity;
- workspace trust status; and
- required network reachability where it can be tested safely without
  credentials or mutation.

`doctor` shall not create, start, stop, delete, or alter a machine; build an
artifact; change trust; or rewrite state. It exits non-zero when a required
check fails and distinguishes warnings from blockers.

## Concurrency and locking

Mutating operations for one workspace identity shall be serialized with a
cross-platform exclusive file lock under the application home's `locks/`
directory. The observable timeout remains 60 seconds.

Operating-system-released locks shall replace PID-bearing lock directories
where native locking provides crash recovery. The implementation shall not
reproduce stale-PID deletion merely for literal compatibility.

After acquiring a lock, a mutating operation shall refresh inventory before
making create or delete decisions. Replacement shall keep the lock for the
entire Feature 0002 handoff. `repair --check` and `doctor` remain lock-free and
read-only.

Any artifact publication or purge behavior that operates across workspaces
shall use a distinct application-home artifact lock. Lock acquisition failure
or timeout shall not authorize an unlocked fallback.

## Interactive process and cleanup behavior

The platform process adapter shall support interactive input/output, terminal
resize where SmolVM supports it, child exit-status propagation, Ctrl-C, and the
termination notifications the host operating system permits the application
to intercept.

The application shall attempt to stop a workspace or login machine after:

- normal guest command completion;
- Ctrl-C;
- another interceptable termination request; or
- a child-process startup failure after the machine was started.

Cleanup is best-effort. Forced process termination, host crashes, power loss,
and failures that prevent application code from running cannot guarantee a
machine stop. Documentation and diagnostics shall state that limitation rather
than promising impossible cleanup.

If the guest command fails and cleanup also fails, the guest failure remains
primary and cleanup is reported separately. If the guest command succeeds and
cleanup alone fails, the command shall report an operational failure.

## Output, errors, and diagnostics

Human-readable output is the only stable output contract in this feature.
Success, warning, usage, and operational failure shall remain distinguishable
without relying solely on color.

CLI misuse exits `2`. Operational failures exit non-zero. Interactive `start`
and `login` preserve the guest process exit status where the host operating
system can represent it. Platform-specific raw status values may be normalized
without changing success versus failure.

`--verbose` shall expose local diagnostic context such as selected paths,
configuration sources, SmolVM capability checks, and operation phases. It
shall not print secrets. Stable machine-readable JSON output is deferred.

Every refusal before mutation shall say that no relevant resources were
changed. Partial cleanup failures shall list the completed and unprocessed
operations without claiming rollback.

## Safety invariants

- Unknown, invalid, or failed inventory never implies machine absence.
- Normal `build` and `start` never delete a machine.
- Normal reconciliation never creates a desired machine when stale-only or
  ambiguous candidates require explicit repair.
- A stale workspace machine is not stopped or deleted before the desired
  machine passes Feature 0002 readiness.
- A machine belonging to another workspace is never stopped or deleted.
- Contradictory managed metadata blocks destructive action.
- Read-only commands perform no lifecycle, state, or trust mutation.
- Configuration or trust parse failure performs no lifecycle mutation.
- A workspace cannot select an executable or silently authorize sensitive
  host access.
- Cache and state publication is atomic and follows successful verification.
- A lock timeout never falls back to unlocked mutation.
- Credential-bearing state is not used when private access cannot be ensured.
- API-key values and guest credentials are never logged or persisted by the
  application.
- Guest-tool updates occur only through reviewed, pinned Smolfile changes.
- Shell-era state is never imported, merged, or deleted.
- SmolVM is never bundled in an application release.
- Neither blocked implementation decision is resolved implicitly in code.

## Testing strategy

### Rust unit and property tests

Pure tests shall cover:

- canonical path normalization and workspace hashing on every platform;
- base hashing from SmolVM version output and exact Smolfile bytes;
- inventory JSON validation and duplicate detection;
- desired, stale, legacy, ambiguous, unrelated, and override classification;
- readiness and replacement planning;
- clean selection and contradictory-label refusal;
- configuration precedence and dangerous-setting trust invalidation;
- state-schema handling; and
- exit and cleanup result precedence.

Property or generated tests shall exercise malformed inventory, adversarial
names and labels, path variants, and plan invariants that prevent unrelated
machine mutation.

### Rust SmolVM test double

A cross-platform Rust executable shall replace `tests/fixtures/smolvm`. It
shall support version and inventory responses, sequenced inventories,
interactive and non-interactive exec records, lifecycle state mutation,
operation logging, and independent failure injection for inventory, build,
pack, create, start, stop, delete, and guest seeding.

The final test suite shall not execute Bash or Python.

### Behavioral coverage

Every scenario in the existing clean, reconciliation, replacement, start, and
Smolfile-configuration tests shall be ported. New tests shall cover behavior
currently lacking direct coverage, including:

- `login` success, failure, cleanup, and argument validation;
- `build` summaries and usage errors;
- `repair --check` read-only behavior;
- every CLI conflict and excess-argument case;
- API-key forwarding without value exposure;
- clean refusal for contradictory labels;
- SmolVM discovery precedence and incompatible capabilities;
- configuration precedence and workspace trust;
- application-home permissions and schema versions;
- interruption and dual-failure cleanup behavior; and
- `doctor` success, warnings, failures, and non-mutation.

### Differential compatibility

Before the old files are deleted, Linux CI shall run a temporary differential
suite against the shell/Python and Rust implementations using equivalent test
doubles and fixtures. It shall compare lifecycle plans, SmolVM operation
ordering, labels, names, exit-code categories, and normalized user-visible
outcomes.

The differential suite may normalize command spelling, native paths, and
approved Q9 compatibility exceptions. Every other difference requires an
explicit specification amendment or a fix before cutover.

### Cross-platform and real-SmolVM verification

The complete mocked suite shall pass on every supported release target. Before
publishing, release CI shall also run at least one real SmolVM lifecycle smoke
test on every supported operating-system and architecture combination. The
smoke test shall cover discovery, build, machine creation, inventory, start,
interactive execution sufficiently to validate TTY plumbing, stop, and safe
cleanup.

Release tests shall unpack each archive in a clean environment without a Rust
toolchain, confirm that no Bash or Python is used, confirm that SmolVM is not
bundled, and exercise both missing-SmolVM and compatible-SmolVM diagnostics.

## Documentation requirements

The cutover shall update `README.md` and any operator documentation to cover:

- release download and verification;
- SmolVM installation as the sole host executable prerequisite;
- the exact supported OS, architecture, and SmolVM matrix;
- old-script to new-subcommand migration;
- command examples and exit behavior;
- application-home layout and permissions;
- configuration files, environment variables, and precedence;
- workspace trust and dangerous settings;
- initial guest provisioning and network requirements;
- workspace identity and fixed guest mount path;
- generation-wide `clean` scope;
- repair and create-before-delete guarantees;
- `doctor` and troubleshooting;
- the lack of shell-era state migration; and
- the lack of self-update and native installers.

Documentation shall not claim that Codex must be installed on the host or that
SmolVM is included in the application archive.

## Acceptance criteria

1. The two required pre-implementation decisions are recorded in this file,
   their acceptance tests are defined, and the feature is no longer marked
   `Blocked` before migration implementation begins.
2. `codex-sandbox` builds and runs on every documented SmolVM-supported release
   target without Bash, Python, or a Rust toolchain on the user's host.
3. Given no compatible `smolvm` on `PATH` and no explicit override, every
   lifecycle command fails before mutation with an actionable installation
   error.
4. Given `--smolvm`, `SMOLVM_BIN`, user configuration, and `PATH` candidates,
   discovery selects the highest-precedence allowed candidate and never reads
   an executable path from workspace configuration.
5. Given a selected SmolVM missing a required capability, preflight fails
   before lifecycle mutation and identifies that capability.
6. Given equivalent native spellings of one workspace path, canonicalization
   produces one workspace identity on that host.
7. Linux identities, machine names, labels, base IDs, and lifecycle operation
   order remain compatible with the current implementation for unchanged
   authoritative inputs.
8. `build`, `start`, `login`, `repair`, and `clean` satisfy their mapped current
   behavior and the safety requirements of Features 0001 and 0002.
9. `login` and `clean` reject unexpected arguments with usage and exit `2`.
10. `start` forwards all arguments following `--` to guest Codex unchanged and
    returns its exit status unless cleanup is the only failure.
11. An `OPENAI_API_KEY` is forwarded by name without its value appearing in
    arguments, configuration, state, normal output, or verbose output.
12. A guest seed-copy failure warns but does not prevent `start` or `login`
    from launching Codex.
13. Inventory failure, invalid JSON, duplicate names, or contradictory labels
    authorizes no lifecycle mutation.
14. Replacement creates and freshly verifies the desired machine before the
    first stop or delete of a stale machine.
15. Normal startup never deletes a machine and refuses stale-only ambiguity
    without creating a new desired machine.
16. `clean` reports and affects only the current generation's managed selection
    and preserves unrelated machines, other generations, and `codex-home`.
17. Two concurrent mutating operations for one workspace serialize, and a
    waiting operation fails after 60 seconds rather than mutating unlocked.
18. `repair --check` and `doctor` perform no machine, artifact, state, lock,
    cache, permission, or trust mutation.
19. A dangerous workspace setting is ignored or refused until the user grants
    trust for the canonical workspace and exact configuration digest.
20. Changing a trusted dangerous setting invalidates the stored trust decision.
21. Credential-bearing operations refuse to continue when user-private access
    cannot be verified or enforced.
22. Unknown newer application-state schemas fail without rewriting state or
    mutating SmolVM resources.
23. Shell-era `artifacts/` and `.smolvm/` contents are neither imported nor
    deleted by the Rust application.
24. The Rust SmolVM test double supports every success, sequencing, and failure
    case needed by the ported suite without invoking Bash or Python.
25. Every existing behavioral test has a Rust replacement, and the uncovered
    CLI, trust, permission, API-key, login, and doctor cases listed in this
    feature have direct tests.
26. The Linux differential suite reports no unapproved semantic difference
    before the old implementation is removed.
27. The mocked suite passes on every supported release target, and real SmolVM
    smoke tests pass on every supported OS and architecture combination.
28. Release archives run in clean environments without a Rust toolchain and
    contain neither SmolVM nor a prebuilt guest artifact.
29. Every release archive has a verified checksum, signature, and provenance.
30. Updated documentation covers every item in **Documentation requirements**.
31. The final repository contains no production or test Bash scripts, Python
    lifecycle helper, or script-based SmolVM fixture, and none is required by
    build, test, packaging, or runtime workflows.
32. No intermediate migration stage is presented or published as a supported
    release.

## Implementation stages

### Stage 0: Resolve the blockers

1. Decide and document default Smolfile packaging and discovery.
2. Decide and document provisioning-download integrity.
3. Add acceptance tests implied by both decisions.
4. Review the amended specification and move the status from `Blocked` to
   `Proposed` before production migration work begins.

### Stage 1: Establish the Rust boundaries

1. Create the Rust workspace and `codex-sandbox` binary.
2. Define typed configuration, inventory, identity, lifecycle plans, errors,
   and persisted schemas.
3. Implement the pure domain layer and Rust SmolVM test double test-first.
4. Keep all production entry points on the current implementation.

### Stage 2: Port lifecycle behavior

1. Implement configuration, trust, state, platform, and SmolVM adapters.
2. Port inventory classification and readiness validation from Python.
3. Port build, start, login, repair, clean, and doctor workflows.
4. Port every existing test and add the missing behavioral coverage.

### Stage 3: Prove portability and parity

1. Add the complete supported-platform CI matrix.
2. Run the Linux differential suite while the old implementation remains.
3. Run real SmolVM lifecycle smoke tests on every supported target.
4. Resolve every difference or record an approved specification exception.

### Stage 4: Validate distribution

1. Build and unpack every release archive in a clean environment.
2. Verify checksums, signatures, provenance, and resource discovery.
3. Confirm SmolVM and prebuilt guest artifacts are absent.
4. Complete installation, operation, security, and troubleshooting docs.

### Stage 5: Atomic cutover

1. Make `codex-sandbox` the only supported implementation.
2. Remove the eight production Bash files.
3. Remove `sandbox-inventory.py`.
4. Remove all Bash tests and the Bash SmolVM fixture after their Rust
   replacements pass.
5. Remove any remaining Bash or Python requirement from build, test,
   packaging, and runtime workflows.
6. Publish no release until the complete definition of done below passes.

## Definition of done

This feature is complete only when:

- both implementation blockers were resolved before migration work began;
- the production and test implementation is Rust-only;
- no Bash or Python is required to build, test, package, or run it;
- the complete mocked suite passes on every supported platform;
- real SmolVM smoke tests pass on every supported platform and architecture;
- Linux differential parity passed before the old files were removed;
- signed, checksummed, provenance-bearing release archives were produced and
  validated without bundling SmolVM;
- all required documentation is complete;
- all legacy shell scripts, the Python helper, and script-based fixtures have
  been removed; and
- the single atomic cutover is ready to publish as the first supported Rust
  implementation.
