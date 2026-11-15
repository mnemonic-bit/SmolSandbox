# Bug 0004: Native macOS CI cannot start on the retired macOS 14 runner

## Status and summary

Fix implemented locally; hosted recovery validation pending.
Report prepared on 2026-11-15 for `smolsandbox 0.10.0`.
The workflow still selects `macos-14`, which GitHub Actions retired on
2026-11-02. GitHub rejects the runner configuration before the macOS job
executes any steps. Three native jobs pass, but the workflow conclusion is
`cancelled` and the macOS artifact is absent.

The cause is supported by the actual Actions annotations, job records,
workflow source, and GitHub's retirement announcement. The initial report
recorded the bug without applying a fix or changing the software version.
The implementation record below separates local checks from pending hosted
recovery validation.

## Reported and verified behavior

The user reported a grey exclamation indicator, three completed platform
builds, and a macOS job that did not run. The supplied error is:

```text
The configuration 'macos-14-arm64-us-default' is not supported
```

Read-only GitHub REST API inspection on 2026-11-15 confirms the exact
message in both of these macOS check annotations:

| Execution on 2026-11-14 | Source revision | macOS job |
| --- | --- | --- |
| [Feature branch push, run 43591744170](https://github.com/mnemonic-bit/SmolSandbox/actions/runs/43591744170) | `530f0578ce12593ec2365c7e2175c206ef0a75d8` | [132437743473](https://github.com/mnemonic-bit/SmolSandbox/actions/runs/43591744170/job/132437743473) |
| [Develop push, run 43591773427](https://github.com/mnemonic-bit/SmolSandbox/actions/runs/43591773427) | `af4496c8e4b797613a3ca621e5f5029663e4a2e1` | [132437829109](https://github.com/mnemonic-bit/SmolSandbox/actions/runs/43591773427/job/132437829109) |

Both runs have workflow conclusion `cancelled`. Both macOS jobs have
conclusion `cancelled`, `runner_name: ""`, and `steps: []`. Each macOS
annotation has `annotation_level: "failure"` and the error above. The
cancelled conclusion explains why the overall run does not show success
despite three successful jobs. No manual cancellation is needed to explain
the observed outcome.

For the develop run, the [jobs API](https://api.github.com/repos/mnemonic-bit/SmolSandbox/actions/runs/43591773427/jobs?per_page=100)
and [artifact metadata](https://api.github.com/repos/mnemonic-bit/SmolSandbox/actions/runs/43591773427/artifacts)
establish:

| Runner | Native target | Job result | Uploaded artifact name |
| --- | --- | --- | --- |
| `ubuntu-24.04` | `x86_64-unknown-linux-gnu` | passed | `smolsbx-x86_64-unknown-linux-gnu` |
| `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | passed | `smolsbx-aarch64-unknown-linux-gnu` |
| `windows-2025` | `x86_64-pc-windows-msvc` | passed | `smolsbx-x86_64-pc-windows-msvc` |
| `macos-14` | `aarch64-apple-darwin` | failed to start; API conclusion `cancelled` | absent |

The three successful jobs report success for formatting, both Clippy
configurations, both test configurations, the release build, baseline
independence, and artifact upload. Artifact metadata was inspected; archive
contents and executable behavior were not inspected. The feature branch
push independently has the same three successful jobs and artifact names.

Expected: all four native jobs obtain supported runners, execute their
checks, and upload the target artifacts. Actual: macOS runner allocation
is rejected, so checkout, Rust installation, tests, compilation, and upload
never execute for `aarch64-apple-darwin`.

## Cause and proposed correction

The affected [workflow source](https://github.com/mnemonic-bit/SmolSandbox/blob/af4496c8e4b797613a3ca621e5f5029663e4a2e1/.github/workflows/rust.yml#L21-L25)
contains:

```yaml
          - runner: macos-14
            target: aarch64-apple-darwin
```

The job selects `runs-on: ${{ matrix.runner }}`. The workflow and
`Cargo.toml` at the develop revision match the corresponding workspace
files at `530f0578ce12593ec2365c7e2175c206ef0a75d8` byte for byte.

[GitHub's runner retirement announcement](https://github.com/actions/runner-images/issues/13518)
states that all macOS 14 GitHub Actions images retired on 2026-11-02.
The November 23 extension applies only to Azure DevOps Intel images.
The observed Actions runs occurred after the GitHub retirement date.

The unsupported runner label explains the failure before any application
code runs. The message's `macos-14-arm64-us-default` value is the rejected
hosted configuration; the workflow itself supplies `macos-14`. Changing
Rust code, package pins, or the Rust target cannot make the retired runner
available. Re-running the unchanged workflow does not address the cause.

The maintainer should replace only the macOS matrix runner with
`macos-15` while retaining `aarch64-apple-darwin` and all existing checks.
[GitHub's supported runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners#standard-github-hosted-runners-for-public-repositories)
lists `macos-15` as an ARM64 runner. The retirement announcement also
recommends `macos-26` or `macos-latest`; an explicit `macos-15` selection
keeps the intended OS version visible in the workflow. An Intel runner
would not preserve the repository's native macOS ARM64 validation target.

Under [repository versioning rules](../AGENTS.md), the eventual workflow
bug fix requires a patch bump in `Cargo.toml` and the matching
`smolsandbox` entry in `Cargo.lock`: `0.10.1` if applied directly to this
`0.10.0` baseline.

## Reproduction and diagnostic limits

The two existing Actions executions reproduce the same scheduler error.
No new workflow was dispatched during this investigation. The following
read-only command was run from the repository root and returned exit 1
with the exact error. The command checks a historical annotation; it does
not allocate a runner or validate a fix.

```sh
python3 - <<'PY'
import json, sys, urllib.request
url = 'https://api.github.com/repos/mnemonic-bit/SmolSandbox/check-runs/132437829109/annotations'
with urllib.request.urlopen(url) as response:
    annotations = json.load(response)
message = "The configuration 'macos-14-arm64-us-default' is not supported"
if any(annotation.get('message') == message for annotation in annotations):
    print('failed: ' + message)
    sys.exit(1)
print('passed: unsupported-configuration annotation absent; inspect job steps and conclusion separately')
PY
```

Output:

```text
failed: The configuration 'macos-14-arm64-us-default' is not supported
```

A local Rust test cannot exercise GitHub's hosted runner allocation. The
existing hosted executions supply failure evidence; a new hosted execution
with the corrected workflow is required to verify recovery. Local Rust
checks were `not_run` for this report because no software changed. Fix,
regression implementation, and successful recovery remain outside this
report's completed work.

## Acceptance for the eventual fix

The maintainer should record the new workflow revision and run URL, then
verify these criteria:

- AC-01: The macOS job obtains an ARM64 runner and executes checkout;
  the unsupported-configuration annotation is absent. Result: `not_run`.
- AC-02: The macOS job passes formatting, both Clippy configurations,
  both test configurations, release compilation, and baseline independence.
  Result: `not_run`.
- AC-03: All four jobs and the overall workflow conclude `success`.
  All four expected artifacts exist, including
  `smolsbx-aarch64-apple-darwin`; inspect the macOS archive for `smolsbx`.
  Result: `not_run`.
- AC-04: The software fix includes the required patch bump and passes the
  seven Rust regression commands in `AGENTS.md`. Result: `not_run`.

Append the new native results to
[Feature 0010 validation](../FEATURES/feature-0010-validation.md).
Native mocked CI success does not establish real SmolVM provisioning,
authentication, or terminal behavior; those checks remain separate under
[RUST-APPLICATION.md](../RUST-APPLICATION.md#validation-and-cutover-gates).

## Implementation and local validation: 2026-11-15

The local fix changes `.github/workflows/rust.yml` from `macos-14` to
`macos-15`. The target remains `aarch64-apple-darwin`. The other three
matrix entries, validation steps, and artifact configuration are unchanged.
`Cargo.toml` and the `smolsandbox` entry in `Cargo.lock` select `0.10.1`.
[GitHub's supported runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners#standard-github-hosted-runners-for-public-repositories)
was checked on 2026-11-15 and lists `macos-15` as ARM64.

Validation used source revision
`530f0578ce12593ec2365c7e2175c206ef0a75d8` plus these local changes on
native `x86_64-unknown-linux-gnu`, Rust/Cargo `1.100.0`.
The historical annotation command above was rerun before the fix and
returned exit 1 with the reported unsupported-configuration message.
That historical annotation cannot verify a changed workflow. Runner
allocation requires a new hosted execution; no local unit test was added
for GitHub's scheduler.

| Check | Result | Evidence |
| --- | --- | --- |
| Workflow change scope | passed | Exact comparison with `HEAD` found only `macos-14` replaced by `macos-15`; ARM64 target and all steps preserved |
| Matching package versions | passed | Parsed manifest and lockfile both select `0.10.1` |
| `cargo fmt --all -- --check` | passed | Exit 0 |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | Exit 0 |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | Exit 0 |
| `cargo test --locked --all-targets` | passed | Exit 0; 118 tests passed |
| `cargo test --locked --features test-adapter --all-targets` | passed | Exit 0; 185 tests passed |
| `cargo build --locked --release --bin smolsbx` | passed | Exit 0; release executable reports `smolsbx 0.10.1` |
| `cargo run --locked --example check_baseline_independence` | passed | Exit 0; isolated source checks and nine compiled test executables per configuration passed with empty `PATH` |

AC-04: `passed`. AC-01, AC-02, and AC-03: `not_run` for the corrected
workflow because the fix remains local. A new hosted run must establish
macOS allocation, native checks, overall success, and artifact contents.
No new workflow revision or run URL exists for this local fix.
Real SmolVM guest checks: `not_run`.

Session logs are in `/tmp/bug0004-rust-checks-iswtmveb/1.log` through
`7.log`; these temporary paths are not durable CI artifacts. The
[Feature 0010 validation record](../FEATURES/feature-0010-validation.md#bug-0004-local-runner-fix-validation-2026-11-15)
preserves this result alongside previous native observations. Hosted
recovery validation remains pending.
