# Feature 0009: Transfer shell regressions to independent Rust tests

## Status

Proposed

## Summary

Complete the regression-test transfer required by
[Feature 0004](feature-0004-replace-shell-scripts-with-cross-platform-rust-application.md).
Preserve the behavioral checks in the shell suite and replace every Rust test
dependency on the retained shell/Python baseline with independent expectations.
The Rust suite must build and run after the baseline files are removed.

This feature can be implemented while the baseline remains in the repository.
Completing this feature does not authorize baseline deletion or establish that
Feature 0004's real SmolVM cutover gates have passed.

## Problem

The Rust suite covers the main lifecycle operations, but some checks still
depend on the implementation scheduled for retirement:

- `tests/differential.rs` invokes Python and loads `sandbox-inventory.py` and
  `sandbox-port-state.py` to calculate expected results at test time.
- `differential_start_matches_shell_creation_labels_order_and_guest_status`
  in `tests/application.rs` invokes Bash and `start-codex.sh`.
- `codex_template_preserves_baseline_provisioning_pins_and_verification` in
  `src/config.rs` includes `devtools-base.smolfile` at compile time and compares
  the Codex template against that file.

Deleting those files would break the Rust suite or discard its parity checks.
The shell suite also contains checks that need independent Rust application
coverage, including concurrent port update/start ordering and detailed
provisioning and Codex seed configuration assertions.

## Goals

- Account for every behavioral assertion in the eight shell test scripts.
- Transfer assertions that lack equivalent Rust coverage.
- Preserve the cases currently compared against the Python and shell baseline.
- Make the Rust suite independent of host Bash, host Python, and baseline files.
- Retain the existing lifecycle, ownership, ordering, and failure guarantees.

## Non-goals

- Deleting the production scripts, Python helpers, or shell fixtures before
  Feature 0004's cutover gates pass.
- Importing or deleting shell-era application state.
- Reproducing shell command names, compatibility aliases, or argument quirks
  that Feature 0004 explicitly replaces.
- Rewriting guest provisioning shell commands or removing guest Python.
- Adding download-integrity requirements from
  [Feature 0008](feature-0008-provisioning-download-integrity.md).
- Redesigning the lifecycle implementation or adding public commands.
- Completing real guest, authentication, or terminal validation through mocks.

## Regression inventory and traceability

The implementer must record the source revision and any relevant uncommitted
changes used as the comparison baseline. Create a durable coverage map under
`FEATURES/` that identifies each behavioral assertion or group of equivalent
assertions, its source location, the Rust test symbol, and the expected result.

Use these source files as the inventory boundary:

| Source | Behaviors to account for |
| --- | --- |
| `tests/allow-hosts-test.sh` | Host validation, repeatability, set normalization, immutable identity, and argument forwarding |
| `tests/clean-sandbox-test.sh` | Generation and workspace scope, ownership contradictions, unrelated state, deletion failures, and preserved state |
| `tests/devtools-base-config-test.sh` | Provisioning tools and pins, existing verification, Codex seed values, default hosts, and runtime policy revision |
| `tests/help-test.sh` | Help content, output streams, exit status, prerequisite independence, non-mutation, usage errors, and pass-through arguments |
| `tests/port-publishing-test.sh` | Exact sets, limits, capability refusal, metadata validation, recovery, concurrency, running machines, update/start failures, and state removal |
| `tests/purge-sandbox-test.sh` | Policy selection, refusal, running-generation protection, inventory changes, artifact selection, recovery, and partial failure |
| `tests/reconcile-workspace-test.sh` | Identity recovery, ownership, stale and ambiguous candidates, replacement readiness, failure ordering, locks, and cache publication |
| `tests/start-codex-test.sh` | Guest launch, seeding, secret references, argument boundaries, cleanup, and exit behavior |

Include the cases in `tests/differential.rs` and the shell-dependent application
test in the same map. Existing independent Rust tests may satisfy mapped cases;
duplicate tests are not required. Identify any shell-only assertion superseded
by an explicit Feature 0004 decision and cite that decision. A missing Rust
test is not a reason to declare an assertion superseded.

## Required Rust coverage

### Lifecycle ordering and concurrency

Use the native Rust SmolVM test double in `tests/support/smolvm.rs` to assert
observable operations, command arguments, inventory, and retained files.
Add missing application tests identified by the coverage map, including:

1. Start two invocations for the same workspace. Pause the first invocation
   during port reconciliation and verify that the second invocation cannot
   update or start the selected machine before the first finishes its update,
   verification, and start sequence.
2. Inject a port update failure and a post-update verification failure. Verify
   that neither failure reaches machine start or interactive harness execution
   and that recovery state remains available for the next invocation.
3. Inject a machine start failure. Verify that the harness is not executed,
   invocation-owned stop cleanup is attempted, and the command fails.
4. Transfer the remaining replacement, cleanup, purge, cache, and locking cases
   from the coverage map, including preservation after partial failures.

Use explicit readiness and release coordination for concurrency tests. Bound
every wait and clean up child processes on failure. A sleep followed only by
an ordering assumption does not establish serialization. Test the lifecycle
lock boundary through application processes; testing a lock primitive alone
does not cover the port update/start sequence.

### Provisioning and seed configuration

Replace comparison with `devtools-base.smolfile` with assertions against the
embedded Codex template in `templates/codex.smlbx`. Preserve the detailed
checks for installed tools, existing pins and verification commands, Docker
compatibility, codebase-memory installation, and parsed Codex seed settings.

Assert the expected network policy and runtime revision through the Rust
configuration and lifecycle surfaces. Verify that workspace creation uses the
selected definition's default hosts plus normalized user additions. Do not
retain a second production allowlist solely to reproduce the shell layout.

Cover guest seed command construction and warning-only seeding failures with
the Rust test double. Static template assertions do not establish successful
guest provisioning or prove that guest file-copy behavior ran; preserve the
corresponding real guest checks under Feature 0004.

### Independent expected results

Convert the Python differential cases into Rust table-driven tests or retained
data fixtures with explicit expected values and rejection outcomes. Preserve
inventory normalization, port metadata, ownership, replacement readiness,
purge plans, manifest/journal decisions, publication, and interrupted recovery.

Expected results may be captured from the baseline during implementation, then
reviewed against the documented contract. Record their source revision and
meaning in the coverage map. Runtime tests must not regenerate expectations
from the retired implementation or from the Rust function under test. Retained
fixtures must contain data, not an executable copy of the old implementation.

Replace the shell startup comparison with independent assertions for machine
names, labels, mutation order, guest argument boundaries, and exit status.
Deleting the comparison without retaining those checks does not satisfy this
feature.

## Baseline dependency removal

Remove all Rust test execution, import, inclusion, copying, and discovery of
the retired shell/Python baseline. This includes compile-time references such
as `include_str!("../devtools-base.smolfile")`, runtime script paths, embedded
Python oracle programs, and indirect dependencies through fixtures or build
steps.

The dependency-removal check must cover the eight production entry points,
`sandbox-common.sh`, `sandbox-inventory.py`, `sandbox-port-state.py`,
`devtools-base.smolfile`, every `tests/*-test.sh` file, and
`tests/fixtures/smolvm`. Historical source names in the coverage map are allowed.
The authoritative provisioning inputs remain `templates/*.smlbx`.

Verify independence in an isolated copy of the source tree with those baseline
files absent. Build the Rust suite in that copy and run the compiled tests
without host Bash or Python available to test subprocesses. Keep the Rust
toolchain and native build prerequisites available for compilation. Do not
delete the retained baseline from the working repository to perform this check.

## Validation and evidence

Run these checks for both the normal source tree and the isolated copy:

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --features test-adapter --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --features test-adapter --all-targets
cargo build --locked --release --bin smolsandbox
```

Run the independent mocked suite on the release targets in
`.github/workflows/rust.yml`: Linux x86-64, Linux ARM64, macOS ARM64, and
Windows x86-64. Keep platform-specific permission and signal tests on their
applicable native targets. A platform exclusion requires an explicit reason;
an exclusion must not hide a remaining script dependency.

Record commands, revisions, target platforms, test results, and the baseline
dependency-removal result in a validation record under `FEATURES/`. Use
`passed`, `failed`, `not_run`, or `inconclusive` for each check. Preserve the
historical Feature 0004 results and distinguish new evidence from those runs.

## Acceptance criteria

- **AC-1:** The coverage map accounts for every assertion in the inventory
  boundary with an independent Rust test or a cited Feature 0004 replacement
  decision. No case remains unmapped.
- **AC-2:** Application tests prove workspace serialization through port
  update, verification, and start, using bounded process coordination.
- **AC-3:** Failure tests prove that update, verification, and start failures
  prevent subsequent prohibited operations and preserve the required state.
- **AC-4:** Independent template and seed tests preserve the detailed shell
  provisioning/configuration assertions without reading the old definition.
- **AC-5:** Every differential case and the shell startup comparison retain
  independently specified expectations without invoking Bash or Python.
- **AC-6:** The isolated copy compiles and passes the Rust checks with baseline
  files absent. Compiled tests pass without host Bash/Python available.
- **AC-7:** Native mocked runs pass on every release target, with results and
  any platform-specific exclusions recorded.
- **AC-8:** The baseline remains retained until Feature 0004's separate cutover
  gates pass. The validation record makes no claim that mocks establish real
  SmolVM, authentication, or terminal behavior.

## Implementation outline

1. Record the baseline revision and build the assertion coverage map.
2. Identify existing independent Rust coverage for each mapped assertion.
3. Extend the Rust test double with the required failures and coordination.
4. Add missing regression tests and replace live baseline comparisons.
5. Replace the old-definition inclusion with direct template assertions.
6. Verify baseline independence and execute the native validation matrix.
7. Record results and hand the evidence to the Feature 0004 cutover work.
