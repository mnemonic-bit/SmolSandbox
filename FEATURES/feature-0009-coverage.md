# Feature 0009 regression coverage map

Comparison revision: `55df3db989ba410e61ae64883224badf5115e613`.
The eight shell suites, production scripts, Python helpers, and native test
double were unchanged at the start of this transfer. The initial worktree had
an unrelated Windows ACL test change in `src/platform.rs` (SHA-256
`97f4b94c919cbf2cf4d0020961b5c9251b22370aeff6737d7172bfd448cd96e6`).
That change is preserved. Features 0009 and 0010 were untracked specifications;
their SHA-256 values were respectively
`ba85163440c4df0f0d4d637f0b4f58fb322ef88641192341846c6fff03a2a469`
and `21bfed49577bbdbba1c6d10ff469cd820c0c3bc982617d889756f99e5409aef3`.

Source line ranges below refer to the comparison revision. Rows group
assertions with the same condition and expected result. Setup, fixture
construction, and final PASS messages introduce no additional behavioral
assertions. Rust symbols are the durable lookup keys; line numbers in the
Rust implementation may change.

Symbol prefixes identify these test files/modules:

| Prefix | Rust location |
| --- | --- |
| `app::` | `tests/application.rs`, module `lifecycle` |
| `application::` | `tests/application.rs`, top-level tests |
| `cli::` | `tests/cli_contract.rs` |
| `config::` | `src/config.rs`, module `tests` |
| `domain::` | `src/domain.rs`, module `tests` |
| `ownership::` | `src/domain.rs`, module `parity_tests` |
| `ports::` | `src/ports.rs`, module `tests` |
| `port_parity::` | `src/ports.rs`, module `parity_tests` |
| `independent::` | `tests/differential.rs` |

## Allow hosts

Source: `tests/allow-hosts-test.sh`.

| ID | Source lines and assertions | Independent Rust symbols | Expected result |
| --- | --- | --- | --- |
| AH-1 | 35–48: repeatable options, equals syntax, before/after workspace, reordered and repeated hosts | `cli::repeated_options_and_argument_boundaries_preserve_exact_values`; `app::allowlist_is_a_set_and_ports_do_not_change_immutable_hash` | Parsing preserves argument values; effective sets select the same name and custom classification. |
| AH-2 | 49–63: custom/path labels and exactly one copy of every effective host; duplicates of a default host remain default | `app::allowlist_is_a_set_and_ports_do_not_change_immutable_hash`; `app::creation_uses_selected_default_hosts_plus_normalized_additions_and_revision`; `app::startup_has_independent_names_labels_order_arguments_and_exit_status` | Create arguments contain normalized default-plus-user hosts once; all six ownership labels have their specified values. |
| AH-3 | 65–80: changed hosts and changed harness-home mount change identity | `app::allowlist_is_a_set_and_ports_do_not_change_immutable_hash` | Each immutable-policy change selects a different workspace name. |
| AH-4 | 82–88: missing host value rejected | `cli::invalid_hosts_fail_before_tools_or_state_access` | Missing, empty, leading-dash, CR, and LF values exit 2 before tool or state access. |

## Clean

Source: `tests/clean-sandbox-test.sh`.

| ID | Source lines and assertions | Independent Rust symbols | Expected result |
| --- | --- | --- | --- |
| CL-1 | 51–68: delete all current-generation builders, login revisions, and workspaces; preserve another generation and unrelated names | `app::generation_clean_removes_only_current_machines_artifacts_and_caches` | Only owned current-generation machines are stopped/deleted; summary reports the number removed. |
| CL-2 | 70–90: exact current artifacts/pointers removed; other artifacts and Codex state retained | `app::generation_clean_removes_only_current_machines_artifacts_and_caches`; `app::generation_clean_partial_delete_failure_preserves_artifacts_caches_and_later_machines` | Successful cleanup removes current artifacts/caches; failed deletion preserves artifacts/caches and unprocessed machines. Definitions and harness state survive. |
| CL-3 | 92–154: path-scoped cleanup spans generations and configurations, preserves other paths/login/artifact/pointers/harness state, removes both port-state forms, reports count/path | `app::workspace_clean_spans_generations_preserves_other_paths_and_rejects_contradictions`; `app::clean_workspace_preserves_artifacts_definitions_and_harness_home` | Only workspaces with the selected canonical path hash are deleted; shared state remains. |
| CL-4 | 156–179: contradictory matching ownership fails before any stop/delete | `app::workspace_clean_spans_generations_preserves_other_paths_and_rejects_contradictions`; `ownership::managed_inventory_rejects_all_contradictory_ownership_shapes` | Nonzero result and actionable ownership error; inventory and shared state remain. |
| CL-5 | Feature 0009 partial-failure extension | `app::failed_delete_retains_port_state_and_definition`; `app::generation_clean_partial_delete_failure_preserves_artifacts_caches_and_later_machines` | Port state is removed only after successful corresponding deletion; no rollback is claimed. |

## Provisioning and seed configuration

Source: `tests/devtools-base-config-test.sh`.

| ID | Source lines and assertions | Independent Rust symbols | Expected result |
| --- | --- | --- | --- |
| CF-1 | 15: base image | `config::codex_template_preserves_provisioning_tools_pins_and_verification` | Embedded Codex image is `debian:bookworm-slim`. |
| CF-2 | 19–40: registry, tool installer, browser, Rust, and .NET default hosts | `config::codex_template_default_hosts_are_exposed_by_definition` | `Definition::parse` exposes the complete specified 35-host set. |
| CF-3 | 41–56: no moving-main CBM setup; installer/download pinned to v0.11.0; Codex client selection and version check | `config::codex_template_preserves_provisioning_tools_pins_and_verification` | Exact existing installation and verification fragments remain, after Codex installation. |
| CF-4 | 58–94: Docker packages/wrappers/dispatch/symlink, installer dependencies, nested smoke command, compose version | `config::codex_template_preserves_provisioning_tools_pins_and_verification` | Parsed init commands retain all package and compatibility assertions, plus overlay storage settings. |
| CF-5 | 96–120: extracted seed TOML model/effort/context/compaction/status-line/colors/trust | `config::codex_template_seed_settings_are_parsed_and_explicit` | Parsed seed settings match literal expected values, including ordered status fields. |
| CF-6 | 125: runtime revision 5 | `app::startup_has_independent_names_labels_order_arguments_and_exit_status`; `app::creation_uses_selected_default_hosts_plus_normalized_additions_and_revision` | Created workspace name and revision label use `5`. |
| CF-7 | 126–139: definition/runtime default-host equality | `config::codex_template_default_hosts_are_exposed_by_definition`; `app::startup_has_independent_names_labels_order_arguments_and_exit_status`; `app::creation_uses_selected_default_hosts_plus_normalized_additions_and_revision` | Workspace create arguments use the selected definition's defaults and normalized additions. There is one production allowlist in the selected definition. |
| CF-8 | Former `src/config.rs:500–508` whole-baseline inclusion and pin assertions | `config::codex_template_preserves_provisioning_tools_pins_and_verification` | Direct template assertions retain all existing tools/pins/install options/version checks and snapshot seeding without reading the old definition. |

Static commands establish configuration and argument construction. Real
provisioning and guest copying remain Feature 0004 checks.

## Help and usage

Source: `tests/help-test.sh`.

| ID | Source lines and assertions | Independent Rust symbols or explicit replacement | Expected result |
| --- | --- | --- | --- |
| HE-1 | 24–58, 72–74: every help page exits 0, stdout has synopsis/options/help/content, stderr empty | `cli::help_is_complete_and_does_not_require_tools_configuration_or_mutation` | Every native command page satisfies stream/status/content checks. |
| HE-2 | 76–85: help precedence among otherwise invalid shell arguments | [Feature 0004, “Command-line interface”](feature-0004-replace-shell-scripts-with-cross-platform-rust-application.md#command-line-interface), lines 187–229: native subcommands replace shell entry points and unknown options/excess arguments are usage errors | Old permissive shell argument scanning is superseded. Valid native help invocations remain covered by HE-1. |
| HE-3 | 87–120: command-specific content for build, launch, login, repair, clean, purge | `cli::help_is_complete_and_does_not_require_tools_configuration_or_mutation`; `cli::repeated_options_and_argument_boundaries_preserve_exact_values` | Native pages explain current-directory defaults, repeatable/equal-form hosts, host validation/placement, loopback same-number TCP mappings, guest forwarding, authentication, repair modes, shared-state preservation, conservative/forced purge, mutual exclusion and running-generation protection. Exact shell synopsis/alias wording is superseded by Feature 0004's native subcommand mapping. |
| HE-4 | 122–126: no prerequisite inspection or lifecycle/tree mutation | `cli::help_is_complete_and_does_not_require_tools_configuration_or_mutation`; `application::help_and_version_are_independent_of_configuration` | With absent definition/tools and empty tool directory, sentinels and recursive tree contents remain unchanged; no SmolVM log appears. |
| HE-5 | 128–154: usage errors retain status 2 and stderr | `cli::usage_errors_exit_two_on_stderr_before_any_lifecycle_work`; `cli::invalid_hosts_fail_before_tools_or_state_access`; `cli::invalid_ports_fail_before_tools_or_state_access` | Invalid native equivalents exit 2, stdout empty, stderr nonempty, tree unchanged. |

## Port publishing

Source: `tests/port-publishing-test.sh`.

| ID | Source lines and assertions | Independent Rust symbols | Expected result |
| --- | --- | --- | --- |
| PO-1 | 54–83: canonical set, no baked ports, update/remove arguments, verify before start, bound manifest and absent journal | `app::startup_has_independent_names_labels_order_arguments_and_exit_status`; `app::start_recovers_machine_and_reconciles_exact_ports_without_changing_identity`; `independent::port_publication_commit_and_interrupted_recovery_have_fixed_expectations` | One workspace is created without ports; mutable update precedes verification/start; committed state has exact machine/incarnation/hash/set. |
| PO-2 | 85–95: changed and empty sets reuse overlay/name and remove union of known old/new ports | `app::start_recovers_machine_and_reconciles_exact_ports_without_changing_identity`; `ports::interrupted_update_recovers_union_and_commits_exact_set` | No recreation; exact requested set replaces earlier set; omitted ports clear mappings. |
| PO-3 | 97–104: guest `--port` after delimiter does not publish | `cli::repeated_options_and_argument_boundaries_preserve_exact_values`; `app::startup_has_independent_names_labels_order_arguments_and_exit_status` | Guest arguments, spaces, empty arguments and delimiter boundaries remain intact. |
| PO-4 | 106–139: 64 distinct ports accepted, invalid formats and 65 ports refused before tool calls | `cli::port_limit_counts_distinct_normalized_values`; `cli::invalid_ports_fail_before_tools_or_state_access` | Decimal 1..65535, normalization and distinct limit are enforced; usage failures leave state unchanged. |
| PO-5 | 141–153: incompatible update capability fails before inventory/mutation | `app::incompatible_port_capability_refuses_before_inventory_or_mutations` | Missing remove-port support fails closed before inventory. |
| PO-6 | 155–183: malformed public metadata rejected without mutation; supported metadata restored | `app::malformed_port_metadata_and_unknown_positive_sets_fail_before_mutation`; `independent::public_port_metadata_has_fixed_validation_expectations`; `port_parity::invalid_public_metadata_and_unsupported_state_fail_before_publication` | Invalid count/timestamp/state cannot authorize machine or state mutation. |
| PO-7 | 185–204: unknown positive set cannot be adopted without valid manifest | `app::malformed_port_metadata_and_unknown_positive_sets_fail_before_mutation`; `ports::unknown_positive_mapping_refuses_adoption`; `independent::port_decisions_have_fixed_manifest_journal_drift_and_running_set_expectations` | Nonzero refusal includes diagnostic guidance; no update/start/exec. |
| PO-8 | 206–221: zero-port new incarnation safely replaces stale local binding | `app::new_zero_port_incarnation_replaces_stale_manifest_without_changing_identity`; `ports::dry_run_has_no_side_effects_and_new_incarnation_resets_zero_ports`; `port_parity::zero_port_invalid_manifest_can_be_replaced_but_future_journal_cannot`; `independent::port_decisions_have_fixed_manifest_journal_drift_and_running_set_expectations` | Stale binding never authorizes positive removals; safe zero-port state can be replaced. |
| PO-9 | 223–237: interrupted update retains journal and next run removes old/interrupted/new union | `app::interrupted_port_update_is_recovered_before_next_start`; `app::port_update_and_verification_failures_preserve_recovery_and_never_start`; `independent::port_publication_commit_and_interrupted_recovery_have_fixed_expectations` | Recovery state survives failure; successful retry publishes exact set and removes journal. |
| PO-10 | 239–271: update and verification failures never start or run harness | `app::port_update_and_verification_failures_preserve_recovery_and_never_start` | Both injected failures stop the sequence and retain recovery files for retry. |
| PO-11 | 273–306: concurrent workspace updates stay with their verification/start | `app::concurrent_starts_serialize_port_update_verification_and_start` | Bounded readiness/release and actual artifact-lock contention prove first update, verification, start precede second locked inventory/update/start. |
| PO-12 | 308–326: matching running set succeeds without update/start/stop; different set refused | `app::running_port_mismatch_is_refused_without_stopping_machine`; `app::exact_running_port_set_executes_harness_without_update_start_or_owned_stop`; `port_parity::running_requires_matching_manifest_count_and_absent_journal` | An active session is never stopped to change mappings. |
| PO-13 | 328–349: bind/start failure prevents guest execution and attempts stop cleanup | `app::start_failure_skips_seed_and_harness_and_attempts_owned_stop` | Command fails, stop is attempted, no seed/harness runs. |
| PO-14 | 351–363: successful delete removes manifest and journal | `app::successful_workspace_deletion_removes_manifest_and_pending_journal`; `app::workspace_clean_spans_generations_preserves_other_paths_and_rejects_contradictions`; `app::failed_delete_retains_port_state_and_definition`; `port_parity::commit_requires_valid_binding_and_unchanged_target` | Both forms disappear only after successful deletion. |

## Purge

Source: `tests/purge-sandbox-test.sh`.

| ID | Source lines and assertions | Independent Rust symbols | Expected result |
| --- | --- | --- | --- |
| PU-1 | 51–68: conservative legacy removal, exact two artifact suffixes, current artifact/tmp/directory retention, summary | `app::purge_selects_exact_artifact_names_and_reports_removals`; `domain::artifact_inventory_ignores_directories_and_nonmatching_names` | Selected stopped legacy machine and exact files are removed; unrelated files/directories remain; counts match. |
| PU-2 | 70–89: aborted initialization recovery | `app::purge_missing_artifact_allows_only_stopped_builder_recovery`; `domain::purge_without_current_artifact_only_recovers_stopped_builder` | With missing current artifact, only stopped current builder is deleted; old machines/artifacts remain; count summary records the limited scope. |
| PU-3 | 91–135: missing artifact without builder, running builder, empty current artifact | `app::purge_missing_artifact_allows_only_stopped_builder_recovery`; `ownership::missing_current_artifact_requires_stopped_current_builder` | Refuse without deleting machines/artifacts. An empty existing artifact is not aborted-init recovery. |
| PU-4 | 137–168: conservative custom-config refusal and force selection | `app::purge_policies_preserve_running_machines_and_current_artifact`; `independent::purge_plans_have_fixed_policy_running_protection_and_recovery_expectations` | Conservative exits 3 with diagnostic and no deletion; force deletes custom stopped machines while retaining current defaults. |
| PU-5 | 170–199: successful build tolerates conservative purge refusal | `app::build_preserves_custom_configurations_when_automatic_purge_refuses` | Build succeeds with warning and old artifact retained. |
| PU-6 | 201–215: force-all removes stopped current workspace/login but protects running older generation | `app::purge_policies_preserve_running_machines_and_current_artifact`; `domain::purge_protects_running_generations_and_current_artifact` | No running machine stop/delete; its generation's artifacts and current artifact survive. |
| PU-7 | 217–227: incomplete current metadata refuses | `app::purge_policies_preserve_running_machines_and_current_artifact`; `domain::purge_refuses_current_generation_incomplete_metadata`; `independent::purge_plans_have_fixed_policy_running_protection_and_recovery_expectations` | Ownership failure precedes deletion and artifacts remain. |
| PU-8 | 229–244: fresh inventory changes stopped candidate to running | `app::purge_fresh_running_change_prevents_all_deletions`; `ownership::running_state_change_protects_previously_selected_artifact` | Revalidation refuses before machine/artifact deletion. |
| PU-9 | Feature 0009 partial-failure extension | `app::purge_partial_machine_failure_preserves_all_artifacts_and_later_machines` | Completed deletes remain completed; failed/unprocessed machines and every artifact survive. |

## Reconciliation and replacement

Source: `tests/reconcile-workspace-test.sh`.

| ID | Source lines and assertions | Independent Rust symbols | Expected result |
| --- | --- | --- | --- |
| RE-1 | 12, 91–100: revision, existing desired recovery, cache publication without creation | `app::desired_machine_recovery_republishes_missing_and_stale_generation_caches`; `app::existing_desired_replacement_is_not_recreated_and_preserves_unrelated_machines`; `app::creation_uses_selected_default_hosts_plus_normalized_additions_and_revision` | Existing desired machine reused; cache values match selected base/revision 5. |
| RE-2 | 101–118: stale-only normal start refusal, repair guidance, unchanged caches | `app::stale_only_normal_start_preserves_every_cache_and_machine`; `app::replacement_prepares_and_verifies_before_deleting_stale_machine` | Normal start fails without mutation; explicit replacement is required. |
| RE-3 | 119–140: malformed or unavailable inventory cannot become empty | `app::malformed_inventory_never_authorizes_creation_or_deletion`; `app::inventory_command_failure_preserves_machines_and_caches` | Failed inventory never creates/deletes/starts or changes caches. |
| RE-4 | 141–181: create missing desired exactly once with all labels/default hosts | `app::startup_has_independent_names_labels_order_arguments_and_exit_status`; `app::concurrent_creation_and_replacement_register_and_delete_each_machine_once`; `config::codex_template_default_hosts_are_exposed_by_definition` | Name, six labels, resource arguments, mounts, normalized hosts and creation count satisfy the explicit contract. |
| RE-5 | 183–200: build missing base, pack nonempty artifact, delete current/obsolete builders | `app::builders_are_packed_before_workspace_creation_and_removed_when_artifacts_exist` | Builder creation/pack/publication precede workspace creation; recovered builders are cleaned. |
| RE-6 | 201–220: create and fresh readiness before stale deletion | `app::replacement_prepares_and_verifies_before_deleting_stale_machine` | Verified desired survives; stale delete follows creation/readiness. |
| RE-7 | 222–262: running stale refusal without force, forced create-before-stop-before-delete | `app::running_stale_replacement_requires_force_and_stops_before_deleting` | Unforced path has no mutation; forced path preserves readiness ordering. |
| RE-8 | 264–286: pack/create failure preserves stale machine and caches | `app::failed_replacement_preserves_stale_machine_and_caches`; `app::replacement_rejects_failed_creation_and_every_unready_refresh_without_cleanup` | No stale stop/delete or final cache publication after failed preparation. |
| RE-9 | 288–306: partially registered failed create preserved for inspection | `app::failed_registered_replacement_is_preserved_for_inspection` | Diagnostic identifies preserved replacement; stale state/caches unchanged. |
| RE-10 | 308–388: failed/malformed/missing/duplicate/contradictory fresh inventory | `app::replacement_rejects_failed_creation_and_every_unready_refresh_without_cleanup`; `independent::ownership_classification_and_replacement_readiness_have_fixed_expectations` | Every readiness failure prevents stale cleanup and preserves caches. |
| RE-11 | 390–405: existing failed desired not recreated or ready | `app::existing_desired_replacement_is_not_recreated_and_preserves_unrelated_machines`; `app::replacement_rejects_failed_creation_and_every_unready_refresh_without_cleanup` | Unsupported desired state fails before cleanup/cache publication. |
| RE-12 | 407–418: ready existing desired not recreated; stale removed and caches published | `app::existing_desired_replacement_is_not_recreated_and_preserves_unrelated_machines` | Reuses ready desired; publishes selected base/revision after cleanup. |
| RE-13 | 420–455: stale delete/stop failure short-circuits cleanup, preserves desired/later machines/caches | `app::replacement_partial_cleanup_failure_preserves_desired_later_machines_and_caches` | Failed stop never reaches delete; failed delete never reaches later cleanup; desired is never rollback-deleted. |
| RE-14 | 457–476: explicit unlabelled override accepted, contradictory managed override refused | `app::explicit_machine_override_accepts_legacy_ownership_and_rejects_contradictions`; `app::replacement_rejects_failed_creation_and_every_unready_refresh_without_cleanup` | Only explicit legacy override bypasses generated naming; managed contradictions remain fatal. |
| RE-15 | 478–487: unrelated machine untouched | `app::existing_desired_replacement_is_not_recreated_and_preserves_unrelated_machines` | Replacement cleanup selects only owned stale workspace candidates. |
| RE-16 | 489–530: concurrent starts and replacements create/delete once | `app::concurrent_creation_and_replacement_register_and_delete_each_machine_once`; `app::concurrent_starts_serialize_port_update_verification_and_start` | Fresh inventory is read under native locks; exactly one desired registration and stale deletion occur. |

## Codex launch

Source: `tests/start-codex-test.sh`.

| ID | Source lines and assertions | Independent Rust symbols or explicit replacement | Expected result |
| --- | --- | --- | --- |
| ST-1 | 18–36: seeding failure is warning-only | `app::seed_commands_are_non_clobbering_and_failures_only_warn` | Guest launch continues and determines status after seed failure. |
| ST-2 | 39–60: repeated starts construct non-archival, no-clobber seed copy without permission errors | `app::seed_commands_are_non_clobbering_and_failures_only_warn` | Every invocation constructs `cp -r --update=none /opt/codex-home/. /root/.codex/`; existing guest-file preservation still requires real guest execution. |
| ST-3 | 62–69: interactive Codex flags/help then stop | `app::startup_has_independent_names_labels_order_arguments_and_exit_status`; `app::guest_status_and_cleanup_failure_precedence` | Exact interactive argv includes `codex --no-daemon`; guest status wins over cleanup failure; cleanup-only failure is nonzero. |
| ST-4 | 71–86: compatibility wrapper forwards guest help | `cli::repeated_options_and_argument_boundaries_preserve_exact_values`; [Feature 0004 “Command-line interface”](feature-0004-replace-shell-scripts-with-cross-platform-rust-application.md#command-line-interface), lines 216–229 | Native `start -- --help` preserves forwarding. The shell alias itself is explicitly superseded by the native subcommand mapping. |
| ST-5 | Feature 0009 retained lifecycle guarantees | `app::profiles_mount_only_selected_state_and_forward_only_codex_secret_reference`; `app::termination_during_start_seed_and_guest_stops_the_started_machine` | Codex secrets travel by environment-variable name; native Unix termination attempts invocation-owned stop during start/seed/guest stages. |

## Independent baseline expectations

The former six differential tests are retained as six platform-independent
tests. JSON fixtures are data, captured once from the comparison revision and
reviewed against Feature 0004's inventory, ownership, readiness, purge, and port
contracts. Tests never regenerate expected values. `expected: null` means
rejection. Purge artifact paths are projected to filenames because temporary
directory names and native path syntax are not behavior under comparison.

| ID | Former `tests/differential.rs` source | Independent Rust symbol and data | Expected result |
| --- | --- | --- | --- |
| DF-1 | `inventory_validation_and_normalization_match_python_baseline`, 111–163 | `independent::inventory_validation_and_normalization_have_fixed_expectations`; `tests/data/inventory-cases.json` | 41 cases: 11 accepted normalized records and 30 rejections. Names, default/normalized state and label values retain their expectations. |
| DF-2 | `public_port_metadata_validation_matches_python_baseline`, 166–214 | `independent::public_port_metadata_has_fixed_validation_expectations`; `tests/data/port-metadata-cases.json` | 378 cases: 24 accepted and 354 rejected across states, counts and creation timestamps. |
| DF-3 | `ownership_classification_and_replacement_readiness_match_python_baseline`, 217–283 | `independent::ownership_classification_and_replacement_readiness_have_fixed_expectations`; `tests/data/ownership-cases.json` | 384 category/source/reason tuples: 88 desired, 64 stale, 136 ambiguous, 96 unrelated; 36 ready. |
| DF-4 | `purge_plans_match_python_for_policies_running_protection_and_recovery`, 286–389 | `independent::purge_plans_have_fixed_policy_running_protection_and_recovery_expectations`; `tests/data/purge-cases.json` | 273 cases: 194 accepted plans and 79 rejections, including three interrupted-builder recovery cases; compare alternate flag, managed/selected records, protected generations, artifacts and recovery scope. |
| DF-5 | `port_decisions_match_python_for_manifest_journal_drift_and_running_sets`, 427–497 | `independent::port_decisions_have_fixed_manifest_journal_drift_and_running_set_expectations`; `tests/data/port-state-cases.json` | 675 cases: 256 accepted actions and 419 rejections; exact removable union and dry-run file preservation. |
| DF-6 | `port_publication_commit_and_interrupted_recovery_match_python`, 500–524 | `independent::port_publication_commit_and_interrupted_recovery_have_fixed_expectations` | Five explicit steps compare actions, manifest and journal: initial commit, interruption, recovered union, empty-set commit, no-op; final removal clears both files. |
| DF-7 | `tests/application.rs:799–875`, `differential_start_matches_shell_creation_labels_order_and_guest_status` | `app::startup_has_independent_names_labels_order_arguments_and_exit_status` | Independently specified machine name, labels, resources/mounts/hosts, mutation order, exact guest arguments and exit 17 replace the live shell comparison. |

Oracle source SHA-256 values at capture were
`86b5da416ed15ae82a6de0eda42565050d5cde6e2603e2dce8f7a32d36ea3318`
for `sandbox-inventory.py` and
`47b662e12262f7365a32791be582f49b9b29017785bc6ee5db1b04560dac5f37`
for `sandbox-port-state.py`. Historical names are provenance, not build inputs.

## Independence and platform evidence

`examples/check_baseline_independence.rs` creates an isolated source copy with
the full Feature 0009 baseline-removal boundary absent. The example executes
all six required Cargo checks, then discovers and executes both configurations'
compiled test binaries with empty `PATH`. Set
`BASELINE_INDEPENDENCE_KEEP_COPY=1` to retain the copy for further native
execution restrictions or inspection. The normal workflow runs the example
on every release target. Empty `PATH` establishes tool-search independence;
the validation record separately identifies any stronger OS isolation used.

Unix signal and POSIX permission/symlink assertions remain on Unix targets.
Windows ACL assertions remain on Windows. The existing default-feature
`tests/application.rs` Windows exclusion avoids lifecycle/configuration tests
writing to the real native user home: Windows home discovery ignores the Unix
`HOME` test override. The full application suite runs on Windows with
`test-adapter`, which supplies a private test home. CLI contract and fixed-data
tests run on every target without that exclusion.

See [the validation record](feature-0009-validation.md) for commands and actual
results. The retained baseline, historical Feature 0004 evidence, real guest,
authentication, and terminal gates remain separate.
