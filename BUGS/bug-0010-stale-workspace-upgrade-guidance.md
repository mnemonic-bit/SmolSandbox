# Bug 0010: Stale workspace diagnostics do not explain upgrade recovery

## Status and summary

Reported on 2026-11-23 against `smolsandbox 0.16.0` at
`fc05a8db17f9deac61f21dad8f2c98619eb033c9`. The correction is implemented
in package `0.16.1`; validation and remaining native checks are recorded in
the [implementation validation](../FEATURES/feature-0010-validation.md#bug-0010-stale-workspace-guidance-implementation-2026-11-23).

After updating the application, a returning user receives a stale workspace
error that does not explain the mismatch or provide complete recovery
commands. The candidate line ends with an empty reason. The suggested
read-only inspection repeats that empty reason, and successful replacement
does not display the command that resumes the workspace.

The requested outcome is actionable output throughout inspection and
replacement, with separate guidance for adopting newer embedded templates.
Preserve explicit replacement authorization and existing lifecycle behavior.
The original report applied no software correction. Both Cargo versions
were `0.16.0` under [AGENTS.md](../AGENTS.md).

## Reported behavior and impact

The user reported this output after trying to install and use the latest
application:

```text
Workspace candidate: codex-workspace-c325d79d25d4-r5-ea93eac1aaf8 (stopped, labels):
ERROR: no machine matches the current sandbox definition; run smolsbx repair --check, then repair --replace-stale with the same workspace and additional hosts; no machines were created or deleted
```

The user expected an explanation of how to resume work after an update
carrying newer templates. The supplied excerpt does not identify the
installed application version, SmolVM version, selected definition, workspace
path, full inventory, or invoking command. The report baseline identifies
the inspected checkout, not a verified version of the user's executable.

The candidate name contains creation-policy revision `5`. The inspected
application requires revision `7`. A revision mismatch alone reproduces the
reported error with unchanged definition bytes. The user's exact local
cause remains unverified because the full inventory and executable version
were not supplied.

The message gives the user incomplete information at several stages:

| Stage | Current output gap | Required user guidance |
| --- | --- | --- |
| `build` or `start` refuses a stale-only workspace | The candidate has no reason; the error attributes the mismatch to the sandbox definition. | Identify the actual mismatch, show existing and desired values, and give a complete read-only inspection command. |
| `repair --check` | The desired name and candidates are printed, but stale reasons remain empty and there is no replacement plan or next command. | Explain selection, identify eligible replacements, describe deletion consequences, and show the applicable replacement command. |
| `repair --replace-stale` before mutation | Candidates are printed without an explanation of the intended handoff or discarded state. | Name the planned replacement and deletion candidates; explain verification before deletion and guest-local disk loss. |
| Successful replacement | `Workspace machine ready: ...` supplies no command to resume work. | Show a complete `smolsbx start` command for the selected configuration. |
| Adoption of newer embedded templates | Repair guidance does not explain that existing definition files remain unchanged. | Explain separate-file generation, comparison, explicit selection or editing, and the distinction between machine replacement and template adoption. |

## Source analysis and intended behavior

[Lifecycle::new](../src/lifecycle.rs#L135) derives the base identity from
selected definition bytes and the SmolVM version. Workspace creation options
determine the workspace hash. The separate `REVISION` constant participates
in workspace names and ownership checks; its current value is `7`.

Feature 0019 added creation metadata and
[required revision 5 to become 6](../FEATURES/feature-0019-readable-machine-identity-metadata.md#L352).
Bug 0007 changed the managed-label namespace and
[advanced the revision to 7](bug-0007-machine-label-prefix.md#L205).
Consequently, an application update can require explicit replacement even
when the user has not edited the definition file.

[domain::classify](../src/domain.rs#L211) assigns an empty reason to ordinary
stale candidates, including the base/revision mismatch branch.
[Lifecycle::reconcile](../src/lifecycle.rs#L448) prints that reason for both
normal refusal and `repair --check`. The same method emits the generic
definition-mismatch error and performs explicit replacement.
[lifecycle::run](../src/lifecycle.rs#L1030) prints replacement success without
a subsequent start command.

The refusal itself follows the documented
[reconciliation contract](../RUST-APPLICATION.md#reconciliation-and-replacement).
Normal startup does not create or delete machines when only stale or
ambiguous candidates are available. Explicit replacement prepares the base,
creates the desired machine if needed, and verifies readiness from fresh
inventory before deleting eligible stale machines. Preparation or
verification failure preserves the stale machines. Partial cleanup failure
can leave the verified replacement and some stale machines present.

[Template adoption](../RUST-APPLICATION.md#template-adoption) is separate.
Updating the executable does not refresh or merge existing user-owned
definitions. Repair uses the selected definition; repair does not copy a
new embedded template into that file. Changing definition bytes selects a
new base generation. A policy-revision mismatch does not establish that the
selected definition differs from the embedded template or that the user
wants to adopt a newer template.

## Reproduction and observed validation

The investigation used the Rust SmolVM test double in disposable directories:

1. Generate a Codex definition using the baseline's `create-config` command.
2. Run `start` to create a managed workspace machine.
3. Change only the test inventory's workspace name from `-r7-` to `-r5-`
   and its `SmolSandbox.revision` label from `7` to `5`.
4. Run `start` again with the same definition, workspace, and runtime version.
5. Run `repair --check` with the same selections.
6. Run `repair --replace-stale` in that disposable environment.
7. Run `start` again.

Step 4 exited `1` and reproduced the user's diagnostic, including the blank
reason. The inventory bytes remained unchanged. Step 5 exited `0`, printed
an `r7` desired machine and an `r5` candidate with an empty reason, and left
inventory unchanged. Steps 6 and 7 exited `0`. The selected definition bytes
remained unchanged. The disposable reproduction script was not retained.

The following focused regression command was also run on the report baseline:

```text
cargo test --locked --features test-adapter --test application stale -- --nocapture
```

Result: `passed`; exit `0`, six tests passed, 65 tests filtered out, in
3.20 seconds. The six tests cover stale-only refusal, preparation and
verification before deletion, failed replacement preservation, running stale
replacement authorization, generation-cache recovery, and port-incarnation
state. These tests establish existing lifecycle behavior; they do not assert
the enhanced diagnostics requested below.

Structural investigation used the installed `codebase-memory` skill,
symbol snippets, the reconciliation caller trace, and direct source reads.
Coverage reported no recorded issue for `src/lifecycle.rs`, `src/domain.rs`,
`src/config.rs`, `tests/application.rs`, `README.md`, and
`RUST-APPLICATION.md`; that signal does not establish graph completeness.
The [Bug 0010 investigation record](../FEATURES/feature-0010-validation.md#bug-0010-stale-workspace-guidance-investigation-2026-11-23)
preserves these observations separately from eventual-fix validation.

## Requested correction and acceptance

The implementer owns the diagnostic changes, regression checks, and operating
documentation. Exact prose is an implementation choice; the information and
behavior below are required. All eventual-fix criteria are `not_run` at
report preparation.

- AC-01: Every displayed stale candidate has a nonempty explanation. For a
  revision-only mismatch, both normal refusal and `repair --check` show the
  existing and required creation-policy revisions. Distinguish revision,
  base-identity, and explicit-name selection mismatches when established by
  classification. Identify unknown evidence without inventing a cause.
  Verify an unchanged-definition revision-5-to-7 fixture and the other stale
  classification branches.
- AC-02: A stale-only `build` or `start` refusal explains why explicit
  replacement is required and accurately reports that no machines were
  created or deleted. Display a complete `smolsbx repair --check` command
  using the resolved workspace, selected definition, and additional hosts.
  Preserve an explicit runtime selection when supplied. Quote paths and
  argument values for the supported host shell, including spaces. Do not
  include secrets or guest harness arguments in repair commands. Verify
  default selection, custom `--smolfile`, `BASE_SMOLFILE`, additional hosts,
  and explicit `--smolvm` selection.
- AC-03: `repair --check` explains whether a desired machine exists and names
  the eligible stale candidates. For eligible replacement, display a
  complete `smolsbx repair --replace-stale` command with the same effective
  selections. Explain that a running stale machine requires stopping or
  explicit `--force` authorization. For ambiguous ownership, explain the
  conflict and required inspection; do not present replacement or `--force`
  as a way to bypass ambiguity. Verify stopped, running, ambiguous, and
  desired-plus-stale inventories. Inspection remains read-only and lock-free.
- AC-04: Before replacement mutation, display the planned desired machine and
  stale deletion candidates. Explain that replacement readiness is verified
  before deletion, that deletion discards guest-local writable disk state,
  and that mounted project files, the selected definition, and persistent
  harness home remain. Include the consequence in the read-only inspection
  output so users can decide before invoking replacement. Verify output
  ordering against recorded runtime calls. Retain the existing explicit
  `--replace-stale` authorization without adding an interactive prompt.
- AC-05: After successful replacement, display a complete next `smolsbx
  start` command with the same workspace, selected definition, additional
  hosts, and explicit runtime selection. Do not imply that replacement has
  already launched the guest harness. On failed preparation, verification,
  or partial deletion, retain accurate preservation and partial-completion
  messages; do not print the successful handoff guidance. Verify success
  and each failure stage with the test double.
- AC-06: Upgrade recovery output includes a short explanation that an
  executable update preserves the selected definition and that replacement
  uses that definition. Provide a separate template-adoption path: generate
  the selected profile into a new, nonexisting destination with
  `create-config --harness <profile> --smolfile <new-path>`, compare with the
  displayed selected definition, and explicitly select or edit the desired
  file. Supply commands or a precise reference to the operating instructions.
  Do not claim that a newer template caused the mismatch without evidence.
  Verify that repair preserves definition bytes and that separate-file
  generation refuses to overwrite an existing destination.
- AC-07: Update README recovery guidance and the authoritative operating
  reference to match the enhanced output. Preserve machine classification,
  identity inputs, readiness checks, creation-before-deletion ordering,
  failure preservation, running-machine authorization, exit statuses, and
  user-owned definitions. Review fixed classification-reason expectations
  when adding reasons; retain independent category and ownership checks.
- AC-08: Include matching PATCH version bumps in `Cargo.toml` and the
  `smolsandbox` entry in `Cargo.lock` for the implementation. A diagnostic
  correction requires no lifecycle `REVISION` increment. Run the seven Rust
  checks in [AGENTS.md](../AGENTS.md), including both feature configurations.
  Append source revisions, commands, results, and pending native or real
  guest checks to [Feature 0010 validation](../FEATURES/feature-0010-validation.md).

## Remaining evidence and scope

Real SmolVM reproduction on the user's host is `not_run`. The exact installed
version and full inventory remain unknown. Eventual-fix diagnostic tests,
native shell quoting checks, the seven full Rust checks, and real guest
checks are `not_run` for this report-only change. No real machine was
created, replaced, or deleted during the investigation.

Automatic template migration, automatic workspace replacement, in-place
machine relabeling, and guest disk migration are outside this correction.

## Implementation follow-up: 2026-11-23

Package `0.16.1` adds evidence-based stale reasons, complete quoted recovery
commands, read-only replacement plans, deletion consequences, and a start
command after successful replacement. Recovery commands retain the resolved
workspace, selected definition, runtime, and additional hosts. Upgrade output
points to separate template adoption without attributing an unverified cause
to the mismatch. Lifecycle policy revision remains `7`.

The report preparation observations and `not_run` results above describe the
original report-only state. The implementation validation record preserves
the new regression results and pending native shell and real guest checks
separately.
