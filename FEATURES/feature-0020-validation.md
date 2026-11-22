# Feature 0020 validation

Status: `completed`. AC-01 through AC-10 passed local verification for
[Feature 0020](feature-0020-list-output-formats.md) on native
`x86_64-unknown-linux-gnu`.

## Source and environment

Validation date: 2026-11-22. The implementation working tree is based on
`c78944ac68b2b1cac35521c4aadf9a3d943dae51`. The baseline package version was
`0.13.1`; the implementation version is `0.14.0` in both Cargo files.
Machine policy `REVISION` remains `7`; report schema version remains `1`.
The Feature 0020 specification and two Feature 0021 documents were already
untracked when implementation began. Feature 0021 is outside this change.

The native target is `x86_64-unknown-linux-gnu`, running Linux `6.18.60`.
Rust is `1.100.0 (0ef1bc386 2026-11-09)` and Cargo is
`1.100.0 (3acc79c0c 2026-11-05)`. Verification uses the Rust SmolVM test
double. No new hosted CI or real guest result is claimed.

The software diff SHA-256 is
`a171253a6a33756d56ba8b67769445a58c27935040828bd32f75e73125aee515`,
computed from the exact output of:

```sh
git diff --binary c78944ac68b2b1cac35521c4aadf9a3d943dae51 -- Cargo.toml Cargo.lock src tests | sha256sum
```

Documentation is excluded from this fingerprint. Local command logs and
exit-code results are under `/tmp/feature-0020/`.

## Repository checks

Commands ran from the repository root. The production release build excludes
`test-adapter`; `target/release/smolsbx --version` returned `smolsbx 0.14.0`
with exit 0. `target/release/smolsbx list --help` also exited 0 and displayed
the format choices, default text behavior, alias, and conflicts.

| Command | Result | Exit code and evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | passed | 0; `fmt.log` |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0; `clippy.log` |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0; `clippy-adapter.log` |
| `cargo test --locked --all-targets` | passed | 0; `test.log`; 147 tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0; `test-adapter.log`; 223 tests |
| `cargo build --locked --release --bin smolsbx` | passed | 0; `release.log`; production features |
| `cargo run --locked --example check_baseline_independence` | passed | 0; `independence.log`; isolated formatting, both lint and test configurations, production release build, and ten compiled test executables per configuration with empty `PATH` |

## Acceptance evidence

| Criterion | Result | Evidence or next action |
| --- | --- | --- |
| AC-01 | passed | `format_syntax_positions_alias_and_empty_report_preserve_the_contract` covers both formats, option syntaxes, workspace positions, `--all`, explicit selection, repeatable host additions, and verbosity |
| AC-02 | passed | Byte-for-byte JSON alias comparison, compact newline assertions, and full default human-output fixtures |
| AC-03 | passed | `list_format_argument_errors_are_offline_and_specific_to_list` and `invalid_format_selectors_do_not_query_an_available_runtime` check invalid/conflicting/repeated selectors, exit 2, empty stdout, other commands, and absent runtime calls |
| AC-04 | passed | Mixed legacy/current inventories, both-format fixed metadata status and diagnostic expectations, missing-default nulls, exact empty-report values, and all-profile creation metadata comparisons |
| AC-05 | passed | `structured_reference_and_warning_strings_preserve_types_and_controls` and `structured_formats_keep_native_strings_nested_inputs_and_sorted_rows` check top-level, reference, warnings, and nested values containing sensitive scalars, paths, Unicode, punctuation, line breaks, and controls |
| AC-06 | passed | Representative YAML mapping, sequence, indentation, null, and empty-array assertions; repeat-output equality; sorted names; compact single-object JSON |
| AC-07 | passed | `human_output_has_exact_separators_for_zero_one_and_multiple_entries` and `missing_default_preserves_nulls_and_exact_human_warning_blocks` compare complete output with zero, one, and multiple entries, with and without warnings; multiline metadata stays escaped |
| AC-08 | passed | Both-format operational failure and verbose fixtures preserve stdout/stderr behavior; Linux `/dev/full` checks return exit 1 on write failure; source review confirms serialization completes before writing |
| AC-09 | passed | `every_format_preserves_absent_state_and_query_only_calls` covers human, alias, JSON, and YAML with absent and invalid application state; exact logs allow only version and inventory queries |
| AC-10 | passed | Offline help, documentation, matching version 0.14.0, and unchanged revision 7 passed review; all seven required repository checks exited 0 |

Separate implementation and regression agents completed the source and fixtures.
An independent reviewer found no actionable defects after checking the frozen
implementation against AC-01 through AC-09. The reviewer independently ran
the focused adapter suites: 24 listing tests, 14 CLI tests, and 3 metadata
tests passed with exit 0. The direct reference/warning serializer unit test
and `git diff --check` also passed independently.

The Rust serializer is `serde-saphyr` 2.2.0, with serialization enabled for
production and deserialization enabled for tests. The selected options request
YAML 1.2 quoting, two-space expanded sequences, escaped multiline strings,
and no language directive or document-start marker. Source review confirmed
explicit `null`, explicit empty arrays, and no automatic anchors. Report
construction and machine inspection remain unchanged; format selection occurs
only during rendering. Templates, identity hashing, labels, definition bytes,
and lifecycle policy have no production changes in this diff.

## Pending native and real guest evidence

Native Linux ARM64, macOS ARM64, Windows x86-64, and hosted CI execution are
`not_run` for this implementation. Real SmolVM and real guest validation are
`not_run`. These limits do not establish new native or real guest results
for the broader Feature 0010 cutover. Preserve previous observations in
[Feature 0010 validation](feature-0010-validation.md).
