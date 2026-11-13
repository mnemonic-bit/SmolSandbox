# smolsandbox: operation and validation

This is the authoritative operating reference for the Rust `smolsandbox`
application. Start with the [README first-use sequence](README.md#first-use).
The application supports Claude, Codex, OpenCode, and Hermes Agent through
complete user-owned Smolfile definitions and a separately installed SmolVM.

The Rust documentation reflects source removal in commit
`28087885c2b84077c62ee92c5560a2cab1b68997`. Final cutover acceptance remains
pending because real guest validation has not been established for every
required profile and release target. The
[Feature 0010 validation record](FEATURES/feature-0010-validation.md) separates
observed checks from outstanding gates. The historical completion decisions
for Features 0004 and 0009 remain recorded in their validation records.

## Executable and host requirements

Build the application from this checkout with a Rust toolchain:

```text
cargo build --locked --release --bin smolsandbox
```

Copy `target/release/smolsandbox` (Windows: `smolsandbox.exe`) to a directory
on `PATH`. The executable embeds every configuration template at compile
time. Running the copied executable needs no checkout, adjacent resources,
host Bash/Python, host harness installation, or Rust toolchain. Guest
provisioning can use shell commands, Python, and other tools from its template.

Install [SmolVM](https://github.com/smol-machines/smolvm#install) separately.
SmolVM is neither bundled nor downloaded by this application. Runtime
selection uses `--smolvm <path>`, then nonempty `SMOLVM_BIN`, then `PATH`.
An explicit selection that fails is an error; another executable is not
silently selected. Capability preflight checks the required SmolVM operations,
inventory JSON, and native virtualization prerequisites before lifecycle
mutation. Help, version, and configuration generation require no SmolVM.

The [native CI matrix](.github/workflows/rust.yml) defines these release targets:

| Host | Architecture | Native prerequisite | Rust target |
| --- | --- | --- | --- |
| Linux | x86-64 | Read/write access to `/dev/kvm` | `x86_64-unknown-linux-gnu` |
| Linux | ARM64 | Read/write access to `/dev/kvm` | `aarch64-unknown-linux-gnu` |
| macOS | ARM64 | Hypervisor.framework | `aarch64-apple-darwin` |
| Windows | x86-64 | Windows Hypervisor Platform | `x86_64-pc-windows-msvc` |

macOS Intel and Windows ARM64 are outside this release matrix. A configured
CI job or successful cross-compilation does not establish native execution or
real guest compatibility. Recorded native mocked results and real guest
procedures have separate evidence requirements under
[validation and cutover gates](#validation-and-cutover-gates).

## Configuration and profiles

Generate one complete definition without network access or SmolVM:

```text
smolsandbox create-config --harness codex
```

The command accepts exactly `claude`, `codex`, `opencode`, and `hermes`.
Each embedded template installs development tools and one harness in the
guest. The default destination is `~/.smolsandbox/default.smlbx`.
Creation is atomic and refuses to replace an existing destination, including
one published concurrently. Invalid choices create no directories or files.

Use `--smolfile <path>` to choose a destination. Relative destinations resolve
against the invocation directory; absolute paths are accepted. Missing parent
directories are created. Existing destination files are never overwritten,
and existing directory permissions are preserved for a custom destination.
Configuration creation ignores `BASE_SMOLFILE` and repository discovery.

```text
smolsandbox create-config --harness claude --smolfile claude.smlbx
smolsandbox build --smolfile ./claude.smlbx /path/to/project
smolsandbox login --smolfile ./claude.smlbx
smolsandbox start --smolfile ./claude.smlbx /path/to/project
```

Inspect and edit the generated file. Preserve its single
`# codex-sandbox-harness: <identifier>` metadata line. That marker and the
`codex-sandbox.` managed-label namespace are retained compatibility
identifiers. Moving or updating the executable does not rewrite existing
definitions, credentials, or machine metadata.

### Whole-file selection

Lifecycle commands select exactly one complete file, without merging:

1. Use `--smolfile <name-or-path>` when supplied.
2. Otherwise, use nonempty `BASE_SMOLFILE` as an explicit path.
3. Otherwise, select the filename `default.smlbx`.

A bare flag filename is matched exactly under the selected workspace's
repository `.smolsandbox/` directory, then under `~/.smolsandbox/`. The
repository is the nearest workspace ancestor with a `.git` directory or file.
Without a workspace argument, selection uses the invocation directory.
Outside a repository, filename lookup searches only the user directory.
A missing named file never selects the default. An invalid, unreadable, or
unsupported nearer file fails instead of falling back to a farther candidate.

An absolute path or a value containing a native path delimiter is an explicit
path. Relative explicit paths resolve against the invocation directory,
including when another workspace is supplied. Explicit paths bypass filename
lookup. `BASE_SMOLFILE` always has explicit-path semantics, even for a bare
filename. The executable's directory is not a search path.

```text
smolsandbox start --smolfile claude.smlbx /path/to/project
smolsandbox build --smolfile ./configs/custom.smlbx /path/to/project
```

In the first example, `claude.smlbx` must already exist in the repository or
user lookup directory. A file generated in the invocation directory is selected
with an explicit path such as `./claude.smlbx`.

### Template adoption

User definitions remain user-owned. Updating the executable does not refresh
or merge an existing definition. To adopt a newer embedded template, generate
a separate destination with `create-config --harness <profile> --smolfile
<new-path>`, compare the files, and explicitly select or edit the desired
file. Existing edits and harness state are preserved. Changing definition
bytes selects a new base generation on the next lifecycle operation.

All four templates in version `0.8.2` give the Ruff installation a 120-second
pip socket timeout and 10 connection retries. GNU `timeout` limits that
installation to 300 seconds, sends `TERM`, and sends `KILL` after another
10 seconds if pipx has not exited. Signals apply to the monitored process
group. The limit applies to the Ruff installation, including pipx setup,
rather than the complete sandbox build.
Connection retries do not guarantee resumption of an interrupted wheel.
The Ruff version pin and hostname allowlist are unchanged.

For an existing definition, replace its Ruff installation entry with:

```toml
"PIP_DEFAULT_TIMEOUT=120 PIP_RETRIES=10 timeout --verbose --kill-after=10s 300s pipx install ruff==0.16.8",
```

Run `smolsandbox build --smolfile <definition>` with the edited definition.
The changed definition selects a new base generation. If a workspace from
the previous generation exists, use the reconciliation and replacement
procedure below before starting the replacement workspace. Alternatively,
generate and compare a separate updated definition using the template
adoption procedure above. The executable never rewrites an existing file.
Use `--verbose` on a failed build to display runtime output; pipx reports
the requested package and the guest pip error-log path. A socket timeout
includes the failing host; the overall deadline reports the signal sent.

### Harness profiles and authentication

The application uses these profile-specific commands and mounts:

| Profile | Guest launch | Guest authentication command | Persistent guest state mount |
| --- | --- | --- | --- |
| Codex | `codex --no-daemon` | `codex login --device-auth` | `/root/.codex` |
| Claude | `env CLAUDE_CONFIG_DIR=/root/.claude claude` | `env CLAUDE_CONFIG_DIR=/root/.claude claude auth login` | `/root/.claude` |
| OpenCode | `opencode` | `opencode auth login` | `/root/.local/share/opencode` |
| Hermes Agent | `hermes` | `hermes setup` | `/root/.hermes` |

Run `smolsandbox login` after `build` to execute the selected authentication
command. Login requires an existing current-generation login machine and
never creates a missing one. Use the same `--smolfile` selection as the build.
Only the selected harness's persistent home is mounted. Initial seeding copies
missing files without overwriting existing credentials, settings, or sessions;
a seed failure produces a warning and preserves existing contents.

Each login machine has the profile's authentication allowlist. If a selected
provider needs other hosts, authenticate inside the harness on a workspace
machine with the required `--allow-host` additions. Harnesses manage their own
provider settings and credentials. Claude also supports `/login` within its
interactive harness; OpenCode login selects a provider; Hermes login runs its
setup workflow.

For Codex workspace starts, a nonempty host `OPENAI_API_KEY` is passed by a
SmolVM environment-variable secret reference. The value is never written into
machine arguments or the definition. Other profiles do not receive Codex
launch flags or that secret reference by default.

## Lifecycle commands

All lifecycle commands accept `--smolvm`. Commands other than `remove` accept
`--smolfile`. `build`, `start`, `list`, `remove`, and `repair` accept an optional
workspace; `clean` accepts a workspace only with `--workspace`. The current
directory is the default workspace. `remove --global` rejects a workspace.

| Command | Effect |
| --- | --- |
| `build [WORKSPACE]` | Build or reuse the selected base, then prepare login and workspace machines. |
| `start [WORKSPACE] [-- HARNESS_ARGS...]` | Reconcile the workspace and exact port set, then launch the selected guest harness. |
| `login` | Run the selected profile's authentication workflow in its prepared login machine. |
| `repair --check [WORKSPACE]` | Inspect workspace reconciliation without writing state or taking lifecycle locks. |
| `repair --replace-stale [WORKSPACE]` | Prepare and verify the desired machine before deleting eligible stale machines. |
| `list [WORKSPACE] [--all] [--json]` | Inspect managed machines and compare identities without lifecycle locks or application writes. |
| `clean [--workspace [WORKSPACE]]` | Delete the selected generation or one workspace's managed machines. |
| `remove [WORKSPACE] [--global] [--all] [--force] [--dry-run]` | Delete eligible older-policy machines, with independent scope, revision, running-state, and preview options. |
| `purge [--force \| --force-all]` | Delete eligible stopped machines and unprotected superseded artifacts. |

A completed build attempts conservative purge. A refusal or failure in that
post-build purge is reported as a warning and does not change a successful
build into a failure.

All commands accept the Boolean global option `--verbose` before the
subcommand or among application arguments, before or after WORKSPACE.
Repeating the flag has the same effect as specifying it once. No environment
variable or persistent setting enables verbosity.

By default, routine SmolVM management output and internal noninteractive
guest-command output are suppressed on success and failure. Application
results, selection information, warnings, and errors remain visible. Failures
identify the operation and explain how to enable runtime diagnostics.
With `--verbose`, management stdout and stderr stream live to application
stderr. Version, inventory, and capability query stdout remains internal;
verbose mode exposes query stderr. `list --json` writes the same single JSON
object to stdout in either mode, with no partial JSON on inventory failure.

```text
smolsandbox build /path/to/project
smolsandbox --verbose build /path/to/project
smolsandbox build --verbose /path/to/project
smolsandbox start /path/to/project --verbose
smolsandbox list --all --json --verbose
smolsandbox start /path/to/project -- --verbose
```

Interactive shell, harness, and login sessions inherit stdin, stdout, and
stderr in both modes. Prompts, authentication instructions, and terminal
control remain visible. Preparations before the session and cleanup after
the session follow the management output policy. Runtime connection messages
from the attached interactive process remain visible with the guest output.
`start --verbose` configures application output; `start -- --verbose` passes
the flag unchanged to the guest harness. Application flags before the first
standalone `--` are never forwarded to the guest.

Application help and version require neither a definition nor SmolVM.
Arguments after the first standalone `--` on `start` belong to the guest
harness and retain their argument boundaries:

```text
smolsandbox --help
smolsandbox --version
smolsandbox start --help
smolsandbox start /path/to/project -- --help
```

The third command displays application options; the fourth launches the guest
harness with its own `--help` argument. Unknown options, conflicting modes,
excess positional arguments, and invalid option values exit `2`. Operational
failures exit nonzero.

### Identity and additional hosts

The complete selected definition bytes and SmolVM version determine the base
generation. Workspace identity covers the base artifact path, runtime CPU and
memory allocation, canonical workspace and harness-state mounts, and the
complete effective network allowlist. The runtime revision also participates
in managed machine names and ownership. Reopening the same configuration and
runtime revision reuses the workspace machine and its writable overlay.

Repeat `--allow-host` on `start` or `repair` to extend the selected definition's
allowlist. Values are sorted and deduplicated. Additional hosts may appear
before or after the workspace, before the standalone `--`. A host is nonempty,
contains no line breaks, and does not begin with `-`.

```text
smolsandbox start --allow-host packages.example.com /path/to/project --allow-host downloads.example.com
smolsandbox repair --check --allow-host packages.example.com --allow-host downloads.example.com /path/to/project
```

Repeat the same additions when inspecting or replacing that workspace
configuration. A host already in the definition does not make a configuration
custom; an additional host beyond the definition does. Changing immutable
options selects a different workspace machine. Ports are mutable and excluded
from immutable identity.

### List machines and compare selection

List all workspace generations, profiles, revisions, and configurations
associated with the canonical current directory or an explicit workspace:

```text
smolsandbox list
smolsandbox list /path/to/project
smolsandbox list --all
smolsandbox list --all --json
smolsandbox list --smolfile ./codex.smlbx --allow-host packages.example.com /path/to/project
```

`--all` adds other workspaces, shared builders and login machines, and
unassociated legacy rows. An explicit workspace still selects the comparison
context with `--all`. A repository subdirectory remains a separate workspace.
The workspace path hash establishes association across generations; a legacy
name or configuration hash cannot recover a path. Scoped output counts
workspace candidates with missing or malformed path hashes without claiming
that those candidates belong to the reference workspace.

The reference uses the same selected definition, runtime version, artifact
directory, harness home, immutable options, and machine-name override as
`start`. Repeat the same `--allow-host` additions to compare a custom
configuration. `list` rejects `--port`, mutation flags, and guest arguments.
The raw runtime state remains separate from the identity assessment:
`selected`, `different-policy`, `different-base`, `different-configuration`,
`other-workspace`, `shared`, `unassessed`, or `unknown`. Reasons describe all
known differences relative to the invocation. An unfamiliar runtime state
remains visible with an explanation that lifecycle compatibility is unknown.
Assessments neither authorize deletion nor guarantee artifact availability,
port reconciliation, guest boot, or authentication.

When automatic lookup finds no default definition, listing still succeeds
and reports unavailable reference fields. Explicit missing definitions,
including `BASE_SMOLFILE`, and invalid or unreadable selected definitions
remain errors. `--allow-host` requires an available definition. Listing never
falls back past an invalid nearer definition.

Human output prints the reference and full machine identities. `--json`
writes one object with schema `version: 1`, `scope`, canonical `workspace`,
`reference`, `unassociated_workspace_count`, `warnings`, and sorted `machines`.
The reference contains `definition`, `base_id`, `revision`,
`workspace_sha256`, and `desired_machine`; unavailable values are null.
Each row contains `name`, raw `state`, `kind`, `base_id`, `revision`,
`workspace_path_sha256`, `workspace_sha256`, `configuration`, `source`,
`assessment`, and `reasons`. Sources are `labels`, `legacy-name`, or `unknown`.
Malformed inventory is fatal and produces no partial JSON; contradictory
per-machine metadata remains visible as `unknown`.

Listing calls only SmolVM's version and inventory interfaces. It skips
virtualization and mutation capability preflight, takes no lifecycle locks,
and does not create directories, rewrite caches, seed credentials, recover
port journals, or mutate machines. The report describes one snapshot.
SmolVM itself may maintain its own registry while answering inventory queries.

### Remove managed machines

`remove` deletes guest writable state without an interactive confirmation.
By default, removal selects managed workspace machines whose policy revision
is older than the executable's current revision (`5`), across profiles,
bases, and configurations. Package version changes, different definitions,
and different configurations alone do not make a machine stale. Revisions
compare numerically, including leading zeros and values too large for native
integers. Current and newer revisions remain protected unless `--all` is set.

```text
smolsandbox remove --dry-run
smolsandbox remove
smolsandbox remove /path/to/project --all --force
smolsandbox remove --global --all --force --dry-run
smolsandbox remove --smolvm /path/to/smolvm /path/to/project
```

Workspace scope uses the canonical invocation directory or explicit WORKSPACE,
including symlink resolution. Git roots do not widen the scope; repository
subdirectories and directories without Git metadata are valid workspaces.
Only a valid full workspace path hash establishes association. Machines with
missing or malformed path hashes are counted without assigning a workspace;
workspace removal excludes those machines. Other workspaces and shared login
and builder machines remain preserved in workspace scope.

The three selection options compose independently:

| Scope and revision options | Selected identities |
| --- | --- |
| No options | Associated workspace machines with older policy revisions |
| `--all` | Associated workspace machines of every revision |
| `--global` | Managed workspace and login machines with older policy revisions |
| `--global --all` | Managed workspace, login, and revisionless builder machines |

For each row, omission of `--force` deletes only stopped or created machines
and reports eligible running machines as skipped. Adding `--force` stops
selected running machines, verifies the same incarnation is stopped, and
then deletes them. An unknown selected runtime state refuses the operation.
`--all` never changes ownership, workspace scope, or permission to stop.
Global scope includes only the selected runtime's visible user registry.
Complete managed labels can establish ownership for custom names. Complete
supported legacy names establish global ownership but cannot recover a path.
Incomplete or contradictory in-scope metadata refuses removal even with force.

`--dry-run` composes with every combination and reports planned `delete`,
`stop-and-delete`, and `skip-running` actions. Preview queries inventory and
runtime version without lifecycle locks, application directory creation,
permission changes, port recovery, or cache updates. SmolVM may maintain its
own registry while answering queries. A preview is one snapshot, so later
execution can find a different inventory. Empty selection and bounded skips
exit successfully; invalid arguments exit `2`, and operational failures exit
nonzero.

Removal needs no definition, artifact, generation cache, replacement, or guest
boot. `--smolvm` and `SMOLVM_BIN` select the runtime. `--smolfile`,
`--allow-host`, `--port`, `--replace-stale`, `--check`, `--workspace`,
`--force-all`, and guest arguments are rejected. Mutation preflight checks only
required inventory, deletion, and stop capabilities; virtualization, creation,
packing, and guest execution are not prerequisites.

Execution sorts the plan by full machine name, validates selected creation
timestamps and safe state filenames, then acquires the application mutation
lock and checks fresh inventory before mutations. Each target is revalidated
immediately before stop or deletion. Changed identity, incarnation, reported
ports, or relevant state refuses further processing. Planned running skips
remain skipped; a planned stopped target starting does not authorize a new
stop. Runtime queries and deletion are not an atomic transaction because
external SmolVM clients do not honor application locks.

After deletion succeeds, fresh inventory must confirm the name is absent
before the configured `STATE_ROOT` port manifest and pending journal are
removed. Failure preserves port state when deletion or absence verification
fails. A local cleanup failure after confirmed deletion reports the partial
result. Processing stops on any failure, reports completed and unprocessed
targets, and never claims rollback or restarts stopped machines.

Every cached base artifact, definition, generation and compatibility cache,
host harness home, credential, and unrelated resource remains preserved.
Missing lock directories may be created during execution. Existing startup
recovery handles removed machines; `purge` remains responsible for artifacts.

### Exact-set TCP ports

Repeat `--port` on `start` to publish the same guest and host TCP port numbers:

```text
smolsandbox start --port 3000 /path/to/project --port 5173 --port 9229
```

Ports are ASCII decimal integers from 1 through 65535. Duplicates and leading
zeros are normalized; at most 64 distinct ports are accepted. Listeners bind
to host loopback by default, so port 3000 can be reached at
`http://127.0.0.1:3000`. Publishing a port neither starts the guest service nor
changes the service's guest bind address.

Every start requests an exact set. Omitting all `--port` values clears previous
mappings; adding or removing ports preserves machine identity and writable
state. Port manifests and pending journals track the machine incarnation and
requested set. Interrupted updates recover from that journal before start.
A running machine with a different or unknown port set is refused without
stopping it. Stop the active session before requesting another set.

### Reconciliation and replacement

Normal reconciliation derives desired identity from the selected definition,
runtime version, and immutable options, then checks SmolVM inventory. Missing
or stale compatibility caches do not prevent recovery of a verified matching
machine. Malformed inventory fails, and ambiguous ownership never authorizes
replacement. A verified desired machine may be reused while other stale or
ambiguous candidates remain; the application warns about those candidates.
If only stale machines match the workspace, normal startup refuses rather
than creating or deleting machines.

Inspect candidates and then explicitly authorize replacement:

```text
smolsandbox repair --check /path/to/project
smolsandbox repair --replace-stale /path/to/project
smolsandbox repair --replace-stale --force /path/to/project
```

`repair --check` is read-only and lock-free. Replacement prepares the current
base, creates the desired machine if needed, and verifies readiness from fresh
inventory before deleting eligible stale machines. `--force` authorizes
stopping running stale machines only after that readiness verification. Only
positively associated workspace machines are eligible; unrelated machines
remain untouched.

A failed preparation or verification preserves stale machines and caches.
A newly registered desired machine is retained for inspection after failure.
If later deletion fails, cleanup stops, leaving the verified desired machine
and unprocessed stale machines available; compatibility caches are not
rewritten after that partial handoff.

### Session completion and failures

The application stops a workspace machine that the invocation started after
the guest exits or an interceptable termination. A workspace machine already
running before `start` remains running. Login stops its login machine after
the authentication command. Forced termination or a host crash cannot
guarantee cleanup.

The guest's nonzero exit status remains primary when stopping the machine also
fails. A cleanup failure after a successful guest command produces an
operational failure. Failed start attempts still attempt owned-machine stop;
seeding failure warns and continues with existing harness contents.

### Cleanup and purge

Cleanup deletes managed guest disk state. Select the definition and scope
before invoking a cleanup command:

```text
smolsandbox clean --workspace /path/to/project
smolsandbox clean
smolsandbox purge
smolsandbox purge --force
smolsandbox purge --force-all
```

`clean --workspace` stops and deletes positively owned workspace machines for
the canonical path across generations and configurations. It preserves
builders, login machines, artifacts, generation caches, and other workspaces.
Default `clean` stops and deletes the selected generation's managed machines,
then removes its cached artifact and generation caches. Both clean modes can
stop a selected running machine. Both preserve definition files, harness
homes, and unrelated resources.

Purge policies operate only on eligible stopped managed machines:

| Policy | Selected machines |
| --- | --- |
| `purge` | Superseded generations; refuses all cleanup if a current custom workspace configuration exists. |
| `purge --force` | Superseded generations and current custom workspace configurations, while preserving current defaults. |
| `purge --force-all` | All stopped managed builders, login machines, and workspaces in scope. |

Every purge policy preserves running machines, their artifact generations,
and the current artifact. Purge never stops a running machine. Ambiguous
managed metadata or a relevant inventory change stops cleanup. A missing
current artifact permits only the bounded recovery of a stopped current
builder; other machines and artifacts remain preserved for that recovery.

Machine deletion precedes removal of the machine's port manifest and pending
journal. A failed deletion preserves that port state and stops processing later
machines. Generation clean and purge preserve artifacts when a machine
deletion fails. Definitions and harness state survive every cleanup policy.

## State and overrides

The application uses the native current user's `~/.smolsandbox/` directory:

```text
~/.smolsandbox/
  default.smlbx
  artifacts/
    codex-devtools-<base-id>.smolmachine
  state/
    generation.json
    base-id
    machine-revision
    ports/
      <machine>.json
      <machine>.pending.json
  locks/
    artifacts.lock
    workspace-<workspace-path-sha256>.lock
  codex-home/
  harness-home/
    claude/
    opencode/
    hermes/
```

`generation.json` records versioned generation state; `base-id` and
`machine-revision` are compatibility caches. Port manifests store committed
sets, and pending journals permit recovery of interrupted updates. Unknown
newer state formats fail while preserving the files. Locks coordinate artifact
cleanup and workspace lifecycle work. Codex uses `codex-home/`; the other
profiles have separate directories under `harness-home/`.

The canonical workspace is mounted at `/workspace/myrepo` in the Linux guest.
Only the selected profile's host harness home is mounted at the guest state
path listed in the profile table. No generated application state is written
to the workspace by default.

| Environment variable | Meaning |
| --- | --- |
| `BASE_SMOLFILE` | Explicit definition path when `--smolfile` is absent; relative to the invocation directory. |
| `SMOLVM_BIN` | Runtime executable path when `--smolvm` is absent. |
| `ARTIFACTS_DIR` | Override the artifact directory. |
| `STATE_ROOT` | Override generation caches and port-state directory. |
| `CODEX_HOME_DIR` | Override persistent Codex state for the Codex profile. |
| `BUILDER_MACHINE` | Override the builder machine name. |
| `LOGIN_MACHINE` | Override the login machine name. |
| `CODEX_MACHINE` | Override the workspace machine name; ownership validation still applies. |
| `OPENAI_API_KEY` | Forward a nonempty key to Codex workspace execution by secret reference. |

Nonempty relative directory overrides resolve against the invocation directory.
Native temporary-file creation follows platform conventions, including
`TMPDIR` on Unix. `SMOLVM_TEST_HOME` is available only in `test-adapter` builds;
it is not a production application-home override. There is no production
`CODEX_SANDBOX_HOME` or `SANDBOX_ENSURE_LOGIN` interface.

## Historical shell workflow and state

The shell workflow, Python helpers, definition, shell tests, and fixture were
removed from `src-old/` in commit `28087885c2b84077c62ee92c5560a2cab1b68997`.
The sources remain available in Git history at the preceding revision,
`16318afadc4b37fc5ec7919997ee83de1b19abcf`. Earlier feature specifications and
research preserve historical command names and paths.

Rust starts with fresh application state. It neither imports nor removes
legacy repository-local `artifacts/` or `.smolvm/` contents. Those paths remain
legacy state; automatic migration and automatic deletion are not provided.
The historical executable name `codex-sandbox` has no executable wrapper or
alias. Existing native Rust definitions and state survive the executable
rename. Returning users can map historical shell actions to Rust commands:

| Removed historical shell entry point | Rust command |
| --- | --- |
| `build-sandbox.sh`, `build-login.sh` | `build` |
| `start-codex.sh`, `codex-sandbox.sh` | `start` |
| `start-login.sh` | `login` |
| `repair-sandbox.sh` | `repair` |
| `clean-sandbox.sh` | `clean` |
| `purge-sandbox.sh` | `purge` |

The table maps actions; the former shell names are not Rust compatibility
commands. The metadata marker, managed resource names, and label namespace
retain their historical spelling to preserve identity and ownership.
[Feature 0008](FEATURES/feature-0008-provisioning-download-integrity.md) tracks
additional provisioning-download integrity independently of migration and
documentation completion.

## Validation and cutover gates

Run the independent Rust checks from the checkout:

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --features test-adapter --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --features test-adapter --all-targets
cargo build --locked --release --bin smolsandbox
cargo run --locked --example check_baseline_independence
```

The tests use fixed expectations and a Rust SmolVM test double.
`check_baseline_independence` copies the source while excluding `src-old/`,
builds and runs both feature configurations, builds a release executable,
and runs the compiled tests with an empty `PATH`. This checks that the Rust
suite needs neither historical shell/Python sources nor host script tools.

`test-adapter` enables native-home injection and a virtualization bypass for
the Rust test double. Distributed executables use the default features;
exclude `test-adapter` from release builds. The
[regression coverage map](FEATURES/feature-0009-coverage.md) identifies the
independent policy tests. [AGENTS.md](AGENTS.md) contains the maintenance rules
for templates, runtime revision, and lifecycle regression checks.

The [native CI matrix](.github/workflows/rust.yml) runs mocked suites and builds
the executable on each release target, including Windows credential-permission
tests. The [Feature 0009 validation record](FEATURES/feature-0009-validation.md)
records the user's confirmation that CI was green; no CI run URL accompanied
that confirmation. Preserve the dated observations separately from new results.
The [Feature 0004 validation record](FEATURES/feature-0004-validation.md) retains
its user-directed `Implemented` marker and historical execution results;
that marker does not establish real guest validation. Physical baseline
removal is now recorded separately in the
[current Feature 0010 audit](FEATURES/feature-0010-validation.md#source-removal-and-documentation-audit-2026-11-04).

Final cutover acceptance requires recorded native mocked execution, real
guest validation for every profile and release target, and physical baseline
removal. Source removal has passed. Keep final acceptance pending until the
remaining execution evidence is recorded. The configured CI workflow runs
mocked checks; real guest authentication and terminal checks require separate
native execution.

On compatible native hosts, use a disposable native user account and workspace
for the real guest procedure. For each profile:

1. Generate and retain a complete profile definition with `create-config`.
2. Build the guest and inspect `smolvm machine ls --json` for managed labels.
3. Run authentication and the interactive guest harness.
4. Publish a guest service, change the exact port set, and clear the mappings.
5. Check guest exit status, terminal resize, Ctrl-C, and stop behavior.
6. Replace an eligible stale machine and verify creation precedes deletion.
7. Exercise workspace clean, generation clean, and purge with unrelated state.
8. Copy only the release executable to an unrelated directory and repeat
   startup without host Bash/Python on `PATH`.

Record the source revision, native target, profile, commands or smoke procedure,
and `passed`, `failed`, `not_run`, or `inconclusive` for each result. Include
CI run links when CI supplies the evidence. Real provisioning, interactive
terminal behavior, and authentication require observed results of their own.
