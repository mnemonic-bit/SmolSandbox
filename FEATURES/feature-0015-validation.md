# Feature 0015 validation

The current version is `0.10.0`, following the
[feature-version decision on 2026-11-14](#feature-version-decision-2026-11-14).
The 2026-11-13 audit below preserves the earlier `1.0.0` observations.

## Local implementation audit: 2026-11-13

Feature status: `In progress`. The command rename is implemented locally.
AC-07 remains `not_run` because native CI execution and uploaded artifacts
have not been observed for these changes. This record does not mark the
feature `Implemented` or change previous native and real guest results.

The implementation baseline is commit
`5eeed20ad571f7e772185a67eef1e4cad5e15e1b`, package version `0.9.1`.
The specification observed an earlier `0.8.1` baseline. The current baseline
includes later provisioning changes; the rename preserves those template
bytes and increments the major version from `0.9.1` to `1.0.0`.
Existing edits to README and the Feature 0010 and 0014 records were preserved.
The implementation uses the `smolsandbox` package and library, the `smolsbx`
production target, and the unchanged `smolvm-test-double` target.

Validation ran on native `x86_64-unknown-linux-gnu` with Rust/Cargo `1.100.0`.
The tested software snapshot SHA-256 is
`30fbf4528376fee51f568aee3db8312c4bc53169137c2f2e55d476ecaa0fc7ff`.
The digest concatenates relative path, NUL, file bytes, and NUL in lexical
path order for `Cargo.toml`, `Cargo.lock`, and all files under `src/`,
`tests/`, `templates/`, `examples/`, and `.github/workflows/`.
Documentation is outside this digest.

## Acceptance results

| Criterion | Result | Evidence |
| --- | --- | --- |
| AC-01 | passed | `cargo metadata --locked --no-deps --format-version 1` exited 0; package/library remain `smolsandbox`; production target is `smolsbx`; no `smolsandbox` binary target; manifest and lockfile both select `1.0.0` |
| AC-02 | passed | Fresh release build and temporary local installation exited 0; both executables printed exactly `smolsbx 1.0.0\n`; neither output directory contained `smolsandbox` or `smolsandbox.exe` |
| AC-03 | passed | CLI contracts passed in both configurations; every subcommand's help and top-level version identity were checked with `smolsbx` and `smolsbx.exe` basenames; offline channels, status, and filesystem preservation assertions passed |
| AC-04 | passed | Both complete test configurations passed, including guidance, parsing, verbose output, lifecycle behavior, and exact guest arguments after `--` |
| AC-05 | passed | Baseline/renamed binary comparison reused the same managed machine with equal identities, runtime calls, inventory, reports, and preserved state; details below |
| AC-06 | passed | Source diff preserves identity construction, persisted paths/formats, metadata identifiers, and `REVISION = "5"`; all four templates equal baseline bytes |
| AC-07 | not_run | Workflow inspection passed for four native targets, both configurations, default-feature release builds, artifact names/paths, and `if-no-files-found: error`; native CI runs and uploaded filenames remain unobserved |
| AC-08 | passed | Both existing relocation regressions and a copied release executable passed; baseline-independence exited 0 and ran nine compiled test executables per configuration with empty `PATH` |
| AC-09 | passed | Active README, operating reference, context, and agent build instructions use `smolsbx`; migration guidance explains retained state; remaining old-name references were audited by meaning |

## Commands and distribution checks

| Command | Result | Exit code and observation |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0 after formatting the renamed assertions |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0 |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0 |
| `cargo test --locked --all-targets` | passed | 0; 118 tests, including 66 library tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0; 185 tests, including 65 library tests |
| `cargo build --locked --release --bin smolsbx` | passed | 0; default features |
| `cargo run --locked --example check_baseline_independence` | passed | 0; isolated source checks, renamed release target, and nine compiled test executables per configuration with empty `PATH` |
| `git diff --check` | passed | 0 |

The six Cargo command logs after formatting are in
`/tmp/feature0015-checks-pkgm_d0n/1.log` through `6.log`.
The command/exit-code summary is `results.json` in that directory.
These temporary paths are session evidence, not retained CI artifacts.

The fresh distribution commands both exited 0:

```text
cargo build --locked --release --bin smolsbx --target-dir /tmp/feature0015-distribution-bb9l5o5y/fresh-target
cargo install --locked --path . --bin smolsbx --root /tmp/feature0015-distribution-bb9l5o5y/installation --target-dir /tmp/feature0015-distribution-bb9l5o5y/fresh-target
```

Direct execution of `fresh-target/release/smolsbx --version` and
`installation/bin/smolsbx --version` returned exit 0, empty stderr, and exactly
`smolsbx 1.0.0\n`. Checks confirmed both old executable basenames absent
from those fresh output directories. Existing checkout build outputs and
previously installed executables were preserved.

The fresh release executable was copied to
`/tmp/feature0015-relocation-so035b_k/smolsbx`. With an empty `PATH` and an
isolated home, `create-config --harness PROFILE --smolfile PATH` exited 0
for Claude, Codex, Hermes, and OpenCode. Each generated definition matched
the corresponding embedded template byte-for-byte. Direct `--version` also
exited 0 with exact identity. The copied executable needed no adjacent
templates or host Bash/Python. `results.json` in that directory records the
four configuration results.

## Existing-state compatibility comparison

`python3 /tmp/feature-0015-compat-hvd8mygx/compare.py` exited 0.
The baseline source came from `git archive` of the baseline commit.
Both application builds enabled `test-adapter`; the experiment used the
baseline Rust test double reporting fixed version `smolvm test 1.19.2`.
The binaries reported `smolsandbox 0.9.1` and `smolsbx 1.0.0` respectively.

The baseline generated the Codex definition and created the existing
workspace machine with a prepared artifact and a credential sentinel.
The experiment saved that fixture and restored identical copies to the
same paths before each reuse invocation:

| Input or identity | Value |
| --- | --- |
| Workspace | `/tmp/feature-0015-compat-hvd8mygx/fixture/project` |
| Application home | `/tmp/feature-0015-compat-hvd8mygx/fixture/home/.smolsandbox` |
| Definition SHA-256 | `555c0968909e87c15810a1aaed1a6a5ca0a23a66d4f37715758b3f006da5d1cb` |
| Base identity | `679114224b0b` |
| Policy revision | `5` |
| Workspace path SHA-256 | `e9a7d14ed26647cc9a9ec93ca9c55282a2b5349fe54809db56e121e7f8c5ce02` |
| Workspace identity SHA-256 | `a2fd678cdbc877e8d610d157c3d80a66e01f5492e2651601493ca71f3276abc2` |
| Reused machine | `codex-workspace-679114224b0b-r5-a2fd678cdbc8` |

Both binaries ran the following argument sequences and returned exit 0:

```text
start --port 3000 -- --help 'a b' '' --port 9000
list --json
```

The comparison passed for inventory, list report, runtime argument arrays,
stdout, stderr, and all 18 fixture entries, including file bytes and modes.
Definition, artifact, caches, locks, port manifest, and credential sentinel
were preserved. Neither reuse invocation created or deleted a machine or
created `.smolsbx`. Both performed the same existing port update before
start, guest execution, and stop. The guest received the arguments after
`--` unchanged, including spaces, the empty argument, and `--port 9000`.

`result.json`, `baseline-reuse.argv.jsonl`, `renamed-reuse.argv.jsonl`,
the corresponding `.snapshot.json` files, and captured stdout/stderr are
in `/tmp/feature-0015-compat-hvd8mygx/`. The credential is a test sentinel;
this comparison does not establish real authentication or guest behavior.

## Preserved templates and occurrence audit

| Template | Baseline and implementation SHA-256 |
| --- | --- |
| `templates/claude.smlbx` | `4feaf28a5c58dcb94885ee8855bac4ae54d6542110a06818c745e7e732bad06f` |
| `templates/codex.smlbx` | `555c0968909e87c15810a1aaed1a6a5ca0a23a66d4f37715758b3f006da5d1cb` |
| `templates/hermes.smlbx` | `9395ffdfeb732ee88f5264988a403211b6b70bcdf91057a9cf932c2e8f9657db` |
| `templates/opencode.smlbx` | `093c954746cc07e86873e3503d244e9de84931908c08fd491af84e80b60b5afe` |

`git grep -n -I smolsandbox` exited 0. Retained source/test occurrences
refer to the library crate or `.smolsandbox` paths. Active documentation
retains product prose, package names, paths, and explicit migration history.
The unchanged template comment names the product. Prior feature and bug
specifications and validation transcripts retain their original commands
and observations. New text in this record uses the old command only to
identify baseline execution, preserved package names, or absence checks.
The occurrence listing is `/tmp/feature0015-occurrence-audit.txt`.
Independent source/build/test review found no actionable defects.

## Pending native and real guest evidence

Record native CI run links and actual uploaded filenames for each target:

| Native target | Expected CI artifact | Result |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `smolsbx-x86_64-unknown-linux-gnu` containing `smolsbx` | not_run |
| `aarch64-unknown-linux-gnu` | `smolsbx-aarch64-unknown-linux-gnu` containing `smolsbx` | not_run |
| `aarch64-apple-darwin` | `smolsbx-aarch64-apple-darwin` containing `smolsbx` | not_run |
| `x86_64-pc-windows-msvc` | `smolsbx-x86_64-pc-windows-msvc` containing `smolsbx.exe` | not_run |

The local Linux checks establish the local results above; they do not
establish those four CI jobs or artifact uploads. Existing real SmolVM
provisioning, interactive terminal, authentication, and profile/platform
checks remain pending as recorded in Feature 0010. The separately recorded
Feature 0010 and 0014 completion decisions remain unchanged.

## Feature-version decision: 2026-11-14

The user requested a minor version bump because the repository has no other
users. This decision supersedes Feature 0015's original major-version
requirement. Relative to the implementation baseline `0.9.1`, the command
rename now selects `0.10.0` in `Cargo.toml` and the matching `smolsandbox`
entry in `Cargo.lock`. The operating reference and AC-01 reflect that
decision. The earlier `1.0.0` execution results remain historical evidence.

The source revision is `792ae2d6b101c9cb8d581db6225f2479669613bc` plus the
pending Feature 0015 changes. The software snapshot SHA-256, calculated with
the same file scope and algorithm as the earlier audit, is
`d02bba540cf19c66f393f00353756afb6076453440128329f736fc83aa73f63f`.
The host remains `x86_64-unknown-linux-gnu` with Rust/Cargo `1.100.0`.

| Check | Result | Evidence |
| --- | --- | --- |
| `cargo metadata --locked --no-deps --format-version 1` | passed | Exit 0; package/library remain `smolsandbox`; package version is `0.10.0`; binary targets remain `smolsbx` and `smolvm-test-double` |
| `cargo fmt --all -- --check` | passed | Exit 0 |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | Exit 0 |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | Exit 0 |
| `cargo test --locked --all-targets` | passed | Exit 0; 118 tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | Exit 0; 185 tests |
| `cargo build --locked --release --bin smolsbx` | passed | Exit 0; default features |
| `cargo run --locked --example check_baseline_independence` | not_run | Execution is still running |
| Debug and release executable version output | passed | Both direct `--version` invocations exited 0 with exactly `smolsbx 0.10.0\n` and empty stderr |
| Independent version-change review | passed | No findings; all 79 dependency package entries remain unchanged |

The seven required command logs are in
`/tmp/feature0015-minor-checks-aet7v72n/1.log` through `7.log`;
`results.json` records their exit codes. These temporary paths are session
evidence. Native CI artifact verification and real guest evidence remain
pending. Feature 0015 retains its `In progress` status.
