# Bug 0006: GitHub releases omit downloadable binary ZIPs

## Status and outcome

In progress. Report prepared on 2026-11-21 for `smolsandbox 0.11.0`, release
`v0.11.0`, and main commit `b025321647c5c0bc2719bf12fbc3650a0156c6b4`.
The user followed the README release badge and found a source-code ZIP
instead of ready-to-run software. Users cannot download the compiled
executable from that release.

The release Action must attach one binary ZIP for each supported operating
system and architecture. Each ZIP must contain only the compiled executable.
The README release badge must continue to open the GitHub release page where
those ZIPs are available.

This report changes no software or version. The eventual fix requires a
patch bump in `Cargo.toml` and the matching `smolsandbox` entry in
`Cargo.lock` under [AGENTS.md](../AGENTS.md): `0.11.1` if the implementation
baseline remains `0.11.0`.

## Observed behavior and evidence

Read-only GitHub REST API inspection on 2026-11-21 confirms that
[release v0.11.0](https://github.com/mnemonic-bit/SmolSandbox/releases/tag/v0.11.0)
has release ID `453632353`, `draft: false`, `prerelease: false`, and
`assets: []`. The
[release API response](https://api.github.com/repos/mnemonic-bit/SmolSandbox/releases/tags/v0.11.0)
includes `zipball_url` and `tarball_url` for source archives, but no uploaded
release assets.

The [workflow artifacts API](https://api.github.com/repos/mnemonic-bit/SmolSandbox/actions/runs/44896711807/artifacts?per_page=100)
lists four unexpired artifacts for the original main push:

| Target | Workflow artifact | Artifact ID |
| --- | --- | --- |
| Linux x86-64 | `smolsbx-x86_64-unknown-linux-gnu` | `14704756239` |
| Linux ARM64 | `smolsbx-aarch64-unknown-linux-gnu` | `14704023961` |
| macOS ARM64 | `smolsbx-aarch64-apple-darwin` | `14703749250` |
| Windows x86-64 | `smolsbx-x86_64-pc-windows-msvc` | `14704626449` |

These are Actions artifacts, separate from assets attached to a GitHub
Release. Artifact metadata was inspected; archive contents were not
downloaded during this report.

The local baseline is `a90c901651b6bf16bbfe74548295d824f9a2895e`.
[README.md](../README.md) already links its `Release` badge to
`https://github.com/mnemonic-bit/SmolSandbox/releases/latest`. That
destination satisfies the requested navigation and must be preserved.
The development `Pre-release` badge has a separate purpose and is outside
this report's release-download requirement.

## Cause and relationship to Feature 0017

The `native` matrix in
[rust.yml](../.github/workflows/rust.yml) compiles all four production targets
without `test-adapter` and uploads executables through `actions/upload-artifact`.
The `release` job checks out source and runs
[release.py](../.github/scripts/release.py); the job has no step that retrieves
the four artifacts or uploads binary ZIPs to the release. The `publish`
function creates or recovers release metadata without transferring binaries.
The source of both files at the affected main commit was verified during
the [Bug 0005 investigation](bug-0005-release-publication-verification-race.md).

The original [Feature 0017 specification](../FEATURES/feature-0017-automatic-github-releases.md#scope-and-ownership)
explicitly made binary attachments optional.
[RUST-APPLICATION.md](../RUST-APPLICATION.md#github-releases) likewise states
that binaries remain Actions artifacts. The current implementation follows
that earlier scope. This report records the user's new mandatory binary
distribution requirement and supersedes that optional treatment for the
eventual correction. Update those documents when implementing the fix.

Bug 0005 concerns an independent publication-verification failure. Fixing
that race alone will not attach binaries. Conversely, a matching published
release must not cause binary-upload recovery to return early while required
ZIP assets are missing.

## Required release contents

Use the four targets already supported by the native matrix. For version
`<version>`, attach exactly these four project-uploaded assets:

| Operating system and architecture | Release asset name | Sole ZIP entry |
| --- | --- | --- |
| Linux x86-64 | `smolsbx-v<version>-x86_64-unknown-linux-gnu.zip` | `smolsbx` |
| Linux ARM64 | `smolsbx-v<version>-aarch64-unknown-linux-gnu.zip` | `smolsbx` |
| macOS ARM64 | `smolsbx-v<version>-aarch64-apple-darwin.zip` | `smolsbx` |
| Windows x86-64 | `smolsbx-v<version>-x86_64-pc-windows-msvc.zip` | `smolsbx.exe` |

Each archive must contain exactly one file at the archive root. Do not
include source files, directories, configuration templates as separate
files, documentation, checksums, debug symbols, build outputs other than
the executable, or a nested artifact ZIP. The executable already embeds
its templates. Preserve Unix executable permissions when packaging and
extracting the Linux and macOS archives.

Every executable must come from the successful native production build for
the exact immutable release commit. Package all targets from the same run;
do not select artifacts from a moving branch or another run. Preserve the
existing validation gates and main-push publication restrictions.

[GitHub documents](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
that release pages automatically include source ZIP and tarball links.
Those platform-generated links are separate from uploaded release assets;
they do not satisfy any binary-download criterion. The executable-only rule
applies to every delivered binary ZIP, and the Action must upload only the
four binary ZIPs listed above. This report does not claim that the Action
can remove GitHub's automatically generated source links.

## Reproduction

1. Open `README.md` and follow the `Release` badge.
2. Open release `v0.11.0` and inspect its downloadable assets.
3. Observe that no ZIP identifies a supported binary target.
4. Download the source-code ZIP, as reported by the user, and observe that
   the archive contains repository source instead of the compiled executable.

The metadata failure can also be checked without downloading archives:

```sh
python3 - <<'PY'
import json
import sys
import urllib.request

url = 'https://api.github.com/repos/mnemonic-bit/SmolSandbox/releases/tags/v0.11.0'
with urllib.request.urlopen(url, timeout=30) as response:
    release = json.load(response)
targets = (
    'x86_64-unknown-linux-gnu',
    'aarch64-unknown-linux-gnu',
    'aarch64-apple-darwin',
    'x86_64-pc-windows-msvc',
)
expected = {f'smolsbx-v0.11.0-{target}.zip' for target in targets}
actual = {asset['name'] for asset in release['assets']}
missing = sorted(expected - actual)
if missing:
    print('failed: missing binary ZIPs: ' + ', '.join(missing))
    sys.exit(1)
print('passed: all four expected binary ZIP asset names exist')
PY
```

Result: `failed`, exit 1; all four binary ZIPs are missing. This command
checks asset names only. Archive contents and executable behavior require
the acceptance checks below.

## Acceptance for the eventual fix

The implementer owns workflow packaging, release upload and recovery logic,
regression tests, documentation updates, and validation evidence.

- AC-01: A successful eligible main run attaches all four named ZIPs to the
  matching release. Verify the release asset list and download each asset
  through its public release URL, without accessing Actions artifacts.
- AC-02: Each ZIP contains exactly the sole root file specified in the
  target table. Inspect archive entries and reject extra files, directories,
  source trees, nested archives, and debug symbols. Verify extracted Unix
  executables retain executable permissions.
- AC-03: Each extracted executable has the expected target architecture and
  reports the committed package version through `--version` on its native
  operating system. Record build-run provenance tying all four executables
  to the release tag's resolved commit. Production builds exclude
  `test-adapter`.
- AC-04: If a required artifact, packaging step, or upload is missing or
  fails, the release job fails with a target-specific diagnostic. The job
  must not report a complete binary release based only on tag and release
  metadata. Exercise each failure through focused regression tests.
- AC-05: A rerun recovers missing ZIPs on an existing matching release.
  Simulate interruption after a partial upload and verify recovery. A rerun
  with all four verified ZIPs preserves their asset IDs and bytes, release
  notes, and tag SHA without duplicate uploads. Conflicting existing asset
  contents fail explicitly without silent replacement.
- AC-06: The README `Release` badge opens the latest normal GitHub release,
  where all four binary ZIPs are downloadable. Verify the rendered link and
  destination after a hosted publication. Source-only archives do not count.
- AC-07: Update the Feature 0017 packaging requirement and operating
  documentation, include the required matching patch versions, and pass the
  release regression suite and seven Rust commands in `AGENTS.md`. Keep the
  canonical and inline release scripts synchronized if either changes.
- AC-08: Record a hosted publication and rerun with run URLs, release URL,
  tag SHA, asset IDs, archive listings, and native `--version` results.
  Append the new native results and pending guest checks to
  [Feature 0010 validation](../FEATURES/feature-0010-validation.md).

All eventual-fix acceptance checks are `not_run`. No workflow, README,
release script, tag, release, asset, or package version was changed for this
report. Local Rust checks are `not_run` because this change only adds a bug
report. The earlier Bug 0005 report and validation observations remain
unchanged.

## Implementation and local validation: 2026-11-21

The implementation baseline is `a346389f99b461f7cd745bdb9bbc2c31c64752c0`
plus the Bug 0006 working-tree changes. Both package versions advance from
`0.11.1` to `0.11.2`. The original report and observations above describe
the pre-fix state; the following results describe the local implementation.
Hosted acceptance remains pending.

Native jobs now run `.github/scripts/package_release.py` after the existing
Rust validation gates. Packaging checks the native executable header,
creates a deterministic ZIP containing only the root executable, restores
the stored Unix permissions during extraction, and runs the extracted
executable with `--version`. The four jobs upload ZIPs as Actions artifacts.
The release job downloads those artifacts from the same workflow run.
Artifact overwrite permits full workflow reruns; release assets remain
subject to separate byte verification.

Publication validates all four local ZIPs before creating tags or releases.
For new and matching existing releases, publication lists assets with
pagination, compares existing bytes, uploads missing ZIPs, and downloads
each asset to verify the complete set. Matching assets retain their IDs
and bytes. Conflicting assets fail without replacement or deletion.
Concurrent duplicate uploads succeed only after the resulting asset bytes
match. Release notes and tag identity retain the existing recovery rules.
The canonical and inline release programs are synchronized. The README
Release badge destination is unchanged.

The original regression command was
`python3 -m unittest discover -s .github/scripts -p test_release_assets.py -v`.
Before the fix, the matching published-release case returned
`already-published` with all four required assets missing: `failed`, exit 1.
The same case now passes. Packaging workflow regressions also failed before
the packaging and download steps were added.

| Command or check | Result | Evidence |
| --- | --- | --- |
| `python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v` | passed | Exit 0; 85 tests: 52 metadata/transport/workflow, 24 binary asset/transport, and nine packaging/workflow tests |
| `cargo fmt --all -- --check` | passed | Exit 0 |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | Exit 0 |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | Exit 0 |
| `cargo test --locked --all-targets` | passed | Exit 0; 118 tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | Exit 0; 185 tests |
| `cargo build --locked --release --bin smolsbx` | passed | Exit 0; default features; executable reports `smolsbx 0.11.2` |
| `cargo run --locked --example check_baseline_independence` | passed | Exit 0; isolated source checks and nine compiled test executables per configuration passed with empty `PATH` |
| Actual native Linux ZIP smoke | passed | Packaged the compiled release executable in a temporary fixture root; sole entry `smolsbx`, mode `0755`, ELF x86-64, extracted output `smolsbx 0.11.2` |
| `/tmp/bug0006-tools/actionlint .github/workflows/rust.yml` | passed | Exit 0; actionlint 1.7.12, official archive SHA-256 verified |
| Independent agent review | passed | Packaging, workflow transfer, upload, recovery, and all 24 asset/transport tests reviewed; two diagnostic findings corrected |
| README badge source | passed | Existing latest normal release destination preserved |
| Hosted publication, public downloads, and rerun | not_run | No GitHub credential or GitHub CLI is available; no hosted workflow, release, tag, or asset was changed |
| Other native targets and real SmolVM guests | not_run | No new hosted or real guest execution evidence was collected |

The actual native Linux ZIP has SHA-256
`5e940ed4db20f68393c7d39ea6a3dd0f085b73d4480f2a5e28a80acdb1fb4c01`.
Its executable comes from the baseline plus local changes, rather than a
hosted release commit. Native validation used Rust/Cargo `1.100.0` and
Python `3.11.2` on `x86_64-unknown-linux-gnu`.

The following SHA-256 values identify the software and regression inputs:

| File | SHA-256 |
| --- | --- |
| `Cargo.toml` | `c047a8a0c9d2af3173189801497aaad18e88417b1ad5aa28c3dd3c4c7331932b` |
| `Cargo.lock` | `ec3d56e9c5ba9ea92720bf81153463d78aedc673148624af50c2077f863418c0` |
| `.github/workflows/rust.yml` | `6ab797e4cd581e5bb84ca782c301961600bbea35203cc9c5a6b9e306452aa4b6` |
| `.github/scripts/release.py` | `a9c144369d56ea5267b618e853aec432a78af0c95b73012964e393793e778aea` |
| `.github/scripts/package_release.py` | `cd274a4f5459c3fd9c3a1e5ed717e31ccd3393f842c702e338126426cecfdf8e` |
| `.github/scripts/test_release.py` | `c5e5728911cc0939371d1aaa0c6421f2a16a347bba4bd614c09c5fca51bf53ae` |
| `.github/scripts/test_release_assets.py` | `4570c98fde989d21a9d9202b5d4b883a0f76aabcd52ed24acc22e819db7cc177` |
| `.github/scripts/test_package_release.py` | `2472903d116c4f516656d812503b71fcb5d898d1e55a1f54297ade84f1e6e833` |

AC-01, the full four-target execution in AC-03, the hosted portions of
AC-05 and AC-06, and AC-08 remain `not_run`. AC-02 and AC-04 have local
regression coverage; AC-02 has actual Linux executable evidence only.
AC-07 passed locally, including all seven required Rust commands.
The next hosted run must record run and release URLs, tag SHA, asset IDs,
public download archive listings, all four native `--version` results,
partial recovery, and a complete rerun preserving asset IDs and bytes.
