# Feature 0009 validation record

Date: 2026-10-24 UTC. Comparison revision:
`55df3db989ba410e61ae64883224badf5115e613`. Feature status remains `Proposed`:
native release-target execution for AC-7 is `not_run` on three targets.

The [coverage map](feature-0009-coverage.md) records the comparison inputs,
initial uncommitted changes, every shell assertion group, independent Rust
symbols, fixed-fixture provenance, and explicit Feature 0004 CLI replacements.
The initial Windows ACL test changes and both proposed feature specifications
were preserved. No retained baseline file or provisioning template was changed.

The tested implementation is the uncommitted change on the comparison
revision. Its 26 Rust/build/template/data/workflow inputs have aggregate SHA-256
`6133e79f35024445f5bb4257816273ed7683c6545180bc298bfd7bac9d28d2b1`.
The digest sorts unique relative paths and hashes each UTF-8 path, NUL, file
bytes, NUL. Inputs are `Cargo.toml`, `Cargo.lock`, `.github/workflows/rust.yml`,
the three top-level Rust integration tests, and `.rs`, `.json`, `.smlbx` files
under `src/`, `templates/`, `tests/data/`, `tests/support/`, and `examples/`.
Documentation is excluded from this digest.

Execution host: `x86_64-unknown-linux-gnu`. Toolchain:
`rustc 1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1.
These results are new Feature 0009 evidence. The historical
[Feature 0004 validation record](feature-0004-validation.md) is unchanged.

## Required Rust checks

Every `passed` entry below exited 0. The isolated copy was
`/tmp/.tmpAwu5Ft`; its retained baseline files were absent before compilation.

| Command | Normal source | Isolated source |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | passed |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | passed |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | passed |
| `cargo test --locked --all-targets` | passed | passed |
| `cargo test --locked --features test-adapter --all-targets` | passed | passed |
| `cargo build --locked --release --bin codex-sandbox` | passed | passed |

Default-feature runs passed 55 unit, 9 application, 6 CLI contract, and 6 fixed
expectation tests. Feature-enabled runs passed 55 unit, 54 application, 6 CLI
contract, and 6 fixed expectation tests. Binary/example harnesses contain zero
tests. The fixed expectation tests retain 1,751 matrix cases and five explicit
publication/recovery steps. No tests were ignored on Linux.

The application serialization test pauses the first native adapter during
port update, confirms the second application actually encounters a contended
lifecycle lock, and releases the first only after checking that the second
has not updated/started the machine. Caller-tagged operations establish
first update < verification < start < second inventory under the lock.
Readiness, releases, and child completion have deadlines; failure cleanup
releases gates and kills/waits for owned application children. Port update,
verification, and start failure tests preserve recovery and prohibit later
operations. Additional tests cover replacement, purge, cleanup, cache
publication, selected host policies, seed construction, and failure precedence.
Native help descriptions and CLI contract assertions also preserve the shell
suite's semantic help checks under the native command interface.

## Baseline independence

| Check | Result | Evidence |
| --- | --- | --- |
| Full removal boundary | passed | The isolated copy omitted all eight production entry points, `sandbox-common.sh`, both Python helpers, `devtools-base.smolfile`, every `tests/*-test.sh`, and `tests/fixtures/smolvm`. All six Rust checks passed afterward. |
| Native compiled tests with empty tool search path | passed | `BASELINE_INDEPENDENCE_KEEP_COPY=1 cargo run --locked --example check_baseline_independence` exited 0. Seven test executables per feature configuration ran with `PATH=""`; both configurations passed. |
| Native compiled tests with no host interpreters in the process root | passed | `python3 -u /tmp/feature0009-chroot.py /tmp/.tmpAwu5Ft` exited 0. Fourteen compiled test runners passed inside temporary `chroot` roots, with only matching native binaries, their dynamic libraries, empty temporary directories, and a regular empty `/dev/null` file. No interpreter, script, baseline file, toolchain, or source file was installed in that process root. |
| Host-subprocess/build-input audit | passed | Reviewed `Command::new`, embedded resource references, manifest/build inputs and test fixtures. Test subprocesses invoke native Cargo test binaries or the selected native SmolVM adapter. Templates and template assertions describe guest shell/Python commands; they do not execute those commands on the host. No `build.rs` or executable baseline oracle remains. |
| Baseline retention | passed | `git diff --name-only HEAD -- '*.sh' '*.py' devtools-base.smolfile tests/fixtures/smolvm` produced no paths. Baseline files remain in the working repository. |
| Coverage symbol lookup | passed | All 73 assertion groups are mapped; all 126 prefixed Rust symbol references resolve to existing test functions. A separate source review confirmed the help-content transfer and differential source ranges. |
| Whitespace | passed | `git diff --check HEAD` exited 0. |

The Rust example is the repeatable platform-independent removal check. It
copies source without the baseline, runs all six checks with the native
toolchain available, discovers test executables through Cargo's JSON output,
then runs the compiled executables with empty `PATH`. Its default behavior
removes the copy on completion. `BASELINE_INDEPENDENCE_KEEP_COPY=1` retains the
copy for inspection or additional OS restrictions. Empty `PATH` alone does
not prevent absolute interpreter invocation; the separate Linux process-root
check establishes interpreter absence for this execution.

The Linux process-root check used a temporary orchestration script with
SHA-256 `8aab5a1a999f8ddace96681d0db268360e8e82cab5a67b1ef76ebc3363babfef`.
Python and `ldd` ran outside the process root to collect/copy native artifacts;
test runners and their children could not access those tools. The procedure
was:

1. Process the modes sequentially. Compile each mode with `cargo test --locked --all-targets --no-run --message-format=json`, adding `--features test-adapter` for the second mode.
2. Collect every compiler artifact with `profile.test=true` and a non-null `executable`: seven per mode.
3. Create a temporary root for the current mode with `/tmp` and `/dev/null`. Copy its seven runner paths and matching native `target/debug/codex-sandbox` and `smolvm-test-double` binaries at their original absolute paths inside that root.
4. Resolve each native binary's dynamic libraries with `ldd`. Copy those libraries and the ELF loader at their original absolute paths inside the root.
5. Assert that the root contains no Bash/Python executable and no `.sh` or `.py` files.
6. Execute each runner through `/usr/sbin/chroot <temporary-root> <absolute-runner-path>` with empty `PATH` and a 180-second process deadline. Require every exit status to be 0.
7. Remove the temporary process root after its seven runners finish. Repeat for the other mode, including its matching application binaries, for fourteen runner executions in total.

The first restricted-root setup failed because `/dev/null` was absent; a
second setup could not open a newly created device in this container. A
regular empty file supplied the tests' null stdin. The corrected setup passed
without changing application/test code. These were validation-environment
failures, not a remaining interpreter dependency.

Temporary command logs are `/tmp/feature0009-normal-{1..6}.log`,
`/tmp/feature0009-isolated.log`, and `/tmp/feature0009-chroot-final.log`.
This record retains the commands, results, counts, source identity, and
isolation conditions independently of those temporary logs.

## Release targets

| Native release target | Result | Evidence or required next action |
| --- | --- | --- |
| Linux x86-64, `x86_64-unknown-linux-gnu` | passed | Required normal/isolated checks, both empty-PATH configurations, and both restricted-root configurations passed in this workspace. |
| Linux ARM64, `aarch64-unknown-linux-gnu` | not_run | This workspace has no native ARM64 execution host. Run the configured `ubuntu-24.04-arm` workflow job. |
| macOS ARM64, `aarch64-apple-darwin` | not_run | This workspace has no native macOS execution host. Run the configured `macos-14` workflow job. |
| Windows x86-64, `x86_64-pc-windows-msvc` | not_run | This workspace has no native Windows execution host. Run the configured `windows-2025` workflow job. |

`.github/workflows/rust.yml` now runs default-feature and feature-enabled
Clippy/tests, release build, and the isolated removal/empty-PATH example on
every native release runner. Workflow configuration is not an execution
result. AC-7 remains `not_run` until the three remaining native jobs pass.

Additional compilation checks provide narrower evidence:

| Command | Result |
| --- | --- |
| `cargo check --locked --features test-adapter --all-targets --target aarch64-unknown-linux-gnu` | passed |
| `cargo check --locked --features test-adapter --all-targets --target aarch64-apple-darwin` | passed |
| `cargo check --locked --features test-adapter --all-targets --target x86_64-pc-windows-gnu` | passed |

Windows GNU compilation does not establish Windows MSVC execution. POSIX
permission/symlink and Unix signal tests stay on Unix; Windows ACL tests stay
on Windows. Default-feature application tests retain their existing Windows
exclusion because native home discovery would otherwise use the real user's
home. Feature-enabled Windows runs use an isolated test home and include the
full application suite. Independent data and CLI contract tests have no OS
exclusion. No platform exclusion hides a script dependency.

## Acceptance and separate cutover gates

| Criterion | Result | Evidence |
| --- | --- | --- |
| AC-1 | passed | All eight shell suites and former live comparisons are accounted for in the coverage map. |
| AC-2 | passed | Application contention/readiness/release test proves serialization through port verification and start. |
| AC-3 | passed | Update, verification, start, replacement, purge, and cleanup failure regressions passed. |
| AC-4 | passed | Independent parsed template/seed/host tests and native seed command tests passed. |
| AC-5 | passed | Six fixed expectation tests and independent startup assertions passed without host oracles. |
| AC-6 | passed | Required isolated builds/checks, empty-PATH runners, and Linux interpreter-free process-root runners passed. |
| AC-7 | not_run | Three native release-target jobs still require execution. |
| AC-8 | passed | Baseline retained, historical record preserved, real cutover claims explicitly withheld. |

Real SmolVM guest provisioning, guest file-copy effects, all-profile guest
smoke tests, interactive authentication, and terminal behavior remain
`not_run` for this feature. Mocked operations and static template assertions
do not establish those results. Feature 0004 owns those separate cutover
gates; Feature 0008's download-integrity work remains separate. No baseline
deletion or shell-era state import is authorized by these results.
