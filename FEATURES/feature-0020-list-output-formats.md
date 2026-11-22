# Feature 0020: Select YAML or JSON output for list

## Status

Implemented and locally accepted in application version `0.14.0`.
AC-01 through AC-10 passed on native `x86_64-unknown-linux-gnu`, including
all seven required Rust checks. Acceptance evidence and remaining native or
real guest limits are recorded in
[Feature 0020 validation](feature-0020-validation.md).

## Outcome and baseline

Users can request YAML with `smolsbx list --format yaml` or JSON with
`smolsbx list --format json`. When neither `--format` nor the existing
`--json` flag is supplied, `list` retains its human-readable text output
with blank lines separating the reference, warnings, summary, and machine
entries.

Specification preparation observed commit
`e9957bbee4816dc09ff5549fcbe3d5ae80793324`, application version `0.13.0`,
and a clean working tree on 2026-11-22. The baseline already supports
`list --json`. `src/cli.rs` parses that boolean flag;
`src/lifecycle.rs::list` constructs one report and passes the flag to
`src/listing.rs::Report::print`. The JSON branch serializes the report as
one compact object followed by a newline. The human branch prints reference
lines, warnings, summary text, and machine entries without blank separators.

Use [RUST-APPLICATION.md](../RUST-APPLICATION.md#list-machines-and-compare-selection)
as the operating reference. Preserve the report and inspection contracts
from [Feature 0012](feature-0012-list-workspace-machines.md) and the
implemented metadata additions from
[Feature 0019](feature-0019-readable-machine-identity-metadata.md).

Structural exploration used the installed `codebase-memory` skill and
project `workspace-myrepo`, graph generation `2026-11-22T12:22:04Z`.
Symbol snippets confirmed CLI parsing, report dispatch, and human formatting.
The coverage check reported no recorded issues for those source paths.

## Command interface

```text
smolsbx list
smolsbx list /path/to/project
smolsbx list --format yaml
smolsbx list --format=json
smolsbx list --all --format yaml /path/to/project
smolsbx list /path/to/project --format json --verbose
smolsbx list --all --json
```

1. Add `--format <FORMAT>` to `list`. Accept exactly the lowercase values
   `yaml` and `json`, using both space-separated and equals-sign syntax.
   Accept the option before or after the optional workspace argument.
2. When both output selectors are omitted, select human-readable text.
   Do not infer a format from terminal detection, redirection, or environment
   variables. No explicit text value is required; reject `--format text`.
3. Retain `--json` as a supported compatibility alias for JSON output.
   Keep existing `--json` scripts working and document `--format json` as
   the primary spelling. Do not emit a deprecation warning.
4. Reject any combination of `--json` and `--format`, including
   `--json --format json`, regardless of argument order. Reject repeated
   `--format` options, even when their values agree. These rules avoid
   precedence depending on argument order.
5. Reject a missing value, an empty value, unknown values, and other casing
   such as `JSON`. Argument errors exit `2`, write a diagnostic to stderr,
   leave stdout empty, and invoke no SmolVM operation.
6. Keep `--format` specific to `list`. Preserve existing selection options,
   `--all`, repeatable `--allow-host`, workspace semantics, and verbosity.
   Keep rejecting mutation flags, `--port`, and guest arguments for `list`.

`smolsbx list --help` must explain accepted values, the default text output,
the retained `--json` alias, and the selector conflict. Help remains offline
and exits `0` without resolving a workspace or invoking SmolVM.

## Structured output contract

Both formats represent the same existing report with schema `version: 1`.
Select serialization only after constructing the report; output selection
must not change discovery, filtering, sorting, reference identity,
assessments, metadata validation, or warnings.

Preserve every current report field and meaning. Top-level fields are
`version`, `scope`, `workspace`, `reference`,
`unassociated_workspace_count`, `warnings`, and `machines`. Preserve all
current reference and machine fields, including decoded `base_inputs`,
`machine_inputs`, metadata status, metadata diagnostics, and path provenance.
Keep unavailable values null and keep empty arrays as arrays. Do not omit
fields based on the selected format. A format-only change does not increment
the report schema version.

JSON output from `--format json` must match existing `--json` output for
the same report, including compact serialization and its final newline.
Retain one JSON object on stdout.

YAML output is one YAML 1.2 document whose root is a mapping. Use block
style for mappings and nonempty sequences, two-space indentation, no YAML
tags or anchors, and no document-start or document-end markers. End the
document with one newline. Empty arrays, including an empty `machines`
array, use `[]`; null values use `null`. Preserve the JSON types: integers
remain integers, strings remain strings, and decoded metadata remains
nested mappings and arrays rather than strings containing JSON. Quote and
escape strings whenever needed to preserve values such as `null`, `true`,
numeric-looking strings, paths, colons, hashes, and embedded line breaks.

Keep machine arrays sorted by full name in both formats. Preserve the
existing order of reasons, warnings, and creation options. YAML mapping
order must be deterministic for an unchanged report; consumers should use
field names rather than depend on mapping order.

Structured stdout must contain only the selected document. Include report
warnings in the `warnings` array. Keep verbose diagnostics and external
query stderr on stderr under the existing verbosity contract. Use native
string values in both formats; do not apply the human formatter's visible
control-character escaping before serialization. Serialized control
characters must remain safely encoded by the selected format.

An empty inventory scope succeeds with `machines: []` in either format;
do not print `No matching machines.` outside the document. Unknown rows,
unavailable automatic default definitions, and per-row metadata problems
retain their existing successful report behavior. Runtime, workspace,
explicit definition, and fatal inventory failures retain nonzero exit
status and diagnostics on stderr, with empty stdout. Complete serialization
before writing stdout so a serialization failure cannot emit a partial
document. An operating-system write failure may leave bytes already written
and must return a nonzero exit status.

## Human-readable spacing

Keep current labels, table columns, detail lines, full values, ordering,
and terminal-control escaping. Change only the separators as follows:

1. Keep the four reference lines together, from `Reference workspace:`
   through `Current policy revision:`.
2. If warnings exist, insert exactly one empty line after the reference
   block, then print the consecutive `WARNING:` lines as one block.
3. Insert exactly one empty line before the summary block. The summary
   block contains `Unassociated workspace candidates:` and the existing
   assessment explanation. If warnings are absent, this separator directly
   follows the reference block.
4. Insert exactly one empty line after the summary block, before either
   the table header or `No matching machines.`.
5. Keep the table header adjacent to the first machine row. Keep each
   machine row adjacent to all of its reason and metadata detail lines.
   Insert exactly one empty line between complete machine entries.
6. Emit no leading empty line or trailing empty line. End the last content
   line with one newline. An empty scope prints no table header.

Here an empty line contains no spaces or tabs. Values containing newlines
must remain escaped in human output, so machine metadata cannot introduce
extra separators.

## Scope, ownership, and compatibility

The implementer owns CLI parsing and help in `src/cli.rs`, report dispatch
in `src/lifecycle.rs`, serialization and spacing in `src/listing.rs`,
independent regressions in `tests/listing.rs`,
`tests/identity_metadata.rs`, and `tests/cli_contract.rs`, documentation,
and matching dependency and version updates. Use a Rust YAML serializer;
the installed executable must not require an external YAML tool, Python,
or a repository checkout.

Preserve list's read-only behavior: only runtime version and inventory
queries, no virtualization preflight, no application lifecycle locks,
no application writes, and no machine mutations. Preserve definition
selection, all report semantics, templates, labels, hashing, state paths,
and lifecycle policy. This feature requires no `REVISION` increment.

Update `README.md` and `RUST-APPLICATION.md` during implementation with
YAML, JSON, default text, and retained `--json` examples. The feature does
not add formats to other commands, output files, filters, custom columns,
or a new persistent configuration setting.

Specification preparation left software version `0.13.0` unchanged.
Implementation is a feature: increment MINOR in `Cargo.toml` and the
matching `smolsandbox` package entry in `Cargo.lock`, resetting PATCH to
zero. From the observed baseline the implementation version is `0.14.0`;
derive the bump from the implementation baseline if that baseline advances.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | Both syntaxes and both values select the requested format before or after WORKSPACE, with `--all`, selection options, host additions, and verbosity. | CLI integration fixtures covering equivalent invocation forms and identical report content. |
| AC-02 | Omitted selectors produce human output; existing `--json` produces exactly the same bytes as `--format json` for the same report. | Default-output assertions and byte-for-byte alias comparisons. |
| AC-03 | Missing, empty, unsupported, differently cased, repeated, and conflicting selectors exit 2 with empty stdout and no runtime calls; other commands reject `--format`. | Argument-error matrix, including both selector orders and a call log. |
| AC-04 | YAML and JSON parse to equivalent typed report values with schema version 1 and all existing fields, nulls, arrays, metadata, and diagnostics. | Parse each output independently against fixed expected values for mixed inventories, legacy rows, missing automatic defaults, and empty scopes. |
| AC-05 | Structured values round-trip spaces, Unicode, Windows paths, equals signs, colons, hashes, quotes, controls, and YAML-sensitive scalar strings without type changes or added rows. | Independent serializer fixtures covering top-level, reference, and nested machine values, including `null`, `true`, `00123`, and strings containing line breaks. |
| AC-06 | YAML follows the document and indentation contract; machine ordering and repeated serialization are deterministic. JSON retains its existing compact representation. | Representative output assertions, repeated-output comparisons, and parsed machine-order checks. |
| AC-07 | Human output has exactly the specified blank separators while preserving all existing content and escaping. | Full-output fixtures with zero, one, and multiple machines, with and without warnings, and multiline metadata sentinels. |
| AC-08 | Operational failures leave structured stdout empty; successful verbose output remains parseable with diagnostics on stderr. Formatting does not change exit status or assessment semantics. | Run existing inventory and query-failure fixtures for both formats, including malformed inventory and verbose mode. |
| AC-09 | Every format preserves no-write, no-lock, no-preflight, and no-mutation guarantees, including absent application directories. | Existing state snapshots and version/inventory-only call-log assertions parameterized by format. |
| AC-10 | Offline help and active documentation explain the format choices, text default, compatibility alias, and conflicts. Matching software versions receive the minor bump and lifecycle policy stays unchanged. | Help contract assertions, documentation and version diff review, and all seven repository Rust checks. |

## Implementation evidence

During implementation, create `FEATURES/feature-0020-validation.md` with
the tested revision, application version, native target, commands, exit
codes, and AC-01 through AC-10 results using `passed`, `failed`, `not_run`,
or `inconclusive`. Run all seven independent Rust checks in
[AGENTS.md](../AGENTS.md), including both feature configurations, the
production release build without `test-adapter`, and baseline independence.
Append new results and pending native or real guest checks to
[Feature 0010 validation](feature-0010-validation.md), preserving previous
observations and their source revisions. Keep implementation acceptance
pending while a mandatory criterion lacks passing evidence.

Software checks were `not_run` during specification preparation because
this change adds no executable behavior, tests, dependencies, or build
configuration. No new native-runtime or real guest result is claimed.
