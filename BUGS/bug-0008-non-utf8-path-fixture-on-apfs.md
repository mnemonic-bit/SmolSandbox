# Bug 0008: Non-UTF-8 definition fixture fails before lifecycle validation on APFS

## Status and summary

Correction implemented in the working tree for `smolsandbox 0.14.1` on
2026-11-22. Native macOS/APFS verification remains `not_run`. See the
implementation record below; the original investigation follows.

Report prepared on 2026-11-22 against `smolsandbox 0.14.0` at
`dcc72bec291736a225633e3afe5694bb793ee92f`.

The reported integration-test panic matches an unconditional `fs::copy`
used to create a definition with an invalid UTF-8 filename. The test enables
this fixture on every Unix host. APFS rejects such filenames, so fixture
creation can fail before the test invokes the lifecycle command under test.
The source confirms the fixture assumption; macOS/APFS is the likely failing
environment, pending the original job details or native reproduction.

The requested outcome is a portable regression that retains rejection and
no-mutation coverage without requiring APFS to create an unsupported name.
This report applies no software fix. Both Cargo versions remain `0.14.0`
under [AGENTS.md](../AGENTS.md).

## Reported behavior and impact

The user supplied this failure while integrating changes into `main`:

```text
---- lifecycle::metadata_paths_reject_non_utf8_before_lifecycle_writes stdout ----

thread 'lifecycle::metadata_paths_reject_non_utf8_before_lifecycle_writes' (24338) panicked at tests/application.rs:1564:52:
called `Result::unwrap()` on an `Err` value: Os { code: 92, kind: Uncategorized, message: "Illegal byte sequence" }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    lifecycle::metadata_paths_reject_non_utf8_before_lifecycle_writes

test result: FAILED. 69 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 141.05s

error: test failed, to rerun pass `--test application`
Error: Process completed with exit code 101.
```

Expected: the regression checks that unsupported metadata paths produce an
application error before lifecycle mutation. Actual: fixture setup panics
while copying a definition to an invalid filename. The definition branch
never reaches its `build` invocation or its rejection and preservation
assertions.

The [native workflow](../.github/workflows/rust.yml#L564) runs adapter tests
before release compilation and packaging. A failure in that step prevents
the affected native job from producing its release ZIP. The release job
requires all native jobs to succeed, so one failing native job blocks
publication. The supplied excerpt establishes a test failure; release-mode
compilation and real SmolVM behavior were not exercised by this failure.

The user reported errors in three builds but supplied only this first
failure. This report does not attribute either remaining error to Bug 0008.
The failing run URL, job target, filesystem, and source SHA were not supplied
or inspected. The report baseline above identifies the inspected checkout,
not a verified source SHA for the reported execution.

## Source analysis and likely cause

The test is inside the `test-adapter` lifecycle module and has a separate
`#[cfg(unix)]` guard. In
[tests/application.rs](../tests/application.rs#L1551), the test loops over
`definition = false` and `definition = true`:

1. Both branches construct a native path using
   `OsString::from_vec(b"path-\xff".to_vec())`.
2. The first branch passes the nonexistent path through `ARTIFACTS_DIR`.
   No invalid filename needs to be created for that branch.
3. The second branch calls `fs::copy(f.definition(), &invalid).unwrap()` at
   line 1564, then selects the copied definition through `BASE_SMOLFILE`.
4. Only after fixture preparation does the branch execute `build` and assert
   that stderr contains `path is not UTF-8`.

The source line and column match the reported panic. The test incorrectly
equates Unix native-path byte support with filesystem support for creating
every non-UTF-8 filename.

Apple's archived [APFS filename reference](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/FAQ/FAQ.html)
states that APFS requires valid UTF-8 filenames for creation. Apple's
[Darwin error definitions](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/sys/errno.h)
assign `EILSEQ` the value `92` and description `Illegal byte sequence`.
Inference: the macOS/APFS fixture rejection explains the exact supplied
error. The workflow's macOS entry uses `macos-15` with
`aarch64-apple-darwin`; the actual failing job remains unverified.

The application already checks metadata path encoding in
[`Lifecycle::new`](../src/lifecycle.rs#L137), including artifact and
definition paths. Configuration resolution first inspects the selected
definition with `fs::symlink_metadata` in
[`find_definition`](../src/config.rs#L349), then reads the definition in
[`Config::resolve_inner`](../src/config.rs#L264). Consequently, removing
the fixture copy alone cannot make the definition branch reach
`Lifecycle::new`: a nonexistent or filesystem-rejected definition fails
earlier during selection or reading.

The alternatives considered were a copy-specific macOS restriction and a
missing or unreadable source definition. Native copy-versus-create probes
remain pending. Locally, a valid destination, the invalid destination, and
direct creation all succeed. APFS's documented creation restriction
supports the filesystem hypothesis over a copy-only failure; native
confirmation is still required. No production UTF-8 validation defect is
demonstrated by the supplied panic.

## Reproduction and diagnostic results

Run this exact test from the repository root on the affected native host:

```sh
cargo test --locked --features test-adapter --test application lifecycle::metadata_paths_reject_non_utf8_before_lifecycle_writes -- --exact --nocapture
```

The command selects one test with both branches. On a filesystem that
rejects the invalid fixture name, the expected reproduction is a panic at
the definition-copy line. The supplied CI output is the failure evidence;
no macOS execution was available during report preparation.

The exact command above was run on native `x86_64-unknown-linux-gnu` with
Rust and Cargo `1.100.0` at the report baseline. Result: `passed`, exit `0`:

```text
running 1 test
test lifecycle::metadata_paths_reject_non_utf8_before_lifecycle_writes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 69 filtered out; finished in 0.13s
```

A disposable Rust probe used the same `OsString` bytes and `fs::copy` call
with a six-byte source file. A valid-destination copy supplied the control;
`fs::write` to the invalid destination tested direct creation separately.
The local output was:

```text
valid_copy=Ok(6)
invalid_copy=Ok(6)
invalid_write=Ok(())
```

Probe execution: `passed`; reproduction of the reported OS error locally:
`inconclusive`. The probe and fixtures were removed. Neither Linux result
establishes macOS success. A native run should repeat the copy and direct
creation checks in the same temporary-file location used by the test.

Structural exploration used the required `codebase-memory` skill. The graph
returned the test and `Lifecycle::new` source, but its test call trace
returned no callees. Direct source inspection established the branch order
and configuration filesystem operations. Coverage reported no recorded issue
for the cited test, lifecycle, configuration, workflow, and manifest paths;
that result does not establish complete graph coverage.

The full Rust checks and real SmolVM checks were `not_run`: the requested
change records a bug report and local diagnostic evidence only. Native
reproduction, implementation, and verification of a correction remain open.

## Proposed correction

The implementer should separate the two regression cases so an unsupported
definition fixture cannot prevent independent artifact-path coverage.
Keep the `ARTIFACTS_DIR` rejection case on Unix because that case does not
require creating an invalid filename.

Retain the end-to-end invalid-definition test where the temporary filesystem
supports the fixture. Use an explicit platform restriction or a narrowly
checked filesystem capability probe for that fixture-dependent case. A
capability probe must not silently accept unrelated copy or setup errors.
Skipping the entire existing test would also lose usable artifact-path
coverage on macOS.

Cover definition-path encoding rejection separately at the `Lifecycle::new`
boundary using a constructed configuration and definition bytes. That case
can carry a non-UTF-8 `PathBuf` without creating a file under that name.
Preserve application rejection and no-mutation checks; replacing invalid
bytes with lossy UTF-8 would remove the tested condition.

## Acceptance for the eventual fix

The implementer owns the test correction and validation. All criteria below
are `not_run` for a fix at report preparation.

- AC-01: On native macOS ARM64 with an APFS temporary directory, the focused
  regression completes without a fixture panic. Record the source SHA,
  filesystem, exact command, selected tests, and results.
- AC-02: The artifact-path integration case executes on macOS and Linux,
  rejects the invalid path, and preserves the current machine-inventory,
  mutation-log, artifacts, locks, and harness-home assertions.
- AC-03: On a Linux temporary filesystem accepting invalid UTF-8 filenames,
  the definition-path integration case still reaches the application and
  verifies the encoding diagnostic and existing no-mutation assertions.
  An unsupported-fixture branch does not suppress unrelated setup errors.
- AC-04: A filesystem-independent regression exercises the actual
  `Lifecycle::new` definition-path guard with invalid native path bytes.
  Verify that bypassing that guard makes the regression fail.
- AC-05: The correction includes a patch bump in `Cargo.toml` and the
  matching `smolsandbox` entry in `Cargo.lock`, including for a test-only
  change. If applied directly to this baseline, use `0.14.1`. A test-only
  correction does not change machine-creation policy or require a
  lifecycle `REVISION` increment.
- AC-06: Run all seven Rust checks in [AGENTS.md](../AGENTS.md). Verify the
  adapter suite on native macOS and both Linux release architectures.
  Keep the Windows suite passing with Unix-only cases excluded. Record
  hosted results and any remaining checks in
  [Feature 0010 validation](../FEATURES/feature-0010-validation.md), retaining
  prior observations and source revisions.

Next evidence to collect: the original CI run URL and failing target, then
a native macOS/APFS execution of the focused command. Those details can
confirm the platform diagnosis without changing the scope of the other two
reported build errors.

## Implementation: 2026-11-22

The artifact-path integration regression now runs independently on Unix.
The definition-path integration regression runs on Linux, retaining the
invalid-filename fixture and all rejection and no-mutation assertions.
macOS does not create that unsupported fixture. A separate Unix unit
regression passes invalid definition-path bytes directly to `Lifecycle::new`
without creating the definition file and asserts that the temporary root
remains empty. Its runtime constructor exists only in Unix test builds.

Both integration cases and the constructor regression were `passed` on
native Linux x86-64. Temporarily bypassing only the definition-path guard
made the constructor regression `failed`, exit `101`; restoring the guard
returned the regression to `passed`, exit `0`. No production machine policy,
template, or lifecycle revision changed. Both Cargo versions are `0.14.1`.

Commands, full-check results, and pending native verification are recorded
in [Feature 0010 validation](../FEATURES/feature-0010-validation.md#bugs-0008-and-0009-corrections-2026-11-22).
