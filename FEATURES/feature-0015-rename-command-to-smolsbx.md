# Feature 0015: Rename the command to smolsbx

## Status

In progress

The command rename is implemented locally. Local checks and the pending
native CI artifact verification are recorded in
[Feature 0015 validation](feature-0015-validation.md). AC-07 remains
`not_run`; the feature is not marked `Implemented`.

## Summary

Rename the user-facing command from `smolsandbox` to `smolsbx`. Deliver
`smolsbx` on Unix and `smolsbx.exe` on Windows. Use `smolsbx` in CLI identity,
command recommendations, build instructions, and CI artifact names.

Preserve existing configuration, authentication, machine identity, and
lifecycle behavior. A user with an existing sandbox can switch to the new
command without regenerating a definition, authenticating again, or
recreating a machine solely because of the rename.

## Problem and desired outcome

The requested shorter command reduces typing for routine operations. The
first-use sequence becomes:

```text
smolsbx create-config --harness codex
smolsbx build /path/to/project
smolsbx login
smolsbx start /path/to/project
```

All existing subcommands and options remain available with the same syntax
after the executable name. For example:

```text
smolsbx start --port 3000 /path/to/project -- --help
smolsbx --verbose list --all --json
smolsbx remove --dry-run /path/to/project
```

## Baseline and ownership

The implementer owns the rename, regression checks, documentation updates,
and implementation evidence. Specification preparation observed commit
`583d5db794c77f91f5187495a3f34b4df525e3e9`, package version `0.8.1`, and a
clean working tree before this document was added.

At that baseline, `Cargo.toml` names the package and production binary
`smolsandbox`. `src/cli.rs` sets `bin_name = "smolsandbox"`. Integration tests
locate the executable through `CARGO_BIN_EXE_smolsandbox`. The workflow in
`.github/workflows/rust.yml` and the baseline-independence example build the
`smolsandbox` target. `README.md` and `RUST-APPLICATION.md` document that name.

Read the implementation baseline before editing. Record differences that
change this scope in the implementation validation record.

## Scope decisions

The following choices interpret the request as a command rename. These are
proposed feature decisions, not observations of an implemented interface.

| Surface | Required result |
| --- | --- |
| Production Cargo binary target | `smolsbx` |
| Delivered executable | `smolsbx` or `smolsbx.exe` |
| Help, usage, and version identity | `smolsbx` |
| Active application command recommendations | Invoke `smolsbx` |
| CI artifact name | `smolsbx-${{ matrix.target }}` |
| Cargo package and library crate | Retain `smolsandbox` |
| Integration-test executable variable | `CARGO_BIN_EXE_smolsbx` |
| Configuration and persistent state directories | Retain `.smolsandbox` |
| SmolVM runtime and guest harness commands | Retain their existing names |

Do not provide a compatibility binary, wrapper, symlink, or second Cargo
target named `smolsandbox`. Users must update scripts and shell aliases that
invoke the old command. Previously installed executables remain the user's
responsibility; the renamed application must not find or remove those files.

The product may continue to be called smolsandbox in explanatory prose.
Preserve Rust imports such as `use smolsandbox::cli::Cli`; those imports name
the library crate rather than the command.

## Requirements

### Build, installation, and distribution

Change the production binary target in `Cargo.toml` to `smolsbx`. Preserve the
`smolvm-test-double` target and dependency resolutions. Update executable
references in tests and `examples/check_baseline_independence.rs`.

Both commands below must select the production application:

```text
cargo build --locked --release --bin smolsbx
cargo install --locked --path . --bin smolsbx
```

Update `.github/workflows/rust.yml` to build the renamed target and upload
`smolsbx-${{ matrix.target }}` with the matching executable path. Preserve all
four native targets, both test configurations, and `if-no-files-found: error`.
Distributed builds must continue to exclude `test-adapter`.

Removing the old command is a breaking interface change. On 2026-11-14, the
user requested a feature version bump because the repository has no other
users. This decision supersedes the original major-version requirement and
the general breaking-change versioning rule for Feature 0015 only. Increment
the minor version in `Cargo.toml` and the matching `smolsandbox` package entry
in `Cargo.lock`, resetting the patch component to zero. From the implementation
baseline `0.9.1`, the resulting version is `0.10.0`. Preserve the earlier
`1.0.0` validation observations as historical evidence.

### CLI identity and command behavior

Top-level help, every subcommand's help, usage errors, and `--version` must
identify the command as `smolsbx`. Version output must be
`smolsbx <manifest-version>` followed by a newline. Identity must be consistent
for the Unix and Windows executable basenames even though the Cargo package
remains `smolsandbox`.

Update command recommendations in `src/config.rs`, `src/lifecycle.rs`, and
`src/ports.rs`, including configuration generation, authentication, build,
repair, and upgrade guidance. A diagnostic may still quote an actual path,
metadata identifier, or resource name containing an older name.

Preserve parsing, defaults, output channels, exit statuses, verbose output
policy, and exact argument forwarding after `--`. Help and version must
remain available without SmolVM, configuration, authentication, network
access, or application-state mutation. A copied executable must retain the
existing ability to run outside the checkout without adjacent templates or
host Bash/Python.

### Configuration and existing resources

Preserve the following compatibility surfaces:

- User and repository `.smolsandbox` discovery, definitions, credentials,
  artifacts, caches, lock paths, and persistent harness homes.
- Environment variable names, configuration fields, guest mounts, and harness
  executable names.
- The `# codex-sandbox-harness:` marker and `codex-sandbox.` managed labels.
- Managed machine and artifact names, identity hashes, state formats, and port
  manifests and journals.
- The machine policy `REVISION` in `src/lifecycle.rs`.
- Every byte of the four embedded files in `templates/*.smlbx`.

For identical definition bytes, SmolVM version, workspace, and effective
creation options, the renamed executable must derive the same identities and
reuse the same verified managed machines. The application version bump must
not become an additional identity input. Preserve ownership checks and the
existing refusal of contradictory or ambiguous metadata.

Do not migrate user files to `.smolsbx`, rewrite definitions, change network
policy, or introduce new machine metadata as part of this rename.

### Documentation and prior specifications

Update active command examples, executable paths, and build instructions in
`README.md`, `RUST-APPLICATION.md`, `CONTEXT.md`, and `AGENTS.md`. Explain the
command migration and retained `.smolsandbox` paths in the operating reference.
Apply `AGENT-LANG.md` and the `writing-for-agents` skill when editing agent
instructions during implementation.

Review remaining `smolsandbox` occurrences by meaning. Package names, crate
imports, state paths, product prose, and historical evidence are permitted.
An active recommendation to run the old command is not permitted. A blanket
replacement or zero-match search is not an acceptance rule.

Preserve prior feature specifications and validation transcripts as records
of their original scope and observations. This feature supersedes
[Feature 0011](feature-0011-rename-executable-to-smolsandbox.md) only for the
production command, CLI identity, and distributed artifact names. Package,
crate, and persisted identity choices remain as specified above. Preserve
existing native and real guest validation status; the rename does not close
unrelated cutover gates.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | Cargo metadata exposes production target `smolsbx`, package and library `smolsandbox`, and no binary target `smolsandbox`. The manifest and lockfile both select the user-requested feature version `0.10.0`. | Inspect `cargo metadata --locked --no-deps --format-version 1`, `Cargo.toml`, and `Cargo.lock`. |
| AC-02 | A fresh release build and temporary local installation produce the platform's `smolsbx` executable and no old-name production executable. | Build with a fresh `--target-dir`; install with a temporary `--root`; execute each resulting binary directly with `--version`. |
| AC-03 | Top-level and all subcommand help, version output, and representative usage errors identify `smolsbx`; their offline, exit-status, and state-preservation contracts hold. | Update and run the CLI contract checks in `tests/cli_contract.rs`, including executable-suffix and exact version assertions. |
| AC-04 | Application command recommendations invoke `smolsbx`. Existing parsing, verbose output, guest forwarding, and lifecycle behavior remain unchanged. | Update relevant output assertions; run the existing CLI and application regressions in both feature configurations. |
| AC-05 | Equivalent pre-rename and post-rename inputs produce equal identities and reuse an existing managed workspace without rename-induced creation or deletion. Existing state and credentials remain available. | Compare the baseline and renamed binaries using isolated copies of the same state fixture, definition, workspace path, and fixed SmolVM test-double version; inspect identities, state snapshots, and runtime calls. |
| AC-06 | Template bytes, persisted paths and formats, metadata identifiers, and machine policy revision remain unchanged. | Compare the implementation diff, template hashes, and existing state and identity assertions against the baseline. |
| AC-07 | All four existing native CI targets build and upload the renamed executable under the new artifact name. | Record native CI run links and inspect the uploaded artifact filenames. |
| AC-08 | The copied `smolsbx` executable retains relocation behavior, and the baseline-independence example builds the new target. | Run the existing relocation regression and `cargo run --locked --example check_baseline_independence`. |
| AC-09 | Active instructions use the renamed command and explain migration; remaining old-name references have an allowed meaning. | Review the documentation diff and occurrence audit, preserving historical evidence and existing cutover status. |

## Verification and evidence

Run the repository checks after implementation, using the renamed build
target:

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --features test-adapter --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --features test-adapter --all-targets
cargo build --locked --release --bin smolsbx
cargo run --locked --example check_baseline_independence
```

Use fresh output directories for AC-02. Existing target directories may
contain stale `smolsandbox` binaries; do not delete unrelated outputs or
installed executables to establish artifact absence.

Audit tracked occurrences with `git grep -n -I smolsandbox`. Review new files
separately. Classify retained occurrences using the scope decisions above.

During implementation, create `FEATURES/feature-0015-validation.md`. Record the
source revision, commands, exit codes, native targets, artifact names,
compatibility comparison inputs, and outcomes for AC-01 through AC-09. Use
`passed`, `failed`, `not_run`, or `inconclusive`. Append relevant validation
results and pending native or real guest checks to
[Feature 0010 validation](feature-0010-validation.md), preserving prior
observations and their source revisions. Mark this feature `Implemented`
only after every mandatory acceptance criterion has passed.
