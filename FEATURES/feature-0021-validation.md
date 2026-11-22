# Feature 0021 validation: Windows ARM64 release target

## Status and source

Audit date: 2026-11-22. Status: In progress. The implementation is present;
acceptance remains pending native Windows ARM64 and disposable hosted release
evidence. Local fixtures do not establish native Windows execution, action
compatibility, or real guest support.

The tested source is revision
`dcc72bec291736a225633e3afe5694bb793ee92f` plus the local Feature 0021 changes.
Both Cargo files use package version `0.15.0`, a minor increment from the actual
`0.14.0` implementation baseline. Lifecycle policy revision remains `7`.
Templates, runtime host policy, user definitions, and machine identity have no
implementation changes.

The tracked implementation diff SHA-256 is
`7235a599224e400471d608c9ef2d307139c63702336a89771947b17dcba255d1`,
computed from the exact bytes emitted by:

```sh
git diff --binary dcc72bec291736a225633e3afe5694bb793ee92f -- Cargo.toml Cargo.lock .github examples README.md RUST-APPLICATION.md assets | sha256sum
```

The new, untracked-at-validation badge
`assets/badges/download-windows-arm64.svg` has SHA-256
`cf24c93982575ac388a940821eec07ef0ae2fcf29b5f611b222e307eed391e70`.
The diff fingerprint excludes this validation record, the specification status
update, and the appended Feature 0010 audit.

## Local environment and commands

Execution host: native Linux `x86_64-unknown-linux-gnu`; Rust/Cargo `1.100.0`.
`rustc -vV` reported commit `0ef1bc386059596dada8fb65b50425aab2ea637a`,
commit date `2026-11-09`, and LLVM `23.1.5`. Python is `3.11.2`.
No hosted runner image was used. Hosted runner image versions, workflow URLs,
Actions artifact IDs, disposable release IDs, and uploaded release asset IDs
are unavailable because those checks are `not_run`.

| Command or check | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | Exit 0 after workflow/example changes were frozen |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | Exit 0 after workflow/example changes were frozen |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | Exit 0 after workflow/example changes were frozen |
| `cargo test --locked --all-targets` | passed | Exit 0; 147 tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | Exit 0; 223 tests |
| `cargo build --locked --release --bin smolsbx` | passed | Exit 0; default features; production binary reports `smolsbx 0.15.0` |
| `SMOLSBX_EXPECTED_RUST_HOST=x86_64-unknown-linux-gnu cargo run --locked --example check_baseline_independence` | passed | Exit 0; compiler/Cargo native hosts verified; isolated formatting, both lint/test configurations, production build, and ten compiled test executables per feature configuration with empty `PATH` |
| `python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v` | passed | Exit 0; 96 tests, including inline/canonical eligibility equality |
| `/tmp/feature-0021/actionlint -shellcheck= -pyflakes= .github/workflows/rust.yml` | passed | Exit 0; actionlint `1.7.12`; optional external ShellCheck/Pyflakes checks disabled because those tools are unavailable |
| `git diff --check` | passed | Exit 0 |
| Local production offline CLI | passed | Empty `PATH` and nonexistent `SMOLVM_BIN`; `--help`, `--version`, and explicit Codex `create-config` each exited 0; generated definition exists |
| Baseline toolchain mismatch rejection | passed | Compiled example exited 1 with expected host `aarch64-pc-windows-msvc` on Linux x86-64; diagnostic names the required native host before isolated checks |
| Download button rendering | passed | Chromium via Playwright loaded all five SVGs; desktop 1100px and mobile 375px screenshots show visible labels without clipping; exact ARM64 URL/alternative text and SVG XML checked |
| Independent agent review | passed | Separate reviews found no concrete defects in packaging/publication and workflow/example changes |

Actionlint was obtained from the official
[v1.7.12 release](https://github.com/rhysd/actionlint/releases/tag/v1.7.12).
The downloaded Linux AMD64 archive matched its published checksum:
`8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8`.
Local command logs and screenshots were collected under `/tmp/feature-0021/`;
that temporary directory is not a durable hosted artifact.

An initial offline smoke attempt ran while the release build was still in
progress and observed the previous `0.14.0` binary. After the release build
finished, the same version assertion passed with `0.15.0` and all three offline
commands passed. No source change was required.

## Acceptance evidence

| Criterion | Result | Evidence and remaining work |
| --- | --- | --- |
| AC-01 | passed | Independent five-target runner map, unique ARM64 entry, and actionlint syntax validation passed |
| AC-02 | not_run | Hosted Windows ARM64 run required; workflow records OS/architecture/image, verifies compiler and Cargo hosts, and runs both feature configurations and baseline independence |
| AC-03 | passed | Independent PE fixtures accept `0xAA64`/`0x8664`, reject substitutions in both directions and x86, and reject malformed DOS/PE signatures, out-of-bounds offsets, and truncated DOS/COFF headers before executable invocation |
| AC-04 | inconclusive | Five-target fixtures pass root-entry, regular-file, byte, deterministic ZIP, and version invocation checks; production feature selection reviewed; native ARM64 extraction/execution remains `not_run` |
| AC-05 | inconclusive | All-five missing-local-package checks fail before writes; exact uploaded sets, immutable SHA, own-run downloads, and full native-matrix success guard reviewed; disposable hosted failure/publication gates remain `not_run` |
| AC-06 | inconclusive | Five-target fake-API recovery, fifth-upload interruption, all-target conflicts, complete rerun, and preservation of historical four-target release IDs/bytes/notes/tags passed; disposable hosted API evidence remains `not_run` |
| AC-07 | passed | Trigger, permission, eligibility, dependency, and inline/canonical equality regressions passed |
| AC-08 | inconclusive | Five README buttons rendered locally; release tables, exact ARM64 link/alternative text, runtime limitation, and historical recovery guidance reviewed; public ARM64 ZIP download after first release remains `not_run` |
| AC-09 | inconclusive | Linux offline production smoke passed; Windows ARM64 workflow contains offline CLI and production unsupported-host smoke with query-only test-double calls and state preservation; native Windows execution remains `not_run` |
| AC-10 | inconclusive | Matching minor bump and local Rust/Python checks passed; original-four-plus-ARM64 hosted execution and comparison of historical hosted tags/assets remain `not_run` |

## Pending hosted and real guest work

Run the committed workflow on an allowed branch. Record the immutable commit,
workflow URL, runner image, ARM64 host toolchain, check results, Actions artifact
IDs, PE inspection, and extracted version output. A failure, cancellation, or
skip of the ARM64 job must prevent publication.

Use a disposable repository to observe publication, interrupted-upload
recovery, complete rerun, conflicting-asset preservation, and failure gates.
Record release/asset IDs and compare retained IDs, bytes, notes, and tag SHA.
Do not create test releases in the production repository. Recover historical
releases only with their original workflow revision and required target set.

After the first eligible normal release, download the public ARM64 ZIP and
record its root entry, PE machine, bytes, version, and release asset ID.
No remote tag, release, asset, branch rule, or permission was changed by this
implementation session.

Real guest checks remain `not_run`. Windows ARM64 virtualization remains
unsupported; runner execution, mocked lifecycle results, and CLI distribution
do not enable sandbox creation or close the existing real guest cutover gates.
