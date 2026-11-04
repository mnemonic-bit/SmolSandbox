# Feature 0012 validation

## Tested source and scope

Local validation ran on 2026-10-25 for package `smolsandbox 0.6.0`, using
Rust `1.99.1` on native `x86_64-unknown-linux-gnu`. The tested source was the
uncommitted Feature 0012 working tree based on revision
`e092975222c645a129de7460707b18d884e205cc`; the baseline commit alone does not
contain the implementation. No new CI run or native result on another target
was observed.

The changed software files have aggregate SHA-256
`be973ad7f570df62b93b6364bace3011c2789b15532a74a3ff78d379eba27cd1`,
computed from the following ordered checksum manifest:

```text
sha256sum Cargo.toml Cargo.lock src/cli.rs src/config.rs src/lib.rs src/lifecycle.rs src/listing.rs tests/listing.rs | sha256sum
```

The release executable excludes `test-adapter`. The listing fixture uses a
runtime version without the test-adapter virtualization bypass, records runtime
calls, and permits only `--version` and `machine ls --json`. Application state
snapshots include file bytes, directory membership, and permissions. The
definition and inventory fixtures are local; no real guest was inspected.

## Commands and results

All commands ran from the repository root.

| Command | Result | Exit code | Evidence |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0 | Final source formatting |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0 | Default features, including listing tests |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0 | Adapter features, including missing-default selection checks |
| `cargo test --locked --all-targets` | passed | 0 | 60 library, 12 application, 8 CLI contract, 8 fixed-data, and 6 listing tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0 | 60 library, 57 application, 8 CLI contract, 8 fixed-data, and 7 listing tests |
| `cargo build --locked --release --bin smolsandbox` | passed | 0 | Production release without `test-adapter` |
| `cargo run --locked --example check_baseline_independence` | passed | 0 | Isolated source without retained baseline; both feature configurations, release build, and eight compiled test executables per configuration with empty `PATH` |
| `target/release/smolsandbox --version` | passed | 0 | Printed `smolsandbox 0.6.0` |
| `target/release/smolsandbox list --help` | passed | 0 | Scope, comparison context, legacy limits, supported options, and read-only behavior |
| `git diff --check` | passed | 0 | No whitespace errors |

The isolated check log was written to
`/tmp/feature-0012-baseline-independence.log`. The source copy was
`/tmp/.tmp7cRpoK` and was removed automatically after success. These temporary
paths identify local execution evidence, not published CI artifacts.

## Acceptance criteria

The test symbols below are in `tests/listing.rs` unless another path is named.
Results describe local execution; the native target limitations remain explicit.

| Criterion | Result | Evidence and remaining work |
| --- | --- | --- |
| AC-01 | passed | `mixed_inventory_preserves_association_assessment_and_complete_identities` checks scoped rows across bases, revisions, configurations, and profile generations while excluding other paths and shared machines |
| AC-02 | inconclusive | `workspace_arguments_preserve_relative_and_repository_subdirectory_semantics` passed locally for explicit and relative paths, Unix symlinks, repository subdirectories, and `--all` reference selection; native Windows path execution remains not_run |
| AC-03 | passed | Mixed inventory includes labelled custom builders/login/workspaces, complete legacy names, namespace-labelled rows, and unrelated names that remain excluded |
| AC-04 | passed | Mixed inventory and five `src/listing.rs` unit tests check precedence, simultaneous differences, custom names, unknown metadata, and unfamiliar raw states |
| AC-05 | passed | `reference_identity_uses_normalized_hosts_and_name_override` and the mixed fixture use independent hash inputs with fixed revision, CPU, memory, mount, and definition expectations; CLI tests reject `--port`; existing lifecycle creation and port identity tests passed |
| AC-06 | passed | Mixed inventory and listing unit tests check missing/malformed path counts, legacy limits, and matching-path contradictions without inferred paths |
| AC-07 | passed | `absent_default_is_unassessed_but_explicit_or_invalid_definition_is_an_error` passed with adapter features; explicit missing definitions, `BASE_SMOLFILE`, invalid nearer definitions, and a selected directory fail without JSON output |
| AC-08 | passed | Mixed fixture checks full human identities, JSON fields and nulls, sorted names, reasons, and association; empty-scope checks assert an explicit human message and `machines: []` |
| AC-09 | passed | `listing_skips_preflight_and_preserves_absent_and_invalid_application_state` checks absent directories, invalid generation/port state, preserved bytes and permissions, and exactly version/inventory calls with virtualization bypass disabled |
| AC-10 | passed | `inventory_failures_emit_no_partial_json` checks runtime failure, malformed JSON, duplicate/missing names, invalid labels/state, non-array inventory, and empty stdout; contradictory rows remain inspectable in the mixed fixture |
| AC-11 | passed | `listing_help_and_usage_are_offline_and_reject_mutation_options`, release help, and independent documentation review cover README and operating-reference examples and claims |
| AC-12 | passed | Both local regression suites passed; creation-policy review found no change to templates, labels, machine naming, hash inputs, `REVISION`, cleanup validators, repair, or port behavior |

An independent agent reviewed CLI/configuration flow, read-only execution,
metadata validation, assessment precedence, output, and active documentation.
The review reported no blocking findings. That review establishes neither
native Windows execution nor real SmolVM behavior.

## Pending native and real runtime checks

| Check | Result | Reason and next action |
| --- | --- | --- |
| Native Windows x86-64, macOS ARM64, and Linux ARM64 suites | not_run | This session ran only on Linux x86-64. Run the existing native CI matrix for the implemented source and retain the source revision and run URL |
| Real SmolVM inventory and managed labels | not_run | `smolvm` is absent from `PATH` and `/dev/kvm` is absent. Run scoped, explicit-workspace, and all-machine listing against a real managed inventory on a compatible host |
| Guest provisioning, authentication, and terminal behavior | not_run | Listing mocks do not exercise guests; retain the separate Feature 0010 cutover gates |
| Physical baseline removal | not_run | `src-old/` remains present. The isolated source check does not remove the retained baseline |

These results do not complete Feature 0010's cutover or replace historical
validation observations. See [Feature 0010 validation](feature-0010-validation.md)
for the continuing native, guest, and removal requirements.

## Current-source verification: 2026-11-04

Feature 0012 was already implemented in commit `16318af`. This verification
tested revision `5f439cdbb5db8fbb2bb142533bf101be77de8da9`, package `0.7.0`,
with Rust `1.99.1` on native `x86_64-unknown-linux-gnu`. Two agents independently
reviewed the implementation against the specification and reported no concrete
implementation gaps. No software files changed during this verification;
the package version remains unchanged.

All commands below ran from the repository root.

| Command | Result | Exit code | Evidence |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0 | Current source formatting |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0 | Default configuration |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0 | Adapter configuration |
| `cargo test --locked --all-targets` | passed | 0 | 60 library, 12 application, 11 CLI contract, 8 fixed-data, and 9 listing tests; `/tmp/feature-0012-current-default.log` |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0 | 60 library, 64 application, 11 CLI contract, 8 fixed-data, and 10 listing tests; `/tmp/feature-0012-current-adapter.log` |
| `cargo build --locked --release --bin smolsandbox` | passed | 0 | Production release without `test-adapter` |
| `target/release/smolsandbox list --help` | passed | 0 | Scope, comparison context, legacy association limits, and read-only behavior |
| `target/release/smolsandbox --version` | passed | 0 | Printed `smolsandbox 0.7.0` |
| `cargo run --locked --example check_baseline_independence` | passed | 0 | Isolated checks and eight compiled test executables per feature configuration passed with empty `PATH`; `/tmp/feature-0012-current-independence.log` |
| `git diff --check` | passed | 0 | No whitespace errors |

| Criterion | Result | Current-source evidence and remaining work |
| --- | --- | --- |
| AC-01 | passed | Mixed-inventory fixture and agent review of path association across generations, revisions, profiles, and configurations |
| AC-02 | inconclusive | Native Linux explicit/relative paths, symlinks, subdirectories, and `--all` context passed; native Windows execution remains not_run |
| AC-03 | passed | Mixed fixture includes shared, legacy, and labelled custom-name machines while excluding unrelated rows |
| AC-04 | passed | Fixed identity expectations and listing unit tests cover precedence, simultaneous differences, and unfamiliar states |
| AC-05 | passed | Independent identity fixture, normalized hosts, name override, port rejection, and unchanged shared `Lifecycle::new` calculation |
| AC-06 | passed | Missing/malformed path counts and matching-path contradictions remain visible without inferred associations |
| AC-07 | passed | Adapter fixture checks absent defaults, explicit missing definitions, invalid nearer definitions, and unavailable-reference host additions |
| AC-08 | passed | Human/JSON assertions cover full identities, ordering, reasons, nulls, and empty scopes |
| AC-09 | passed | State snapshots and runtime-call assertions verify read-only inspection without virtualization preflight |
| AC-10 | passed | Failure fixtures preserve empty JSON stdout for fatal inventory failures; contradictory rows remain inspectable |
| AC-11 | passed | Release help, help tests, README, and operating-reference review |
| AC-12 | passed | Both regression configurations passed; agent review found no changes to creation policy, templates, labels, hash inputs, revision, cleanup validators, or repair behavior |

Native Windows, macOS ARM64, and Linux ARM64 execution remains `not_run` in
this session. Real SmolVM inventory and guest validation remains `not_run`:
`smolvm` is absent from `PATH`, and `/dev/kvm` is absent. Temporary log paths
above identify local observations rather than durable CI artifacts. Earlier
results and their source revisions remain unchanged.
