# Feature 0013 validation

## Tested source and scope

Local validation ran on 2026-11-04 for `smolsandbox 0.8.0`, using Rust
`1.99.1` on native `x86_64-unknown-linux-gnu`. The tested source is the
uncommitted Feature 0013 working tree based on revision
`eee2aff09efdc81cde8593bf4f822feaf28b0a16`; that revision alone does not
contain removal. The starting package version was `0.7.0`, so this feature
increments the version to `0.8.0`. Machine policy revision remains `5`.

The changed software files have aggregate SHA-256
`10233854ad5123f858387c9f95a9ab3e42e8ce32c0987983b367a92d44818e6e`,
computed from this ordered checksum manifest:

```text
sha256sum Cargo.toml Cargo.lock src/cli.rs src/lib.rs src/lifecycle.rs src/ports.rs src/removal.rs tests/cli_contract.rs tests/removal.rs | sha256sum
```

The implementation agent owns `src/removal.rs`; separate agents authored CLI
and integration coverage. The CLI agent also reviewed the removal safety
checks and active documentation. Independent integration tests and review
identified global sequencing issues for legacy machines and snapshot
comparison issues for skipped running machines;
both were corrected before the final checks. Final source review reported
no remaining concrete gap in the checks that authorize mutations.

All removal executions used the Rust SmolVM test double and disposable
temporary directories. The default-feature suite exercises previews and
no-op execution without a virtualization bypass. `test-adapter` supplies a
temporary native application home for mutation tests. No real VM was stopped
or deleted. Production release builds exclude `test-adapter`.

## Required checks

| Command | Result | Exit code and evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0; `/tmp/feature-0013-verified-fmt.log` |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0; `/tmp/feature-0013-verified-clippy.log` |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0; `/tmp/feature-0013-verified-clippy-adapter.log` |
| `cargo test --locked --all-targets` | passed | 0; 61 library, 12 application, 13 CLI, 8 fixed-data, 9 listing, and 10 removal tests; `/tmp/feature-0013-verified-default.log` |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0; 61 library, 64 application, 13 CLI, 8 fixed-data, 10 listing, and 24 removal tests; `/tmp/feature-0013-verified-adapter.log` |
| `cargo build --locked --release --bin smolsandbox` | passed | 0; `/tmp/feature-0013-verified-release.log` |
| `cargo run --locked --example check_baseline_independence` | passed | 0; isolated formatting, both Clippy configurations, both suites, production release, and nine compiled test executables per configuration with empty `PATH`; `/tmp/feature-0013-verified-independence.log` |

The final command results are also recorded in
`/tmp/feature-0013-verified-results.json`. These temporary logs are local
evidence, not durable repository artifacts.

## Acceptance criteria

All test names below refer to `tests/removal.rs`, except CLI checks in
`tests/cli_contract.rs`. Results describe local mocked execution. Native and
real runtime limitations are recorded separately below.

| Criterion | Result | Evidence |
| --- | --- | --- |
| AC-01 | passed | `all_eight_scope_revision_and_force_combinations_have_fixed_deletion_sets` asserts exact sorted deletion sets for stopped, created, running, older, current, future, other-workspace, and shared rows. Selection uses fixed policy revision expectations. |
| AC-02 | inconclusive | Local canonical invocation, explicit relative path, Unix symlink, directory without Git, repository subdirectory, missing path, and non-directory cases passed in `explicit_relative_and_subdirectory_paths_preserve_canonical_workspace_scope`. Native Windows execution is `not_run`; the fixture uses native Windows path normalization when run there. |
| AC-03 | passed | The eight-combination matrix covers equal/newer revisions and running protection; `stale_comparison_uses_decimal_values_and_preserves_base_or_name_only_differences` covers leading zeros and configuration-independent selection. |
| AC-04 | passed | `forced_stop_delete_and_absence_queries_have_fixed_order`, `successful_stop_requires_same_incarnation_stopped_before_delete`, and `failed_stop_and_delete_preserve_port_state_and_later_targets` verify stop failure, running/created refreshes, incarnation, ownership, ports, and exact query ordering. |
| AC-05 | passed | The matrix covers all eight global/all/force combinations, complete custom labels, legacy workspace/login/builder names, unrelated prefixes, running shared machines, and revisionless builders. Multiple legacy targets also exercise continued global processing. |
| AC-06 | passed | `scoped_preview_counts_unassociated_candidates_without_guessing_paths` covers missing/malformed path labels and legacy names. The mixed matrix preserves other workspaces and shared machines in local scope. |
| AC-07 | passed | `ownership_and_inventory_errors_refuse_before_any_application_write` and `global_managed_namespace_requires_complete_labels_without_name_fallback` cover contradictory kinds, deterministic-name contradictions, incomplete metadata, namespace-only labels, and invalid classifications. The matrix verifies complete custom-name ownership. |
| AC-08 | passed | Decimal fixtures include a 200-digit future revision, long zero-prefixed older revisions, equal revisions, and current-policy machines under different names/bases. Revisionless builders are protected unless global all-revisions removal is requested. A unit test covers zero normalization. |
| AC-09 | passed | `dry_run_is_definition_independent_lock_free_and_does_not_change_permissions` asserts exactly version/inventory runtime calls, unchanged registry bytes and permissions, no application directory creation, and successful preview with a missing explicit definition environment override. |
| AC-10 | passed | Malformed JSON, invalid root/row types, duplicate/missing names, invalid fields, unknown selected states, unsafe names, and missing incarnation data fail before mutations. `invalid_numeric_inventory_field_types_refuse_even_on_unrelated_rows` covers both numeric fields on selected and unrelated rows. |
| AC-11 | passed | Under-lock and per-target sequences cover changed state, incarnation, ports, ownership, path association, name reuse, later-target changes, and new targets. `application_mutation_lock_blocks_removal_until_fresh_inventory_can_be_checked` holds the actual application lock while changing inventory. Original running skips remain skipped after an external stop. |
| AC-12 | passed | Port-state tests verify cleanup after confirmed absence, failed delete, post-delete inventory failure, name reuse, and local cleanup failure. `later_delete_failure_reports_completed_deletions_and_unprocessed_targets` checks earlier success, failed/later target preservation, and partial reporting. Resource snapshots preserve artifacts and caches. |
| AC-13 | passed | CLI tests cover all 16 flag/preview combinations, option placement, global/workspace conflicts, rejected flags and guest arguments, and offline help. `empty_and_running_only_selections_succeed_without_capability_queries_or_state_creation` covers no-op output and skips. Failure tests assert nonzero operational exits and partial-result target names. |
| AC-14 | passed | Resource preservation tests, both complete regression suites, and baseline-independent validation passed. Creation identity, `REVISION`, templates, inventory parser behavior, listing assessments, and existing lifecycle policies are unchanged; the existing name validator is shared by crate visibility only. |
| AC-15 | passed | Agent review checked README, operating-reference removal guidance, and CLI help against scope, stale revisions, the eight selection combinations, previews, running handling, guest writable state deletion, preserved resources, and concurrency limitations. Local link audit passed for 18 file links. |

The production executable reported `smolsandbox 0.8.0`; version, root help,
and removal help passed with empty `PATH` and missing SmolVM. Nine removal
examples from README and the operating reference passed CLI parsing, then
failed operationally with the deliberately missing runtime or workspace
(exit `1`, never argument-error exit `2`). The temporary application home
remained empty. No removal command was sent to a real runtime.

## Pending native and real runtime evidence

| Check | Result | Remaining evidence |
| --- | --- | --- |
| Native Linux x86-64 mocked execution | passed | The local checks above |
| Native Windows x86-64 mocked execution | not_run | Run the required commands and record the tested revision and CI run URL; include native path and credential ACL results |
| Native Linux ARM64 and macOS ARM64 mocked execution | not_run | Record native suite/build results and CI run URLs |
| Real SmolVM removal on each compatible release target | not_run | SmolVM is absent from `PATH`, and this host has no `/dev/kvm` |

On compatible hosts, use only explicitly disposable managed VMs. Record the
source revision, native target, SmolVM version, exact global/all/force/dry-run
combination, selected machine names and incarnations, observed stop/delete
results, and retained artifacts, definitions, credentials, and caches. Cover
both workspace and global scopes, preview preservation, skipped running
machines, forced stopping, and legacy/custom ownership. Append observed
results to this record and [Feature 0010 validation](feature-0010-validation.md).
Mocked removal tests do not establish real guest behavior or the remaining
Feature 0010 cutover gates. Historical observations remain unchanged.
