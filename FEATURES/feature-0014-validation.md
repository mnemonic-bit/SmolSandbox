# Feature 0014 validation

Current feature status: `Implemented`. See the
[2026-11-13 completion decision](#completion-decision-2026-11-13).
Earlier sections preserve observations from their recorded source revisions.

## Tested source and scope

Date: 2026-11-04. Package: `smolsandbox 0.7.0`. Native target:
`x86_64-unknown-linux-gnu`, Rust `1.99.1`.
The tested source is the uncommitted Feature 0014 implementation working tree
based on revision `28087885c2b84077c62ee92c5560a2cab1b68997`.
That revision alone does not contain this implementation.

The software checksum uses this ordered manifest:

```text
sha256sum Cargo.toml Cargo.lock src/cli.rs src/main.rs src/smolvm.rs src/lifecycle.rs tests/application.rs tests/cli_contract.rs tests/listing.rs tests/support/smolvm.rs tests/support/verbose.rs | sha256sum
```

The aggregate SHA-256 is
`ef55f1b24def348a4be34907e28c9418208966a3eed0565109431c98d1635a63`.

`Cli::parse_application` handles the built-in `help` command's global flag
placement before delegating help rendering and validation to Clap. Other
commands and arguments after a standalone `--` retain their original bytes.

`Smolvm::run` now sends management output to null streams by default. In
verbose mode, child stdout uses a duplicated application stderr handle and
child stderr inherits application stderr. Management output has no
application buffer or forwarding thread. `Lifecycle::command` continues to
route management operations, including `Lifecycle::seed`, through that path.

`Smolvm::capture` captures query protocol bytes and streams stderr by bounded
chunks when verbose. Both query readers are joined before return, including
after cancellation. Discovery uses the same capture implementation while
preserving its null stdin. Capability inspection retains its existing stderr
protocol consumer. Query stdout remains internal in both modes.

`Lifecycle::guest` and `Smolvm::interactive` retain `machine exec -it`, selected
machines, argument boundaries, secret references, and inherited standard
handles. Three formerly ignored stop failures now produce warnings before
the existing continuation. Exit precedence, operation count, creation
arguments, labels, hash inputs, machine names, `REVISION` (`5`), ports, lock
ordering, and mutation policy remain unchanged in the reviewed diff.

## Commands and results

All Rust commands run from the repository root, serially. The release build
excludes `test-adapter`. Local logs use the prefix `/tmp/feature-0014-`;
these temporary files are local observations, not published CI artifacts.

| Command | Result | Exit code | Evidence |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0 | `fmt.log` |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0 | `clippy-default.log` |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0 | `clippy-adapter.log` |
| `cargo test --locked --all-targets` | passed | 0 | `tests-default.log` |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0 | `tests-adapter.log` |
| `cargo build --locked --release --bin smolsandbox` | passed | 0 | `release.log`; production features |
| `cargo run --locked --example check_baseline_independence` | passed | 0 | `baseline-independence.log`; isolated checks and eight compiled test executables per feature mode with empty `PATH` |
| `target/release/smolsandbox --version` | passed | 0 | Printed `smolsandbox 0.7.0` |
| `target/release/smolsandbox help build --verbose` | passed | 0 | Release help displayed the global flag |
| `git diff --check` | passed | 0 | Source and documentation whitespace check |

Development checks found and corrected a test's `WARN`/`WARNING` assertion,
an empty-stdout assertion on `build`'s visible selection output, and a Clippy
byte-literal style violation. An interim compile also ran while a test module
was being created. An earlier adapter run failed virtualization preflight
while default/adapter builds overlapped; executable overwrite is the suspected
cause. Review also found Clap's built-in help parser bypasses global options;
the application entry point now accepts `help build --verbose` and equivalent
placements without changing help path validation. The passing serial suites
above supersede those development attempts.

## Acceptance criteria

Test symbols without another file qualifier belong to
`tests/support/verbose.rs`, included by `tests/application.rs` with
`test-adapter`. The default suite passed 60 library, 12 application, 11 CLI
contract, 8 fixed-data, and 9 listing tests. The adapter suite passed 60
library, 64 application, 11 CLI contract, 8 fixed-data, and 10 listing tests.

| Criterion | Result | Evidence and limits |
| --- | --- | --- |
| AC-01 | passed | `management_output_policy_covers_build_seed_ports_and_cleanup` checks create, start, update, pack, stop, delete, internal exec, application results, and guest output in both modes |
| AC-02 | passed | `verbose_management_streams_both_outputs_before_exit_and_drains_before_guest` receives both markers while the child waits at a release gate; markers appear only on stderr |
| AC-03 | passed | `tests/cli_contract.rs`: global placement across all commands including built-in help, both workspace positions, repeated flags, preserved usage errors, offline help/version, and no `-v` or quiet alias |
| AC-04 | passed | CLI boundary test passed; interactive fixture additionally checks guest `--verbose`, `--help`, empty argument, and a value containing spaces against recorded argv |
| AC-05 | passed | Bidirectional prompt fixture checks inherited pipes, stdin round-trip, guest stdout/stderr, ANSI bytes, and exact argv in both modes; login fixture checks visible session output |
| AC-06 | inconclusive | Four native Linux mocked PTY cases passed as detailed below; real SmolVM, authentication, and native Windows console observations remain not_run |
| AC-07 | passed | Management query markers, existing fixed identity checks, and `tests/listing.rs::verbose_queries_preserve_json_identity_calls_and_read_only_state` check protocol isolation, identical parsed reports, exact calls, and state snapshots |
| AC-08 | passed | Nine-operation failure matrix covers version, capability help, inventory, creation, packing, start, stop, deletion, and seeding; listing tests cover spawn failure and malformed inventory with empty JSON stdout; clean stop failure warns and continues |
| AC-09 | passed | Simultaneous 1 MiB stdout and 1 MiB stderr complete in both modes; gated handoff verifies preparation before guest and cleanup afterward; initial-query cancellation joins readers before return |
| AC-10 | passed | Existing `guest_status_and_cleanup_failure_precedence` and `termination_during_start_seed_and_guest_stops_the_started_machine` now check both output modes; independent lifecycle ownership and failure-preservation regressions remain in the suite |
| AC-11 | passed | Offline CLI checks passed; listing mode comparison checks identical identity/report/state and version/inventory-only calls; interactive fixture checks a synthetic secret sentinel and secret reference; creation-policy review found no policy change |
| AC-12 | passed | Independent help and README/operating-reference review covers default suppression, live verbose stderr, visible application errors, guest boundary, and inherited interactive I/O; offline help fixtures passed |
| AC-13 | passed | All seven required Rust commands exited 0; both isolated feature modes and eight compiled test executables per mode passed with empty `PATH`; independent creation-policy diff review passed |

## Native terminal observations

`cargo build --locked --features test-adapter --bins` passed with exit code 0.
`python3 /tmp/feature0014-pty.py` passed with exit code 0. The driver SHA-256
is `9b90bd04af38734cbd58e9cdf1034746904b6b3a5a7d1836d42ea5f4735b15e6`;
the observed results are retained locally in `/tmp/feature-0014-pty.log`.
The final source passed a repeat of the PTY driver with exit code 0 after the
help-placement fix; its four case results are in
`/tmp/feature-0014-pty-final.json`.

Target: Linux x86-64. Runtime: Rust test double reporting
`smolvm test 1.19.2`. Profile: Codex. Terminal: PTY with
`TERM=xterm-256color`.

| Output mode | Interaction | Result | Guest/application status | Cleanup |
| --- | --- | --- | --- | --- |
| Default | Prompt and typed line, then guest exit | passed | 17 | Owned machine stop observed |
| Verbose | Prompt and typed line, then guest exit | passed | 17 | Owned machine stop observed |
| Default | Prompt and typed line, then Ctrl-C | passed | 130 | Owned machine stop observed |
| Verbose | Prompt and typed line, then Ctrl-C | passed | 130 | Owned machine stop observed |

All four cases observed terminal detection for stdin/stdout/stderr, guest
stdout and stderr, exact ANSI green-ready bytes, input round-trip, dimensions
24x80 before resize and 33x101 afterward, and interactive `machine exec -it`.
Routine management markers appeared only with verbose. This records ANSI
bytes and terminal dimensions; it does not establish real guest rendering,
raw mode, echo behavior, or authentication.

## Pending native and real runtime checks

| Native target | Profiles | Output modes | Result | Next evidence |
| --- | --- | --- | --- | --- |
| Linux x86-64 | Claude, Codex, OpenCode, Hermes | Default and verbose | not_run | Real SmolVM shell/harness and login smoke checks |
| Linux ARM64 | Claude, Codex, OpenCode, Hermes | Default and verbose | not_run | Native mocked suite and real SmolVM checks |
| macOS ARM64 | Claude, Codex, OpenCode, Hermes | Default and verbose | not_run | Native mocked suite and real SmolVM checks |
| Windows x86-64 | Claude, Codex, OpenCode, Hermes | Default and verbose | not_run | Native mocked suite, console, and real SmolVM checks |

This host has no `smolvm` executable or `/dev/kvm`; no new native CI result
was observed. For each real runtime check, record source revision, target,
SmolVM version, profile, terminal type, output mode, shell/harness interaction,
authentication, resize, Ctrl-C, guest status, and cleanup. Include native
Windows console observations and CI run URLs when available.

`src-old/` remains present. Baseline-independent validation does not establish
physical removal or complete the pending Feature 0010 cutover gates.

## Completion decision: 2026-11-13

Feature status: `Implemented`. The user requested that Feature 0014 be
marked completed. The decision was recorded on source revision
`5eeed20ad571f7e772185a67eef1e4cad5e15e1b` with documentation changes only;
the software version remains unchanged.

Previous observations, acceptance results, and source revisions are
preserved. AC-06 remains `inconclusive` in the recorded validation, and
the pending native and real runtime checks remain `not_run`. The user
supplied no new CI or guest results with the completion request.

Feature 0010 was separately marked `Implemented` at the user's request on
2026-11-13. Its [completion decision](feature-0010-validation.md#completion-decision-2026-11-13)
and dated audits supersede the earlier feature-status and baseline-presence
statements above without changing those historical observations.
