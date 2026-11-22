# Feature 0018 validation

Status: `In progress`. The Codex pin, independent regression expectation,
matching application versions, and active adoption guidance are updated.
Implementation acceptance remains pending real SmolVM guest checks.

## Source and environment

Validation date: 2026-11-22. The implementation is an uncommitted working
tree based on `7151713dcf703e02bcca772994243bf2b8e0ae92`, application
`0.12.0`. The Feature 0018 specification was already present as an untracked
file when implementation began. Historical specifications and observations
retain their original versions and source revisions.

The native host is Linux x86-64 (`x86_64-unknown-linux-gnu`), kernel
`6.18.60`. Rust is `1.100.0 (0ef1bc386 2026-11-09)` and Cargo is
`1.100.0 (3acc79c0c 2026-11-05)`. No SmolVM executable is on `PATH`, and
`/dev/kvm` is absent. No real guest architecture or guest Codex version was
observed.

The software diff SHA-256 is
`1a995172adcaf2985c9c19bb77ae2482dff1d036404a4a4273d58584e879d367`,
computed from `git diff --binary HEAD -- Cargo.toml Cargo.lock src/config.rs
templates/codex.smlbx`. Documentation is excluded from this fingerprint.

## Upstream availability and declared compatibility

Availability: `passed`. Read-only official npm registry metadata requests
and tarball probes returned HTTP 200 on 2026-11-22.

Reproduction command: `python3 /tmp/feature-0018/verify_upstream.py >
/tmp/feature-0018/upstream.log`, exit 0. The script SHA-256 is
`a97fdc1a745b3ed7768f3774c6bd75cb1df83c8a3d4959f6784ac72dd123bf33`;
the log SHA-256 is
`e0d5658702092141ff79334cdf44cdec28143f5945ba3579f235b8ce65ad9545`.
The log retains metadata, tarball probes, verified main-package integrity,
tagged source excerpts, and release metadata. Temporary logs are local
session artifacts; the source links and observed values are recorded here.

| Package metadata | Observed version | Declared requirements |
| --- | --- | --- |
| [Main package](https://registry.npmjs.org/@openai%2Fcodex/0.171.0) | `0.171.0` | Node `>=16` |
| [Linux x64 package](https://registry.npmjs.org/@openai%2Fcodex/0.171.0-linux-x64) | `0.171.0-linux-x64` | OS `linux`, CPU `x64`, Node `>=16`; no declared `libc` field |
| [Linux ARM64 package](https://registry.npmjs.org/@openai%2Fcodex/0.171.0-linux-arm64) | `0.171.0-linux-arm64` | OS `linux`, CPU `arm64`, Node `>=16`; no declared `libc` field |

The main package declares optional aliases
`@openai/codex-linux-x64 = npm:@openai/codex@0.171.0-linux-x64` and
`@openai/codex-linux-arm64 = npm:@openai/codex@0.171.0-linux-arm64`.
The [main tarball](https://registry.npmjs.org/@openai/codex/-/codex-0.171.0.tgz)
was downloaded and inspected. HEAD requests established availability of the
[Linux x64 tarball](https://registry.npmjs.org/@openai/codex/-/codex-0.171.0-linux-x64.tgz)
and [Linux ARM64 tarball](https://registry.npmjs.org/@openai/codex/-/codex-0.171.0-linux-arm64.tgz).
Neither Linux binary package was executed.

The [official release metadata](https://api.github.com/repos/openai/codex/releases/tags/rust-v0.171.0)
reports a normal release published at `2026-11-20T00:44:47Z`.
The [tagged npm wrapper](https://github.com/openai/codex/blob/rust-v0.171.0/codex-cli/bin/codex.js)
selects `x86_64-unknown-linux-musl` or `aarch64-unknown-linux-musl` and
forwards CLI arguments. The [tagged installation requirements](https://github.com/openai/codex/blob/rust-v0.171.0/docs/install.md)
name Debian 10+, 4 GB minimum RAM, and 8 GB recommended RAM. The template's
Debian 13, Node 26, and 8 GB satisfy those declared requirements; this is a
source-based inference, not a guest observation.

Interface presence: `passed` in tagged upstream source. The
[TUI CLI](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/tui/src/cli.rs)
retains `--no-daemon`; the
[login CLI](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/cli/src/main.rs)
retains `--device-auth`; the
[MCP CLI](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/cli/src/mcp_cmd.rs)
retains `list`, `get`, `add`, `remove`, `login`, and `logout`. Source presence
does not establish successful launch, authentication, or MCP queries.

## Scope review

Independent source review: `passed`, no actionable findings. Read-only
byte comparisons exited 0: `templates/codex.smlbx` equals its baseline with
only `0.169.0` replaced by `0.171.0`; the other three templates and
`src/lifecycle.rs` are unchanged. Parsed installation occurs exactly once
before MCP installation. Both package entries select `0.12.0`.

The existing independent expectation now names the exact new install
command. `REVISION` remains `5`. Other tool versions, network allowlists,
model and seed settings, selected skills, prepared-home snapshot, launch,
device authentication, and missing-files-only seeding remain unchanged.

The operating reference names `0.171.0` and gives explicit generation,
comparison, and selection commands. Existing definitions retain their
pins; updating the executable does not rewrite those files.

## Repository checks and fixtures

All commands below ran from the repository root after the software edits,
against the source fingerprint and native toolchain recorded above. Logs
are under `/tmp/feature-0018/`.

| Command | Result | Exit code and evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0; `fmt.log` |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0; `clippy.log`; default features |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0; `clippy-adapter.log` |
| `cargo test --locked --all-targets` | passed | 0; `test.log`; 118 tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0; `test-adapter.log`; 185 tests |
| `cargo build --locked --release --bin smolsbx` | passed | 0; `release.log`; production build without `test-adapter` |
| `cargo run --locked --example check_baseline_independence` | passed | 0; `independence.log`; isolated source checks and compiled tests with empty `PATH` |

All four named template regressions passed in the complete test suites:
`four_templates_are_complete_and_isolated`,
`codex_template_default_hosts_are_exposed_by_definition`,
`codex_template_preserves_provisioning_tools_pins_and_verification`, and
`codex_template_seed_settings_are_parsed_and_explicit`. Existing configuration,
selected-byte identity, allowlist, replacement, and seeding regressions also
passed in the applicable feature configurations.

Fresh generation and retained-file fixture command: `python3
/tmp/feature-0018/fixtures.py > /tmp/feature-0018/fixtures.log 2>&1`, exit 0.
The production release executable reports `smolsbx 0.12.0`. With empty
`PATH` and a missing SmolVM selection, the executable generated all four
definitions byte-for-byte from the templates. The Codex destination used the
documented `create-config --harness codex --smolfile
./codex-0.171.0.smlbx` example. Parsing that file found exactly one new Codex
install command before MCP installation. Repeated generation returned 1
and retained each destination's original bytes.

The fixture retained a baseline `codex-current.smlbx` and synthetic
`auth.json`, `config.toml`, `skills/custom/SKILL.md`, and
`sessions/retained.jsonl` files. SHA-256 comparisons before and after
configuration generation and two adapter-backed starts were equal. The old
and new definitions selected base IDs `679114224b0b` and `6cecaa883cfc`,
respectively, with test runtime version `smolvm test 1.19.2` and unchanged
revision `5`. Both starts recorded the exact missing-files-only seed command.
The second start recovered the existing workspace machine.

The test double records seeding and harness invocations; it does not execute
the guest copy or Codex. These fixture results establish host retention,
identity selection, and unchanged seeding commands. Real guest seeding,
authentication retention, restart, and state migration remain `not_run`.

## Acceptance and remaining evidence

| Criterion | Result | Evidence or next action |
| --- | --- | --- |
| AC-01 | passed | Fresh release generation, parsed exact command/order, and independent exact-command regression |
| AC-02 | passed | Independent diff review and four focused template regressions exited 0 |
| AC-03 | passed | Existing configuration, identity, and seeding regressions; retained definition/home byte comparisons; distinct base IDs. Execution is adapter-backed as described above |
| AC-04 | not_run | SmolVM and `/dev/kvm` are absent; real guest checks are unavailable |
| AC-05 | passed | Matching manifest and lockfile versions `0.12.0` |
| AC-06 | passed | Active guidance names `0.171.0`; generation example exited 0; historical observations preserved |
| AC-07 | passed | All seven required commands exited 0 after the final software edit |

| Guest architecture | Provisioning and version | Launch and device authentication | MCP query | Retained-home restart |
| --- | --- | --- | --- | --- |
| Linux x64 | not_run | not_run | not_run | not_run |
| Linux ARM64 | not_run | not_run | not_run | not_run |

On compatible native hosts, run the operating reference's real guest
procedure for both Linux guest architectures. Record the source revision,
application and SmolVM versions, native host and guest architectures,
commands, exit codes, `codex --version`, launch, device authentication, MCP
query, and restart with retained settings and authentication. Preserve the
persistent home. Record any observed upstream state migration. Compatibility
of migrated state with older Codex versions remains unestablished.

Linux ARM64, macOS ARM64, and Windows x86-64 native checks are `not_run`
for this working tree. No hosted CI run was requested or observed. Local
mocked checks do not establish those native targets or real guest behavior.
Keep Feature 0018 `In progress` until every mandatory criterion passes.
