# Feature 0019: Add README licence and binary download badges

## Status

In progress

Specification preparation added no README badges, licence grant, release
assets, workflow changes, or software version bump. Local implementation and
remaining acceptance checks are recorded in
[Feature 0019 validation](feature-0019-validation.md). Implementation acceptance
is pending. The repository owner selected the MIT licence in the follow-up
request. Implementation must add a root `LICENSE` file and a README licence
badge linking to that file.

## Outcome and baseline

Visitors to `README.md` can identify the project licence and download the
latest published binary ZIP for their operating system and architecture
from the README header. A compact MIT licence badge accompanies the existing
status badges. Four larger download buttons form a separate row.

Specification preparation observed commit
`f21fa1ae3541814ee94e1b8f45b1828fe75ea3e5` and application version `0.12.0`
on 2026-11-22. The working tree was clean. The README already contains
Release, Pre-release, and Build and tests (develop) badges. No tracked
`LICENSE`, `LICENCE`, or `COPYING` file was found by filename search, and
`Cargo.toml` has no package licence declaration. The owner's follow-up
request establishes MIT as the required project licence.

The reference is the [DeepTutor README](https://github.com/HKUDS/DeepTutor/blob/main/README.md),
observed on 2026-11-22. DeepTutor uses a compact Shields.io licence badge
and a separate row of linked, repository-owned SVG download buttons at
32-pixel height. Its download links use GitHub's latest-release asset
route. Adopt that distinction between status badges and download actions;
use SmolSandbox's own platform matrix and visual assets.

Local release sources define Linux x86-64, Linux ARM64, macOS ARM64, and
Windows x86-64. Each uploaded ZIP contains only `smolsbx` or `smolsbx.exe`.
Current ZIP names include the version, for example
`smolsbx-v0.12.0-aarch64-apple-darwin.zip`. Release verification requires
exactly four uploaded binary assets. Published GitHub asset availability
was not checked during specification preparation.

Structural exploration used the installed `codebase-memory` skill and
project `workspace-myrepo`, graph generation `2026-11-22T10:14:32Z`.
Source reads confirmed packaging and publication behavior. The coverage
check reported no recorded issues for the relevant indexed paths;
`Cargo.lock` was read directly because its freshness was untracked.

## Scope and ownership

The implementer owns the README header, four SVG buttons, release filename
changes, matching release regressions, operating guidance, version bump,
and implementation evidence, including the MIT licence file and badge.
The repository owner supplies any copyright attribution that cannot be
established from repository records.

Primary implementation files are `README.md`, new SVGs under
`assets/badges/`, `.github/scripts/package_release.py`,
`.github/scripts/release.py`, `.github/workflows/rust.yml`, the release
tests under `.github/scripts/`, `RUST-APPLICATION.md`, `Cargo.toml`, and
`Cargo.lock`. Add a root `LICENSE` file containing the MIT licence.

Preserve the meanings, sources, and destinations of the existing Release,
Pre-release, and build badges. Preserve the four native production targets,
executable-only ZIP contents, verification of uploaded bytes, release
eligibility checks, and publication permissions. Preserve the existing
application commands, templates, state, and lifecycle policy.

The feature does not add installers, another operating system or
architecture, package-manager distribution, or an automatic README commit
after publication. Badge presence does not establish real guest validation.

## Required behavior

### Licence badge

1. Add the complete standard MIT licence text as the root `LICENSE` file.
   Include the copyright year and holder established from repository
   records or supplied by the repository owner. Do not leave template
   placeholders in the published file.
2. If the implementation baseline already has a licence file, review that
   file against the required MIT licence before editing it. Keep one
   authoritative root `LICENSE` file.
3. Add a compact badge labelled `License` beside the existing status
   badges. Display `MIT` and link directly to `LICENSE` in this repository.
   The following Markdown satisfies the badge label and link requirement:

   ```markdown
   [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
   ```

4. Use accessible alternative text that names the MIT licence. Match
   the existing status-badge scale. A Shields.io image is permitted, as
   already used by the README.
5. Set `[package].license = "MIT"` in `Cargo.toml` so package metadata agrees
   with the licence file and badge.

### Download buttons

Place four linked SVG buttons below the compact status badges and above
the introductory application description. Use a consistent 32-pixel image
height, platform-specific labels, and a download action visible in each
button. Store the SVGs under `assets/badges/` so button images do not depend
on a new image service. Use original assets with text and simple shapes.

The labels and matching alternative text identify these four choices:

| Visible platform label | Required alternative text | Stable binary ZIP name |
| --- | --- | --- |
| Linux x86-64 | Download smolsbx for Linux x86-64 (ZIP) | `smolsbx-x86_64-unknown-linux-gnu.zip` |
| Linux ARM64 | Download smolsbx for Linux ARM64 (ZIP) | `smolsbx-aarch64-unknown-linux-gnu.zip` |
| macOS Apple Silicon | Download smolsbx for macOS Apple Silicon (ARM64, ZIP) | `smolsbx-aarch64-apple-darwin.zip` |
| Windows x86-64 | Download smolsbx for Windows x86-64 (ZIP) | `smolsbx-x86_64-pc-windows-msvc.zip` |

Each button links directly to
`https://github.com/mnemonic-bit/SmolSandbox/releases/latest/download/<stable-binary-ZIP-name>`.
The destination is the uploaded binary ZIP for the selected target.
GitHub documents the route in
[Linking to releases](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases).
GitHub source archives, Actions artifacts, and the release listing do not
satisfy the button's direct-download requirement.

Keep the buttons readable in GitHub light and dark themes. Allow buttons
to wrap at a 375-pixel viewport without introducing horizontal scrolling.
Use ordinary links and images supported by GitHub Markdown; require no
JavaScript or custom stylesheet. Keep each download action understandable
when its image cannot load through its alternative text.

Below the buttons, include a plain-text latest-release link and a short
instruction to extract the matching ZIP and copy `smolsbx` or
`smolsbx.exe` to a directory on `PATH`. Retain source-build instructions.
Keep the separate SmolVM installation requirement and link to host
requirements visible in the installation guidance. Do not offer macOS
Intel or Windows ARM64 downloads under the current release matrix.

### Stable release asset names

GitHub's latest-release asset route needs an exact filename. Replace the
version-bearing binary ZIP names with the four stable names above for
releases produced by the implementation. Keep the release tag and title
as `v<version>` and retain the embedded executable's version. Download
URLs for a specific release remain available as
`releases/download/v<version>/<stable-binary-ZIP-name>`.

Update the packager, publication checks, workflow-embedded release logic,
and release regression expectations together. Continue to require exactly
four uploaded ZIPs. Do not add duplicate versioned aliases. This naming
requirement supersedes the version-bearing filename requirement in
[Feature 0017](feature-0017-automatic-github-releases.md) for new releases.
Preserve earlier specifications and validation observations as historical
records; update active release guidance to explain the naming transition.

Preserve existing published releases, tags, asset IDs, names, and bytes.
Recovery of an older release uses its original workflow revision and
original filename scheme. Recovery of a new release uses stable names
and the existing missing-asset and conflicting-content rules.

The first normal release containing the implementation supplies all four
stable assets. On branches containing the new buttons before that release
is published, state that direct downloads require the first release with
stable asset names and retain the plain-text release-page link. Do not
claim live-download acceptance until all four destinations have been
verified against a published normal release. A failed upload remains a
failed publication; recover using the existing release procedure.

Increment the application minor version in `Cargo.toml` and the matching
`smolsandbox` entry in `Cargo.lock` when implementing the release changes.
From the observed `0.12.0` baseline, the implementation version is
`0.13.0`. If the baseline changes, derive the minor bump from that baseline.
Keep `REVISION` unchanged because machine-creation policy is unchanged.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | The root `LICENSE` contains the complete MIT licence text and copyright attribution without placeholders. The compact License badge displays MIT and opens `LICENSE`. Cargo package licence metadata is `MIT`. | Inspect `LICENSE`; click the rendered README badge; compare the badge label, destination, and `[package].license` value. |
| AC-02 | The README header retains the three existing badges and displays four larger download buttons with the specified labels, alternative text, and target mappings. | Inspect Markdown destinations and render the README on GitHub. |
| AC-03 | All four buttons are readable in light and dark themes and wrap without horizontal scrolling at a 375-pixel viewport. The download actions remain identifiable with images disabled. | Record rendered observations for both themes, the narrow viewport, and image-disabled links. |
| AC-04 | Packaging and publication require exactly the four stable ZIP names. Each ZIP contains only the correct production executable with the existing architecture, permission, and version checks. | Run packaging and release regressions and inspect native build artifacts for all four targets. |
| AC-05 | Each live button follows redirects to the matching uploaded ZIP in the latest normal release. The links require no filename edits for a later version. | Record the selected release tag, four original URLs, final download URLs, HTTP results, and ZIP entries. Use two version fixtures to verify unchanged button destinations and stable packaging names. |
| AC-06 | New-release recovery preserves verified assets, restores missing assets, rejects conflicting contents, and preserves unexpected assets while reporting failure. Historical releases retain their original assets. | Run release recovery regressions; compare recorded historical asset metadata before and after the first new release. |
| AC-07 | Installation guidance explains ZIP extraction, executable placement, separate SmolVM installation, and the transition to stable names. The plain-text release link and source-build instructions remain available. | Review README and operating-reference diffs and follow the documented download path. |
| AC-08 | The manifest and lockfile agree on the required minor version bump. Required local Rust and release-automation checks pass. | Inspect both package entries; record the commands and exit codes listed below. |

## Verification and evidence

During implementation, create `FEATURES/feature-0019-validation.md`.
Record the source revision, package version, owner's licence decision,
affected files, commands, exit codes, rendered README observations, and
AC-01 through AC-08 results. Use `passed`, `failed`, `not_run`, or
`inconclusive`. Keep hosted checks pending when no suitable release or
GitHub rendering evidence is available. Mark the feature `Implemented`
only after all mandatory criteria have passed.

Run `python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v`
for release-automation regressions. Run all seven independent Rust checks
from [AGENTS.md](../AGENTS.md), including both feature configurations,
the production build without `test-adapter`, and baseline independence.
Record native packaging evidence for all four release targets. Append
new validation observations and pending native or real guest checks to
[Feature 0010 validation](feature-0010-validation.md), preserving previous
observations and source revisions. No new real guest behavior is introduced
by this feature.

## Licence decision

The repository owner selected MIT in the follow-up request. No licence
selection remains pending. Adding the MIT licence file, matching package
metadata, and linked README badge is mandatory for AC-01.
