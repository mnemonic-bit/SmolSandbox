# Repository instructions

- Use the installed `codebase-memory` skill for structural codebase exploration.
- Before writing natural-language text to any file, read AGENT-LANG.md and apply its rules to that text.

## Versioning

- Include a version bump in `Cargo.toml` and the matching `smolsandbox` package
  entry in `Cargo.lock` with every software change, including changes to tests,
  dependencies, and build configuration.
- Use `MAJOR.MINOR.PATCH`: increment `PATCH` for bug fixes, `MINOR` for features,
  and `MAJOR` for breaking changes. Reset lower components to zero when
  incrementing `MINOR` or `MAJOR`.
- If a change only adds feature specifications or bug reports, leave the
  software version unchanged.

## Rust policy and regression checks

- Use [RUST-APPLICATION.md](RUST-APPLICATION.md) for operating commands,
  configuration selection, state locations, and lifecycle guarantees.
- Edit generated provisioning and network defaults in `templates/*.smlbx`.
  `Profile::template` in `src/config.rs` embeds those files in the executable.
  Lifecycle operations use the complete selected user-owned definition;
  `Definition::parse` supplies its `[network].allow_hosts` values to
  `Lifecycle::new` in `src/lifecycle.rs`. Additional `--allow-host` values
  extend that set. Login-only authentication allowlists are defined separately
  by `Profile::login_hosts` in `src/config.rs`. There is no second Rust workspace
  allowlist to synchronize.
- Preserve user-owned definitions when changing templates. Changed definition
  bytes select a new base generation automatically. The SmolVM version also
  participates in the base identity; immutable machine-creation options select
  the workspace configuration. Port mappings are mutable and do not change
  workspace identity.
- When Rust machine-creation policy changes for unchanged definition bytes,
  increment `REVISION` in `src/lifecycle.rs`. Existing machines retain their
  creation-time policy. Update the independent revision expectations in
  `startup_has_independent_names_labels_order_arguments_and_exit_status` and
  `creation_uses_selected_default_hosts_plus_normalized_additions_and_revision`
  in `tests/application.rs`.
- For template policy changes, review `four_templates_are_complete_and_isolated`,
  `codex_template_default_hosts_are_exposed_by_definition`,
  `codex_template_preserves_provisioning_tools_pins_and_verification`, and
  `codex_template_seed_settings_are_parsed_and_explicit` in `src/config.rs`.
- For network and identity changes, review
  `allowlist_is_a_set_and_ports_do_not_change_immutable_hash` and the creation
  checks above. For replacement changes, review
  `replacement_prepares_and_verifies_before_deleting_stale_machine` and the
  failure-preservation checks in `tests/application.rs`. Use the
  [Feature 0009 coverage map](FEATURES/feature-0009-coverage.md) to find the
  remaining independent lifecycle, purge, and port regressions.

Run the independent Rust checks from the repository root:

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --features test-adapter --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --features test-adapter --all-targets
cargo build --locked --release --bin smolsbx
cargo run --locked --example check_baseline_independence
```

`test-adapter` enables test-only home injection and virtualization bypass for
the Rust SmolVM test double. Distribute builds without `test-adapter`.
`check_baseline_independence` runs the checks in a source copy without the
historical shell/Python baseline and executes compiled tests with an empty
`PATH`. Native CI targets are defined in
[rust.yml](.github/workflows/rust.yml); cross-compilation does not establish
native execution or real guest behavior.

## Historical baseline and cutover evidence

The shell/Python baseline and its tests were removed in commit
`28087885c2b84077c62ee92c5560a2cab1b68997`; consult Git history for those sources.
Use the Rust templates, policy, and checks above for maintenance.
When recording validation, preserve previous observations and their source
revisions. Append new results and pending native or real guest checks to
[Feature 0010 validation](FEATURES/feature-0010-validation.md).
For cutover status, use the latest dated audit in that record. Source removal
and baseline-independent tests do not establish passing real SmolVM checks.
