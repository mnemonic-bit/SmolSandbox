# Bug 0009: Identity metadata fixtures use inconsistent Windows path separators

## Status and summary

Correction implemented in the working tree for `smolsandbox 0.14.1` on
2026-11-22. Native Windows verification remains `not_run`. See the
implementation record below; the original investigation follows.

Report prepared on 2026-11-22 against `smolsandbox 0.14.0` at
`dcc72bec291736a225633e3afe5694bb793ee92f`.

Two adapter integration tests construct default paths with embedded `/`
characters while production constructs the same directories with separate
native path components. On Windows, those constructions produce different
strings. The supplied CI assertions compare exact creation-input strings,
so the native Windows test step fails.

The additional log identified by the user as Windows ARM shows the same
failure pattern. Both supplied executions belong to Bug 0009; no separate
bug report is required.

The requested outcome is portable test fixtures that preserve independent,
exact metadata, argument, and identity-hash checks. This report applies no
software fix. Both Cargo versions remain `0.14.0` under
[AGENTS.md](../AGENTS.md).

## Reported behavior and impact

The supplied Windows output reports one passing test and these two failures:

```text
every_profile_records_exact_inputs_for_all_machine_kinds
tests\identity_metadata.rs:310:21
assertion `left == right` failed: codex/workspace: SmolSandbox.machine-inputs

resolved_input_variants_record_exact_identity_changes_and_exclude_mutable_settings
tests\identity_metadata.rs:567:13
assertion `left == right` failed

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.21s
error: test failed, to rerun pass `--test identity_metadata`
Error: Process completed with exit code 1.
```

The differing suffixes are shown below; the temporary-directory prefix is
omitted. The guest mount targets retain `/` in both forms.

| Assertion | Left | Right |
| --- | --- | --- |
| First test, artifact | `.smolsandbox\artifacts\codex-devtools-df0f255b951d.smolmachine` | `.smolsandbox/artifacts\codex-devtools-df0f255b951d.smolmachine` |
| First test, harness home | `.smolsandbox\codex-home:/root/.codex` | `.smolsandbox/codex-home:/root/.codex` |
| Second test, artifact | `.smolsandbox/artifacts\codex-devtools-df0f255b951d.smolmachine` | `.smolsandbox\artifacts\codex-devtools-df0f255b951d.smolmachine` |
| Second test, harness home | `.smolsandbox/codex-home:/root/.codex` | `.smolsandbox\codex-home:/root/.codex` |

The workspace path, workspace mount, resources, hosts, and metadata version
match in the supplied assertions. The reversed separator direction in the
second failure follows from comparing a later invocation with the original
inventory, rather than comparing production with a constructed expectation.

### Additional Windows ARM report: 2026-11-22

The user subsequently supplied a second log identified as the Windows ARM
pipeline. Both test names, assertion locations (`310:21` and `567:13`),
metadata fields, and left/right separator differences match the first log.
The artifact basename remains
`codex-devtools-df0f255b951d.smolmachine`. Temporary-directory names, thread
IDs, and elapsed time differ; none changes the asserted path mismatch.

| Supplied execution | Result | Test summary |
| --- | --- | --- |
| Original Windows report | failed | 1 passed, 2 failed; 8.21 seconds; process exit 1 |
| User-reported Windows ARM pipeline | failed | 1 passed, 2 failed; 8.72 seconds; process exit 1 |

The second log therefore supplies additional evidence for the same fixture
defect. The path construction and exact-string comparisons described below
do not depend on CPU architecture. No distinct ARM failure appears in the
supplied excerpt.

The second excerpt does not identify the runner architecture, Rust target,
source SHA, or run URL. The inspected workflow matrix configures only the
Windows x86-64 target, so Windows ARM is retained as the user's description
of the second execution, not independently verified architecture evidence.
The maintainer should record the second job's runner architecture and Rust
target when collecting native verification.

The [native workflow](../.github/workflows/rust.yml#L528) configures
`windows-2025` with target `x86_64-pc-windows-msvc`. Its adapter test step
precedes release compilation, packaging, and artifact upload. A failure
there prevents that job from producing its release ZIP. On `main`, release
publication requires every native job to succeed.

The excerpt establishes test assertion failures after successful compilation
of the test executable. It does not demonstrate a Rust compilation error or
a real SmolVM failure. The original run URL and tested SHA were not supplied;
the report baseline identifies the inspected checkout, not a verified SHA
for the failing run.

## Source analysis

In [Config::resolve_inner](../src/config.rs#L264), application home is already
`<native user home>\.smolsandbox`. Defaults use `home.join("artifacts")`,
`home.join("codex-home")`, or `home.join("harness-home").join(profile)`.
[application_home](../src/config.rs#L312) supplies the application-home
component separately. Windows therefore inserts native separators at every
directory boundary.

In the [first test](../tests/identity_metadata.rs#L217), expectations instead
use `f.home.join(".smolsandbox/artifacts")`,
`f.home.join(".smolsandbox/codex-home")`, and
`f.home.join(".smolsandbox/harness-home").join(profile)`.
Joining those strings does not rewrite their embedded `/` characters to
`\`. Formatting the paths preserves the mixed spelling, exactly as shown
on the right of the first supplied assertion. Codex is the first profile;
its panic prevents the loop from reaching the other profiles.

In the [second test](../tests/identity_metadata.rs#L447), the initial `build`
uses application defaults and records native separators. Later invocations
explicitly set `ARTIFACTS_DIR` and `CODEX_HOME_DIR`, even for variants that
intend to retain the defaults. The fallback paths at lines 483 and 488 use
the same embedded-slash construction as the first test.
[component_path and absolute](../src/config.rs#L306) preserve the spelling
of absolute overrides. The later invocation therefore records mixed
separators in both its arguments and metadata.

The earlier exact expectations in the second test use those same overrides
and agree with the later invocation. Failure occurs in the `mutable` branch
at [line 567](../tests/identity_metadata.rs#L566), where
`assert_eq!(inputs, original_inputs)` compares mixed override paths with
native defaults. The test has inadvertently changed immutable creation
inputs while attempting to vary only mutable settings.

[CreationInputs](../src/lifecycle.rs#L29) supplies option values to command
arguments and metadata. Metadata serialization sorts and deduplicates exact
strings; it does not rewrite path separators.
[domain::hash_set](../src/domain.rs#L34) hashes the serialized canonical
option array. Consequently, these separator changes also change the
workspace identity hash. Filesystem equivalence does not imply equality
under the specified exact-string identity contract in
[Feature 0019](../FEATURES/feature-0019-readable-machine-identity-metadata.md).

Three explanations were considered: inconsistent fixture construction,
metadata serialization rewriting separators, and runtime path rewriting.
The fixture constructions explain both failures and their reversed left/right
values. Source inspection shows that serialization preserves the strings,
and the first test reads recorded create arguments before its later listing
checks. No production metadata or real-runtime defect is established by
the supplied output.

## Reproduction and diagnostic results

Run the focused suite from the repository root on native Windows:

```text
cargo test --locked --features test-adapter --test identity_metadata -- --nocapture
```

The existing tests directly assert the reported symptom. The supplied CI
output is the Windows failure evidence. Native Windows reproduction was
`not_run`: the report environment provides only the Linux Rust target and
has no Windows runtime. Native reproduction and minimization remain pending;
the source analysis above supports a fixture correction without claiming
that a Windows fix has been verified.

Local execution used native `x86_64-unknown-linux-gnu`, Rust and Cargo
`1.100.0`, and the report baseline:

| Check | Result | Evidence |
| --- | --- | --- |
| Focused command above | passed | Exit 0; all three tests passed in 7.79 seconds |
| `cargo test --locked --test identity_metadata -- --nocapture` | passed | Exit 0; zero tests, because the file requires `test-adapter` |
| Disposable replay of the two option spellings | failed | Exact object comparison raised an assertion; both JSON round trips preserved their strings and the two serialized option arrays produced different SHA-256 values |
| Native Windows reproduction and eventual fix | not_run | Requires a Windows execution of the adapter suite |
| Seven full repository Rust checks and real SmolVM checks | not_run | Report-only change; no software correction was applied |

The replay used fixed strings from the observed separator pattern and Python
standard-library JSON and SHA-256 operations. It confirms the byte-comparison
and hash consequence; it does not execute Windows `PathBuf` or the application.
No replay file or fixture was retained.

Structural exploration used the required `codebase-memory` skill, exact
symbol snippets, and the configuration dependency trace. Coverage reported
no recorded issue for the cited Rust, workflow, specification, and manifest
paths; `Cargo.lock` was not tracked by the index and was read directly.
Coverage results do not establish graph completeness.

## Proposed correction and acceptance

The implementer should construct fixture defaults one directory component
at a time, starting with `f.home.join(".smolsandbox")`, then joining
`"artifacts"`, `"codex-home"`, or `"harness-home"` and the profile. Apply
the same construction to both tests so default-equivalent overrides retain
the original creation-input spelling. Keep expected values independent of
production helpers.

Preserve exact metadata, create-argument, and hash assertions. Replacing all
backslashes, comparing normalized paths, or changing production hashing
would conceal the fixture error or change the identity contract. Guest paths
such as `/root/.codex` and `/workspace/myrepo` retain their existing spelling.

The implementer owns the correction and validation. All eventual-fix criteria
below are `not_run` at report preparation.

- AC-01: On native Windows x86-64, the focused adapter command passes all
  three tests. The all-profile test reaches all four profiles and all three
  machine kinds while retaining exact metadata and argument comparisons.
  For the user-reported Windows ARM job, record the actual runner architecture
  and Rust target and verify that the same focused command passes there.
- AC-02: The variant test retains all five variants. Default-equivalent
  artifact and harness-home overrides produce the original exact options
  and hash in the `mutable` branch. Each immutable variant still changes
  identity, and mutable settings remain excluded from metadata.
- AC-03: The correction changes test construction only. Production path
  spelling, serialization, hashes, machine policy revision `7`, templates,
  user-owned definitions, and existing machines retain their behavior.
- AC-04: Include a matching PATCH bump in `Cargo.toml` and the `smolsandbox`
  entry in `Cargo.lock`, including for a test-only correction. If applied
  directly to this baseline, use `0.14.1`. A test-only correction requires
  no lifecycle `REVISION` increment.
- AC-05: Run the seven Rust checks in [AGENTS.md](../AGENTS.md). Verify the
  adapter suite on native Windows and retain passing Linux and macOS native
  results. Record source SHAs, commands, outcomes, CI run links, and pending
  native or real guest checks in
  [Feature 0010 validation](../FEATURES/feature-0010-validation.md), preserving
  previous observations.

Next evidence: both Windows job URLs and SHAs, including the second job's
runner architecture and Rust target, followed by adapter runs after the
fixture correction. Real SmolVM acceptance remains separate from mocked
test success.

## Implementation: 2026-11-22

Five host-path constructions in `tests/identity_metadata.rs` now join each
directory component separately. The expected paths remain independent of
production helpers. All four profiles, all three machine kinds, all five
input variants, and the exact metadata, argument, and hash assertions remain
covered. Guest path strings and production path and identity behavior did
not change. Both Cargo versions are `0.14.1`.

The focused adapter suite was `passed` on native Linux x86-64 with all three
tests. Native Windows x86-64 verification and verification of the
user-reported Windows ARM execution remain `not_run`; the latter job's
actual runner architecture and Rust target remain unverified. Commands,
full-check results, and pending native verification are recorded in
[Feature 0010 validation](../FEATURES/feature-0010-validation.md#bugs-0008-and-0009-corrections-2026-11-22).
