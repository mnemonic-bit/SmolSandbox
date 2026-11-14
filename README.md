# smolsandbox

[![Version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fmnemonic-bit%2FAI-Sandbox%2Fdevelop%2FCargo.toml&query=%24.package.version&label=version&prefix=v&color=blue)](https://github.com/mnemonic-bit/AI-Sandbox/blob/develop/Cargo.toml)
[![Build and tests (develop)](https://github.com/mnemonic-bit/AI-Sandbox/actions/workflows/rust.yml/badge.svg?branch=develop&event=push)](https://github.com/mnemonic-bit/AI-Sandbox/actions/workflows/rust.yml?query=branch%3Adevelop+event%3Apush)

Run Claude, Codex, OpenCode, or Hermes Agent in a persistent SmolVM sandbox
with `smolsbx`.
The Rust executable embeds the four configuration templates. Workspace files
are mounted into the guest, while each harness keeps its own persistent state.

## Build or copy the executable

Building from this checkout requires a Rust toolchain:

```text
cargo build --locked --release --bin smolsbx
```

Copy `target/release/smolsbx` to a directory on `PATH`. On Windows, copy
`target/release/smolsbx.exe`. A copied executable needs no checkout,
adjacent templates, host Bash/Python, host harness installation, or Rust
toolchain. Guest provisioning still uses the commands in the selected template.

Alternatively, install the command from this checkout:

```text
cargo install --locked --path . --bin smolsbx
```

If you previously used the `smolsandbox` command, update scripts and shell
aliases to invoke `smolsbx`. Existing `.smolsandbox` definitions, credentials,
and machines remain available. See [command migration](RUST-APPLICATION.md#command-migration)
for the retained state paths and installation behavior.

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
smolsbx create-config --harness codex
smolsbx build /path/to/project
smolsbx login
smolsbx start /path/to/project
```

Choose `claude`, `codex`, `opencode`, or `hermes` with `--harness`. Generation
is offline and writes `~/.smolsandbox/default.smlbx` atomically. An existing
file is never overwritten. Inspect the generated definition before building.
The selected template installs the harness and development tools in the guest.

To keep another profile in a named file, use `--smolfile` for creation and
selection:

```text
smolsbx create-config --harness claude --smolfile claude.smlbx
smolsbx build --smolfile ./claude.smlbx /path/to/project
smolsbx login --smolfile ./claude.smlbx
smolsbx start --smolfile ./claude.smlbx /path/to/project
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
smolsbx build --smolvm /path/to/smolvm /path/to/project
```

Repeat `--allow-host` to extend the selected definition's network policy.
Repeat `--port` to publish guest TCP services on the same host loopback ports:

```text
smolsbx start --allow-host packages.example.com --allow-host downloads.example.com --port 3000 --port 5173 /path/to/project
```

Each start requests an exact port set. Omitting every `--port` clears previous
mappings. Port changes preserve machine identity; host additions affect
immutable workspace identity. The guest service must listen on the requested
port. See [lifecycle commands](RUST-APPLICATION.md#lifecycle-commands) for
limits, running-machine refusal, replacement, and exit-status behavior.

Application help is available without configuration or SmolVM. Arguments
after `--` are passed to the guest harness:

```text
smolsbx --help
smolsbx start --help
smolsbx --version
smolsbx start /path/to/project -- --help
```

Routine SmolVM management and provisioning output is suppressed by default.
Application results, warnings, and errors remain visible. Add `--verbose`
before the command or among its application arguments to stream runtime
stdout and stderr live to application stderr:

```text
smolsbx build /path/to/project
smolsbx --verbose build /path/to/project
smolsbx start /path/to/project --verbose
smolsbx list --all --json --verbose
```

Interactive shell, harness, and login sessions always inherit terminal input
and output, including prompts and authentication instructions. Preparations
and cleanup follow the management output policy. Query stdout remains
internal, so `list --json` keeps its single JSON object on stdout in both
modes. Failures include application diagnostics even without `--verbose`.
`start --verbose` configures smolsandbox output; `start -- --verbose` passes
`--verbose` unchanged to the guest harness. Repeating the application flag
has the same effect as specifying it once.

## Inspect and clean up

List machines associated with the current directory, or inspect every managed
machine including shared builders and login machines:

```text
smolsbx list
smolsbx list /path/to/project
smolsbx list --all
smolsbx list --all --json
```

Listing compares each machine's identity with the current invocation and
shows runtime state separately. Repeat the selected definition and any
`--allow-host` additions used by `start` to compare that configuration.
Legacy workspace names cannot establish a path association; use `--all` to
see those rows. Listing takes no lifecycle locks and writes no application
state. Assessments do not authorize deletion or guarantee a successful start.

Remove older-policy workspace machines for the canonical current directory:

```text
smolsbx remove --dry-run
smolsbx remove
smolsbx remove /path/to/project --all --force
smolsbx remove --global --all --force --dry-run
```

`remove` deletes guest writable state without a confirmation prompt. Stale
means a policy revision older than the executable's current revision; a
different base or configuration alone is not stale. `--all` includes current
and newer revisions, `--force` permits stopping selected running machines,
and `--global` includes all managed workspaces and shared machines visible
to the selected runtime. These options are independent. Builders have no
revision and require `--global --all`. Without `--force`, eligible running
machines are skipped. A repository subdirectory is its own workspace;
unassociated legacy workspaces require global scope.

`--dry-run` previews the selection without application writes or locks.
Removal needs no definition, base artifact, or generation cache. Artifacts,
definitions, generation caches, host credentials, and persistent harness state
survive removal. See the [removal rules](RUST-APPLICATION.md#remove-managed-machines)
for validation, concurrency, and partial-failure behavior.

Inspect stale machines before authorizing replacement:

```text
smolsbx repair --check /path/to/project
smolsbx repair --replace-stale /path/to/project
```

Replacement verifies the desired machine before deleting eligible stale
machines. Add `--force` only when authorizing a running stale machine to be
stopped after that verification. Repeat the configuration's `--allow-host`
options when repairing a workspace with additional hosts.

The cleanup commands delete managed machines and can discard their guest disk
state. `clean --workspace` targets one workspace across generations; `clean`
targets the selected generation. `purge` removes eligible superseded resources
and refuses when a current custom configuration exists. Purge force policies
expand eligibility while preserving running machines and protected artifacts.
Definitions and persistent harness state survive these commands. Review the
[cleanup and purge rules](RUST-APPLICATION.md#cleanup-and-purge) before use.

```text
smolsbx clean --workspace /path/to/project
smolsbx clean
smolsbx purge
smolsbx purge --force
smolsbx purge --force-all
```

## Operating reference and project status

[Operation and validation](RUST-APPLICATION.md) is the authoritative reference
for profiles, environment overrides, application state, lifecycle safeguards,
and contributor checks. Repository maintenance instructions are in
[AGENTS.md](AGENTS.md).

The former shell/Python sources and fixtures were removed in commit
`28087885c2b84077c62ee92c5560a2cab1b68997`; they remain available in Git history.
Rust uses `~/.smolsandbox/` and neither imports nor removes the legacy
repository-local `artifacts/` or `.smolvm/` state.

Features 0004 and 0009 retain their recorded `Implemented` status following
the user's CI confirmation. Feature 0010 is marked `Implemented` at the user's
request on 2026-11-13. Its
[completion decision](FEATURES/feature-0010-validation.md#completion-decision-2026-11-13)
preserves previous validation results and pending native and real guest checks.
