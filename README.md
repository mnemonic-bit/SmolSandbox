# smolsandbox

Run Claude, Codex, OpenCode, or Hermes Agent in a persistent SmolVM sandbox.
The Rust executable embeds the four configuration templates. Workspace files
are mounted into the guest, while each harness keeps its own persistent state.

## Build or copy the executable

Building from this checkout requires a Rust toolchain:

```text
cargo build --locked --release --bin smolsandbox
```

Copy `target/release/smolsandbox` to a directory on `PATH`. On Windows, copy
`target/release/smolsandbox.exe`. A copied executable needs no checkout,
adjacent templates, host Bash/Python, host harness installation, or Rust
toolchain. Guest provisioning still uses the commands in the selected template.

Install [SmolVM](https://github.com/smol-machines/smolvm#install) separately.
The application does not install or bundle SmolVM. The configured release
matrix covers Linux x86-64 and ARM64 with KVM, macOS ARM64 with
Hypervisor.framework, and Windows x86-64 with Windows Hypervisor Platform.
See [host requirements and validation scope](RUST-APPLICATION.md#executable-and-host-requirements)
for native prerequisites and the distinction between mocked and real guest
validation.

## First use

Generate a default definition, build its guest, authenticate, and launch the
selected harness in a project:

```text
smolsandbox create-config --harness codex
smolsandbox build /path/to/project
smolsandbox login
smolsandbox start /path/to/project
```

Choose `claude`, `codex`, `opencode`, or `hermes` with `--harness`. Generation
is offline and writes `~/.smolsandbox/default.smlbx` atomically. An existing
file is never overwritten. Inspect the generated definition before building.
The selected template installs the harness and development tools in the guest.

To keep another profile in a named file, use `--smolfile` for creation and
selection:

```text
smolsandbox create-config --harness claude --smolfile claude.smlbx
smolsandbox build --smolfile ./claude.smlbx /path/to/project
smolsandbox login --smolfile ./claude.smlbx
smolsandbox start --smolfile ./claude.smlbx /path/to/project
```

For `create-config`, a relative destination is relative to the current
directory. For lifecycle commands, `./claude.smlbx` is an explicit path;
a bare `claude.smlbx` searches the workspace repository's `.smolsandbox/`
directory, then `~/.smolsandbox/`. See the authoritative
[configuration rules](RUST-APPLICATION.md#configuration-and-profiles) for
precedence, whole-file selection, and explicit template adoption.

## Launch options and help

Select a separately installed runtime with `--smolvm`:

```text
smolsandbox build --smolvm /path/to/smolvm /path/to/project
```

Repeat `--allow-host` to extend the selected definition's network policy.
Repeat `--port` to publish guest TCP services on the same host loopback ports:

```text
smolsandbox start --allow-host packages.example.com --allow-host downloads.example.com --port 3000 --port 5173 /path/to/project
```

Each start requests an exact port set. Omitting every `--port` clears previous
mappings. Port changes preserve machine identity; host additions affect
immutable workspace identity. The guest service must listen on the requested
port. See [lifecycle commands](RUST-APPLICATION.md#lifecycle-commands) for
limits, running-machine refusal, replacement, and exit-status behavior.

Application help is available without configuration or SmolVM. Arguments
after `--` are passed to the guest harness:

```text
smolsandbox --help
smolsandbox start --help
smolsandbox --version
smolsandbox start /path/to/project -- --help
```

## Inspect and clean up

List machines associated with the current directory, or inspect every managed
machine including shared builders and login machines:

```text
smolsandbox list
smolsandbox list /path/to/project
smolsandbox list --all
smolsandbox list --all --json
```

Listing compares each machine's identity with the current invocation and
shows runtime state separately. Repeat the selected definition and any
`--allow-host` additions used by `start` to compare that configuration.
Legacy workspace names cannot establish a path association; use `--all` to
see those rows. Listing takes no lifecycle locks and writes no application
state. Assessments do not authorize deletion or guarantee a successful start.

Inspect stale machines before authorizing replacement:

```text
smolsandbox repair --check /path/to/project
smolsandbox repair --replace-stale /path/to/project
```

Replacement verifies the desired machine before deleting eligible stale
machines. Add `--force` only when authorizing a running stale machine to be
stopped after that verification. Repeat the configuration's `--allow-host`
options when repairing a workspace with additional hosts.

The cleanup commands delete managed machines and can discard their guest disk
state. `clean --workspace` targets one workspace across generations; `clean`
targets the selected generation. `purge` removes eligible superseded resources
and refuses when a current custom configuration exists. The force policies
expand eligibility while preserving running machines and protected artifacts.
Definitions and persistent harness state survive these commands. Review the
[cleanup and purge rules](RUST-APPLICATION.md#cleanup-and-purge) before use.

```text
smolsandbox clean --workspace /path/to/project
smolsandbox clean
smolsandbox purge
smolsandbox purge --force
smolsandbox purge --force-all
```

## Operating reference and project status

[Operation and validation](RUST-APPLICATION.md) is the authoritative reference
for profiles, environment overrides, application state, lifecycle safeguards,
and contributor checks. Repository maintenance instructions are in
[AGENTS.md](AGENTS.md).

The former shell workflow is retained as [historical source in `src-old/`](src-old/README.md).
Rust uses `~/.smolsandbox/` and neither imports nor removes the legacy
repository-local `artifacts/` or `.smolvm/` state.

Features 0004 and 0009 retain their recorded `Implemented` status following
the user's CI confirmation. Feature 0010 prepares Rust-first documentation;
final cutover still requires the real guest evidence and baseline removal
listed in the [Feature 0010 validation record](FEATURES/feature-0010-validation.md).
