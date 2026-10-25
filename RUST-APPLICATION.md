# smolsandbox: operation and validation

Feature 0004 remains `Proposed`. The Rust implementation is available for
validation. The existing scripts remain the parity baseline until the native
mocked suite and real SmolVM smoke tests pass on every release target. No
intermediate Rust stage is a supported release.

The Rust executable was renamed from `codex-sandbox` to `smolsandbox`. Update
callers to invoke `smolsandbox`; no old-name executable alias is provided.
Existing native definitions, credentials, application state, and machines are
retained. Codex remains one of the four supported harnesses. This rename does
not complete the Feature 0004 or Feature 0010 cutover gates.

## Executable and host requirements

Build the application with:

```text
cargo build --locked --release --bin smolsandbox
```

Copy `target/release/smolsandbox` (Windows: `smolsandbox.exe`) to any
directory. The executable contains every configuration template. Running the
copied executable requires no checkout, adjacent resources, host Bash, host
Python, host harness installation, or Rust toolchain.

Install [SmolVM](https://github.com/smol-machines/smolvm#install) separately.
SmolVM is neither bundled nor downloaded by this application. Select SmolVM
with `--smolvm <path>`, then `SMOLVM_BIN`, then `PATH`, in that order. An explicit
selection that fails is an error. Capability preflight checks the selected
executable, required CLI operations, inventory JSON, and native virtualization
prerequisites before lifecycle mutation.

The validation matrix follows the native CLI distributions documented by
[SmolVM v1.16.2](https://github.com/smol-machines/smolvm/blob/v1.16.2/README.md):

| Host | Architecture | Native prerequisite | Rust target |
| --- | --- | --- | --- |
| Linux | x86-64 | Read/write access to `/dev/kvm` | `x86_64-unknown-linux-gnu` |
| Linux | ARM64 | Read/write access to `/dev/kvm` | `aarch64-unknown-linux-gnu` |
| macOS | ARM64 | Hypervisor.framework | `aarch64-apple-darwin` |
| Windows | x86-64 | Windows Hypervisor Platform | `x86_64-pc-windows-msvc` |

macOS Intel and Windows ARM64 are outside this release matrix. Compiling a
target does not establish native execution or guest compatibility. CI defines
the mocked native runs; real guest tests remain separate cutover gates.

## Configuration and profiles

Generate one user-owned definition without network access or SmolVM:

```text
smolsandbox create-config --harness codex
```

The accepted choices are `claude` (Claude), `codex` (Codex), `opencode`
(OpenCode), and `hermes` (Hermes Agent). Each template installs development
tools and the selected harness inside the guest. The command creates
`~/.smolsandbox/default.smlbx` atomically. An existing destination, including
one created concurrently, is never overwritten. Invalid choices create no
directories or files.

Inspect and edit the generated Smolfile. Preserve its single
`# codex-sandbox-harness: <identifier>` metadata line. The marker and the
`codex-sandbox.` managed-label namespace are retained compatibility identifiers;
the executable rename does not rewrite definitions or machine metadata.
Configuration selection uses exactly one complete file; definitions are never
merged. Updating or moving the executable never refreshes an existing definition.

Lifecycle commands select a definition in this order:

1. Use `--smolfile <name-or-path>` when supplied.
2. Otherwise, use `BASE_SMOLFILE` as an explicit path.
3. Otherwise, select the filename `default.smlbx`.

A bare flag filename is matched exactly under the selected workspace's
repository `.smolsandbox` directory, then under `~/.smolsandbox`. Without a
workspace argument, discovery uses the invocation directory. Outside a
repository, filename lookup searches only the user directory. A missing named
file never selects the default. An invalid, unreadable, or unsupported nearer
file fails instead of selecting a farther candidate.

A flag value containing a native path delimiter is an explicit path. Relative
paths resolve against the invocation directory. Explicit paths bypass filename
lookup. `BASE_SMOLFILE` always has explicit-path semantics, even for a bare
filename. The executable's directory never participates in discovery.

```text
smolsandbox start --smolfile claude.smlbx /path/to/project
smolsandbox build --smolfile ./configs/custom.smlbx /path/to/project
```

To keep all four profiles, generate one default, rename that file to a chosen
filename, then generate another default. `create-config` always targets the
user's `default.smlbx`; it ignores `BASE_SMOLFILE` and repository discovery.

| Profile | Guest launch | `smolsandbox login` | Persistent guest state |
| --- | --- | --- | --- |
| Codex | `codex --no-daemon` | `codex login --device-auth` | `/root/.codex` |
| Claude | `env CLAUDE_CONFIG_DIR=/root/.claude claude` | Same environment, `claude auth login` | `/root/.claude` |
| OpenCode | `opencode` | `opencode auth login` | `/root/.local/share/opencode` |
| Hermes Agent | `hermes` | `hermes setup` | `/root/.hermes` |

These conventions follow the selected profiles' authentication documentation:
[Claude](https://code.claude.com/docs/en/authentication),
[OpenCode](https://opencode.ai/docs/cli/), and
[Hermes Agent](https://hermes-agent.nousresearch.com/docs/getting-started/installation/).
Only the selected harness's home is mounted. Each login machine uses the
profile's authentication allowlist. For providers outside that allowlist,
authenticate through the harness in a workspace machine with the required
`--allow-host` additions. Credentials remain the harness's concern.

## Lifecycle commands

```text
smolsandbox build /path/to/project
smolsandbox login
smolsandbox start /path/to/project -- --help
smolsandbox start --allow-host packages.example.com --port 3000 /path/to/project
smolsandbox repair --check /path/to/project
smolsandbox repair --replace-stale --force /path/to/project
smolsandbox clean --workspace /path/to/project
smolsandbox clean
smolsandbox purge
smolsandbox purge --force
smolsandbox purge --force-all
```

Every lifecycle command accepts `--smolfile` and `--smolvm`. Help and version
require neither configuration nor SmolVM. Unknown options, conflicting modes,
excess positional arguments, and invalid values exit `2`. Operational errors
exit nonzero. Arguments after the first standalone `--` belong to the guest
harness and retain their original argument boundaries.

`--allow-host` is repeatable on `start` and `repair`. Additions extend the
definition's allowlist. Sorted, deduplicated immutable creation options select
the workspace machine. Repeat the same additional hosts on repair commands
for that configuration. The complete selected file bytes and SmolVM version
determine the base generation.

`--port` is repeatable on `start`. Ports are ASCII decimal integers from 1
through 65535, with at most 64 distinct values. Ports publish TCP on host
loopback using the same guest and host number. Every launch requests an exact
set; omission clears previous mappings. Port changes retain machine identity
and disk state. A running machine with a different or unknown set is refused
without stopping that machine.

Normal reconciliation preserves stale machines. Explicit replacement prepares
and verifies the desired machine from fresh inventory before deleting eligible
stale machines. `--force` permits stopping running stale machines only after
that readiness gate. `repair --check` is read-only and lock-free.

`login` requires an existing current-generation login machine. It never creates
a missing login machine. Startup seeds missing harness files without
overwriting existing state. Codex forwards a nonempty `OPENAI_API_KEY` by
SmolVM environment-variable reference; the value is never placed in arguments
or configuration. Other profiles do not receive Codex flags or this secret
reference by default.

The application stops a machine started by its invocation after the guest
command finishes or an interceptable termination. An already-running workspace
machine remains running. The guest exit status remains primary if cleanup
also fails; cleanup failure alone produces operational failure. Forced
termination and host crashes cannot guarantee cleanup.

Default `clean` removes the selected generation's machines, artifact, and
generation caches. `clean --workspace [path]` removes only positively owned
workspace machines across generations and configurations. Both preserve
definition files and harness homes. Port state is removed only after the
associated machine has been deleted.

Conservative `purge` refuses all cleanup when a current custom workspace
configuration exists. `--force` may remove stopped custom configurations and
legacy machines while retaining current defaults. `--force-all` may remove
all stopped managed machines in scope. Every policy preserves running
machines, their artifact generations, and the current artifact. Purge never
stops a running machine.

## State and overrides

```text
~/.smolsandbox/
  default.smlbx
  artifacts/
  state/
    generation.json
    base-id
    machine-revision
    ports/
  locks/
  codex-home/
  harness-home/
    claude/
    opencode/
    hermes/
```

The application uses the native current user's home. Applicable component
overrides are `ARTIFACTS_DIR`, `STATE_ROOT`, `CODEX_HOME_DIR`, `BUILDER_MACHINE`,
`LOGIN_MACHINE`, and `CODEX_MACHINE`. Native temporary-file creation honors
the platform's temporary-directory conventions, including `TMPDIR` on Unix.
There is no `CODEX_SANDBOX_HOME` or `SANDBOX_ENSURE_LOGIN` interface. State and
port formats are versioned; unknown newer formats fail without rewriting.

The Rust application starts with fresh application state. It neither imports
nor removes shell-era `artifacts/` or `.smolvm/` contents. No generated state is
written to the workspace by default. The workspace is mounted at
`/workspace/myrepo` in the Linux guest.

| Shell entry point | Rust command |
| --- | --- |
| `build-sandbox.sh`, `build-login.sh` | `build` |
| `start-codex.sh`, `codex-sandbox.sh` | `start` |
| `start-login.sh` | `login` |
| `repair-sandbox.sh` | `repair` |
| `clean-sandbox.sh` | `clean` |
| `purge-sandbox.sh` | `purge` |

The final cutover removes the old entry points without wrappers. Provisioning
pins and existing verification are preserved in the Codex template.
[Feature 0008](FEATURES/feature-0008-provisioning-download-integrity.md) tracks
additional download integrity work independently; that work does not block
Feature 0004.

## Validation and cutover gates

Run the Rust checks with:

```text
cargo fmt --all -- --check
cargo clippy --locked --features test-adapter --all-targets -- -D warnings
cargo test --locked --features test-adapter --all-targets
cargo build --locked --release --bin smolsandbox
```

`test-adapter` enables a test-only native-home injection and virtualization
bypass for the Rust SmolVM test double. The default release has neither
bypass. Do not distribute builds with that feature enabled. Linux differential
tests also invoke the retained shell/Python baseline; the application itself
does not invoke those host tools.

The [native CI matrix](.github/workflows/rust.yml) runs the mocked suite and
builds the executable on every release target. CI configuration alone is not
evidence that those runs passed. Before removing the baseline, record passing
native mocked runs and execute the following real SmolVM checks on each target
with each profile:

1. Generate and retain a separate complete definition for the profile.
2. Build the guest and inspect `smolvm machine ls --json` for managed labels.
3. Run the profile's authentication workflow and interactive guest harness.
4. Publish a guest service, change the exact port set, and clear the mappings.
5. Verify guest exit status, terminal resize, Ctrl-C, and stop behavior.
6. Replace an eligible stale machine and verify creation precedes deletion.
7. Exercise workspace clean, generation clean, and purge with unrelated state.
8. Copy only the executable to an unrelated directory and repeat startup
   without host Bash/Python on `PATH`.

Real guest provisioning, interactive terminal behavior, and cross-platform
execution require their own observed results. Keep the scripts and their
fixtures until every required gate has passed.

The historical [Feature 0004 validation record](FEATURES/feature-0004-validation.md)
retains the executable names used in its recorded commands and lists outstanding
cutover evidence. Current commands use `smolsandbox`.
