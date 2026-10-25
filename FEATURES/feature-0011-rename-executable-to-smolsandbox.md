# Feature 0011: Rename the executable to smolsandbox

## Status

Proposed

## Summary

Rename the Rust application executable from `codex-sandbox` to `smolsandbox`.
Use `smolsandbox` in the Cargo package, Rust crate references, application help,
messages, build instructions, CI artifacts, and active documentation. Preserve
the command syntax and lifecycle behavior.

Treat persisted metadata, retained shell interfaces, and historical evidence
as explicit compatibility exceptions. The executable rename must not require
users to regenerate definitions, authenticate again, or recreate machines.

This specification defines implementation work. Writing this specification
does not rename the application or establish that implementation checks passed.

## Problem and desired outcome

The application supports Claude, Codex, OpenCode, and Hermes Agent, but its
executable name identifies only Codex. The application already stores native
configuration and state under `~/.smolsandbox`. The executable name should
match that application identity.

A user building or copying the application should receive `smolsandbox`
(`smolsandbox.exe` on Windows). Every active instruction for invoking the Rust
application should use that name. For example:

```text
cargo build --locked --release --bin smolsandbox
smolsandbox create-config --harness codex
smolsandbox build /path/to/project
smolsandbox login
smolsandbox start /path/to/project -- --help
```

The `codex` argument still selects the Codex harness. Renaming the application
does not rename any harness or its guest command.

## Baseline and ownership

The implementer owns the rename, documentation changes, and verification
evidence described below. The observed baseline is repository commit
`4624e7a8088c05f26e8137365a836fb009a276cc`, with no tracked working-tree changes
at specification preparation time.

At that baseline:

- `Cargo.toml` names both the package and production binary `codex-sandbox`.
- `src/main.rs` and Rust integration tests import `codex_sandbox`.
- Integration tests locate the binary through
  `CARGO_BIN_EXE_codex-sandbox`.
- `.github/workflows/rust.yml` builds and uploads the old executable name.
- `examples/check_baseline_independence.rs` builds the old binary target.
- `src/config.rs`, `src/lifecycle.rs`, and `src/ports.rs` contain instructions
  that tell users to invoke the old executable.
- `RUST-APPLICATION.md` documents the old Rust command. `README.md` still
  describes the retained shell workflow.
- Templates use `# codex-sandbox-harness:`. Managed machine labels use the
  `codex-sandbox.` namespace. These strings are consumed as metadata.

Read the implementation revision before editing. Record any differences that
change this scope in the implementation validation record.

## Scope decisions

The following decisions make the requested executable rename concrete. They
are proposed product decisions, not claims that an existing release policy
requires these choices.

| Surface | Required result |
| --- | --- |
| Cargo package | `smolsandbox` |
| Production binary target | `smolsandbox` |
| Unix executable basename | `smolsandbox` |
| Windows executable basename | `smolsandbox.exe` |
| Rust library crate and imports | `smolsandbox` |
| Cargo integration-test binary variable | `CARGO_BIN_EXE_smolsandbox` |
| CI artifact name | `smolsandbox-${{ matrix.target }}` |
| Active Rust command examples and instructions | Invoke `smolsandbox` |
| Existing configuration, machine metadata, and state identities | Preserve their current format |
| Retained shell baseline | Preserve its existing filenames and interfaces |
| Historical validation evidence | Preserve recorded commands and results |

No compatibility executable, symlink, wrapper, or second Cargo binary target
named `codex-sandbox` is included. Existing automation that invokes the Rust
executable must change its command to `smolsandbox`. Existing manually
installed copies of the old executable remain the user's responsibility;
the application does not discover, overwrite, or remove those copies.

Renaming the Cargo package also changes the Rust import path. No library crate
alias named `codex_sandbox` is included. This scope assumes that the repository
does not promise a separately stable public Rust library API.

## Functional requirements

### Build and source references

Change the package and production binary names in `Cargo.toml`. Update the
root package entry in `Cargo.lock` to match the manifest. Preserve the package
version and dependency resolutions; the rename does not require dependency
upgrades or a version bump.

Update production code, tests, and examples that refer to the Rust crate,
binary target, binary filename, executable path, or Cargo binary variable.
The `smolvm-test-double` target keeps its current name.

The commands `cargo build --locked --release --bin smolsandbox` and
`cargo install --locked --path . --bin smolsandbox` must select the production
application. Cargo metadata must expose `smolsandbox` as the production target
and library crate, with no `codex-sandbox` binary target.

Verify artifact absence in a fresh target or installation directory. A
previous build can leave `target/release/codex-sandbox` behind; that stale file
does not establish that the renamed manifest still produces the old target.
Do not delete unrelated build outputs or installed executables to make the
verification pass.

### Command-line identity and messages

With the delivered basename, top-level help, every subcommand's help, usage
errors, and `--version` must identify the application as `smolsandbox`.
`--version` must report the manifest version and exit successfully.

Replace references to the old executable in success messages, authentication
guidance, recovery instructions, upgrade messages, code comments describing
the current application, and test expectations. For example, configuration
creation should recommend `smolsandbox build` and `smolsandbox start`.

Preserve subcommands, flags, defaults, argument parsing, output streams, exit
codes, and guest argument forwarding. Help and version requests must remain
independent of SmolVM, configuration files, authentication, and network access.
Those requests must not create or change application state.

A message may quote an unchanged metadata key, legacy machine name, or actual
user path containing the old name. Any command recommendation in the same
message must use `smolsandbox`. A raw substring ban on all application output
would reject valid compatibility diagnostics and is not the acceptance rule.

### CI and executable relocation

Update `.github/workflows/rust.yml` to build `--bin smolsandbox` for each
existing target. Upload `smolsandbox-${{ matrix.target }}` with the matching
`smolsandbox` or `smolsandbox.exe` output. Preserve `if-no-files-found: error`
and the existing native validation matrix.

Update the binary build in `examples/check_baseline_independence.rs`. Preserve
the example's exclusion of the actual retained file `codex-sandbox.sh`.

Run executable relocation checks using the renamed binary. A copied executable
must retain the existing independence from the checkout, adjacent templates,
host Bash/Python, and the Rust toolchain. Lifecycle execution still uses the
separately selected SmolVM executable.

### Documentation and specifications

Review every tracked occurrence of `codex-sandbox`, `codex_sandbox`, and
`Codex Sandbox`. Classify each occurrence using the compatibility rules below.
Replace every occurrence that names the current Rust application, its build
target, its Rust crate, or its distributed artifact.

Update `README.md`, `RUST-APPLICATION.md`, and `CONTEXT.md` where their product
names, command references, or build paths describe the Rust application.
Include a concise migration note: the Rust executable was renamed, callers
must update their command, and existing native application state is retained.
Explain that Codex remains one of the supported harnesses.

Update executable and crate references in the current Rust work described by
Features 0004, 0009, and 0010. Preserve historical shell commands and quoted
validation results in those documents. This feature takes precedence over
older specifications only for application naming.

Preserve the documented cutover status. This rename does not complete Feature
0004, remove the baseline, or satisfy Feature 0010's documentation cutover
prerequisites. If the baseline remains, shell operating instructions must
continue to identify the shell commands accurately.

If implementation changes `AGENTS.md` or other agent instructions, apply
`AGENT-LANG.md` and the installed `writing-for-agents` skill. No unrelated
instruction rewrite is required by this feature.

## Compatibility boundaries and allowed old names

### Persisted metadata and identity

Preserve these identifiers and their readers and writers:

- The `codex-sandbox.` managed-label namespace, including all six managed
  keys, unknown-key validation, and ownership checks.
- The `# codex-sandbox-harness:` metadata marker, including its existing
  exactly-one-marker rule and supported profile identifiers.
- Existing machine names, artifact names, generation hashes, workspace hashes,
  machine revisions, state formats, and port journals.
- Native configuration discovery under repository `.smolsandbox` and user
  `~/.smolsandbox`, including credential, artifact, state, and lock locations.
- Existing environment variables, configuration filenames, guest mounts,
  harness state paths, and harness executable names.

Keep all four embedded template files byte-for-byte unchanged for this rename.
The baseline derives base identity from definition bytes and the SmolVM
version. Rebranding a template comment alone could select a new generation.
Document the retained metadata marker as a compatibility identifier.

Do not add a second label namespace or metadata marker. Do not migrate or
rewrite user definitions. The same definition bytes, SmolVM version, workspace,
and effective creation options must produce the same identity before and
after the executable rename.

The renamed binary must recognize existing managed machines under the same
ownership and reconciliation rules. The renamed binary must preserve the
existing refusal of contradictory metadata and unrelated machines. Reuse does
not imply importing the shell baseline's repository-local state into the Rust
application.

### Retained baseline and historical records

Preserve `codex-sandbox.sh`, its callers, baseline exclusion lists, shell test
assertions about that file, and the shell/Python metadata protocol while those
files remain the comparison baseline. Do not introduce `smolsandbox.sh`.

Preserve historical command transcripts, research quotations, and validation
records, including `FEATURES/feature-0004-validation.md` and
`FEATURES/feature-0009-validation.md`. A command recorded as executed must not
be rewritten as if a different executable had produced the result. Label or
link historical material where an active document could otherwise present
the old invocation as current guidance.

Retained protocol strings in fixtures and source code are permitted only when
they implement or verify these compatibility boundaries. An old executable
reference in an active build command or user recommendation is not permitted.

## Non-goals and constraints

- No new subcommands, flags, harness profiles, or lifecycle policies.
- No changes to provisioning tools, dependency versions, network allowlists,
  published-port behavior, or machine revision values.
- No renaming of Codex itself, its authentication commands, or its state.
- No installers, registry publication, repository rename, or release automation
  beyond updating the existing CI artifact names and paths.
- No removal of retained scripts or waiver of native and real SmolVM gates.
- No replacement of historical evidence with newly named examples.

The repository's network-policy instructions remain binding. This feature
requires no allowlist change. If implementation introduces an allowlist change,
apply the synchronization, revision, assertion, and shell-test requirements in
`AGENTS.md`; record that additional scope explicitly.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | Cargo exposes the package, library crate, and production target as `smolsandbox`; no production target or crate alias uses the old name. | Inspect `cargo metadata --locked --no-deps --format-version 1` and the manifest. |
| AC-02 | A locked release build and local installation produce the platform's `smolsandbox` executable without producing an old-name executable in the fresh output directories. | Build and install into temporary directories; inspect outputs and run installed `--version`. |
| AC-03 | Help for the application and all seven subcommands, version output, and representative usage errors identify `smolsandbox`; their existing exit and mutation behavior is preserved. | Extend `tests/cli_contract.rs` with exact command identity assertions and offline state snapshots. |
| AC-04 | Active application command recommendations use `smolsandbox`, including configuration creation, authentication guidance, repair/build guidance, and unsupported port-state upgrade guidance. | Assert the relevant success and error output in existing tests; review remaining old-name message literals. |
| AC-05 | Rust production code, tests, and examples compile with the renamed crate and Cargo binary variable in both existing feature modes. | Run formatting, Clippy, tests, and the baseline-independence example listed below. |
| AC-06 | All four existing native CI targets build and upload the renamed executable under the new artifact name. | Inspect workflow paths and record native CI results for each target. |
| AC-07 | Existing definitions and managed machine inventories remain usable with unchanged identities and lifecycle decisions. | Compare pre-rename and post-rename runs against the same fixed SmolVM test-double version, definition, workspace, and state fixtures; assert equal IDs and plans and no rename-induced create/delete operations during reuse. |
| AC-08 | Template bytes, persisted metadata keys, state locations, credentials, and machine revision values remain unchanged. | Review the diff; compare template hashes and fixture-backed state/identity assertions against the baseline. |
| AC-09 | The copied `smolsandbox` executable retains the application's existing relocation behavior. | Run the existing relocation test with the renamed binary outside the checkout. |
| AC-10 | Active documentation and forward-looking Rust specifications use the new name and explain the migration without claiming that unrelated cutover gates passed. | Review the documentation diff and follow local build/help examples using the new name. |
| AC-11 | Every remaining old-name occurrence is accounted for as compatibility metadata, the retained baseline, historical evidence, or an explicit rename/migration assertion. | Review the tracked-file audit below and record categories and locations in the validation record. |

AC-07 compares equivalent lifecycle inputs. Do not compare the shell baseline's
state directory with the native application home or treat an expected operation
on an intentionally stale fixture as a rename regression.

## Verification and evidence

Use a fresh target directory for artifact-name checks. Use a temporary
installation root for the local installation check. Execute the renamed
binary from each output directory; adding a build directory to an existing
`PATH` is insufficient evidence because another installed binary could run.

Run these repository checks after implementation:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --features test-adapter --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --features test-adapter --all-targets
cargo build --locked --release --bin smolsandbox
cargo run --locked --example check_baseline_independence
```

For the fresh-output checks, supply temporary directories to these commands:

```text
cargo build --locked --release --bin smolsandbox --target-dir <temporary-target-directory>
cargo install --locked --path . --bin smolsandbox --root <temporary-install-directory>
```

Audit tracked files with:

```text
git grep -n -I -E 'codex[-_]sandbox|Codex Sandbox'
```

Review untracked implementation files separately before committing. Do not
require the tracked-file search to return zero matches: the exceptions in
this specification are deliberate. Review product headings for equivalent
old-name spellings that this exact search does not match.

Run every retained `tests/*-test.sh` script on a compatible host if the
implementation changes a retained baseline file or its tests. The network
policy rule in `AGENTS.md` independently requires that full shell run when
either allowlist changes.

Create `FEATURES/feature-0011-validation.md` during implementation. Record the
tested revision, commands, exit codes, native target results, compatibility
comparison inputs, occurrence classifications, and an outcome for each
acceptance criterion. Use `passed`, `failed`, `not_run`, or `inconclusive`.
Link CI evidence for native targets that are unavailable locally. Do not
report this feature as completed until every mandatory criterion has passed.

The validation record may report completion of this rename independently of
Features 0004 and 0010. Those features retain their own cutover requirements.
