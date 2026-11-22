# Feature 0019 validation

## Local implementation audit: 2026-11-22

Status: In progress. The local implementation is complete; mandatory hosted
rendering, publication, and other native artifact observations remain pending.
The source is commit `f21fa1ae3541814ee94e1b8f45b1828fe75ea3e5` plus the
working-tree changes identified below. Both package versions are `0.13.0`.
The host is native `x86_64-unknown-linux-gnu`, with Rust and Cargo `1.100.0`.
No release, tag, or uploaded asset was created or modified during this audit.

The owner selected MIT in the Feature 0019 licence decision. The root
`LICENSE` contains the standard [MIT text](https://opensource.org/license/mit),
with `Copyright (c) 2026 Marco Ruether`. Attribution comes from the latest
commit's `Marco Ruether <mruether@mnemonic-bit.com>` author identity and the
initial 2026 history using the same email. Cargo metadata and the compact
README badge both declare MIT. The three existing badge lines are unchanged.

The implementation adds four original SVG download buttons and stable ZIP
names in the packager, canonical publisher, and workflow-embedded publisher.
The active operating reference explains the transition and original-workflow
recovery. Application source, templates, dependencies, and lifecycle revision
`5` are unchanged. The feature specification status and this record describe
acceptance; previous feature and bug observations remain preserved.

### Commands and local observations

| Command or check | Result | Exit code and evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0 |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0 |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0 |
| `cargo test --locked --all-targets` | passed | 0; 118 tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0; 185 tests |
| `cargo build --locked --release --bin smolsbx` | passed | 0; default features |
| `cargo run --locked --example check_baseline_independence` | passed | 0; isolated checks and nine compiled test executables per configuration with empty PATH |
| `python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v` | passed | 0; 89 tests; also independently run by the reviewer |
| `git diff --check` | passed | 0 |
| Independent implementation review | passed | No actionable correctness findings in release behavior, mappings, metadata, or guidance |
| Native Linux x86-64 packaging and extracted execution | passed | 0; production executable packaged by `package_release.package` with its real subprocess runner |
| Other three native packaging runs | not_run | Linux ARM64, macOS ARM64, and Windows x86-64 hosts are unavailable in this session |
| Hosted GitHub README rendering and badge click | not_run | Changes remain local; no hosted rendering of this revision was observed |
| Real SmolVM guests | not_run | No SmolVM guest was run; no new guest behavior is introduced |

The native packaging fixture copied the production executable and both version
files into a temporary root, then invoked `package_release.package` for
`x86_64-unknown-linux-gnu`. The SHA argument identified the baseline commit;
the binary contained the local working-tree implementation, not a published
commit. The output was `smolsbx-x86_64-unknown-linux-gnu.zip`, SHA-256
`390854727fdb72a42364fb52ea7d3a2bcf5f0bfd2a7c918669da6770c1f654ff`.
The sole entry was `smolsbx`, 3,252,248 bytes, regular-file mode `0755`.
The packager verified ELF x86-64, byte equality, executable permissions, and
real extracted output `smolsbx 0.13.0`.

The packaging regressions use versions `1.2.3` and `2.0.0` across all four
targets and retain the same filenames while requiring the selected executable
version. Publication regressions use two versions with the same filenames
and preserve earlier release assets. Recovery regressions preserve matching
IDs and bytes, upload missing assets, and reject conflicting contents,
unexpected assets, and versioned aliases without replacing assets. The
workflow's embedded release code matches the canonical script.

### README rendering observations

A local Playwright Chromium fixture rendered the actual README download links
and actual SVG bytes, with Arial 16px text, 16px page padding, and image
`max-width: 100%`. The fixture used data URLs for the SVGs. A first file-URL
fixture could not load local images and timed out; the corrected fixture
passed with exit code 0. These observations do not establish GitHub rendering.

| Theme and viewport | Result | Observation |
| --- | --- | --- |
| Light, 1100px | passed | Four readable buttons on one row; all heights 32px; no horizontal scrolling |
| Dark, 1100px | passed | Four readable buttons on one row; all heights 32px; no horizontal scrolling |
| Light, 375px | passed | One button per line at y=16,48,80,112; no horizontal scrolling |
| Dark, 375px | passed | One button per line at y=16,48,80,112; no horizontal scrolling |
| Images disabled, 375px | passed | All four exact accessible link names retained; visible alternative text wraps without horizontal scrolling |

The button widths are 220, 220, 268, and 244px in README order. The agent
visually inspected dark desktop, light narrow, and images-disabled narrow
screenshots. Session artifacts are `/tmp/feature-0019-readme-render-results.json`
and `/tmp/feature-0019-buttons-{light,dark}-{1100,375}.png`, plus
`/tmp/feature-0019-buttons-images-disabled-375.png`. The measurements above
preserve the observations after those temporary files expire.

### Public release and download observations

A read-only GitHub API request on 2026-11-22 selected normal release
[`v0.11.2`](https://github.com/mnemonic-bit/SmolSandbox/releases/tag/v0.11.2),
with `draft=false`, `prerelease=false`, and target commit
`18d81d8e4a6ff51b115086b93ca892bf62b783ec`. The API returned these historical
assets before the first stable-name release:

| Asset name | Asset ID | Size | API SHA-256 digest |
| --- | --- | --- | --- |
| `smolsbx-v0.11.2-aarch64-apple-darwin.zip` | 776054171 | 2606976 | `3f31517363f3158d03133096479d68aa5a222de9fe0b4f234451c46ef2847dcc` |
| `smolsbx-v0.11.2-aarch64-unknown-linux-gnu.zip` | 776054110 | 2888728 | `2944ab6c47f222713e234587af5497ccd96886763c05df1d18c94b5b44e154df` |
| `smolsbx-v0.11.2-x86_64-pc-windows-msvc.zip` | 776054261 | 3290744 | `4fc421d1a65c26b36f77e16b61dfbef8997592252bfd13708d33b317ad927cad` |
| `smolsbx-v0.11.2-x86_64-unknown-linux-gnu.zip` | 776054005 | 3253136 | `3c7287a72ebb5f31109a346d671c408217d0813c4a747aa59dcd8a842a6d0913` |

The digest values above are API metadata; historical asset bytes were not
downloaded or independently hashed. Compare these IDs, names, sizes, and
digests again after the first stable-name release. No publication was
performed, so that before/after acceptance check remains `not_run`.

Each following original URL was probed with
`curl --location --silent --show-error --max-time 30 --output /dev/null --write-out '%{http_code}\n%{url_effective}' <URL>`.
All curl commands exited 0; HTTP results were 404. No ZIP entry could be
inspected because no stable-name asset was returned.

| Original button URL | Final URL | HTTP result |
| --- | --- | --- |
| https://github.com/mnemonic-bit/SmolSandbox/releases/latest/download/smolsbx-x86_64-unknown-linux-gnu.zip | https://github.com/mnemonic-bit/SmolSandbox/releases/download/v0.11.2/smolsbx-x86_64-unknown-linux-gnu.zip | 404 |
| https://github.com/mnemonic-bit/SmolSandbox/releases/latest/download/smolsbx-aarch64-unknown-linux-gnu.zip | https://github.com/mnemonic-bit/SmolSandbox/releases/download/v0.11.2/smolsbx-aarch64-unknown-linux-gnu.zip | 404 |
| https://github.com/mnemonic-bit/SmolSandbox/releases/latest/download/smolsbx-aarch64-apple-darwin.zip | https://github.com/mnemonic-bit/SmolSandbox/releases/download/v0.11.2/smolsbx-aarch64-apple-darwin.zip | 404 |
| https://github.com/mnemonic-bit/SmolSandbox/releases/latest/download/smolsbx-x86_64-pc-windows-msvc.zip | https://github.com/mnemonic-bit/SmolSandbox/releases/download/v0.11.2/smolsbx-x86_64-pc-windows-msvc.zip | 404 |

The latest-release route agrees with [GitHub's documented asset route](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases).
The README states that direct downloads require the first stable-name normal
release and retains the plain release-page link. Reprobe all four URLs after
publication; record the selected tag, final URLs, HTTP 200 results, ZIP entries,
and native packaging evidence. Hosted acceptance remains pending.

### Acceptance results

| Criterion | Result | Evidence and remaining observation |
| --- | --- | --- |
| AC-01 | inconclusive | Source checks passed for full MIT text, attribution, badge, and Cargo metadata; hosted badge click pending |
| AC-02 | inconclusive | Four exact platform labels, alternative texts, links, and retained status badges passed inspection; hosted GitHub rendering pending |
| AC-03 | inconclusive | Local light/dark, narrow, and image-disabled checks passed; GitHub theme and viewport observations pending |
| AC-04 | inconclusive | Stable-name fixtures and actual Linux x86-64 package passed; other three native artifacts pending |
| AC-05 | inconclusive | Two-version fixtures passed; all four current live destinations return 404 pending first stable-name release |
| AC-06 | inconclusive | Recovery and preservation fixtures passed; historical asset metadata comparison after publication pending |
| AC-07 | passed | README and operating reference retain extraction, PATH, SmolVM, host requirements, source builds, transition, and recovery guidance |
| AC-08 | passed | Metadata agrees on 0.13.0; all seven required Rust checks and 89 Python regressions passed |

### Tested input fingerprints

These SHA-256 values identify the implementation files examined by the local
checks. Validation records and specification status edits are excluded.

| File | SHA-256 |
| --- | --- |
| `.github/scripts/package_release.py` | `8bf1fa968274155dfb923ab70edc42df4fe509874b4bc7146001687c1628e709` |
| `.github/scripts/release.py` | `bb4b0cb5519dde1a981586ec9c1b1ad395d9d2b23c0aabc754d0bacfdf524dda` |
| `.github/scripts/test_package_release.py` | `62b32a926b2d6e8916e37efa57e4cbf5c9449cca470a270b7f7bfca9a00763c0` |
| `.github/scripts/test_release_assets.py` | `23383271e92b6fbb1f4ca5345ba0a2f3a895844115db437fa5b9af22b6a59b7f` |
| `.github/workflows/rust.yml` | `7a4b6478b467ceade84e2dc48eab20f1f9a92fcd96861b8ab8b23407684f7b68` |
| `Cargo.toml` | `a03f032681f96d740225de56ec53e92c1c60cb4638cec010294c33434ef2b252` |
| `Cargo.lock` | `b51883ee5fb98d898dd7513c488000a9af49fe971f33da83977f32bad185613e` |
| `README.md` | `31abb586822bcd4c1c0d3b8db329b9872a4fa616f91bb27c97e8f5d290f73de6` |
| `RUST-APPLICATION.md` | `94a16ea779449c9973a65cd874323554cc72b7a69bd1d6728030c9227f7f2f44` |
| `LICENSE` | `2a8b736fff893ab2b2aaf15ab609149b0e19b477026eb22669eea40919178982` |
| `assets/badges/download-linux-arm64.svg` | `233ef132c1b06ab337b10d726ffbb0ffc4bbe4d1a283345e2ea21c9d8df3bdfe` |
| `assets/badges/download-linux-x86-64.svg` | `c2189e148c217b412c5e451077dbd1c0e68cb65d2c41dd746108a1963b3f085d` |
| `assets/badges/download-macos-arm64.svg` | `203106b73c2a4bd9410373ba8337c2918e4daa53c5288bb1e31ed575acd37850` |
| `assets/badges/download-windows-x86-64.svg` | `6cc2112859b783131c8af8e78421e1b33b4afdb547e6f68286a6909abe606a76` |
