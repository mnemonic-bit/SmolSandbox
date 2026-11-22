# Bug 0007: Machine labels retain the Codex prefix instead of SmolSandbox

## Status and requested outcome

Open. Report prepared on 2026-11-22 against `smolsandbox 0.13.0`, invoked
with `smolsbx`, at commit `e9957bbee4816dc09ff5549fcbe3d5ae80793324`.

New SmolVM machines receive labels under `codex-sandbox.`. The requested
namespace is `SmolSandbox.`, using the repository name's exact spelling and
capitalization, for all harness profiles and machine kinds.

This report adds no software change. Both package versions remain unchanged
under [AGENTS.md](../AGENTS.md).

## Observed behavior and impact

`Lifecycle::label` in [src/lifecycle.rs](../src/lifecycle.rs#L249) builds every
creation label as `codex-sandbox.{key}={value}`. Builder, login, and workspace
creation all use this helper. The namespace therefore identifies a
Codex-specific application even when the selected harness is Claude,
OpenCode, or Hermes.

The canonical workspace path is stored in the JSON value of
`codex-sandbox.machine-inputs`, under `workspace_path`. The separate
`codex-sandbox.workspace-path-sha256` label stores the path association hash.
The requested names are `SmolSandbox.machine-inputs` and
`SmolSandbox.workspace-path-sha256`, respectively.

Source inspection confirms the namespace mismatch. No real SmolVM machine
was created or inspected for this report. The report does not claim that the
current application fails to recognize its own existing labels.

The prefix is also part of ownership recognition, cleanup selection,
descriptive metadata decoding, and documented inventory queries. Inference:
changing only the creation helper would make new labels disagree with the
current readers. The eventual correction must cover both writing and reading.

## Affected label keys

Replace the namespace of these eight defined keys while retaining their
kind-specific meaning and value formats:

| Current key | Requested key |
| --- | --- |
| `codex-sandbox.kind` | `SmolSandbox.kind` |
| `codex-sandbox.base-id` | `SmolSandbox.base-id` |
| `codex-sandbox.revision` | `SmolSandbox.revision` |
| `codex-sandbox.workspace-sha256` | `SmolSandbox.workspace-sha256` |
| `codex-sandbox.workspace-path-sha256` | `SmolSandbox.workspace-path-sha256` |
| `codex-sandbox.configuration` | `SmolSandbox.configuration` |
| `codex-sandbox.base-inputs` | `SmolSandbox.base-inputs` |
| `codex-sandbox.machine-inputs` | `SmolSandbox.machine-inputs` |

Tests also use synthetic suffixes such as `unknown`, `future`, `extra`, and
`note` to exercise namespace recognition. Include those cases in the fix.

## Production readers and writers

| Location at the report revision | Responsibility |
| --- | --- |
| `src/lifecycle.rs:249–251`, `278–279`, `298–299`, `352–354`, `376–389` | `Lifecycle::label` writes the prefix; creation paths supply ownership and descriptive keys for builders, login machines, and workspace machines. |
| `src/lifecycle.rs:691–705` | `Lifecycle::clean` independently constructs prefixed keys to select cleanup candidates. |
| `src/domain.rs:10–17`, `70–85` | `MANAGED_KEYS` defines six ownership keys. `Machine::has_ownership_labels` checks the namespace and excludes the two descriptive keys. `Machine::label` reads through `MANAGED_KEYS`. |
| `src/metadata.rs:5–6`, `233–240`, `292–293` | `BASE_INPUTS` and `MACHINE_INPUTS` define the descriptive keys used for decoding and diagnostics. |
| `src/listing.rs:127`, `347–369`; `src/removal.rs:139–199` | Listing and removal consume the domain ownership helpers and constants. These production consumers have no additional prefix literal. |

The same domain label reader also serves lifecycle readiness, classification,
managed inventory, removal planning, and metadata validation. Preserve those
behaviors when changing the namespace.

## Complete inventory of literal label-prefix uses

The inventory below covers every literal label-namespace occurrence in all
Git-tracked files at the report revision: **22 files, 761
matching lines, and 2700 occurrences**. Each line number refers
to the report revision, before adding this report. Ranges contain consecutive
matching lines; comma-separated entries identify separate matching locations.
Multiple prefix occurrences on one line count separately.

The search excludes `codex-sandbox.sh` and `codex-sandbox.exe`, whose shared
substring denotes historical executable names. The
`codex-sandbox-harness` definition marker, machine names such as
`codex-workspace-*`, and artifact names are separate identifiers outside the
requested label-prefix correction. Git history, ignored build output, and
the text of this new report are outside the inventory.

| File | Matching line numbers | Occurrences | Use |
| --- | --- | --- | --- |
| [FEATURES/feature-0001-reconcile-workspace-sandbox-state.md](../FEATURES/feature-0001-reconcile-workspace-sandbox-state.md#L113) | 113–116 | 4 | Original workspace ownership-label contract. |
| [FEATURES/feature-0003-purge-superseded-sandbox-artifacts.md](../FEATURES/feature-0003-purge-superseded-sandbox-artifacts.md#L159) | 159–160, 165, 168, 173 | 5 | Path association and configuration labels for purge. |
| [FEATURES/feature-0005-configurable-workspace-network-allowlist.md](../FEATURES/feature-0005-configurable-workspace-network-allowlist.md#L197) | 197 | 1 | Immutable workspace identity label. |
| [FEATURES/feature-0007-configurable-sandbox-port-publishing.md](../FEATURES/feature-0007-configurable-sandbox-port-publishing.md#L244) | 244 | 1 | Identity invariants for mutable ports. |
| [FEATURES/feature-0010-validation.md](../FEATURES/feature-0010-validation.md#L106) | 106 | 1 | Historical compatibility observation; preserve the observation and append later validation. |
| [FEATURES/feature-0011-rename-executable-to-smolsandbox.md](../FEATURES/feature-0011-rename-executable-to-smolsandbox.md#L66) | 66, 194 | 2 | Explicit retention of the old metadata namespace during the executable rename. |
| [FEATURES/feature-0012-list-workspace-machines.md](../FEATURES/feature-0012-list-workspace-machines.md#L50) | 50, 90 | 2 | Workspace path association and managed inventory scope. |
| [FEATURES/feature-0013-remove-stale-machines.md](../FEATURES/feature-0013-remove-stale-machines.md#L114) | 114, 155 | 2 | Workspace selection and managed-metadata candidates for removal. |
| [FEATURES/feature-0015-rename-command-to-smolsbx.md](../FEATURES/feature-0015-rename-command-to-smolsbx.md#L143) | 143 | 1 | Explicit retention of the old labels during the command rename. |
| [FEATURES/feature-0019-readable-machine-identity-metadata.md](../FEATURES/feature-0019-readable-machine-identity-metadata.md#L87) | 87, 140, 165, 267 | 4 | Ownership contract, descriptive metadata keys, and raw SmolVM query. |
| [RUST-APPLICATION.md](../RUST-APPLICATION.md#L116) | 116, 381, 441 | 4 | Compatibility policy, descriptive metadata keys, and raw SmolVM query. |
| [src/domain.rs](../src/domain.rs#L11) | 11–16, 72, 75, 632, 634, 651, 942 | 13 | Production ownership keys and namespace detection; unit tests for descriptive and unknown labels. |
| [src/lifecycle.rs](../src/lifecycle.rs#L250) | 250, 694 | 2 | Production label writer and separate cleanup label reader. |
| [src/listing.rs](../src/listing.rs#L569) | 569–574, 587–590, 595, 631, 635, 637–638, 642, 668, 670, 692, 717–718, 720 | 24 | Unit-test inventory builders, classification cases, and malformed labels. |
| [src/metadata.rs](../src/metadata.rs#L5) | 5–6, 324–325, 327, 331, 448–450, 495 | 10 | Production descriptive metadata constants; metadata validation unit tests. |
| [tests/application.rs](../tests/application.rs#L515) | 515, 546–547, 554, 693, 702, 715, 753, 787, 845, 877, 984, 1004, 1200, 1206–1210, 1215, 1532, 1544, 1546, 1624, 1702, 1798–1799, 1817, 1858, 1881, 1931–1932, 1935, 1940, 1942, 1947, 2051, 2366, 2377, 2382, 2384, 2428 | 47 | Lifecycle creation, identity, ownership, replacement, cleanup, and purge regressions. |
| [tests/data/ownership-cases.json](../tests/data/ownership-cases.json#L3) | 3–8, 11–16, 19–24, 27–32, 35–40, 43–48, 51–56, 59–64, 67–72, 75–80, 83–88, 91–96, 99–104, 107–112, 115–120, 123–128, 131–136, 139–144, 147–152, 155–160, 163–168, 171–176, 179–184, 187–192, 195–200, 203–208, 211–216, 219–224, 227–232, 235–240, 243–248, 251–256, 259–264, 267–272, 275–280, 283–288, 291–296, 299–304, 307–312, 315–320, 323–328, 331–336, 339–344, 347–352, 355–360, 363–368, 371–376, 379–384 | 1488 | Ownership classification and readiness fixture matrix. |
| [tests/data/purge-cases.json](../tests/data/purge-cases.json#L2) | 2–271 | 990 | Purge selection fixture matrix. |
| [tests/identity_metadata.rs](../tests/identity_metadata.rs#L249) | 249, 251–252, 254, 261, 280, 288, 347, 375, 398, 403, 409, 417–418, 439, 442, 448, 506, 509–510, 537, 540, 548–549, 556–557, 573 | 27 | Independent creation-label expectations and readable metadata regressions. |
| [tests/listing.rs](../tests/listing.rs#L134) | 134–139, 205–208, 211, 213–214, 217, 219, 224, 226, 228, 231, 251, 377–378, 608, 810–811, 855, 1002, 1051–1052, 1178, 1198 | 34 | Listing, metadata validation, workspace association, and raw-query regressions. |
| [tests/removal.rs](../tests/removal.rs#L106) | 106–109, 207, 244–245, 268, 270, 273, 297–300, 312, 333–335, 384, 426–427, 429–430, 435, 437–438, 611–612, 667, 931 | 37 | Removal ownership, selection, contradiction, and revalidation regressions. |
| [tests/support/verbose.rs](../tests/support/verbose.rs#L342) | 342 | 1 | Verbose lifecycle integration assertion. |

Reproduce the inventory against the fixed baseline with:

```sh
git grep -n -P 'codex-sandbox\.(?!sh\b|exe\b)' e9957bbee4816dc09ff5549fcbe3d5ae80793324 -- .
```

## Relationship to earlier requirements and existing machines

[Feature 0011](../FEATURES/feature-0011-rename-executable-to-smolsandbox.md#L194)
and [Feature 0015](../FEATURES/feature-0015-rename-command-to-smolsbx.md#L143)
explicitly retained the old label namespace during earlier command renames.
[RUST-APPLICATION.md](../RUST-APPLICATION.md#L116) documents that compatibility
policy, and [Feature 0019](../FEATURES/feature-0019-readable-machine-identity-metadata.md#L131)
preserves the existing ownership keys while adding descriptive metadata.
The current implementation follows those earlier requirements. This bug
report records the user's new requirement and supersedes their retention of
the old prefix for newly created machines.

Existing machines retain their creation-time labels. Before implementing the
correction, the implementer must define and document how the corrected
application handles machines with the old prefix and machines containing
both prefixes. The compatibility mechanism remains an open implementation
decision. Cover contradictory values explicitly so the transition cannot
silently select or delete an unrelated machine. Descriptive metadata alone
must continue to provide no ownership authority.

Update current operating instructions and requirements when implementing the
correction. Preserve historical observations and their source revisions;
append new validation evidence rather than rewriting past results.

## Reproduction

1. Inspect `Lifecycle::label` at the report revision. Observe the literal
   `codex-sandbox.` in each generated `--label` value.
2. On a host configured according to `RUST-APPLICATION.md`, create a new
   machine through a normal `smolsbx` lifecycle operation.
3. Run `smolvm machine ls --json` and inspect the new machine's `labels`.
4. Observe keys under `codex-sandbox.`. Expect keys under `SmolSandbox.`.

Step 1 is confirmed by source inspection. Steps 2–4 are a reproduction
procedure for a configured host and were `not_run` during report preparation.

## Acceptance for the eventual fix

The implementer owns the namespace correction, compatibility behavior,
regressions, documentation, and version changes.

- AC-01: Newly created builder, login, and workspace machines use exactly
  `SmolSandbox.` for every application label. Verify complete creation
  arguments independently for all four harness profiles, including both
  descriptive labels and the applicable ownership keys. New creation emits
  no `codex-sandbox.` keys.
- AC-02: Lifecycle readiness, workspace association, list, remove, clean, and
  purge consume the new namespace consistently. Exercise the lifecycle,
  listing, removal, ownership, and purge regressions identified above.
- AC-03: Readable metadata and the documented raw SmolVM query expose the
  canonical workspace path and creation inputs using the new keys. Verify
  the raw-query regression in `tests/listing.rs` and independent metadata
  expectations in `tests/identity_metadata.rs`.
- AC-04: Define and test the transition for old-prefix machines and inventory
  containing both prefixes. Contradictory ownership values cannot silently
  authorize a lifecycle mutation. Descriptive-only labels cannot establish
  ownership. Document any compatibility break and the required user action.
- AC-05: Review every inventory entry. Retain old-prefix occurrences only
  where the occurrence intentionally tests or documents compatibility or
  records historical evidence. Update current queries and requirements to
  use the new prefix. Verify with a repository-wide prefix search.
- AC-06: Apply the package version rule in `AGENTS.md` to the eventual
  implementation in both `Cargo.toml` and `Cargo.lock`. Apply the machine
  creation-policy `REVISION` rule in `src/lifecycle.rs` and update both named
  independent revision expectations in `tests/application.rs`.
- AC-07: Run all seven Rust checks required by `AGENTS.md`. Append new results,
  source revisions, and pending native or real guest checks to
  [Feature 0010 validation](../FEATURES/feature-0010-validation.md).

## Report validation

- Source inspection: `passed`. The label writer, ownership namespace check,
  descriptive metadata constants, and cleanup reader contain the old prefix.
- Inventory: `passed`. A scan of every Git-tracked file produced the complete
  line inventory above. The fixed-revision `git grep` command independently
  identifies the same 22 affected files.
- Structural exploration: `passed`. The required codebase-memory skill traced
  domain label readers and consumers. Coverage reported no recorded issue for
  the cited paths. A partial graph parse at
  `FEATURES/feature-0016-version-update-research.md:36` was checked directly;
  that line contains no label-prefix use. The literal inventory uses full
  source scanning rather than relying on graph completeness.
- Rust checks and real SmolVM reproduction: `not_run`. This change only adds a
  bug report; no software, templates, fixtures, or package versions changed.
- Eventual-fix acceptance AC-01 through AC-07: `not_run`. The namespace
  correction and compatibility decision remain open.

## Implementation and local validation: 2026-11-22

Current status: fixed locally in `smolsandbox 0.13.1`, with machine policy
revision `7`. The report and inventory above preserve the observations at
preparation. The [new validation audit](../FEATURES/feature-0010-validation.md#bug-0007-machine-label-namespace-audit-2026-11-22)
records source revision `e9957bbee4816dc09ff5549fcbe3d5ae80793324` plus the
local software diff fingerprint, commands, and execution limits.

New machines use exactly `SmolSandbox.` for all application labels, across
all four profiles and builder, login, and workspace creation. Ownership,
metadata, listing, removal, clean, and purge readers accept existing
`codex-sandbox.` labels. Readers combine disjoint fields across namespaces;
duplicate ownership values must agree exactly. Any disagreement across the
six ownership fields prevents the affected machine from authorizing a
mutation, including explicit overrides. Descriptive-only labels never
establish ownership. Unequal duplicate descriptive values invalidate the
affected metadata component without changing ownership authority.

Existing labels need no migration. Existing machines retain their
creation-time policy; the revision increment selects new login and workspace
identities through the normal lifecycle rules. Use `smolsbx repair --check`
to inspect selection and `smolsbx repair --replace-stale` for an eligible
stale replacement. If ownership namespaces disagree, inspect the raw SmolVM
inventory and resolve the labels or remove the affected machine through
SmolVM before retrying. User definitions, harness homes, template bytes,
base-generation hashing, and historical executable names remain unchanged.

AC-01 through AC-07: `passed` locally. The exact creation regression failed
before the fix and passed afterward. Independent tests cover complete label
and argument sets, mixed inventories, revision-6 replacement, all six
conflicting ownership pairs, actual removal refusal, cleanup, purge,
descriptive neutrality, readable metadata, and the published raw query.
All seven required Rust checks exited `0`; the default and adapter suites
passed 138 and 213 tests, respectively. The baseline-independence check also
passed ten compiled test executables per configuration with an empty `PATH`.

Real SmolVM execution, other native targets, and hosted CI: `not_run` in this
session. SmolVM and `/dev/kvm` are absent. The validation audit preserves
those pending checks separately from the local results.
