# Bug 0011: Release fails when a newly created tag is temporarily unreadable

## Status and impact

Open. The user reported a publication failure for `smolsandbox 0.16.1` at
`00cbf927ec509c76bb73f8a1197afb3d08c5e7dd`. A single missing tag-reference
read immediately after successful tag creation reproduces the exact error.
The matching tag persists, but that invocation stops before creating a
release or uploading release assets.

Delayed tag visibility is the leading explanation for the hosted failure.
The original HTTP responses were not available, so replication, caching,
and other causes of temporary read visibility remain unverified.

This report applies no software correction. The inspected checkout is
`d7bf50ad4dcdb1390f05b47250ac58c108a7f058`, package `0.16.0`; its release
script, release metadata tests, and workflow are byte-for-byte identical to
those at the failing `0.16.1` commit, as verified through GitHub's contents
API. Both local Cargo versions remain unchanged under
[AGENTS.md](../AGENTS.md). Bug 0010 already exists at the release commit;
this separate failure is Bug 0011.

## Reported failure

The user supplied:

```text
Candidate version: 0.16.1
Selected tag: v0.16.1
Release commit: 00cbf927ec509c76bb73f8a1197afb3d08c5e7dd
Tag URL: https://github.com/mnemonic-bit/SmolSandbox/tree/v0.16.1
Release check failed: Created tag v0.16.1 cannot be verified
Error: Process completed with exit code 1.
```

Expected: after creating a tag, publication tolerates a bounded period of
missing tag reads and proceeds only when the tag resolves to the validated
commit. A conflicting tag still fails without modification.

Actual: the first missing post-creation tag read ends publication. A later
matching read is never attempted by that invocation.

## Hosted evidence

Read-only GitHub REST inspection identified
[run 45459182706, attempt 1](https://github.com/mnemonic-bit/SmolSandbox/actions/runs/45459182706)
for a push to `main` at the reported commit. Eligibility and all five native
jobs have conclusion `success`. The
[release job 138693345769](https://github.com/mnemonic-bit/SmolSandbox/actions/runs/45459182706/job/138693345769)
downloaded the native ZIP artifacts successfully, then failed at
`Publish or recover the release for the validated commit`.

The [tag-reference endpoint](https://api.github.com/repos/mnemonic-bit/SmolSandbox/git/ref/tags/v0.16.1)
returned HTTP 200 with object type `commit` and SHA
`00cbf927ec509c76bb73f8a1197afb3d08c5e7dd`. The
[release-by-tag endpoint](https://api.github.com/repos/mnemonic-bit/SmolSandbox/releases/tags/v0.16.1)
returned HTTP 404 to an anonymous read. This establishes a matching tag and
no publicly readable release at inspection; authenticated draft inventory
was not inspected. The tag URL alone does not establish a completed binary
release.

The API returned run creation time `2026-11-24T18:24:24Z` and update time
`2026-11-24T18:50:54Z`. The session UTC clock reported `2026-11-23`.
These timestamps are retained as returned; the clock discrepancy prevents
using cross-system timestamps to estimate a visibility delay. The release
job log endpoint returned HTTP 403, so the supplied error excerpt remains
the source for the exact hosted diagnostic. No original tag-creation or
tag-read HTTP body was recovered.

## Source analysis

The call path is `run` → `publish` → `checked_state` → `release_state` →
`resolve_tag` in [release.py](../.github/scripts/release.py). At the failing
commit, the script SHA-256 is
`042e7d280f794d3f26cb557129f3acef82e960b390ce2dbf213095d087f52c7d`.

1. `publish` loads the binary assets and inspects the existing tag/release.
2. If no tag is visible, `publish` sends `POST /git/refs`.
3. After successful creation, `publish` immediately calls `checked_state`
   once. That call performs `GET /git/ref/tags/<tag>` with missing reads
   allowed. The creation response is not used as final identity proof.
4. If that read returns no reference, `publish` raises
   `Created tag <tag> cannot be verified` at line 393. The successfully
   created tag is preserved. Release creation and asset upload are not
   reached.

[GitHub's reference documentation](https://docs.github.com/en/rest/git/refs#get-a-reference)
defines HTTP 404 for a reference that does not match an existing ref. The
application converts that allowed missing response to `None`; the
reproduction models this read result without claiming a documented GitHub
consistency guarantee or a verified server-side cause.

The existing five-attempt verification in `verify_publication` starts after
a release creation or draft-publication response. That later retry loop
cannot handle this earlier tag-creation failure. This distinguishes Bug
0011 from [Bug 0005](bug-0005-release-publication-verification-race.md), which
concerns verification after release publication.

Probes also exercised concurrent tag creation. A simulated HTTP 409 or 422
followed immediately by a missing tag read fails with the original API
error, rather than the supplied `Created tag` message. That adjacent path
also lacks bounded visibility recovery. Conflicting tag SHAs and ordinary
permission/API errors have separate failure paths; none is established as
the cause by the supplied diagnostic.

## Deterministic reproduction

Run this command from the repository root. It exercises the real release
decision functions through the existing stateful API fake, without network
writes. Binary loading and transfer are mocked by `ReleaseTests.run_release`;
this reproduction establishes metadata behavior, not ZIP publication.

```sh
python3 - <<'PY'
import sys
sys.path.insert(0, '.github/scripts')
import test_release as tests

tests.SHA = '00cbf927ec509c76bb73f8a1197afb3d08c5e7dd'
tests.TAG = 'v0.16.1'

class DelayedTag(tests.FakeAPI):
    def __init__(self):
        super().__init__()
        self.files[tests.SHA] = {
            'Cargo.toml': tests.manifest('0.16.1'),
            'Cargo.lock': tests.lock('0.16.1'),
        }
        self.pending = 0
        self.repository = 'mnemonic-bit/SmolSandbox'

    def request(self, method, path, data=None, allow_missing=False):
        if method == 'POST' and path == '/git/refs':
            result = super().request(method, path, data, allow_missing)
            self.pending = 1
            return result
        if method == 'GET' and path == '/git/ref/tags/' + tests.TAG and self.pending:
            self.pending -= 1
            self.calls.append((method, path, data))
            return None
        return super().request(method, path, data, allow_missing)

case = tests.ReleaseTests()
case.setUp()
case.api = DelayedTag()
try:
    case.run_release(sha=tests.SHA)
except tests.release.ReleaseError as error:
    print('\n'.join(case.messages))
    print('Release check failed:', error)
    assert str(error) == 'Created tag v0.16.1 cannot be verified'
else:
    raise AssertionError('Expected reported failure')
assert case.api.tags[tests.TAG]['sha'] == tests.SHA
assert not case.api.releases
assert [(method, path) for method, path, _ in case.api.writes] == [('POST', '/git/refs')]
print('Confirmed: matching tag persists; no release was created')
assert case.run_release(sha=tests.SHA) == 'published'
assert sum(path == '/git/refs' for _, path, _ in case.api.writes) == 1
print('Same-commit rerun: published; no second tag creation')
PY
```

Result: `passed`; exit 0. The command confirmed the reported failure,
matching persisted tag, zero releases, and exactly one tag-creation write.
A second invocation then published through the fake without recreating the
tag. Removing the single missing read allowed the first invocation to
publish. The fake's visibility behavior is the only changed input.

The existing regression command also ran:

```text
python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v
```

Result: `passed`; exit 0, 96 tests in 0.320 seconds. The existing suite
passes despite this reproduced failure. Eventual-fix regressions are
`not_run`; no software test files were changed for this report.

## Recovery for the reported release

If the tag still resolves to the reported commit and the original native
artifacts remain available, the repository owner can rerun the original
failed job from run `45459182706`. For an authenticated GitHub CLI session:

```text
gh run rerun 45459182706 --failed --repo mnemonic-bit/SmolSandbox
```

This command was not executed during the investigation.
[GitHub reruns retain the original event SHA and ref](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/re-run-workflows-and-jobs).
The existing recovery code accepts a matching persisted tag and can create
the missing release. A rerun is a recovery attempt, not proof of success;
verify the job conclusion and all five release ZIPs afterward. Manual
dispatch performs validation only under the current workflow. Follow the
[operating recovery instructions](../RUST-APPLICATION.md#recover-a-release-attempt).

Preserve `v0.16.1`. Do not delete, move, or recreate the tag to bypass this
failure. Rerunning the original commit requires no new software version.
An eventual software correction requires a matching PATCH bump in both
Cargo files on its implementation branch. Version `0.16.1` is already used
by the existing tag and must not be selected for a new release commit.

## Requested correction and acceptance

The implementer owns tag-visibility handling and regression coverage in
`.github/scripts/release.py` and `.github/scripts/test_release.py`, plus any
required operating documentation. Preserve the workflow's embedded-script
equality checks and existing release/asset recovery contracts.

- AC-01: After successful tag creation, retry missing tag-reference reads
  within a fixed attempt/delay bound before failing. The existing five
  attempts and one-second delay provide a consistent bound. Verify one
  missing read, several missing reads, and visibility on the last attempt.
  Every successful path establishes the exact validated commit through a
  read; a successful creation response alone does not authorize publication.
- AC-02: Retry eligible missing reads after HTTP 409/422 creation races
  without repeating tag creation. Accept only a matching commit. Preserve
  conflicting tags and fail before any release write. Keep authentication,
  rate-limit, malformed-metadata, and other API failures explicit rather
  than treating every error as delayed visibility.
- AC-03: If the attempt bound expires, fail with the selected tag, expected
  commit, attempt count, and original-workflow rerun guidance. Preserve the
  tag. Verify that no release is created and no assets are uploaded during
  unresolved tag verification.
- AC-04: After tag verification, retain the immediate pre-publication
  conflict checks, annotated-tag resolution, immutable commit selection,
  release-ID verification, and all five binary-asset checks. Recovery of a
  matching tag publishes once; repeated recovery preserves matching
  releases and assets without duplicate or destructive writes.
- AC-05: Add regressions that fail on the current implementation for the
  delayed post-creation tag read. Run the complete release automation suite
  and required repository checks for the eventual software change. Include
  the matching PATCH version bump and append implementation results to
  [Feature 0010 validation](../FEATURES/feature-0010-validation.md).
- AC-06: Validate delayed visibility and rerun recovery in a disposable
  repository before claiming hosted correction. Record the run URL,
  validated commit, tag identity, release ID, and five verified assets.
  Do not use a production tag deletion or rewrite as a test fixture.

## Investigation limits

The exact hosted HTTP sequence remains unobserved. The local reproduction
proves a sufficient failure mechanism; the current matching tag and failed
release job support that explanation without proving the infrastructure
cause. Authenticated log/draft inspection, a hosted rerun, and eventual-fix
checks are `not_run`. The seven Rust checks were not repeated for this
report-only change. No production tag, release, asset, workflow, or
repository setting was changed.
