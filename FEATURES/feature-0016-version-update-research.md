# Feature 0016: Version-update analysis

## Decision and evidence boundary

The user selected Codex CLI `0.164.0` → `0.169.0` and every listed update
component on 2026-11-13. The approved target table in
[Feature 0016](feature-0016-update-codex-and-selected-tool-versions.md)
controls implementation. This analysis preserves the release observations,
alternatives, and initial recommendations used for that decision.
Recommendations to evaluate a migration separately describe compatibility
cost; they no longer exclude the migration from this feature. Release
research does not establish observed compatibility results.

The repository baseline is commit
`583d5db794c77f91f5187495a3f34b4df525e3e9`, application `0.8.1`.
Live primary-source metadata was retrieved on 2026-11-13. npm and PyPI
responses reported that date. The analysis compares published stable
releases against template pins and direct Cargo dependency resolutions;
prereleases are excluded. Versions can change after this snapshot.

Release lookup: `passed`. Guest installation, runtime compatibility, and
implementation regression checks: `not_run`. No software pins changed while
preparing this analysis. Package publication and declared runtime minimums
do not establish successful provisioning.

## npm and Python tools

Shared tools below occur in all four `templates/*.smlbx` files. Codex and
`skills` occur only in `templates/codex.smlbx`. Update the corresponding
independent expectations in `src/config.rs` during implementation.

| Item | Repository pin | Verified stable target | Recommendation and checks before inclusion |
| --- | --- | --- | --- |
| Codex CLI | `0.164.0` | `0.169.0` | Required. Preserve launch, device authentication, home seeding, and MCP registration. The [official changelog](https://learn.chatgpt.com/docs/changelog) records the release; [exact npm metadata](https://registry.npmjs.org/@openai%2fcodex/0.169.0) declares Node `>=16` and Linux x64/ARM64 packages. |
| `skills` CLI | `1.7.0` | `1.8.1` | Reasonable inclusion. [npm metadata](https://registry.npmjs.org/skills/1.8.1) requires Node `>=22.20.0`. Verify the existing noninteractive Codex-global installation of `grill-me` and `grilling`; record the actual Node version. Updating the CLI does not pin the fetched skill content. |
| Vitest | `5.0.1` | `5.1.0` | Reasonable inclusion. [npm metadata](https://registry.npmjs.org/vitest/5.1.0) requires Node `^22.12.0 || ^24.0.0 || >=26.0.0`. Run a small TypeScript test through the installed global command. |
| Playwright npm library and test runner | `1.63.0` for both | `1.65.0` for both | Include as one browser-tooling update. Verify [library metadata](https://registry.npmjs.org/playwright/1.65.0) and [runner metadata](https://registry.npmjs.org/@playwright%2ftest/1.65.0). Both declare Node `>=20`. Install Chromium and launch a browser on Linux x64 and ARM64. |
| Playwright Python binding | `1.63.0` | `1.65.1` | Coordinate with the npm update, but record a separate exact pin. [PyPI metadata](https://pypi.org/pypi/playwright/1.65.1/json) declares Python `>=3.10` and Linux x64/ARM64 wheels. Run the Python browser installation and launch separately. Package patch versions differ between ecosystems; browser compatibility remains unverified. |
| `uv` | `0.12.17` | `0.13.8` | Review migration behavior before inclusion. [PyPI metadata](https://pypi.org/pypi/uv/0.13.8/json) declares Python `>=3.8`. [uv's version policy](https://docs.astral.sh/uv/reference/policies/versioning/) allows breaking changes in minor releases. Exercise environment creation, dependency resolution, and a sample project. |
| `pytest` | `9.1.1` | `9.2.0` | Reasonable inclusion. [PyPI metadata](https://pypi.org/pypi/pytest/9.2.0/json) declares Python `>=3.10`. Run a sample test through the pipx-installed command. |
| Ruff | `0.16.8` | `0.17.5` | Review lint and formatting behavior before inclusion. [PyPI metadata](https://pypi.org/pypi/ruff/0.17.5/json) declares Python `>=3.7`. [Ruff's version policy](https://docs.astral.sh/ruff/versioning/) allows breaking rules, configuration, and formatting changes in minor releases. Compare checks on representative code before adopting the new pin. |

The npm registry's `latest` tags and PyPI's stable release metadata supplied
the target versions. Exact-version URLs above make the proposed targets
reviewable after the moving latest metadata changes. The moving lookup
endpoints were [npm Codex](https://registry.npmjs.org/@openai%2fcodex),
[npm skills](https://registry.npmjs.org/skills),
[npm Vitest](https://registry.npmjs.org/vitest),
[npm Playwright](https://registry.npmjs.org/playwright),
[npm Playwright Test](https://registry.npmjs.org/@playwright%2ftest),
[PyPI uv](https://pypi.org/pypi/uv/json),
[PyPI pytest](https://pypi.org/pypi/pytest/json),
[PyPI Ruff](https://pypi.org/pypi/ruff/json), and
[PyPI Playwright](https://pypi.org/pypi/playwright/json).

Two shared tool pins already match the stable release observed in their
registries:

| Item | Current and observed stable version | Source |
| --- | --- | --- |
| `tsx` | `4.23.15` | [npm metadata](https://registry.npmjs.org/tsx) |
| `debugpy` | `1.8.22` | [PyPI metadata](https://pypi.org/pypi/debugpy/json) |

Do not change these pins solely for version freshness.

For updates within the existing minor lines, PyPI also lists
non-yanked stable releases [uv `0.12.24`](https://pypi.org/pypi/uv/0.12.24/json)
and [Ruff `0.16.10`](https://pypi.org/pypi/ruff/0.16.10/json). Those are
alternative targets to `0.13.8` and `0.17.5`, respectively. Feature 0016
selects the newer minor versions. The patch alternatives remain recorded
research results; they are not additional required installations.

## MCP, .NET, and guest runtime choices

The shared runtime and .NET commands occur in all four templates.
codebase-memory-mcp occurs only in the Codex template.

| Item | Repository baseline | Verified target or alternatives | Recommendation and compatibility cost |
| --- | --- | --- | --- |
| codebase-memory-mcp | `v0.11.0` installer and release | [v0.12.1](https://github.com/DeusData/codebase-memory-mcp/releases/tag/v0.12.1) | Consider inclusion. Update both URLs together. Preserve portable Linux x64/ARM64 binaries and checksum verification. Upstream reports a one-time project index rebuild and changed image-node identities. Verify existing Codex registration and an actual MCP query. |
| .NET SDK | Floating channel `8.0` | Channel `10.0`; observed SDK `10.0.404` and runtime `10.0.15` in the [release index](https://builds.dotnet.microsoft.com/dotnet/release-metadata/releases-index.json) | Highest priority for a scope decision: [.NET 8 reached end of support on 2026-11-10](https://dotnet.microsoft.com/en-us/platform/support/policy/dotnet-core). .NET 10 is LTS. .NET 11 is newer but STS. Decide whether existing projects still require a separately installed .NET 8 runtime. |
| `dotnet-ef` | `8.0.31` | [NuGet lists](https://api.nuget.org/v3-flatcontainer/dotnet-ef/index.json) `8.0.34`, `10.0.15`, and latest `11.0.0` | `8.0.34` is the smallest bump but retains an unsupported major line. Prefer evaluating `10.0.15` with a .NET 10 default. Match the intended project's EF major; a global move to EF 10 or 11 changes which projects the default tool supports. |
| `dotnet-dump` | `8.0.547301` | [10.0.750501](https://api.nuget.org/v3-flatcontainer/dotnet-dump/index.json) | Consider with the diagnostics group. Verify dump collection and analysis against a sample guest process. |
| `dotnet-trace` | `8.0.547301` | [10.0.750501](https://api.nuget.org/v3-flatcontainer/dotnet-trace/index.json) | Consider with the diagnostics group. Verify collection and the intended trace commands. |
| `dotnet-counters` | `8.0.547301` | [10.0.750501](https://api.nuget.org/v3-flatcontainer/dotnet-counters/index.json) | Consider with the diagnostics group. Verify counter discovery and collection. |
| Node.js | NodeSource `setup_22.x` | Channel `26.x`; observed latest LTS `26.13.1`. Supported older lines include `24.22.3` and `22.23.5` on the [official download page](https://nodejs.org/en/download). | Evaluate Node 26 for a longer support period. Node 22 remains supported until 2027-04-30, so a major-channel change is optional now. Verify every global npm tool and native module on both guest architectures. |
| Debian image | `debian:bookworm-slim` | `debian:trixie-slim`; [Debian identifies 13.7 trixie as stable](https://www.debian.org/releases/), and the [official image manifest](https://github.com/docker-library/official-images/blob/master/library/debian) lists the slim tag. | Prefer a separate guest migration. Bookworm has LTS through 2028-06-30. Review distribution package names, libraries, Python, nested containers, and all provisioning steps on x64 and ARM64. |

The [v0.12.1 MCP installer](https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/v0.12.1/install.sh)
selects portable Linux artifacts. Retaining that selection matters because
the release notes identify glibc incompatibility in dynamically linked builds
on Debian 12.

The diagnostic tools share the official
[10.0.750501 release](https://github.com/dotnet/diagnostics/releases/tag/v10.0.750501).
Their inspected NuGet packages contain `net8.0` tools with major runtime
roll-forward enabled. A tool package's major version alone does not establish
a .NET 10 runtime requirement. Test the chosen SDK/runtime and tools together.

The [Node release schedule](https://raw.githubusercontent.com/nodejs/Release/main/schedule.json)
places Node 26 in Active LTS through 2027-10-20 and gives a support end date
of 2029-04-30. Node 24 is in Maintenance LTS. The
[NodeSource 26 installer](https://deb.nodesource.com/setup_26.x) exists;
architecture-specific NodeSource package availability was not verified.
An unchanged `setup_22.x` already resolves the available Node 22 patch on a
fresh build. Changing that URL is a major-channel decision.

## Direct Rust dependencies

All 11 direct dependencies were compared with non-yanked stable crates.io
releases. Eight resolved versions already match the observed latest stable
release. No newer stable release was found inside any direct dependency's
current manifest range. The three available upgrades require manifest edits.

| Dependency | Manifest range | Lockfile baseline | Verified latest stable | Recommendation and compatibility cost |
| --- | --- | --- | --- | --- |
| `anyhow` | `1` | `1.0.104` | [1.0.104](https://crates.io/api/v1/crates/anyhow) | Retain. |
| `clap` | `4` | `4.6.7` | [4.6.7](https://crates.io/api/v1/crates/clap) | Retain. |
| `ctrlc` | `3` | `3.5.2` | [3.5.2](https://crates.io/api/v1/crates/ctrlc) | Retain. |
| `dirs` | `6` | `6.0.0` | [7.0.0](https://crates.io/api/v1/crates/dirs) | Consider separately. Published source changes Windows `preference_dir()` from local to roaming application data. Verify application home selection and state paths on Windows. |
| `fs2` | `0.4` | `0.4.3` | [0.4.3](https://crates.io/api/v1/crates/fs2) | Retain. |
| `regex` | `1` | `1.13.1` | [1.13.1](https://crates.io/api/v1/crates/regex) | Retain. |
| `serde` | `1` | `1.0.229` | [1.0.229](https://crates.io/api/v1/crates/serde) | Retain. |
| `serde_json` | `1` | `1.0.154` | [1.0.154](https://crates.io/api/v1/crates/serde_json) | Retain. |
| `sha2` | `0.10` | `0.10.9` | [0.11.1](https://crates.io/api/v1/crates/sha2) | Consider separately. Requires Rust `>=1.88`; review digest APIs and prove unchanged identity hashes. |
| `tempfile` | `3` | `3.27.0` | [3.27.0](https://crates.io/api/v1/crates/tempfile) | Retain. |
| `toml` | `0.8` | `0.8.23` | [1.2.1+spec-1.1.0](https://crates.io/api/v1/crates/toml) | Consider separately. Requires Rust `>=1.85`; verify definition parsing, accepted syntax, serialization, and diagnostic behavior. |

The `dirs` assessment comes from comparing published
[6.0.0](https://static.crates.io/crates/dirs/dirs-6.0.0.crate) and
[7.0.0](https://static.crates.io/crates/dirs/dirs-7.0.0.crate) source archives;
the upstream Codeberg site was unavailable during research. The
[sha2 changelog](https://github.com/RustCrypto/hashes/blob/master/sha2/CHANGELOG.md)
describes digest-type, feature, and backend API changes. The
[toml changelog](https://github.com/toml-rs/toml/blob/main/crates/toml/CHANGELOG.md)
describes parser replacement, serialization API changes, and TOML 1.1
support. The suffix `+spec-1.1.0` is build metadata, not a prerelease marker.

Transitive Cargo dependencies were not independently audited. Review
transitive changes produced by the three selected direct updates. Avoid an
unrelated blanket lockfile refresh.

## CI actions and floating inputs

| Item | Repository baseline | Verified target or status | Recommendation and compatibility cost |
| --- | --- | --- | --- |
| `actions/checkout` | `@v4` | [v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1), or channel `@v7` | Consider separately from guest updates. Uses Node 24 and requires runner `>=2.327.1`; authenticated Git in container actions requires `>=2.329.0`. Review credential behavior and fork checkout defaults. |
| `actions/upload-artifact` | `@v4` | [v7.0.2](https://github.com/actions/upload-artifact/releases/tag/v7.0.2), or channel `@v7` | Consider with checkout. Uses Node 24. Preserve archive behavior for the workflow's multiple upload paths and verify artifacts on all four native targets. |
| Rust | Floating `stable` in guests and `dtolnay/rust-toolchain@stable` in CI | [Stable manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml) reports `1.100.0`, dated 2026-11-12 | No numeric repository pin to bump. Fresh resolutions already follow stable; existing guest artifacts do not refresh themselves. |
| SmolVM | Separately installed, unpinned executable | [v1.34.4](https://github.com/smol-machines/smolvm/releases/tag/v1.34.4), published 2026-11-13 | No bundled version to bump. Establish a tested runtime version before changing compatibility guidance. SmolVM version participates in base identity. |
| Claude Code | Floating npm install in `templates/claude.smlbx` | No numeric pin | A fresh build already resolves the available release. Pinning would add a new policy decision. |
| OpenCode | Floating npm install in `templates/opencode.smlbx` | No numeric pin | Same selection behavior as Claude Code. |
| Hermes Agent | Floating installer in `templates/hermes.smlbx` | No numeric pin | No numeric bump identified. Pinning installer and payload would add a new policy decision. |

Action compatibility details come from the tagged
[checkout README](https://github.com/actions/checkout/blob/v7.0.1/README.md),
[checkout action definition](https://github.com/actions/checkout/blob/v7.0.1/action.yml),
[upload-artifact action definition](https://github.com/actions/upload-artifact/blob/v7.0.2/action.yml),
and [upload-artifact v7 release notes](https://github.com/actions/upload-artifact/releases/tag/v7.0.0).
The current workflow uses GitHub-hosted native runners. Confirm the actual
runner version in each selected CI run rather than inferring compatibility
from a runner image name.

Distribution packages installed without versions, Docker's floating Alpine
smoke-test image, and fetched skill content are moving inputs rather than
numeric pins. Model selection is also a separate product decision. Broader
pinning and download-verification policy remains covered by
[Feature 0008](feature-0008-provisioning-download-integrity.md).

## Recorded scope decision

All listed update components are included in Feature 0016. The selected
alternatives are uv `0.13.8`, Ruff `0.17.5`, .NET channel `10.0` LTS, and
EF `10.0.15`. Node moves to channel `26.x`; Debian moves to `trixie-slim`.
The actions use exact tags `v7.0.1` for checkout and `v7.0.2` for artifact
upload. The feature's approved target table specifies the remaining tool
and crate versions.

The selection includes the compatibility work for Debian, Node, .NET, Rust
dependencies, and CI actions in this feature. Unchanged pins and previously
floating inputs remain unchanged. The feature requires distinct acceptance
evidence for each migration, all four profiles, both Linux guest
architectures, and all four native release targets.
