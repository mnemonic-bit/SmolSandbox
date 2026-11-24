# Bug 0005: Release verification fails after successful publication

## Status and impact

Resolved

Marked resolved on 2026-11-23 at the user's request. See the
[resolution decision](#resolution-decision-2026-11-23). Previous validation
results and pending checks remain recorded separately.

### Original report

Report prepared on 2026-11-21 for `smolsandbox 0.11.0` and Feature
0017. The main push at `b025321647c5c0bc2719bf12fbc3650a0156c6b4`
published `v0.11.0`, but the release job and overall workflow report failure.
Eligibility and all four native jobs passed.

The verification defect is reproduced locally: a successful release creation
followed by one release-list response that omits the new release produces
the exact reported error. Delayed API visibility is the leading explanation
for the hosted failure. The original HTTP response bodies and headers were
not captured, so server replication, caching, and other causes of the
temporary omission cannot be distinguished.

This report applies no software fix. The package version remains `0.11.0`.
The eventual software fix requires a patch bump in both version files under
[AGENTS.md](../AGENTS.md), selecting `0.11.1` if the baseline is unchanged.

## Reported failure

The user supplied this publication output:

```text
Candidate version: 0.11.0
Selected tag: v0.11.0
Release commit: b025321647c5c0bc2719bf12fbc3650a0156c6b4
Tag URL: https://github.com/mnemonic-bit/SmolSandbox/tree/v0.11.0
Tag: v0.11.0 resolves to b025321647c5c0bc2719bf12fbc3650a0156c6b4
Tag: v0.11.0 resolves to b025321647c5c0bc2719bf12fbc3650a0156c6b4
Tag: v0.11.0 resolves to b025321647c5c0bc2719bf12fbc3650a0156c6b4
Release check failed: Publication of v0.11.0 cannot be verified; rerun the original workflow
Error: Process completed with exit code 1.
```

Expected: publication succeeds once the matching normal release can be
verified, including when a subsequent read briefly omits the release.
Actual: the first missing post-write list entry ends the job immediately,
even though publication has succeeded.

## Hosted evidence

Read-only GitHub REST API inspection on 2026-11-21 identified
[run 44896711807, attempt 1](https://github.com/mnemonic-bit/SmolSandbox/actions/runs/44896711807).
The event is `push`, the branch is `main`, and `head_sha` is the release
commit above. The run was created at `10:56:45Z` and concluded `failure`.

| Job | Job ID | API conclusion |
| --- | --- | --- |
| Release version eligibility | `136823823570` | `success` |
| Linux x86-64 | `136823848397` | `success` |
| Linux ARM64 | `136823848509` | `success` |
| macOS ARM64, `macos-15` | `136823848430` | `success` |
| Windows x86-64 | `136823848431` | `success` |
| release | `136826963386` | `failure` |

The [jobs API](https://api.github.com/repos/mnemonic-bit/SmolSandbox/actions/runs/44896711807/jobs?per_page=100)
reports success for formatting, both Clippy configurations, both Rust test
configurations, production release compilation, baseline independence, and
artifact upload on each native target. The Python release checks passed on
Linux x86-64; that step was intentionally skipped on the other targets.
Artifact archive contents and real SmolVM guest behavior were not inspected.

The failed [publication step](https://github.com/mnemonic-bit/SmolSandbox/actions/runs/44896711807/job/136826963386)
ran from `11:09:49Z` through `11:09:52Z`. The
[release API](https://api.github.com/repos/mnemonic-bit/SmolSandbox/releases/tags/v0.11.0)
returns release ID `453632353`, name and tag `v0.11.0`, `draft: false`,
`prerelease: false`, and `published_at: 2026-11-21T11:09:52Z`.
The [published release](https://github.com/mnemonic-bit/SmolSandbox/releases/tag/v0.11.0)
therefore exists despite the failed step. The
[tag API](https://api.github.com/repos/mnemonic-bit/SmolSandbox/git/ref/tags/v0.11.0)
resolves the tag directly to the expected commit. A later release-list read
also returns that release.

The release script and workflow fetched at the affected commit match the
workspace files at `a90c901651b6bf16bbfe74548295d824f9a2895e` byte for byte.
The release script SHA-256 is
`95106dfe46fdb17227d9762d73eaf9c70bf625dffc8b6836bf11d395600ad217`.
An unauthenticated job-log download returned HTTP 403. The supplied error
excerpt, job metadata, source, and later release state support this report;
the authenticated publication responses remain unavailable.

## Cause and diagnostic limits

In the affected [release script](https://github.com/mnemonic-bit/SmolSandbox/blob/b025321647c5c0bc2719bf12fbc3650a0156c6b4/.github/scripts/release.py#L148),
`run` calls `publish`, which uses `checked_state` and `release_state` to
resolve the tag and find the release through the paginated releases list.

1. `publish` discards the successful `POST /releases` response at line 198.
   The draft-publication `PATCH` response is also discarded at line 195.
2. `publish` immediately calls `checked_state` at line 206. That check
   performs another tag lookup and a release-list lookup.
3. If the list does not contain the selected tag, lines 207–208 raise the
   exact reported error. No second verification read occurs.

The four-iteration loop at line 162 handles selected concurrent-state
transitions. The missing-release exception exits that loop on the first
post-write read; the loop does not provide retries for delayed visibility.

[GitHub's Releases API documentation](https://docs.github.com/en/rest/releases/releases#create-a-release)
provides a release object in the successful creation response and supports
reading a release by ID or tag. The script ignores the returned identity
and relies solely on an immediate collection read. The documentation is not
evidence of a guaranteed propagation delay; that trigger remains an
inference from the observed state and the deterministic local replay.

Three hypotheses were considered: delayed release-list visibility, a
permission difference between creation and reading, and an unexpected API
response. The local delayed-list replay produces the exact error. Injected
HTTP 403 and 503 responses produce explicit `GitHub API HTTP ...` errors
instead. The workflow grants the release job `contents: write`, and the
published release is publicly readable. These observations favor delayed
visibility over a missing publication permission. The exact error requires
a successful list-shaped response without a matching release; a non-list
response has a separate validation error in `release_state`.

## Deterministic reproduction

Run this command from the repository root on the affected source. The
command uses the existing fake API and real `publish` implementation. It
creates a normal release, hides that release from exactly one post-write
list response, and exits with the application's failure status. No GitHub
requests or repository changes occur.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 - <<'PY'
import sys
from unittest import mock
sys.path.insert(0, '.github/scripts')
import test_release as t

api = t.FakeAPI()
api.repository = 'mnemonic-bit/SmolSandbox'
sha = 'b025321647c5c0bc2719bf12fbc3650a0156c6b4'
api.files[sha] = api.files[t.SHA]
api.tags[t.TAG] = {'type': 'commit', 'sha': sha}
original = api.request
hide_next = False

def request(method, path, data=None, allow_missing=False):
    global hide_next
    if method == 'POST' and path == '/releases':
        result = original(method, path, data, allow_missing)
        hide_next = True
        return result
    if method == 'GET' and path.startswith('/releases?') and hide_next:
        hide_next = False
        api.calls.append((method, path, data))
        return []
    return original(method, path, data, allow_missing)

api.request = request
with mock.patch.object(t.release, 'GitHubAPI', return_value=api), mock.patch.dict(
    t.os.environ,
    {'RELEASE_MODE': 'publish', 'RELEASE_SHA': sha, 'GITHUB_STEP_SUMMARY': ''},
):
    code = t.release.main()
print('Release objects stored:', len(api.releases))
print('Exit code:', code)
sys.exit(code)
PY
```

Observed result: `failed`, exit 1, with the exact supplied publication
messages and `Release objects stored: 1`. A separate controlled probe
repeated the omission three times with the same verdict. Removing only the
omission produced `published`; rerunning after the omission produced
`already-published` with zero additional writes.

The existing fake API makes successful release writes immediately visible
to subsequent list reads. The existing race and rerun tests exercise other
recovery paths, so those tests pass without catching this omission.
`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s .github/scripts -p 'test_*.py' -q`:
`passed`, exit 0, 40 tests. Local Rust checks: `not_run` for this report
because no software changed. Hosted recovery: `not_run`.

## Proposed correction and acceptance

The implementer should retain and validate successful publication response
metadata, then verify the matching release using a direct read and bounded
retries when visibility is delayed. Define a finite attempt or elapsed-time
limit and avoid another publication write while verifying a successful
write. Continue checking the resolved tag against the immutable release
commit. Do not translate authorization failures or conflicting state into
success.

Keep the inline script in `.github/workflows/rust.yml` synchronized with
`.github/scripts/release.py`; the existing equality test requires both
copies to match. Extend the fake API to model delayed reads and record the
new request sequence. The implementer owns these corrections and checks:

- AC-01: A successful create followed by temporary list omission succeeds
  within the documented verification limit, with one release creation and
  the expected tag SHA. Verify through a deterministic fake API test.
- AC-02: Draft publication followed by delayed visibility succeeds without
  duplicate creation or loss of existing notes. Verify through a fake API
  test that separates stored state from visible responses.
- AC-03: A release that remains unverifiable beyond the limit fails with
  explicit diagnostics. Tag conflicts, HTTP authorization failures, rate
  limits, and invalid metadata retain failure behavior. Verify through
  focused tests and the complete release test suite.
- AC-04: A rerun after delayed verification preserves the published release
  and tag and performs zero writes. Verify IDs, notes, and request records.
- AC-05: Both version files receive the required patch bump, both script
  copies match, and the release suite and all seven Rust commands in
  `AGENTS.md` pass. Record the software revision and results.
- AC-06: A disposable hosted publication and same-commit rerun both conclude
  `success`, with one matching normal release and unchanged rerun state.
  Record run URLs, release ID, and resolved tag SHA.

All eventual-fix criteria are `not_run`; this report does not implement the
correction or claim recovery validation.

## Recovery for the existing release

The repository owner may rerun the failed jobs of original run
`44896711807` for commit `b025321647c5c0bc2719bf12fbc3650a0156c6b4`.
Once the existing matching release is visible, the current script should
return `already-published` without writes, as the local replay demonstrates.
Hosted recovery remains unverified. A manual `workflow_dispatch` performs
validation only and cannot publish or recover a release.

Preserve the existing `v0.11.0` tag and release. A new merge that retains
`0.11.0` has a different commit identity and will conflict with the existing
tag. Use the original run for recovery, following
[RUST-APPLICATION.md](../RUST-APPLICATION.md#recover-a-release-attempt).

## Implementation and local validation: 2026-11-21

The software fix is implemented locally on source revision
`a90c901651b6bf16bbfe74548295d824f9a2895e` plus the changes described here.
Both version files select `0.11.1`. Hosted acceptance remains pending; the
earlier failure observations and reproduction describe the original source.

`publish` now retains successful create and draft-publication responses.
`publication_metadata` requires a positive integer release ID, the selected
tag, and explicit Boolean draft and prerelease fields. A successful write
must return a normal release; a draft-publication response must retain the
draft's ID. `verify_publication` reads `/releases/{id}` directly and resolves
the selected tag against the immutable release commit on every attempt.
The [GitHub Releases API](https://docs.github.com/en/rest/releases/releases#get-a-release)
documents this direct release-ID lookup.

Verification makes five attempts with one second between attempts, for at
most four sleeps. The existing HTTP timeout remains 60 seconds per request;
the attempt limit is not a five-second elapsed-time deadline. A temporarily
missing release, stale draft, or missing tag consumes an attempt. A different
tag SHA, invalid metadata, published prerelease, or HTTP error fails without
being accepted as success. After a successful publication write, verification
performs only reads. Exhaustion reports the release ID and attempt count and
directs the owner to rerun the original workflow.

The workflow's inline script matches the canonical script. The fake API now
models stored publication state separately from delayed list and direct-read
responses. Tests record request sequences and assert direct lookup, exact
attempt counts, no further publication writes, and unchanged rerun state.
The original report reproduction failed before the fix with exit 1 and one
stored release; the identical reproduction passed after the fix with exit 0
and one stored release. The focused regression test was also observed failing
before the production edit. Implementation and test agents used separate file
ownership; an independent review found no actionable correctness issues.

The tested software files have these SHA-256 values:

| File | SHA-256 |
| --- | --- |
| `.github/scripts/release.py` | `ef12e116aa4e650533e8fe9fa288722f0eec66cd0ce7da1dffb6bc6f3cbcc17a` |
| `.github/scripts/test_release.py` | `87fa2fa75b13fa26d93607f0ea8fce2f816e6d2657c9720f10c88d4d9a6690de` |
| `.github/workflows/rust.yml` | `05def2309a47c543e4f505ad2609ee777ceb07e0b2f1af08953515f7add84530` |
| `Cargo.toml` | `6ab99e23e1258210d6466fa90ba2b0846ab4abcbe6d77ffddeac4459651044e5` |
| `Cargo.lock` | `13dcbb2b75530acce74ed10baced459f004d80c5571fbe8acb74d5a9bbb1f93c` |

Local validation uses native `x86_64-unknown-linux-gnu`, Rust/Cargo `1.100.0`.

| Check | Result | Evidence |
| --- | --- | --- |
| Release suite | passed | `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s .github/scripts -p 'test_*.py' -q` exited 0; 52 tests, including inline equality |
| Recorded fake API sequences | passed | `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/test_release.py --record-api-traces /tmp/bug0005-rust-checks-89tmg5o6/release-api-traces.json` exited 0; 52 tests |
| `cargo fmt --all -- --check` | passed | Exit 0 |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | Exit 0 |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | Exit 0 |
| `cargo test --locked --all-targets` | passed | Exit 0; 118 tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | Exit 0; 185 tests |
| `cargo build --locked --release --bin smolsbx` | passed | Exit 0; production binary reports `smolsbx 0.11.1` |
| `cargo run --locked --example check_baseline_independence` | passed | Exit 0; isolated Rust checks and nine compiled test executables per configuration passed with empty `PATH` |
| `git diff --check` | passed | Exit 0 |
| Disposable hosted publication and same-commit rerun | not_run | No configured GitHub credential or disposable repository is available; no hosted run URL, release ID, or resolved tag SHA exists for the fix |
| Other native targets and real SmolVM guests | not_run | No new hosted or real guest execution was performed |

AC-01 through AC-05 passed locally. AC-06 is `not_run`. The existing `v0.11.0` tag
and release were preserved; the original failed workflow was not rerun.
Local logs and API traces under `/tmp/bug0005-rust-checks-89tmg5o6/` are
temporary session evidence, not durable hosted artifacts. The
[Feature 0010 validation record](../FEATURES/feature-0010-validation.md)
retains the historical observations and records this implementation audit.

## Resolution decision: 2026-11-23

Marked `Resolved` at the user's request. This decision supersedes earlier
open or in-progress status statements and instructions to keep resolution
pending acceptance checks. The software fix and local validation remain
recorded above. Previously recorded hosted publication and rerun checks
remain pending; no new checks were run for this status update.
