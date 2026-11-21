# Feature 0017 validation

## Local implementation audit: 2026-11-21

Status: `In progress`. Release automation is implemented locally. Mandatory
hosted acceptance checks remain `not_run`; Feature 0017 is not marked
`Implemented`.

The source baseline is `af4496c8e4b797613a3ca621e5f5029663e4a2e1` plus the
uncommitted Feature 0017 changes. The package version is `0.11.0` in both
version files. No implementation commit has been created. The following
SHA-256 digests identify the software inputs and recorded fake API evidence:

| File | SHA-256 |
| --- | --- |
| `Cargo.toml` | `d7388bb22f528423eb3fecf84b4f21c0481cceeb92b7a236b6fa9cff759519c8` |
| `Cargo.lock` | `d430a0d12d80c2a680b39c8b46c206d8cbc57899d0d06a580ba2436e23f96744` |
| `.github/workflows/rust.yml` | `f2f62e1d72babf1b8efe161bc01a1c9adc55279374336604cb5b2fde367fe91a` |
| `.github/scripts/release.py` | `95106dfe46fdb17227d9762d73eaf9c70bf625dffc8b6836bf11d395600ad217` |
| `.github/scripts/test_release.py` | `35c684811fc52df4546ce9f31fcfd3fb9f63dc6ea70d376c6850fab4a6822ee2` |
| `FEATURES/feature-0017-api-traces.json` | `61709d85bfd73b9367a77b3ccd108766a16241d5876932ac172265c64c1188cf` |

The host is native `x86_64-unknown-linux-gnu`, with Rust `1.100.0`
(`0ef1bc386`), Cargo `1.100.0` (`3acc79c0c`), and Python `3.11.2`.
Workflow syntax validation uses actionlint `1.7.12`, downloaded from its
[official release](https://github.com/rhysd/actionlint/releases/tag/v1.7.12)
and checked against that release's SHA-256 checksum list.

## Implementation and local checks

The workflow retains the four native targets, existing Rust validation
commands, production builds without `test-adapter`, and uploaded artifacts.
Automatic native validation accepts only pushes to `main` and `develop`.
Manual native validation accepts only those exact branches. PRs into `main`
run the independent `Release version eligibility` check without checkout,
dependency installation, Cargo, or repository-script execution. The inline
Python program reads metadata at the immutable event SHA. An equality test
keeps that program synchronized with `.github/scripts/release.py`.

Main native validation depends on successful early eligibility. An explicit
`always()` condition permits independent develop/manual validation when
publication eligibility is skipped; `!cancelled()` excludes cancellation.
Publication requires a main push and successful eligibility and native jobs.
Only publication has `contents: write`. There is no concurrency group that
cancels running or discards pending release attempts.

| Command | Result | Exit code and observation |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0 |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0 |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0 |
| `cargo test --locked --all-targets` | passed | 0; 118 tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0; 185 tests |
| `cargo build --locked --release --bin smolsbx` | passed | 0; production features |
| `cargo run --locked --example check_baseline_independence` | passed | 0; isolated source checks and nine compiled test executables per feature configuration with empty `PATH` |
| `python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v` | passed | 0; 40 tests |
| `python3 .github/scripts/test_release.py --record-api-traces FEATURES/feature-0017-api-traces.json` | passed | 0; 40 tests and 72 recorded fake API instances |
| `/tmp/feature17-tools/actionlint .github/workflows/rust.yml` | passed | 0; actionlint 1.7.12 |
| `git diff --check` | passed | 0 |

The 40 Python tests include 30 release-decision tests, five transport/CLI
checks, and five workflow checks. Guard tests evaluate 355 combinations:
81 native conditions, 256 publication conditions, and 18 eligibility
conditions. These tests evaluate source expressions; they do not establish
GitHub's hosted scheduling or permission behavior.

## Recorded release state and recovery

[Fake API traces](feature-0017-api-traces.json) retain request methods,
repository-relative paths, write payloads, and final tag/release state for
72 fake API instances. Test identifiers connect each trace to its assertions.
The exporter command above regenerates the artifact without contacting GitHub.
No credential values are recorded.

Fixture release commit `1111111111111111111111111111111111111111` is called
SHA A below; `2222222222222222222222222222222222222222` is SHA B.
All cases below passed their test assertions.

| Scenario | Recorded writes and resulting state |
| --- | --- |
| New `v0.11.0` at SHA A | `POST /git/refs`, `POST /releases/generate-notes`, `POST /releases`; one normal published release, title `v0.11.0`, generated body, `make_latest=legacy` |
| Matching tag, no release | Generate notes and create the missing release; no tag write |
| Matching draft with notes | `PATCH /releases/ID`; existing body retained, draft cleared, prerelease false |
| Matching draft without notes | Generate notes before publishing the draft |
| Matching published release and successful rerun | Zero writes; tag and release, including notes, unchanged |
| Conflicting tag at SHA B | Zero writes; conflicting tag preserved |
| Release without verifiable tag | Zero writes; release preserved |
| Nested annotated tag | Resolve tag objects to SHA A before accepting identity |
| Tag creation succeeds, release creation fails | Tag remains at SHA A; rerun creates only the missing release |
| Concurrent tag or release creation returns 422 | Reread tag/release identity; accept matching state, reject conflicting state |
| Duplicate error without matching server state | Fail instead of treating the creation error as success |
| Authorization, rate limit, and server errors | Explicit errors; lookup failures produce zero writes; ruleset rejection creates no release |
| Two versions complete in reverse order | Final `v0.11.0` tag is SHA A and `v0.12.0` tag is SHA B; neither attempt resolves a moving branch tip |
| Tag appears after successful eligibility | Publication rereads state and rejects the conflicting SHA before publication writes |
| PR tag or draft conflict | Zero writes; uniqueness fails even if the tag matches the synthetic merge SHA |
| Unused version already eligible for development validation | Eligibility creates no tag; first publication uses the unchanged committed version |

Transport checks distinguish an explicitly optional tag lookup's HTTP 404
from HTTP 401, 403, 409, 422, 429, 500, and 503 errors. Required resource
lookups never convert a 404 into success. CLI checks verify nonzero failure
exit status and candidate/tag/release URLs in the summary.

## Acceptance results

`not_run` below means the full mandatory verification is pending. Local
checks in the evidence column may already have passed.

| Criterion | Result | Evidence and remaining verification |
| --- | --- | --- |
| AC-01 | not_run | Dependency/condition review and guard tests passed; disposable hosted success, failure, cancellation, and skip runs remain pending |
| AC-02 | not_run | Event and permission source checks passed; hosted nonpublication and token permission output remain pending |
| AC-03 | passed | Strict TOML/root-lock fixtures, zero-write invalid metadata checks, unused-version first publication, and develop nonpublication guards passed |
| AC-04 | passed | Recorded delayed completion retains SHA A and SHA B for distinct versions; workflow has no concurrency cancellation |
| AC-05 | not_run | Fake normal release/title/generated notes checks passed; disposable hosted PR merge, run URL, tag SHA, and release URL remain pending |
| AC-06 | not_run | Every existing-state row, draft recovery, and fake rerun preservation passed; hosted successful release rerun remains pending |
| AC-07 | passed | Recorded authorization/rate-limit/ruleset failures, partial creation, matching races, and conflicting races passed |
| AC-08 | not_run | Workflow/API source review passed, native targets and commands retained; hosted token permission evidence remains pending |
| AC-09 | passed | Version/docs inspection and all seven required Rust commands passed, including baseline independence |
| AC-10 | not_run | Trigger/guard checks passed; hosted pushes/manual runs/PRs/tags and owner-managed required-check review remain pending |
| AC-11 | not_run | Badge definitions and source destinations inspected; hosted rendering, cache expiry, and no-release/equal/newer-version comparisons remain pending |
| AC-12 | not_run | Local uniqueness, summary, metadata, and API-error checks passed; hosted PR events, drafts, permissions, and merge-conflict behavior remain pending |
| AC-13 | not_run | Local dependency/guard checks, same-commit recovery, and late-conflict tests passed; hosted scheduling after eligibility failure remains pending |
| AC-14 | not_run | Main-only required-check configuration is documented; disposable ruleset and passing/failing merge attempts remain pending |

## Pending hosted and native evidence

No `GITHUB_TOKEN` or `GH_TOKEN` is available in this environment; GitHub CLI
is not installed. No disposable GitHub repository was provisioned. No hosted
workflow, ruleset, tag, or release was changed during implementation.
Workflow/release URLs, hosted event observations, token permission output,
and expected/observed hosted tag SHAs are therefore `not_run`.

Use a disposable repository for publication and deliberately failing merge
checks. Record main/develop/feature/tag push and dispatch cases, the specified
PR events including drafts, publication/rerun URLs, exact tag SHAs, release
title/body/prerelease flag, and main-only ruleset merge outcomes. Verify draft
visibility with the read-only `GITHUB_TOKEN`, including fork PR context; local
fake API coverage does not establish that token's hosted visibility.

The owner must select `Release version eligibility` as a required GitHub
Actions check only for `main`, with no bypass, after the check first appears.
The [operating reference](../RUST-APPLICATION.md#github-releases) lists the
remaining repository prerequisites and recovery procedures.

Linux ARM64, macOS ARM64, and Windows x86-64 native execution for this source
remain `not_run`. Real SmolVM provisioning, authentication, terminal, and
profile/target checks remain `not_run`; release automation and mocked checks
do not establish real guest behavior. Previous observations and status
decisions in [Feature 0010 validation](feature-0010-validation.md) remain
unchanged.

## Interface references checked during implementation

- [GitHub workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax) documents job dependencies, conditions, and permissions.
- [Pull-request workflow events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request) document proposed merge SHA context.
- [GitHub Releases API](https://docs.github.com/en/rest/releases/releases) documents draft listing, generated notes, and `make_latest=legacy`.
- [Git references API](https://docs.github.com/en/rest/git/refs) documents creation of refs at explicit SHAs.
- [Shields release badge](https://shields.io/badges/git-hub-release) and [dynamic TOML badge](https://shields.io/badges/dynamic-toml-badge) document the selected README sources.

## Bug 0006 binary asset audit: 2026-11-21

Status: `In progress`. Mandatory binary release assets supersede the original
optional packaging scope. The [Bug 0006 implementation record](../BUGS/bug-0006-release-binary-zip-assets.md#implementation-and-local-validation-2026-11-21)
identifies the tested baseline `a346389f99b461f7cd745bdb9bbc2c31c64752c0`
plus local changes, software input hashes, and matching `0.11.2` versions.
Previous results above remain observations of their recorded revisions.

The implementation creates four deterministic executable-only ZIPs after
native validation, checks architecture and extracted `--version`, transfers
artifacts from the same workflow run, and verifies release asset bytes.
Published-release recovery now fills missing assets. Complete reruns retain
asset IDs and bytes, release notes, and tag SHA. Conflicting assets fail
without deletion or replacement. The README Release badge is unchanged.

Local validation passed all seven required Rust commands: formatting, both
Clippy configurations, 118 default tests, 185 tests with `test-adapter`, the
production release build, and baseline independence with empty-`PATH` test
execution. The Python suite passed 85 tests, including the canonical/inline
equality check, 24 asset/transport cases, and nine packaging/workflow cases.
actionlint 1.7.12 passed. An independent agent reviewed packaging, artifact
transfer, upload, and recovery; both diagnostic findings were corrected.

The actual native Linux executable was packaged in a temporary fixture root.
Its ZIP contained only root file `smolsbx`, mode `0755`, with an ELF x86-64
header. The extracted executable reported `smolsbx 0.11.2`. The bug record
retains the archive SHA-256. Other target formats have local synthetic
fixtures only; those fixtures do not establish native execution.

| Added criterion | Result | Evidence and remaining verification |
| --- | --- | --- |
| AC-15 | not_run | Local exact-file, executable-permission, architecture, version, and same-run workflow checks passed; four hosted public downloads and all native execution results remain pending |
| AC-16 | not_run | Local missing/corrupt artifact, packaging, upload failure, pagination, conflicting bytes, concurrent duplicate, partial recovery, and complete-rerun preservation cases passed; hosted interrupted upload and rerun remain pending |
| AC-17 | not_run | README badge destination preserved; hosted rendering and four public binary downloads remain pending |

No GitHub credential or GitHub CLI is available. No hosted run, release, tag,
or asset was changed. Record hosted run and release URLs, tag SHA, asset IDs,
archive listings, native `--version` results, partial recovery, and complete
rerun preservation before marking hosted acceptance passed. The latest
[Feature 0010 audit](feature-0010-validation.md#bug-0006-local-binary-release-audit-2026-11-21)
preserves the outstanding native and real guest checks.

Primary interface references checked for this correction:

- [GitHub release asset API](https://docs.github.com/en/rest/releases/assets?apiVersion=2022-11-28): raw binary uploads, paginated listing, octet-stream downloads, and duplicate-name failures.
- [download-artifact v8.0.2 metadata](https://raw.githubusercontent.com/actions/download-artifact/v8.0.2/action.yml): same-run defaults and merged artifact extraction.
- [upload-artifact v7.0.2 metadata](https://raw.githubusercontent.com/actions/upload-artifact/v7.0.2/action.yml): ZIP artifact upload and overwrite behavior for workflow reruns.
