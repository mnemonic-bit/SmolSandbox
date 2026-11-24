# Feature 0019 validation

Status: `In progress`. Creation metadata, readable inventory, compatibility
handling, help, operating guidance, and independent regressions are implemented.
Acceptance remains pending native execution and the real SmolVM label round-trip
required by [Feature 0019](feature-0019-readable-machine-identity-metadata.md).

## Source and environment

Validation date: 2026-11-22. The tested implementation is an uncommitted working
tree based on `c2ed7d1bde477e837e1fb720d42044e5e4a15069`. Both package versions
are `0.13.0`; machine policy `REVISION` is `6`. The Feature 0019 specification
was already present as an untracked file when implementation began. Historical
observations and source revisions are preserved.

The native host is Linux x86-64 (`x86_64-unknown-linux-gnu`), kernel `6.18.60`.
Rust is `1.100.0 (0ef1bc386 2026-11-09)`; Cargo is
`1.100.0 (3acc79c0c 2026-11-05)`. `jq` is available at `/usr/bin/jq`.
`command -v smolvm` returned exit 1 and `/dev/kvm` is absent. No real SmolVM
runtime version, real guest result, or new hosted CI run was observed.

The creation fixtures use the Rust SmolVM test double with normalized versions
`smolvm test 1.19.2`, `smolvm test 1.19.2 build=λ\native`, and
`smolvm test 1.19.3 build=λ\native`. Listing fixtures also use
`smolvm 1.19.2`. Creation cases verify removal of trailing CR/LF while retaining
the remaining version bytes. These strings do not identify an installed real
runtime.

The software content-manifest SHA-256 is
`ccb0536170c55a6597fb2f6010c169a7c92062138c72faa90f85eaeaa4cc89ae`.
For reproduction, compute each named file's SHA-256, then hash the UTF-8 bytes
of the resulting path-to-digest JSON object using Python
`json.dumps(manifest, sort_keys=True, separators=(',', ':'))`, without a trailing
newline. The manifest covers these files:

```text
Cargo.lock
Cargo.toml
src/cli.rs
src/domain.rs
src/lib.rs
src/lifecycle.rs
src/listing.rs
src/metadata.rs
src/removal.rs
tests/application.rs
tests/cli_contract.rs
tests/identity_metadata.rs
tests/listing.rs
tests/removal.rs
```

Documentation is excluded from that fingerprint. The manifest and command logs
are local session artifacts under `/tmp/feature-0019/`; results and source
identity are recorded here for later sessions.

## Repository checks

All commands ran from the repository root. The final formatting, lint, and test
checks followed the last software edit. The release build excludes
`test-adapter` and `target/release/smolsbx --version` returned
`smolsbx 0.13.0`, exit 0.

| Command | Result | Exit code and evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0; `fmt.log` |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0; `clippy.log` |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0; `clippy-adapter.log` |
| `cargo test --locked --all-targets` | passed | 0; `test.log`; 132 tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0; `test-adapter.log`; 203 tests |
| `cargo build --locked --release --bin smolsbx` | passed | 0; `release.log`; production features |
| `cargo run --locked --example check_baseline_independence` | passed | 0; `independence.log`; isolated source checks, production build, and ten compiled test executables per configuration with empty `PATH` |

Earlier development runs exposed an `OsString` test comparison error, two
fixture expectations that contradicted existing authoritative inventory rules,
and two Clippy warnings in new unit tests. Those issues were corrected before
the final checks above. Final review also identified lossy conversion of
non-UTF-8 artifact paths; artifact and definition paths now fail before
lifecycle writes. The Unix regression verifies preservation of machines,
artifacts, locks, and harness state for both unsupported path overrides.

## Acceptance evidence

`passed` below describes the stated local verification. Real runtime
preservation is tracked separately under AC-13. Native Windows path execution
remains pending under AC-11.

| Criterion | Result | Evidence or next action |
| --- | --- | --- |
| AC-01 | passed | `every_profile_records_exact_inputs_for_all_machine_kinds` independently checks both label objects and exact key sets for all four profiles and all three creation kinds; inventory and listing expectations check retained metadata |
| AC-02 | passed | Exact independently assembled arrays match actual create arguments and recorded hashes; `resolved_creation_options_share_arguments_metadata_and_fixed_resource_hashes` preserves fixed digests |
| AC-03 | passed | `allowlist_is_a_set_and_ports_do_not_change_immutable_hash` and `creation_uses_selected_default_hosts_plus_normalized_additions_and_revision` check deduplication, selected defaults, additions, unchanged labels on reuse, and revision 6; all-profile fixtures independently enumerate login hosts |
| AC-04 | passed | `resolved_input_variants_record_exact_identity_changes_and_exclude_mutable_settings` checks artifact, harness-home, workspace, and runtime changes; all-profile cases check mount targets; byte-only edits change fingerprints and base IDs; resource variants retain fixed hashes |
| AC-05 | passed | Fresh mutable-setting variants preserve the exact immutable array; moved definitions and explicit name overrides retain original creation labels; secret sentinels do not enter create labels or reports |
| AC-06 | passed | Mixed inventory, full-value human output, shared/unknown rows, sorting, empty results, reference headers, and terminal-control escaping pass in listing fixtures |
| AC-07 | passed | Parsed schema-version-1 assertions cover new fields, nulls, provenance, and diagnostics; `published_json_filters_select_recorded_paths_and_exact_host_options` executes all three published jq examples successfully |
| AC-08 | passed | Metadata component mutations and recomputed-hash semantic contradictions preserve authoritative assessments; domain classification/purge inventory and removal preview comparisons prove descriptive labels do not establish ownership or change selection |
| AC-09 | passed | Mixed legacy/current fixtures cover reference-only path inference, other workspaces, partial metadata, custom names, missing historical files, and unchanged legacy lifecycle reuse |
| AC-10 | passed | Snapshot and call-log checks permit only version/inventory queries, preserve absent or invalid application state, and inspect recorded missing definition paths without reading them |
| AC-11 | inconclusive | Exact Unicode, spaces, equals signs, controls, Windows drive paths, and UNC fixtures pass; native Unix path and symlink checks pass. Native Windows path normalization and execution were not run on this Linux host |
| AC-12 | passed | Both package versions are 0.13.0; independent revision/name expectations use 6; replacement ordering, failure preservation, removal, purge, and all seven required Rust checks pass |
| AC-13 | not_run | No real SmolVM executable or virtualization device is available; execute the disposable native procedure below |
| AC-14 | passed | CLI help contract checks and independent README/operating-guide review cover readable paths, recorded versus current state, structured labels, legacy limits, and exact JSON filtering |

Independent agent reviews checked metadata validation and formatting, creation
inputs, ownership neutrality, revision expectations, versions, and operating
guidance. The non-UTF-8 finding was corrected and re-reviewed; no actionable
findings remain. `git diff -- templates src/config.rs src/platform.rs` is empty.
The six `MANAGED_KEYS`, definition-byte hashing, canonical option serialization,
and workspace path hashing retain their existing contracts.
An independent evidence review recomputed the software fingerprint, recounted
132 default and 203 adapter test results, and confirmed the stated native and
real runtime limits.

## Pending native and real runtime evidence

Native Linux ARM64, macOS ARM64, and Windows x86-64 execution is `not_run` for
this working tree. No cross-compilation or simulated path fixture establishes
native execution on those targets. Run the required checks on each target and
record source identity, command exit codes, and CI run URLs when available.

AC-13 is `not_run` for every profile. On a supported native host, use a
disposable user and workspace for `codex`, `claude`, `opencode`, and `hermes`.
Record the exact SmolVM `--version` output and native target. Generate each
profile with `create-config --harness PROFILE --smolfile PATH`, then run
`build --smolfile PATH` and capture raw `smolvm machine ls --json` plus
`smolsbx list --all --json --smolfile PATH`. Retain the actual builder create
labels before builder cleanup as well as login and workspace labels.

Create an additional disposable workspace with a long repeated `--allow-host`
set and native paths containing spaces, Unicode, and equals signs. Record the
exact create transcript and both decoded label objects. Use the selected
machine name with `smolvm machine stop --name NAME` and
`smolvm machine start --name NAME`; compare both JSON-valued labels before and
after each operation. Stop and delete the disposable fixture through the
explicit cleanup flow. Record raw inventory and smolsbx JSON, full untruncated
values, exit codes, and observed deletion. Do not substitute upstream source
inspection or the Rust test double for these observations.

Final Feature 0019 acceptance and the broader Feature 0010 cutover remain
pending the missing execution evidence.

## Completion decision: 2026-11-23

Both Feature 0019 specifications, readable machine identity metadata and
README licence and download badges, are marked `Implemented` at the user's request.
This decision supersedes earlier `In progress` status statements and
instructions to keep the feature open until all mandatory criteria pass.
The status change records the user's completion decision. Previously
recorded check results, source revisions, and pending validation remain
unchanged. No new native, hosted, or real SmolVM checks were run for this
status update.
