# Feature 0004 validation record

## Documentation preparation update: 2026-10-25

Feature 0010 makes the Rust application the active documented interface. The
retained shell/Python baseline and its fixtures are archived in `src-old/`;
archiving does not establish the final removal gate. The
[Feature 0010 validation record](feature-0010-validation.md) records new local
checks and outstanding real guest and final cutover evidence. The completion
decision and historical observations below retain their original meaning.

## Completion update: 2026-10-25

Feature status: `Implemented`. The user requested that Feature 0004 be marked
completed after confirming that CI is green. This status records the user's
completion decision. No additional checks were run for this update, and no CI
run URL or real SmolVM results were supplied or independently inspected.

The shell/Python baseline remains in the repository. Feature 0010's
documentation cutover remains separate work. The historical `not_run` results
below retain their original meaning; this status update does not establish
that real guest checks or baseline removal occurred.

## Historical validation: 2026-10-24

Date: 2026-10-24. Feature status: `Proposed`. Cutover result: `not_run`.

The implementation uses one Rust executable, four embedded Smolfile templates,
a subprocess SmolVM adapter, native filesystem permissions and locks, and typed
inventory/lifecycle policy. Existing user changes were retained. Production
scripts and their fixtures remain the pre-cutover parity baseline.

## Executed checks

| Check | Result | Evidence |
| --- | --- | --- |
| Rust mocked suite | passed | `cargo test --locked --features test-adapter --all-targets`, exit 0: 50 unit, 26 application, 6 differential tests |
| Default-feature suite | passed | `cargo test --locked --all-targets`, exit 0: 50 unit, 9 application, 6 differential tests |
| Rust lint | passed | `cargo clippy --locked --features test-adapter --all-targets -- -D warnings`, exit 0 |
| Default-feature Rust lint | passed | `cargo clippy --locked --all-targets -- -D warnings`, exit 0 |
| Formatting | passed | `cargo fmt --all -- --check`, exit 0 |
| Whitespace validation | passed | `git diff --check`, exit 0 |
| Release executable | passed | `cargo build --locked --release --bin codex-sandbox`, exit 0 |
| Linux release relocation | passed | Copied only the default-feature executable to an unrelated temporary directory; `--help`, `--version`, and offline `create-config --harness codex` exited 0 with empty `PATH` |
| Existing shell baseline | passed | Every `tests/*-test.sh` script exited 0; eight scripts executed |
| Windows compilation | passed | `cargo check --locked --target x86_64-pc-windows-gnu --all-targets --features test-adapter`, exit 0 |
| macOS ARM64 compilation | passed | `cargo check --locked --target aarch64-apple-darwin --all-targets --features test-adapter`, exit 0 |
| Linux ARM64 compilation | passed | `cargo check --locked --target aarch64-unknown-linux-gnu --all-targets --features test-adapter`, exit 0 |

The six differential tests compare over 2,000 Rust results with the Python
baseline: inventory normalization and rejection, port metadata, ownership,
replacement readiness, purge plans, manifest/journal decisions, publication,
and interrupted recovery. The application suite separately compares the
Codex startup mutation sequence, names, labels, arguments, and guest exit
status with the shell implementation.

Application tests exercise all four profiles, configuration lookup and
precedence, offline no-clobber creation, relocation, immutable identity,
replacement failures, retained failed registrations, exact port updates,
purge policies, cleanup scope, secret references, failure precedence, and
termination during start, seeding, and harness execution. The independent
review identified and checked corrections to schema validation, early signal
handling, Windows UNC paths, lock namespaces, superseded empty artifacts,
adapter capability checks, and Windows credential ACL inheritance.

## Outstanding cutover evidence at historical validation

| Check | Result | Cause and next action |
| --- | --- | --- |
| Native Linux ARM64, macOS, and Windows mocked execution | not_run | This workspace executes Linux x86-64 only. Run the native matrix in `.github/workflows/rust.yml`. Cross-compilation does not establish runtime behavior. |
| Native Windows descendant ACL verification | not_run | The Windows-specific regression compiles but requires execution on Windows. |
| Real SmolVM smoke tests for every profile and release target | not_run | `smolvm` and `/dev/kvm` are absent from this workspace. Execute the real guest procedure in `RUST-APPLICATION.md` on compatible hosts. |
| Real terminal resize and interactive authentication | not_run | These checks require SmolVM, a terminal, and the selected harness's authentication workflow. |
| Final removal of shell/Python production files and fixtures | not_run | Feature 0004 requires native mocked and real guest validation before deletion. Retain the baseline until those gates pass. |

At historical validation, the outstanding execution results were required
before claiming Feature 0004 completed or removing the baseline. The completion
update above records the subsequent user decision. Feature 0008 remains
separate work and is not a cutover prerequisite.
