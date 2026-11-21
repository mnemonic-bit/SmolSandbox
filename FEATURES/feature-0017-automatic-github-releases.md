# Feature 0017: Publish GitHub releases after merges into main

## Status

In progress

Release automation is implemented locally. Hosted acceptance checks remain
pending; see the [validation record](feature-0017-validation.md).

This document specifies release automation. No workflow, package version,
GitHub setting, tag, or release is changed during specification preparation.

## Outcome and baseline

After the repository owner merges a release pull request into `main`, GitHub
Actions validates the resulting commit, creates a version tag at that exact
commit, and publishes a GitHub Release with generated release notes and four
executable-only binary ZIPs. Failed validation prevents publication. Rerunning
the workflow does not move a tag or duplicate a release or verified asset.

Build and test jobs run only for `main` and `develop`. The README displays
the latest published release version and the current development version
as separate badges.

Before a pull request can merge into `main`, a required read-only Actions
check verifies that the proposed release version is valid and unused.

Specification preparation observed commit
`af4496c8e4b797613a3ca621e5f5029663e4a2e1`, branch `main`, and package version
`0.10.0` on 2026-11-21. The working tree was clean before this document was
added. `.github/workflows/rust.yml` already runs on `push`, `pull_request`,
and `workflow_dispatch`. Its `native` matrix validates Linux x86-64,
Linux ARM64, macOS ARM64, and Windows x86-64, builds production binaries
without `test-adapter`, and uploads artifacts. Workflow permissions are
currently `contents: read`.

The owner reported creating `main`, selecting `main` as the default branch,
and configuring a pull-request requirement targeting `main` and `develop`.
The owner is the only intended merger at either stage. These statements are
configuration inputs reported by the owner, not independently verified
GitHub settings. Distinct merger groups remain outside this feature.

The codebase-memory project list was empty during preparation. Baseline
observations above come from direct reads of the named workflow and manifest
and from Git commands; no indexed architecture claim is made.

## Scope and ownership

The implementer owns the workflow changes, release logic, regression checks,
operating documentation, and implementation evidence. The repository owner
owns GitHub ruleset configuration, repository access, and merging release
pull requests.

Required output is a version tag and a published GitHub Release with generated
release notes and one binary ZIP for each supported native target. The
[Bug 0006 requirement](../BUGS/bug-0006-release-binary-zip-assets.md)
supersedes the original specification's optional binary attachments. Retain
the native build artifacts and production build configuration; Actions
artifacts alone do not satisfy the release-download requirement.

The required scope also includes restricting build triggers to `main` and
`develop` and adding release and pre-release version badges to `README.md`.
The pre-release badge is a display of the development version, not a Git
tag or a published GitHub prerelease.

Pull requests targeting `main` also run a required metadata check named
`Release version eligibility`. This check parses version files and reads
GitHub release state without building or testing application code.

Do not add automatic version commits, package-registry publication, a new
release branch, or changes to application behavior. Do not modify branch
protections or repository access through the release workflow. Do not use
`pull_request_target` or give pull-request jobs release credentials.

## Required behavior

### Trigger and validation

Restrict automatic build and test runs to pushes to the exact branch names
`main` and `develop`. Remove the `pull_request` build trigger. Feature
branches must not build on push or through a pull request, including a
pull request targeting `main` or `develop`. Filtering a pull-request trigger
by its destination branch alone does not satisfy this requirement.

Retain `workflow_dispatch` for validation, but guard build and test jobs so
manual runs build only `refs/heads/main` or `refs/heads/develop`. A manual
dispatch for another branch or a tag must skip build and test jobs. Tag
pushes must not start build and test jobs. Apply these restrictions to every
build and test job added by this feature, including helper workflows.

Retain a separate `pull_request` trigger targeting `main` for the read-only
`Release version eligibility` check described below. Guard native jobs by
event type as well as branch so this metadata trigger cannot start a build.
Pull requests targeting `develop` do not require release-version uniqueness.

Extend `.github/workflows/rust.yml` with a release job that depends on the
successful completion of the early post-merge eligibility check and the
entire `native` matrix in the same run.
Authorize publication only when `github.event_name` is `push` and
`github.ref` is `refs/heads/main`. A merge into protected `main` produces
that event. Direct pushes are prevented by GitHub branch rules rather than
by interpreting commit messages or requiring a particular merge method.

Pushes to `develop` or feature branches, open or closed pull-request events,
tag pushes, and manual dispatches must not publish releases. A push to
`develop` runs validation without creating any release state. The first
eligible merge containing the implementation may publish a release;
existing `main` history is not released retroactively.

Use `github.sha` from the eligible push as the release commit. Read version
files from that commit. Do not resolve the current tip of `main` at publication
time, because a later merge may have advanced the branch. A later merge must
not cancel an earlier release run or cause the earlier run to tag the later
commit. Publish nothing if any required validation job fails, is cancelled,
or is skipped.

### Version selection

Read `[package].version` from `Cargo.toml` with a TOML parser. Check that the
matching root `smolsandbox` package entry in `Cargo.lock` has the same version.
Use the repository's `MAJOR.MINOR.PATCH` format, with three nonnegative
decimal components and no leading zeros except the component `0`. Reject
missing, mismatched, or malformed versions before creating a tag or release.

The release tag and title are both `v<version>`. For example, manifest version
`0.11.0` selects `v0.11.0`. Use the committed version without incrementing,
inferring, or rewriting the version in the release job. Do not infer versions
from pull-request titles, labels, commit messages, or timestamps.

Every new release commit needs an unused release version. Documentation and
feature specifications may accumulate on `develop` for inclusion in the next
software release. A merge into `main` that retains a version already tagged
at another commit fails publication; the workflow does not silently invent
another version. Reruns for the same version and commit follow the recovery
rules below.

Building a version on `develop` does not reserve that version or create its
release tag. Merging `develop` into `main` preserves the committed version;
the merge itself does not require another version bump. For example, a
successful `develop` build of `0.10.1` followed by a merge into `main` at
`0.10.1` can create the first `v0.10.1` release. The development commit and
merge commit may have different SHAs without causing a conflict, because
the development build created no release tag.

The conflicting case is an existing `v0.10.1` tag that resolves to a commit
other than the new release commit. A later merge that still uses `0.10.1`
cannot publish a second release under that tag. Compilation and tests may
otherwise pass; eligibility failure prevents unnecessary main builds and
release publication. A conflict discovered only at final publication still
fails the release job and makes the overall workflow report failure.
A rerun of the original release commit succeeds without duplicating its
existing release. The pre-release README badge never reserves a tag.

### Early release version eligibility check

Implement the Actions check `Release version eligibility` with a stable,
unique job name so the owner can select that exact check in a main-only
required-status-check rule. Show the candidate version, selected tag, and
any conflicting tag or release URL in the check summary. Report API errors
explicitly; do not translate an unverifiable version into success.

Start the check immediately when a pull request targeting `main` is opened,
reopened, updated with new commits, edited, or marked ready for review.
Run on draft pull requests too. Do not wait for approval, a native build,
tests, dependency installation, or another matrix job. Do not use path
filters or version-file-change conditions: a PR with no version change may
be precisely the PR that attempts to reuse a released version.

Read `Cargo.toml` and `Cargo.lock` from the pull request's proposed merge
commit, using that event's immutable commit SHA rather than a moving branch
tip. Parse only the version metadata using runner-provided tools. Use
`contents: read` and perform no repository writes. Do not compile code,
invoke Cargo or build scripts, execute scripts from the PR, install project
dependencies, or use `pull_request_target`. If a merge conflict prevents
reading the proposed merge commit, do not report eligibility as passed;
the PR must first resolve the conflict.

For a new PR into `main`, apply the version syntax and manifest/lockfile
agreement rules above. Query both the selected tag and its GitHub Release,
including draft releases. Fail if either already exists. A PR is proposing
a new release; the synthetic PR merge SHA is not the eventual release SHA,
so matching an existing tag to that synthetic SHA is not a rerun exemption.
An unused version passes even if that version has already built on `develop`.

After a merge into `main`, repeat eligibility against the actual push SHA
before starting the native build matrix. Make the main native jobs depend
on that successful check. In this post-merge context, an existing tag at
the same actual commit is permitted for reruns and partial-release recovery;
apply the existing-state table below. Preserve independent native validation
for pushes and manual runs on `develop` and manual runs on `main`, which do
not publish and must not depend on a skipped publication-only check.

Recheck tag and release identity immediately before any publication writes.
A successful PR check does not reserve a version: another release or manual
tag can appear after the check passes. Detect those changes at the early
post-merge check or final publication check without moving an existing tag.
Allow rerunning the PR check to refresh GitHub state without changing source
files. Document that an existing successful PR check is an observation at
check time, not a guarantee against later external changes.

### README version badges

Replace the existing generic version badge in `README.md` with two clearly
labelled version badges:

| Badge label | Version source | Link destination |
| --- | --- | --- |
| Release | Latest published normal GitHub Release created from `main`, excluding drafts and prereleases | Repository's latest release page |
| Pre-release | `[package].version` from `develop/Cargo.toml`, displayed with a `v` prefix | `Cargo.toml` on `develop` |

Use dynamic badge sources so neither the workflow nor the owner needs to
commit a new README version for each release or development change. Both
badges must select their declared sources regardless of which branch's
README is being viewed. Badge-service caching may delay updates; document
that the linked release or manifest is the authoritative version source.

Use `mnemonic-bit/SmolSandbox` in badge sources and destinations. Correct the
existing README badge links that still reference `mnemonic-bit/AI-Sandbox`,
including the retained development build-status badge. Do not present the
unreleased `main` manifest version as a successfully published release when
no release exists or publication has failed.

The pre-release badge does not create a Git tag, publish a GitHub prerelease,
or append a prerelease suffix to the package version. The release and
pre-release badges may show the same version immediately after a release.
The pre-release badge follows the current committed development version even
if that commit's validation is pending or failed; build status is separate.

### Tag and release creation

Before creating any release state, look up the selected tag and release.
Treat authentication errors, authorization errors, rate limits, and other
API failures as errors rather than evidence that a resource is absent.

Create a missing tag at the exact release commit. If a tag already exists,
resolve the tag to its commit, including annotated tags. Fail if the resolved
commit differs from the release commit. Never move, force-update, or delete
an existing tag.

Create the GitHub Release for the selected tag with title `v<version>` and
GitHub-generated release notes. Publish a normal release rather than a
prerelease. Allow GitHub's semantic-version rules to determine the latest
release; an older workflow finishing later must not explicitly mark its
older version as latest. Retain existing releases and release notes.

Use GitHub CLI or the GitHub API in the same workflow for publication.
Specify the release commit explicitly when creating a missing tag. Do not
depend on a separate tag-triggered or release-triggered workflow to finish
publication, because events created with `GITHUB_TOKEN` do not normally start
those workflows.

### Binary release assets

Package the successful native production build for each target from the exact
immutable release commit. Exclude `test-adapter` from distributed builds.
Download all four packaged artifacts from the publication workflow's own run;
do not select artifacts from another run or a moving branch.

For version `<version>`, upload exactly these four release assets:

| Target | Release asset name | Sole ZIP entry |
| --- | --- | --- |
| Linux x86-64 | `smolsbx-v<version>-x86_64-unknown-linux-gnu.zip` | `smolsbx` |
| Linux ARM64 | `smolsbx-v<version>-aarch64-unknown-linux-gnu.zip` | `smolsbx` |
| macOS ARM64 | `smolsbx-v<version>-aarch64-apple-darwin.zip` | `smolsbx` |
| Windows x86-64 | `smolsbx-v<version>-x86_64-pc-windows-msvc.zip` | `smolsbx.exe` |

Each ZIP contains exactly one regular file at its root. Exclude directories,
source files, separate configuration templates, documentation, checksums,
debug symbols, other build outputs, and nested artifact ZIPs. Preserve Unix
executable permissions in the Linux and macOS archives and during extraction.
Inspect the target architecture. Run the extracted executable's `--version`
on its native operating system before delivering the archive.

Validate all four local ZIPs before release writes. If an artifact, packaging
step, or upload is missing or fails, fail the release job with a diagnostic
identifying the affected target. Verify the uploaded asset bytes before
reporting a complete binary release. Tag and release metadata alone do not
establish completion. GitHub's automatic source ZIP and tarball links remain
separate from the four uploaded assets.

### Reruns, partial failures, and concurrent runs

Use the tag and its resolved commit as the identity of a release attempt.
Apply these outcomes before any attempted write:

| Existing state | Required outcome |
| --- | --- |
| No tag and no release | Create the tag and release for the release commit. |
| Tag at the release commit, no release | Create the missing release without changing the tag. |
| Tag at the release commit, matching draft release | Resume publication of that draft without creating a second release. |
| Tag at the release commit, matching published release | Verify all four binary ZIPs and recover missing assets while preserving the release and its notes. Succeed without writes if every asset already matches. |
| Tag resolves to another commit | Fail without changing the tag or release. |
| Release exists but its expected tag cannot be verified | Fail without changing the existing release. |

If a draft is used, generate release notes before publishing the draft. Keep
existing draft notes on recovery. A failure after tag creation leaves the
tag available for a rerun; do not delete the tag as rollback. A failure after
release creation is recovered by inspecting the server state on rerun.

After an interrupted asset upload, recover only missing ZIPs on the matching
release. Compare existing asset bytes with the expected ZIPs before accepting
those assets. Preserve matching asset IDs and bytes, release notes, and tag
SHA. Fail explicitly on conflicting asset contents or an unexpected uploaded
asset; do not silently replace or delete assets. Handle a concurrent upload
by verifying the resulting asset rather than accepting a duplicate-name error
as success.

Concurrent attempts for the same version must converge on one tag and one
release. Handle an API response that a resource already exists by verifying
the resulting tag and release, rather than treating every creation error as
success. Attempts for different versions must retain their own commit
identities. Avoid concurrency settings that discard pending release runs.

### Permissions and repository configuration

Retain `contents: read` as the workflow default. Grant `contents: write`
only to the release job, and authenticate with the automatically supplied
`GITHUB_TOKEN`. Do not add a personal access token or grant the workflow
permission to write commits to protected branches. Branch rules remain
enforced independently of token permissions.

Document the following GitHub configuration prerequisites for the owner:

1. Keep the pull-request requirement active for exact branch names `main`
   and `develop`.
2. Keep repository write access limited to the owner while the owner is the
   only intended merger. A pull-request requirement alone does not select
   which users may merge.
3. Permit the release identity to create tags matching `v*`. Rules that
   restrict tag updates or deletions may remain active without a bypass.
4. If tag creation is restricted, resolve that restriction before enabling
   automated publication. `contents: write` does not override a ruleset.
5. Review required status checks before removing pull-request builds. Do not
   require a per-PR native build check that this feature no longer runs;
   such a requirement can leave feature pull requests unmergeable. The owner
   configures checks consistent with validation after merges into `develop`
   and `main`. Keep the pull-request requirement active, and do not change
   GitHub rules automatically to make a merge pass.
6. After the metadata workflow has reported its first check, require
   `Release version eligibility` in a ruleset targeting only `main`. Select
   GitHub Actions as the expected check source. Keep bypass permissions
   empty so the owner must obtain a successful check before merging. Do not
   add this requirement to the shared `main`/`develop` ruleset, because
   feature PRs into `develop` do not run this release eligibility check.

The required implementation uses `GITHUB_TOKEN` with tag creation permitted.
A dedicated GitHub App for a separately restricted tag-creation policy is a
future extension, not a required credential setup for this feature. Report
ruleset rejection as a failed release attempt without weakening rules.

### Application version and documentation

Adding release automation changes build configuration and requires a minor
version bump under [AGENTS.md](../AGENTS.md). From the observed `0.10.0`
baseline, implementation selects `0.11.0` in `Cargo.toml` and the matching
`smolsandbox` entry in `Cargo.lock`. Derive the bump from the actual baseline
if another software change lands first. This specification alone does not
change either version file.

Document the release procedure and rerun procedure in
[RUST-APPLICATION.md](../RUST-APPLICATION.md). Explain selecting a new
version before merging, waiting for validation, finding the tag and release,
and rerunning the original workflow after a transient failure. Explain
resolving version conflicts through another reviewed change rather than
moving a published tag. Explain selecting the platform ZIP from the release
page, its executable-only contents, and recovery after partial asset uploads.
The Bug 0006 correction requires a patch bump from its actual software
baseline: `0.11.1` selects `0.11.2` in both version files.

Explain that feature branches receive no CI builds, including before their
pull requests merge. The early read-only eligibility check runs on PRs into
`main`; native validation runs after a merge reaches `develop`, and again
after a merge reaches `main`. Release publication depends on successful
main eligibility and native validation. Document selecting the named check
as required only for `main`, inspecting check output, and rerunning the check
after a transient API failure or stale result. Document manual validation
of only `main` and `develop` and the README badge meanings, source links, and
caching behavior.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | A push to `main` publishes only after every native matrix job succeeds. Failed, cancelled, or skipped validation publishes nothing. | Inspect the job dependency and conditions. Exercise successful and unsuccessful validation runs in a disposable GitHub repository. |
| AC-02 | Pushes to other branches, pull-request events, tag events, and manual dispatches cannot publish. Only the eligible release job receives contents write permission. | Check the event conditions and permission scopes. Record representative nonpublication cases and permission evidence. |
| AC-03 | The manifest and root lockfile package determine one matching version and the exact `v<version>` tag. Invalid or mismatched versions produce no release writes. Building a version on `develop` and promoting that unchanged version to `main` can publish its first release. | Exercise valid, missing, malformed, and mismatched version fixtures through the release logic with a recorded fake API. Verify that a development build creates no tag and that the subsequent main build publishes the unchanged version at the main commit. |
| AC-04 | A release tag resolves to the validated push commit even if `main` advances before publication. No run is discarded because another release version starts. | Exercise two distinct versions with delayed publication and record both final tag SHAs. Review concurrency settings. |
| AC-05 | A successful new attempt produces one normal published release with the expected title and generated notes. | Record an eligible PR merge, workflow URL, tag SHA, release URL, title, notes, and prerelease flag in a disposable GitHub repository. |
| AC-06 | Reruns preserve matching published releases, resume missing or draft releases, and never move conflicting tags. | Exercise every existing-state table row with recorded API calls. Rerun a successful hosted release and compare tag SHA and release contents. |
| AC-07 | API failures and ruleset rejection fail publication without deleting state or broadening access. Duplicate creation races converge only after identity verification. | Simulate authorization failure, rate limiting, partial creation, matching concurrent creation, and conflicting concurrent creation. Record calls and resulting state. |
| AC-08 | Only the release job receives contents write permission. The workflow uses `GITHUB_TOKEN`, writes no branch commits, and preserves existing native validation commands, targets, and production build configuration under the restricted triggers. | Review the workflow and release API calls. Inspect token permission output and the implementation diff. |
| AC-09 | Manifest and lockfile contain the required feature version, and the operating reference explains release prerequisites, version conflicts, and recovery. | Inspect the version entries and documentation. Run the required repository checks. |
| AC-10 | Pushes to `main` and `develop` run all four native jobs. Feature pushes and all pull-request events run no build or test jobs. Manual runs build only `main` or `develop`; tags never build. | Inspect every build trigger and job guard. Record hosted push and manual runs for both permitted branches, a feature branch, and a tag. Exercise a feature PR targeting `develop` and a release PR targeting `main`; verify that neither PR event starts a build. Review owner-managed required checks for dependencies on removed PR builds. |
| AC-11 | README badges show the latest published release and the current `develop` package version using the correct repository URLs. The development badge reserves no tag, and both badges may display the same version. | Inspect badge definitions and destinations. Render the README, compare values against the linked release and manifest after cache expiry, and exercise no-release, equal-version, and newer-development-version cases without creating a development release tag. |
| AC-12 | PRs into `main` report `Release version eligibility` immediately on the specified events, including draft PRs and PRs without version-file changes. The check reads the proposed merge metadata and rejects invalid, mismatched, tagged, or already released versions without building application code. PRs into `develop` do not require version uniqueness. | Record valid and conflicting hosted PR checks, summaries, event types, and permissions. Exercise version metadata and API failure fixtures. Inspect calls to establish no build, dependency installation, repository scripts, or writes occurred. |
| AC-13 | A failed eligibility check blocks the main native matrix. An unused version already built on `develop` passes; an existing tag at the actual main commit permits recovery. A later conflicting tag is caught before publication. | Inspect the job dependency graph. Exercise main failures, development/manual validation, same-commit reruns, and a tag appearing between PR eligibility and publication. Record that no main native job starts after failed early eligibility. |
| AC-14 | The owner can explicitly require the named GitHub Actions check for `main`, and a failing check prevents merging while a passing check permits merging subject to other rules. The eligibility check is not required for `develop`. | In a disposable GitHub repository, configure the main-only required check without bypass, attempt merges for failing and passing versions, and record the rule and merge outcomes. Verify a feature PR into `develop` is not waiting for the main-only check. |
| AC-15 | Every successful eligible main release has exactly the four named binary ZIP assets, each containing only its root executable with Unix executable permissions preserved where applicable. | Download each asset through its public release URL. Inspect archive entries, extract the executable, verify its architecture, and run `--version` on the native target. Record the shared workflow run, immutable release commit, tag SHA, and asset IDs. |
| AC-16 | Missing artifacts, packaging failures, failed uploads, and conflicting asset contents fail with a target-specific diagnostic. Partial uploads recover without replacing verified assets, changing notes, or moving the tag. | Exercise each failure with focused regressions. Interrupt a hosted upload, rerun the original workflow, and compare final asset IDs and bytes. Rerun a complete release and verify zero duplicate uploads. |
| AC-17 | The README Release badge retains its latest-release destination, where all four binary ZIPs can be downloaded. | Render the README and follow the Release badge after hosted publication. Download each binary ZIP without Actions-artifact access; automatic source archives do not satisfy this criterion. |

## Verification and evidence

During implementation, create `FEATURES/feature-0017-validation.md`. Record
the implementation revision, package version, commands, exit codes, workflow
and release URLs, event types, expected and observed tag SHAs, permission
evidence, and AC-01 through AC-17 results. Use `passed`, `failed`, `not_run`,
or `inconclusive`. Preserve secrets when recording command output.

Use local fixtures and a fake API for failure and recovery checks. Use a
disposable GitHub repository for hosted publication checks; do not create
test releases or deliberately failing release merges in the production
repository. If hosted checks cannot run, record those criteria as `not_run`.
Local checks alone do not establish working GitHub permissions or rulesets.

Run all seven independent Rust checks listed in [AGENTS.md](../AGENTS.md)
after implementation, including both feature configurations, the production
release build, and baseline independence. Run workflow syntax validation
with the repository's selected validator and record the tool and version.
The native release gate does not establish passing real SmolVM guest checks.

Append relevant validation observations and pending native or real guest
checks to [Feature 0010 validation](feature-0010-validation.md), preserving
earlier observations and source revisions. Mark Feature 0017 `Implemented`
only after every mandatory acceptance criterion has passed.

Specification preparation checks: implementation and hosted publication
checks are `not_run`, because this change adds only the proposed feature
specification.

## GitHub references

These official references informed the design on 2026-11-21. Recheck relevant
behavior during implementation if GitHub changes the documented interfaces.

- [Workflow triggers and GITHUB_TOKEN event restrictions](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow).
- [Workflow job dependencies and token permissions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax).
- [GitHub CLI release creation, commit targets, and generated notes](https://cli.github.com/manual/gh_release_create).
- [Release management and draft publication](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository).
- [Available branch and tag rules](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets).
- [Ruleset targeting and bypass permissions](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/creating-rulesets-for-a-repository).
- [Pull-request events and proposed merge commit context](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request).
- [Shields.io GitHub Release badges](https://shields.io/badges/git-hub-release).
- [Shields.io dynamic TOML badges](https://shields.io/badges/dynamic-toml-badge).
