# Feature 0016 validation

Status: `In progress`. The selected software updates are implemented;
mandatory real guest and native CI acceptance remains pending.

The implementation is an uncommitted working tree based on
`583d5db794c77f91f5187495a3f34b4df525e3e9`, application `0.9.0`, validated
on 2026-11-13. The native host is Linux x86-64
(`x86_64-unknown-linux-gnu`) with Rust/Cargo `1.100.0`. The working tree
already contained the Feature 0015 specification; this change does not
implement its command rename. Previous observations and source revisions
in other validation records remain unchanged.

The final software diff SHA-256 is
`a8b3c4831facfaff5de14d1476789e6f65c6c96f7b6fdaf21ff41d80c35e08fa`,
computed from `git diff --binary HEAD -- Cargo.toml Cargo.lock
.github/workflows/rust.yml src tests templates`. Documentation is excluded
from that fingerprint.

## Selected updates and compatibility adjustments

Exact release availability: `passed`. Primary npm, PyPI, NuGet, crates.io,
NodeSource, .NET, Debian, MCP, and GitHub action metadata were rechecked
against the approved targets on 2026-11-13. The exact primary-source URLs
are retained in the [version analysis](feature-0016-version-update-research.md).
Availability does not establish successful installation or runtime behavior.

| Surface | Implemented targets |
| --- | --- |
| Codex template | Codex `0.169.0`, skills `1.8.1`, MCP installer and release `v0.12.1` |
| All four templates | Debian `trixie-slim`, NodeSource `setup_26.x`, .NET channel `10.0` |
| Shared npm tools | Vitest `5.1.0`, Playwright and `@playwright/test` both `1.65.0` |
| Shared Python tools | Playwright `1.65.1`, uv `0.13.8`, pytest `9.2.0`, Ruff `0.17.5` |
| Shared .NET tools | EF `10.0.15`; dump, trace, and counters each `10.0.750501` |
| Cargo | `dirs = "7"` resolves `7.0.0`; `sha2 = "0.11"` resolves `0.11.1`; `toml = "1.2"` resolves `1.2.1+spec-1.1.0` |
| CI | Checkout `v7.0.1`, upload-artifact `v7.0.2` |
| Application | Matching manifest and lockfile version `0.9.0` |

Trixie requires `libicu76` and `libssl3t64` in place of Bookworm's
`libicu72` and `libssl3`. Debian package metadata confirms both replacement
packages on amd64 and arm64:
[libicu76](https://packages.debian.org/trixie/libicu76),
[libssl3t64](https://packages.debian.org/trixie/libssl3t64).

`sha2` no longer implements hexadecimal formatting on the whole digest
array. Production hashing now writes each digest byte as two lowercase
hexadecimal digits. Independent integration fixtures were adapted
separately. Fixed SHA-256 values, immutable-option hashes, and base IDs
passed against the old dependencies before the upgrade and pass against
the selected dependencies. `REVISION` remains `5`.

Lockfile changes are limited to the selected crates and their dependency
closures: the SHA-256 digest/block-buffer/crypto-common/cpufeatures family,
new hybrid-array and const-oid, removal of generic-array and version_check,
and TOML serde-spanned/datetime/parser/writer/winnow replacements. All eight
unrelated direct dependency resolutions remain unchanged. The maximum
declared minimum Rust version in the resolved graph is `1.88`, from `sha2`;
the observed toolchain is newer. Metadata audit: `passed`, exit 0,
`/tmp/feature-0016/dependency-audit.json`.

Published `dirs` 6/7 sources retain Windows `home_dir()` selection through
the user profile known folder. The changed `preference_dir()` is unused by
the application. A native-home regression checks `.smolsandbox` under the
runner's user home. Native Windows execution remains `not_run`. Retained
TOML 1.0 fixtures preserve definition bytes, parsed settings, and allowlists
for all four profiles. No application policy deliberately adopts new TOML
1.1 syntax; the upgraded parser accepts its upstream syntax by default.
Existing JSON state and inventory fixtures remain in the full suites.

Source comparison: `passed`. Parsed definitions preserve network allowlists,
machine options, Codex settings, profile isolation, Compose dispatch, nested
container storage, and unrelated floating installers. MCP installer source
at `v0.12.1` retains portable Linux x64/ARM64 selection and checksum
verification before activation. The operating reference explains explicit
definition adoption, runtime/EF migration, separate browser validation,
and the upstream one-time MCP index rebuild.

An observed skills `1.8.1` path change required a provisioning adjustment.
The unchanged noninteractive command installs the selected skills under
`/root/.agents/skills`; its installed source defines that directory for
Codex even in copy mode. The original prepared Codex home lacked both
`grill-me/SKILL.md` and `grilling/SKILL.md` (check exit 1). The template now
materializes those two skill directories under `/root/.codex/skills` after
installation and before the existing home snapshot. The exact copy command
and a prepared-home copy passed in the container, exit 0. The regression
failed with the command absent and passed after the fix. Persistent-home
seeding retains the existing missing-files-only policy. The initial container
script predates this adjustment; the copy was checked separately in that
container. Log: `/tmp/feature-0016/skills-copy-smoke.log`.

## Repository checks

Final command results are recorded below after the last software edit.
Initial compilation found whole-digest `LowerHex` uses in independent test
fixtures; the compatibility edits above resolve those errors. Initial logs
are retained separately from final results under `/tmp/feature-0016/`.

| Command | Result | Exit code and log |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0; `verified-fmt.log` |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0; `verified-clippy.log` |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0; `verified-clippy-adapter.log` |
| `cargo test --locked --all-targets` | passed | 0; `verified-test.log`; 65 library, 12 application, 13 CLI, 8 fixed-data, 9 listing, 10 removal tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0; `verified-test-adapter.log`; 64 library, 64 application, 13 CLI, 8 fixed-data, 10 listing, 24 removal tests |
| `cargo build --locked --release --bin smolsandbox` | passed | 0; `verified-release.log`; production build excludes `test-adapter` |
| `cargo run --locked --example check_baseline_independence` | passed | 0; `verified-independence.log`; isolated checks and nine compiled test executables per feature mode with empty `PATH` |

Offline generation: `passed`, exit 0, `python3
/tmp/feature-0016/offline-config.py`. With an empty `PATH` and missing SmolVM,
the final release executable generated all four definitions byte-for-byte
from the current templates. Repeated creation returned 1 and preserved each
existing file. Help and version (`smolsandbox 0.9.0`) returned 0.
Log: `/tmp/feature-0016/verified-offline-config.log`.

## Additional Linux x64 container observations

These observations use an isolated Podman container with host networking.
They do not exercise SmolVM virtualization, guest egress enforcement,
authentication, interactive launch, retained-home restart, or ARM64.
The initial provisioning script was extracted from the Codex template's
parsed `[dev].init` array into `/tmp/feature-0016/provision-codex.sh`.
Its template SHA-256 was
`7112e9d2ac3d10e25064a4d29f9797d0332a8310c5ea99d8c5f1966a9994e027`.

The image `docker.io/library/debian:trixie-slim` resolved to digest
`sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2`,
architecture `amd64`, Debian `13.7`. The unchanged apt provisioning command
with the two replacement package names installed successfully. Runtime
inspection exited 0: Node `26.13.1`, Python `3.13.5`, .NET SDK `10.0.404`,
ASP.NET/Core runtime `10.0.15`, Rust `1.100.0`.
Logs: `/tmp/feature-0016/container-provision.log` and
`/tmp/feature-0016/container-runtimes.log`.

Codex version and help: `passed`, exit 0. `codex --version` reported
`codex-cli 0.169.0`; top-level and login help retain `--no-daemon` and
`--device-auth`. Authentication and interactive launch remain `not_run`.
Log: `/tmp/feature-0016/container-codex-cli.log`.

MCP indexing/query and Codex registration: `passed`, exit 0. The installed
`codebase-memory-mcp cli --json index_repository` indexed a two-line Python
sample as `feature0016-smoke`; `cli --json search_graph` returned
`feature0016_answer`. `codex mcp list` showed codebase-memory-mcp enabled.
The CLI warned that positional raw JSON arguments are deprecated; the
queries succeeded. Log: `/tmp/feature-0016/container-mcp-smoke.log`.

An independent JSON-RPC stdio check also passed, exit 0:
`/tmp/feature-0016/mcp-smoke.py` initialized MCP protocol `2024-11-05`,
indexed a small Python project, and queried its `add` function through
`search_graph`. Transcript: `/tmp/feature-0016/mcp-smoke.log`.

EF restore/build/migration: `passed`. The `net10.0` sample referenced
`Microsoft.EntityFrameworkCore.Sqlite` and
`Microsoft.EntityFrameworkCore.Design` at `10.0.15`. Under SDK `10.0.404`
and runtime `10.0.15`, these commands each exited 0:

```text
dotnet restore Sample.csproj
dotnet build Sample.csproj --no-restore
dotnet-ef migrations add Initial --project Sample.csproj --no-build
dotnet-ef database update --project Sample.csproj
dotnet run --project Sample.csproj --no-build
```

The initial `/tmp/feature-0016/dotnet-smoke.sh` then exited 1 at the
deprecated `dotnet-counters list --runtime-version 10.0` command, which
directed users to the online metrics documentation. That failure remains
recorded in `dotnet-smoke.log`. The follow-up
`/tmp/feature-0016/diagnostics-smoke.sh` exited 0. With all three tools at
`10.0.750501`, the follow-up discovered the sample process, exposed runtime
metric names in CSV, collected a trace and mini dump, and analyzed managed
threads:

```text
dotnet-counters ps
dotnet-counters collect --process-id "$SAMPLE_PID" --counters System.Runtime --duration 00:00:03 --format csv --output counters.csv
head -n 8 counters.csv
dotnet-trace collect --process-id "$SAMPLE_PID" --duration 00:00:03 --output sample.nettrace
dotnet-dump collect --process-id "$SAMPLE_PID" --type Mini --output sample.dmp
dotnet-dump analyze sample.dmp -c clrthreads -c exit
```

Artifacts were a 7.5 KB counter CSV, a 4.0 MB trace, and a 15 MB mini dump.
Log: `/tmp/feature-0016/diagnostics-smoke.log`. These container observations
do not establish the corresponding operations on both guest architectures.

Development-tool observations: `passed`. With Python `3.13.5`,
`pytest -q test_sample.py` passed one test through global pytest `9.2.0`.
uv `0.13.8` completed `uv venv .venv`, `uv sync`, and
`uv run pytest -q test_sample.py`; the latter passed the same test. The
sample project declared `pytest==9.2.0` as a dependency. The representative
Ruff sample contained two unused imports and an unformatted list assignment.
Ruff `0.16.8` and `0.17.5` produced identical lint JSON and formatted output
with these commands:

```text
ruff-baseline/bin/ruff check representative.py --output-format json
ruff check representative.py --output-format json
ruff-baseline/bin/ruff format --stdin-filename representative.py < representative.py
ruff format --stdin-filename representative.py < representative.py
```

Both lint commands returned 1 for the same expected unused-import findings;
both format commands returned 0. Comparisons of captured JSON and formatted
files returned 0. This bounded sample does not establish unchanged behavior
for every Ruff rule or project. The initial development-tool script exited
2 because its probe used the unsupported Ruff `--stdout` option. The
corrected `/tmp/feature-0016/ruff-vitest-smoke.sh` exited 0 and also ran
`vitest run math.test.ts --globals --maxWorkers 1`: one TypeScript test
passed with Vitest `5.1.0` and Node `26.13.1`. Logs:
`/tmp/feature-0016/devtools-smoke.log` and
`/tmp/feature-0016/ruff-vitest-smoke.log`.

The initial container provisioning command exited 125 at its final nested
Docker smoke command. `docker compose version` passed with Podman `5.4.2`
and podman-compose `1.3.0`. `docker run --rm docker.io/library/alpine echo
nested-podman-via-docker-ok` failed because the container's `/storage` path
was backed by overlayfs, which cannot host the configured nested overlay
driver. The SmolVM template uses an ext4 storage disk; this container result
does not establish failure or success on that disk. The final home snapshot
was not reached by the main script; the selected skill snapshot copy was
checked separately as described above.

The npm Playwright `1.65.0` browser smoke passed in the initial container:
Chromium revision `1254`, version `158.0.8127.18`, launched headless and
rendered a local HTML title. The first browser probe raced the unfinished
headless-shell download and failed; a follow-up selected the installed
Chromium executable explicitly and passed. Log:
`/tmp/feature-0016/container-browser-retry.log`. Container removal at the
nested-Docker failure interrupted the pending Python browser probe (exit
137).

The first reduced Python-browser container omitted `libatomic1`; the
bundled Playwright Node driver could not load `libatomic.so.1` (exit 127).
The complete template's apt provisioning had already installed `libatomic1`
through the development-tool dependencies. The reduced smoke image was
corrected to include that library before retrying. No template adjustment
was required for this reduced-image failure. Initial log:
`/tmp/feature-0016/python-browser-smoke.log`.

The corrected separate Python-browser check passed, exit 0:
`podman run --rm --name feature-0016-python-browser-retry --network host
-v /tmp/feature-0016/python-browser-smoke.sh:/smoke.sh:ro
docker.io/library/debian:trixie-slim sh /smoke.sh`. The script installed
Python Playwright `1.65.1`, ran `python3 -m playwright install --with-deps
chromium`, launched Chromium headless through the Python API, and checked
a rendered local HTML title. The observed Python browser revision was
`1254`, version `158.0.8127.18`, independently matching the npm observation.
Log: `/tmp/feature-0016/python-browser-retry.log`. Both browser observations
remain Linux x64 container results; ARM64 and SmolVM guest checks remain
`not_run`.

## Acceptance and remaining evidence

| Criterion | Result | Evidence or remaining action |
| --- | --- | --- |
| AC-01 | passed | Exact Codex install/order regression and offline generated definition comparison |
| AC-02 | passed | All-profile parsed pin/channel regressions; reviewed compatibility adjustments and preserved policy/settings |
| AC-03 | passed | Configuration preservation, selected-byte/base identity, missing-files-only seeding, and retained-home fixtures in both Rust feature modes |
| AC-04 | not_run | Record real Codex provisioning, device authentication, interactive workspace launch, MCP access, and retained-home restart on x64 and ARM64 SmolVM guests |
| AC-05 | passed | Matching `0.9.0`, exact crate targets, unchanged unrelated direct resolutions, and bounded transitive changes |
| AC-06 | inconclusive | Every selected surface is updated; guest and native compatibility requirements remain pending |
| AC-07 | passed | All seven required commands exited 0 after the final software edit; operating-reference adoption guidance is present |
| AC-08 | not_run | Record all eight profile/guest-architecture combinations with runtime versions, nested Docker/Compose checks, harness launch, and home seeding |
| AC-09 | inconclusive | Linux x64 container EF and diagnostic observations supplement pending x64/ARM64 guest checks |
| AC-10 | inconclusive | Both bindings launched Chromium and tool/MCP/skills smoke checks passed in x64 containers; both guest architectures remain required |
| AC-11 | inconclusive | Fixed compatibility regressions pass locally; native Linux ARM64, macOS ARM64, and Windows x64 results remain required |
| AC-12 | not_run | Run both selected actions on all four native targets; retain run URLs, runner versions, and downloaded artifact names/contents |

SmolVM is absent from `PATH`, and `/dev/kvm` is absent. This host cannot
provide the required real guest evidence. The other three native hosts and
GitHub Actions authentication are unavailable in this session.

| Profile | Linux x64 real guest | Linux ARM64 real guest |
| --- | --- | --- |
| Codex | not_run | not_run |
| Claude | not_run | not_run |
| OpenCode | not_run | not_run |
| Hermes | not_run | not_run |

| Native target | Local Rust checks | Selected CI actions and downloaded artifacts |
| --- | --- | --- |
| Linux x86-64 | passed | not_run |
| Linux ARM64 | not_run | not_run |
| macOS ARM64 | not_run | not_run |
| Windows x86-64 | not_run | not_run |

Both selected actions declare Node 24. Checkout requires runner
`>=2.327.1`; authenticated Git from Docker container actions requires
`>=2.329.0`. The workflow has no Docker container action. Upload-artifact's
tagged definition retains archive mode by default. Reviewed tagged sources:
[checkout](https://raw.githubusercontent.com/actions/checkout/v7.0.1/action.yml),
[runner requirements](https://raw.githubusercontent.com/actions/checkout/v7.0.1/README.md),
[upload-artifact](https://raw.githubusercontent.com/actions/upload-artifact/v7.0.2/action.yml).
Actual runner compatibility and uploaded artifacts remain unobserved.

On compatible native hosts, follow the operating reference's real guest
procedure with each profile and both Linux guest architectures. Record the
tested source revision, actual SmolVM version, resolved runtime/tool/browser
versions, command output and exit codes, upstream Codex state migrations,
and CI run URLs. Keep Feature 0016 `In progress` until every mandatory
criterion has passed. Existing Feature 0010 cutover gates remain pending.
