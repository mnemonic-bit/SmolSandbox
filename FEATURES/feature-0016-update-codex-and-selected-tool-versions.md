# Feature 0016: Update Codex, development tools, runtimes, and CI

## Status

In progress

The user selected every update component listed in the version analysis on
2026-11-13. The required scope includes Codex, shared development tools,
guest runtime migrations, three Rust dependency upgrades, and both GitHub
Actions upgrades. The scope table below resolves alternative targets.
The specification was prepared without software changes. Implementation
results and pending native and guest acceptance checks are recorded in
[Feature 0016 validation](feature-0016-validation.md). The feature remains
`In progress` until every mandatory criterion has passed.

## Outcome and baseline

Newly generated Codex definitions install exactly `@openai/codex@0.169.0`.
All four generated definitions use Debian trixie, Node 26, .NET 10, and the
selected shared tool versions. The host application uses the selected Rust
dependencies, and CI uses the selected action releases. Users can adopt new
definitions explicitly while retaining their existing definitions and
persistent harness state.

Specification preparation observed commit
`583d5db794c77f91f5187495a3f34b4df525e3e9` and application version `0.8.1`
on 2026-11-13. The working tree already contained the proposed
[Feature 0015 command rename](feature-0015-rename-command-to-smolsbx.md).
Feature 0016 does not implement that rename.

At this baseline, `templates/codex.smlbx` installs Codex `0.164.0`.
`Profile::template` in `src/config.rs` embeds the template, and
`codex_template_preserves_provisioning_tools_pins_and_verification` asserts
the exact installation command. Shared development tools are repeated in
all four `templates/*.smlbx` files.

The [official OpenAI changelog](https://learn.chatgpt.com/docs/changelog)
records Codex CLI `0.169.0` on 2026-11-13. The
[published package metadata](https://registry.npmjs.org/@openai%2fcodex/0.169.0)
declares Node `>=16` and Linux x64/ARM64 optional packages. These observations
establish release availability; they do not establish successful guest
provisioning or compatibility with the repository's persistent Codex state.

## Approved scope and targets

Every row below is required. The implementer owns the coordinated updates,
compatibility adjustments, documentation, and acceptance evidence.

| Item | Baseline | Required target | Affected surface |
| --- | --- | --- | --- |
| Codex CLI | `0.164.0` | `0.169.0` | Codex template |
| `skills` CLI | `1.7.0` | `1.8.1` | Codex template |
| codebase-memory-mcp | `v0.11.0` | `v0.12.1` installer and release | Codex template |
| Vitest | `5.0.1` | `5.1.0` | All four templates |
| Playwright npm library and test runner | `1.63.0` | `1.65.0` for both packages | All four templates |
| Playwright Python binding | `1.63.0` | `1.65.1` | All four templates |
| `uv` | `0.12.17` | `0.13.8` | All four templates |
| `pytest` | `9.1.1` | `9.2.0` | All four templates |
| Ruff | `0.16.8` | `0.17.5` | All four templates |
| .NET SDK | Channel `8.0` | Channel `10.0` LTS | All four templates |
| `dotnet-ef` | `8.0.31` | `10.0.15` | All four templates |
| `dotnet-dump` | `8.0.547301` | `10.0.750501` | All four templates |
| `dotnet-trace` | `8.0.547301` | `10.0.750501` | All four templates |
| `dotnet-counters` | `8.0.547301` | `10.0.750501` | All four templates |
| Node.js | NodeSource `setup_22.x` | NodeSource `setup_26.x` | All four templates |
| Debian image | `debian:bookworm-slim` | `debian:trixie-slim` | All four templates |
| Rust `dirs` | Range `6`, resolved `6.0.0` | Range `7`, resolved `7.0.0` | `Cargo.toml` and `Cargo.lock` |
| Rust `sha2` | Range `0.10`, resolved `0.10.9` | Range `0.11`, resolved `0.11.1` | `Cargo.toml` and `Cargo.lock` |
| Rust `toml` | Range `0.8`, resolved `0.8.23` | Range `1.2`, resolved `1.2.1+spec-1.1.0` | `Cargo.toml` and `Cargo.lock` |
| `actions/checkout` | `@v4` | `@v7.0.1` | `.github/workflows/rust.yml` |
| `actions/upload-artifact` | `@v4` | `@v7.0.2` | `.github/workflows/rust.yml` |
| Application package | `0.8.1` | `0.9.0` from this baseline | Matching manifest and lockfile entries |

The user selected all update components, rather than simultaneous installation
of every alternative release. The selected uv and Ruff targets cross their
minor-version compatibility boundaries. EF `10.0.15` matches the selected
.NET 10 default; EF `8.0.34` and `11.0.0` are not additional global installs.
The patch-only uv and Ruff alternatives remain research observations.

Node and .NET retain channel-based selection. Record the actual versions
installed during validation; the analysis observed Node `26.13.1` and .NET
SDK `10.0.404`. Debian retains a named image tag. This feature does not add
immutable image digests, exact SDK patch pins, or pins for previously floating
harness installers. Retain `tsx`, `debugpy`, the other eight direct Rust
dependencies, and existing floating Rust toolchain selection.

The dated [version analysis](feature-0016-version-update-research.md) supplies
release sources and compatibility considerations. Recheck the availability
of each selected target before implementation. A newer release does not
automatically replace a target in this table. If a selected target is
unavailable or incompatible, record the evidence and revise the specification
before substituting another target or omitting the component.

## Required implementation

### Codex provisioning

1. Change the installation command in `templates/codex.smlbx` to
   `npm install -g @openai/codex@0.169.0`.
2. Update the independent exact-command expectation in
   `codex_template_preserves_provisioning_tools_pins_and_verification` in
   `src/config.rs`.
3. Review `four_templates_are_complete_and_isolated`,
   `codex_template_default_hosts_are_exposed_by_definition`, and
   `codex_template_seed_settings_are_parsed_and_explicit` for regressions.

Retain the order that installs Codex before codebase-memory-mcp. Preserve
the installer checksum verification, portable Linux artifact selection,
Codex-only MCP registration, and prepared-home snapshot.

Retain the existing model `gpt-5.6-sol`, reasoning settings, context settings,
permissions, trust entry, status line, and skill selection. The Codex package
update does not select a new model or enable new optional Codex features.

Preserve `codex --no-daemon` for workspace launch and
`codex login --device-auth` for login. Verify both workflows with `0.169.0`.
If the new release cannot support either workflow, record the observed
failure and revise the feature scope before changing application policy.

### Definition adoption and lifecycle

Follow [RUST-APPLICATION.md](../RUST-APPLICATION.md#template-adoption).
Document generating a separate definition, comparing the files, and
explicitly selecting or editing the desired definition. For the current
command name, the generation step is:

```text
smolsandbox create-config --harness codex --smolfile ./codex-0.169.0.smlbx
```

An executable update must not rewrite an existing definition. An unchanged
selected definition must continue to use its existing tool pins and runtime
selection. A user who
selects updated definition bytes obtains a new base generation under the
existing identity rules. Normal reconciliation and explicit replacement
continue to follow the operating reference.

Preserve every profile's persistent harness home and missing-files-only
seeding. Do not reset credentials or replace existing settings to make an
upgrade pass.
Record any state-format migration performed by upstream Codex during the
real guest check; do not promise that an older Codex can read migrated state
without observed evidence.

Preserve network allowlists, machine-creation options, identity algorithms,
and `REVISION` in `src/lifecycle.rs`. Changed template bytes already select a
new base generation. If observed provisioning failures require an additional
host, record the required endpoint and affected provisioning step before
adding the host to the affected template. Review login-only hosts separately;
do not broaden all profiles' authentication allowlists by default.

If an implementation adjustment changes Rust machine-creation policy for
unchanged definition bytes, apply the revision and independent-test rules in
[AGENTS.md](../AGENTS.md).

### Application version and documentation

During implementation, increment the application minor version in
`Cargo.toml` and the matching `smolsandbox` package entry in `Cargo.lock`.
From the observed `0.8.1` baseline, the resulting feature version is `0.9.0`.
If another feature has changed the baseline, derive the bump from the actual
implementation baseline. If the combined scope introduces a breaking
application interface change, apply the repository's major-version rule.

Update transitive lockfile entries required by the three selected Rust
upgrades. Preserve unrelated dependency resolutions. Record any unavoidable
resolution changes in the validation record. Add template-adoption guidance
and guest toolchain migration guidance to the active operating reference.
Preserve historical version observations in earlier specifications and
validation records. Do not replace historical `0.164.0` references merely
to obtain a zero-match search.

If Feature 0015 has been implemented first, use the resulting command and
build-target names in active instructions and validation commands. Do not
reintroduce the old command as part of the version update.

## Coordinated migration requirements

Update corresponding independent provisioning expectations for every selected
template pin and channel. Make compatibility adjustments to provisioning
commands and Rust API calls where the selected migrations require them.
Preserve unrelated application behavior.

### Debian, Node, and .NET

Migrate all four templates to `debian:trixie-slim`, NodeSource `setup_26.x`,
and .NET channel `10.0`. Resolve renamed or replaced distribution packages,
including the current `libicu72` and `libssl3` entries, against trixie's
package metadata. Update comments and version-specific instructions. Retain
Python tooling, Rust tooling, timezone setup, nested Podman/Buildah, Docker
compatibility commands, and container storage behavior.

Provision every profile on Linux x64 and ARM64. Record the resolved image,
Node, Python, .NET SDK, and runtime versions. Verify npm engine requirements
for every selected global tool. Run the existing Docker Compose and nested
container smoke checks. Validate each profile's harness launch and persistent
home seeding after the shared runtime migrations.

Install global EF `10.0.15` with the .NET 10 default. Do not retain .NET 8 as
an additional default runtime or global EF version. Explain that projects
requiring another runtime or EF major need an explicitly edited user
definition or a project-local tool manifest. Verify restore, build, and EF
migrations against a .NET 10 sample using EF 10.

Install all three diagnostic tools at `10.0.750501`. Verify dump collection
and analysis, trace collection, and counter discovery and collection against
a sample guest process. Record the runtime used; package version output
alone does not establish runtime compatibility.

### MCP and development tools

Update the codebase-memory-mcp installer tag and `CBM_DOWNLOAD_URL` tag to
`v0.12.1` together. Verify portable Linux x64 and ARM64 execution, checksum
handling, and Codex-only registration. Exercise an actual MCP query after
indexing a small project. Document the upstream one-time index rebuild for
existing projects; preserve the persistent Codex home during the update.

Use skills CLI `1.8.1` with the existing noninteractive Codex-global command
and the same `grill-me` and `grilling` selection. Verify installed skill
discovery in the prepared home and after persistent-home seeding.

Update npm `playwright` and `@playwright/test` together to `1.65.0`. Update
the Python binding to `1.65.1`. Install and launch Chromium through both
bindings on both guest architectures. Record their browser revisions; do
not infer equal browser revisions from similar package versions.

Run a small TypeScript test with Vitest `5.1.0`. Run a Python test through
pytest `9.2.0`. Exercise environment creation and dependency resolution with
uv `0.13.8`. Compare Ruff `0.17.5` lint and formatting behavior on
representative Python code and document observed migration effects. Preserve
the pipx isolation and executable exposure used by the templates.

### Rust dependency and CI migrations

Change the three manifest ranges and lockfile resolutions to the approved
targets. Adapt Rust API calls to the selected crate versions. Verify that
`dirs` preserves application home and state paths on every native target,
including Windows. Verify that `sha2` produces identical persisted identity
hashes for fixed inputs. Preserve parsing of existing definitions and state
files under the new `toml` crate. Record any deliberately accepted TOML 1.1
syntax separately from compatibility with existing TOML 1.0 files.

Retain floating Rust `stable` selection. Record the actual toolchain used
for validation and verify that the resolved dependency graph meets its
minimum Rust requirements.

Use `actions/checkout@v7.0.1` and `actions/upload-artifact@v7.0.2` in the
existing workflow. Verify runner compatibility with their Node 24 runtime
and checkout's documented minimum runner versions. Preserve workflow
permissions, triggers, the four native targets, both test configurations,
artifact naming, archive behavior, and `if-no-files-found: error`.
Distributed builds continue to exclude `test-adapter`.

Changing floating installers to immutable pins or redesigning download
integrity belongs to [Feature 0008](feature-0008-provisioning-download-integrity.md)
unless the user explicitly includes that work here.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | A new Codex definition contains exactly `npm install -g @openai/codex@0.169.0`. Codex precedes MCP installation. | Inspect the parsed embedded template and generate a definition to a fresh path. Run the updated provisioning regression. |
| AC-02 | All template pins and channels match the approved table. Changes beyond pins implement the required compatibility adjustments. Template isolation, seed settings, verification, and unrelated host defaults remain valid. | Review all four parsed templates, the implementation diff, and the four named template regressions. Record the reason for each compatibility adjustment. |
| AC-03 | Existing definition bytes and persistent harness files survive configuration generation and application seeding. Unchanged definition bytes retain identity; updated definition bytes select a different base generation. | Run existing configuration, seeding, and identity regressions, including `selected_bytes_and_version_define_base_identity`. Compare retained definition and harness fixtures before and after the relevant operations. |
| AC-04 | Fresh Codex guests install `0.169.0`, register codebase-memory-mcp, and support the existing launch and device-auth workflows. Existing-home launch preserves credentials and settings. | Record real SmolVM provisioning and guest `codex --version` output on Linux x64 and ARM64. Exercise login, workspace launch, MCP access, and a retained-home restart. Record the native host for each guest. |
| AC-05 | The application manifest and lockfile have matching versions with the required bump. The three selected crate ranges and resolutions match the table; other resolution changes are justified by the selected upgrades. | Inspect `Cargo.toml`, `Cargo.lock`, and the implementation diff. |
| AC-06 | Every approved update is implemented on its declared surface and has passing compatibility checks. No component is silently omitted or replaced with an alternative target. | Compare every scope-table row against the template, dependency, workflow, and validation records. |
| AC-07 | Repository checks pass, and active documentation explains explicit template adoption. | Run the commands below and review the operating-reference changes. |
| AC-08 | All four profiles provision on trixie with Node 26 and .NET 10 on Linux x64 and ARM64. Retained development tooling, nested containers, harness launch, and home seeding succeed. | Record all eight profile/guest-architecture combinations, resolved runtime versions, provisioning output, Docker smoke checks, and profile launch observations. |
| AC-09 | EF `10.0.15` performs a migration against an EF 10 sample, and all three diagnostic tools perform their declared operations at `10.0.750501`. | Record sample project/runtime versions and commands for restore, build, EF migrations, dumps, traces, and counters on both guest architectures. |
| AC-10 | Both selected Playwright bindings install and launch Chromium. Vitest, pytest, uv, Ruff, skills, and MCP pass the declared smoke checks at their selected versions. | Record version output and smoke commands on both guest architectures; include browser revisions, skill discovery, an MCP query, and Ruff migration observations. |
| AC-11 | Rust dependency upgrades preserve state paths, existing definition/state parsing, and persisted identity hashes. | Run compatibility regressions with fixed baseline fixtures and expected hashes. Record native results for all four release targets, including Windows path behavior. |
| AC-12 | Both selected CI action releases execute on all four native targets and upload the expected release artifacts. | Record CI run links, runner versions, action versions, native check results, and downloaded artifact names and contents. |

## Verification and evidence

Run the repository checks after implementation:

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --features test-adapter --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --features test-adapter --all-targets
cargo build --locked --release --bin smolsandbox
cargo run --locked --example check_baseline_independence
```

During implementation, create `FEATURES/feature-0016-validation.md`. Record
the source revision, selected updates, commands, exit codes, native host,
guest architecture, package versions, and results for AC-01 through AC-12.
Use `passed`, `failed`, `not_run`, or `inconclusive`. Automated mocked tests
do not establish guest provisioning, authentication, or interactive behavior.

Append implementation results and pending native or real guest checks to
[Feature 0010 validation](feature-0010-validation.md), preserving previous
observations and their source revisions. Mark Feature 0016 `Implemented`
only after every mandatory criterion has passed. Preserve unrelated pending
cutover gates.
