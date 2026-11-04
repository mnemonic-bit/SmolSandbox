# Feature 0014: Show routine SmolVM output only with --verbose

## Status

Proposed

This specification defines implementation work. Adding this document does not
change runtime output or implement `--verbose`.

## Summary

Suppress routine output from noninteractive SmolVM operations by default.
Add an application-wide `--verbose` flag that displays that output while the
operations run. Continue showing smolsandbox's own results, warnings, and
errors in both modes.

Preserve direct terminal access for interactive guest sessions. Shell input,
shell output, harness output, authentication prompts, and terminal control
must remain available without `--verbose`. The distinction is the purpose of
the invocation: management and provisioning output is optional; output from
the guest session the user entered is always visible.

## Baseline and affected paths

Specification preparation observed package version `0.6.0` in the uncommitted
Feature 0012 working tree based on commit
`e092975222c645a129de7460707b18d884e205cc`. That commit alone does not contain
the listing implementation. Feature 0013 is a separate proposed removal
feature; neither proposal is implemented by this document.

Source observations at preparation time:

- `Smolvm::run` in `src/smolvm.rs` starts management subprocesses with inherited
  output, allowing their stdout and stderr to appear directly on screen.
- `Smolvm::capture` captures stdout and stderr for version checks, inventory,
  and capability inspection. `Smolvm::discover` separately captures the
  initial version query.
- `Smolvm::interactive` explicitly inherits stdin, stdout, and stderr for
  interactive guest execution.
- `Lifecycle::command` routes management work through `Smolvm::run`.
  `Lifecycle::seed` uses that path for a noninteractive guest shell command.
- `Lifecycle::guest` invokes `machine exec -it` through
  `Smolvm::interactive`. Workspace harness execution and login use this path.

Read the implementation baseline before editing. Record changes to these
paths in the implementation validation record rather than assuming every
`machine exec` invocation has the same output requirements.

## Command interface

```text
smolsandbox build
smolsandbox --verbose build
smolsandbox build --verbose /path/to/project
smolsandbox start /path/to/project --verbose
smolsandbox login --verbose
smolsandbox repair --check --verbose /path/to/project
smolsandbox list --all --json --verbose
smolsandbox start /path/to/project -- --verbose
```

Define `--verbose` as a Boolean global application option, accepted before the
subcommand or among application arguments after it. Permit placement before
or after WORKSPACE. Repeating the flag is idempotent and does not enable
additional verbosity levels. Do not add an environment variable or persistent
setting in this feature.

Preserve the first standalone `--` boundary on `start`. Arguments after that
boundary belong to the guest harness unchanged. In the final example above,
the guest receives `--verbose`; smolsandbox remains in its default output
mode. An application `--verbose` before the boundary must not be forwarded to
SmolVM or to the guest command.

Show `--verbose` in top-level and subcommand help. Accept it consistently on
all current subcommands, including offline `create-config`. A command that
does not call SmolVM has no additional runtime output to show. If
[Feature 0013](feature-0013-remove-stale-machines.md) is implemented, its
`remove` command follows the same output policy; implementing removal is not
a dependency of this feature.

Help and version retain the offline contract even when `--verbose` is
present. Preserve argument-error exit status `2` and existing command
validation. Do not add a `-v` alias or a separate quiet mode in this feature.

## Output policy

Classify each runtime invocation by its purpose before starting the process.
Do not classify solely by executable name, the presence of `machine exec`,
output text, or whether output happens to contain shell prompts.

| Invocation or output | Default mode | With `--verbose` |
| --- | --- | --- |
| Management operations: machine creation, start, stop, update, deletion, artifact packing, and build/provisioning progress | Suppress child stdout and stderr. | Stream both child outputs to application stderr while the operation runs. |
| Internal noninteractive guest commands, including harness-state seeding | Suppress child stdout and stderr. | Stream both child outputs to application stderr while the operation runs. |
| Queries whose stdout is consumed as data: version, inventory JSON, capability help | Capture stdout for the existing consumer; suppress child stderr. | Keep stdout internal; stream child stderr to application stderr. |
| User-facing interactive shell, harness, or login session | Inherit stdin, stdout, and stderr directly. | Inherit stdin, stdout, and stderr directly. |
| smolsandbox results, selection information, warnings, and errors | Preserve existing visibility and output channels. | Preserve existing visibility and output channels. |

Successful default-mode management operations must not leak child banners,
progress bars, provisioning logs, shell-command output, or status messages
through either stdout or stderr. This requirement covers preparations before
an interactive session and cleanup after that session.

With `--verbose`, management output is available during execution rather than
only after the child exits. Forward each child's stdout to application stderr
and each child's stderr to application stderr; preserve byte order within
each source stream. No total ordering between independently produced stdout
and stderr is promised. Do not turn runtime output into application stdout or
change machine-readable output to accommodate verbose logs.

Query stdout remains protocol data in both modes. Do not echo raw inventory
JSON, capability-help responses, or version-response bytes merely because
verbose mode is enabled. An optional application diagnostic may identify a
query and its outcome without dumping its response. Retain the exact raw
stdout needed by existing version identity calculation and JSON parsing.

Do not buffer all management output merely to suppress it. Drain concurrent
streams without pipe deadlocks in verbose mode, including output larger than
the operating system's pipe capacity. Do not create persistent log files or
write runtime output into application caches or the workspace. Existing
protocol capture is separate from suppression of lifecycle output.

Before launching an interactive guest, finish and drain output from completed
management children. Do not leave a verbose forwarding thread running against
the terminal after its operation has completed or spill delayed preparation
logs into the guest session.

## Interactive shell and harness guarantees

Interactive execution must retain the existing `machine exec -it` arguments,
guest argument boundaries, selected machine, secret references, and inherited
standard handles. This feature does not introduce a new shell command or
change which harness is launched.

In both output modes:

- Deliver keyboard input and redirected stdin to the interactive child using
  the existing behavior. Do not replace session stdin with a null stream.
- Display guest stdout and stderr immediately, including shell prompts,
  command results, harness interfaces, device-authentication instructions,
  and login prompts.
- Preserve terminal detection, terminal dimensions and resize propagation,
  ANSI control sequences, raw-mode behavior, and interactive echo behavior.
- Preserve the existing handling of Ctrl-C, termination, guest exit codes,
  machine ownership, and cleanup after the guest exits.
- Preserve argument values and boundaries after `--`, including a guest's own
  `--verbose` or `--help` option.

Do not wrap interactive execution in the management-output capture or
forwarding path. Do not redirect the whole application or globally replace
its standard streams. Do not apply line filtering to the interactive stream.
When SmolVM emits its own connection messages through the same interactive
process handles, those messages remain visible along with the session. The
feature promises suppression of separate management subprocess output, not
reliable separation of runtime and guest bytes inside an attached session.

The noninteractive `sh -lc` used for internal seeding remains a management
operation despite running a shell in the guest. Conversely, a user-facing
interactive shell remains visible even if its executable is also `sh`.

## Failures and diagnostics

Default mode suppresses raw noninteractive child output on failure as well as
success. It must still produce a smolsandbox diagnostic on stderr naming the
failed operation, the relevant machine when applicable, and the exit status
or spawn/read failure. Explain that `--verbose` enables runtime diagnostics.
Do not require verbose mode to learn that an operation failed.

For example, default-mode diagnostics may report:

```text
ERROR: SmolVM machine create failed for <machine> with exit status 71. Use --verbose to display runtime output.
```

Adapt context to version discovery, capability inspection, inventory failure,
JSON parsing, packing, stop, delete, and internal guest operations. Preserve
the existing difference between warnings and fatal errors. A seeding failure
remains a warning when the existing lifecycle policy permits continuation.
A post-build purge failure retains its existing warning behavior.

With `--verbose`, retain the application diagnostic in addition to streamed
runtime output. Do not replay an already streamed child log after failure.
Do not retry failed operations automatically to obtain verbose diagnostics.

Do not change exit-status precedence. A guest's nonzero status remains primary
when cleanup also fails; cleanup failure after guest success still produces
an operational failure. Raw interactive guest errors remain visible without
verbose mode because the session output policy is unchanged.

Application-generated verbose diagnostics must not dump environment values,
credential files, secret contents, definition contents, or reconstructed
guest command lines. Continue passing secrets by existing reference mechanisms.
Do not add runtime logging arguments that change the actual operation.

## Machine-readable output

`list --json` writes the same single schema-versioned object to stdout in
both modes. Route verbose runtime output and application diagnostics only to
stderr. The output flag must not alter the report's schema, assessments,
sorting, identity fields, or warnings array.

If inventory discovery or parsing fails, preserve the existing nonzero exit
and empty JSON stdout contract in both modes. Verbose stderr must not become
part of the captured inventory response. Do not make read-only commands issue
extra runtime calls, acquire locks, or write application files because the
flag is present.

## Compatibility and implementation scope

The implementer owns global argument parsing, explicit process-output policy,
all runtime launch sites including initial discovery, lifecycle diagnostics,
concurrent stream handling, tests, and active documentation updates.

Keep the management and interactive execution interfaces explicit enough that
new call sites must choose their output behavior. Preserve existing standard
input behavior outside the output changes specified here. Avoid a broad
global output switch that could silence the interactive execution path.

The flag changes output presentation only. Preserve runtime operation count
and arguments, definition selection, creation labels, machine names, base and
configuration hash inputs, machine policy `REVISION`, port behavior, lock
order, mutation eligibility, lifecycle ordering, and existing stop/delete
failure preservation. Do not change SmolVM's own default verbosity or depend
on an unverified SmolVM quiet flag.

Update README and `RUST-APPLICATION.md` with default/verbose examples and the
interactive-session exception. Explain that `start -- --verbose` configures
the guest while `start --verbose` configures smolsandbox output.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | Successful noninteractive management and internal seeding output is absent from both application streams by default, while application results remain visible. | Test-double stdout/stderr markers for create, start, update, pack, stop, delete, and internal exec. |
| AC-02 | With verbose enabled, management stdout and stderr appear on application stderr before the child exits and do not appear on application stdout. | Coordinated subprocess fixture that emits markers and waits for release; verify marker receipt before release. |
| AC-03 | Verbose parsing works before/after subcommands and workspace arguments, is idempotent when repeated, and does not change other CLI validation. | CLI parser and process exit-code tests across current commands. |
| AC-04 | Guest arguments after `--` retain their exact boundaries and do not enable application verbosity. | Fixed argv fixtures for guest `--verbose`, `--help`, empty arguments, and values containing spaces. |
| AC-05 | Interactive guest stdout, stderr, stdin, and prompt interaction work identically with and without verbose. | Bidirectional child fixture in both modes; verify guest output, stdin round-trip, and inherited stream behavior. |
| AC-06 | Real terminal behavior remains unchanged: terminal detection, resize, ANSI rendering, Ctrl-C, exit status, and interactive authentication. | Native terminal/PTY or console tests where supported, plus recorded real SmolVM shell/harness and login smoke checks; pipe-only tests do not establish terminal behavior. |
| AC-07 | Version/help/inventory query stdout remains available to its consumer without raw-payload leakage, and verbose diagnostics cannot corrupt JSON. | Query marker fixtures, unchanged fixed identity expectations, parsed `list --json` output, and separate stderr assertions. |
| AC-08 | Runtime spawn failure, nonzero exit, capability failure, malformed inventory, and cleanup/seeding failures remain diagnosable without raw default-mode child logs. | Failure matrix asserting operation context, status, verbose guidance, warning/fatal distinctions, and no partial JSON. |
| AC-09 | Large simultaneous stdout/stderr output completes without deadlock or unbounded lifecycle-log buffering; no forwarding survives session handoff or cancellation. | Concurrent output exceeding pipe capacity, terminal handoff markers, and cancellation fixtures. |
| AC-10 | Guest status, cancellation, cleanup ownership, and stop-failure precedence remain unchanged in both modes. | Existing lifecycle failure tests extended with the output-mode matrix and fixed status expectations. |
| AC-11 | Help/version and offline creation remain offline; verbose adds no runtime calls, state writes, locks, identity changes, or secret-value logging. | Exact call logs, state snapshots, synthetic secret sentinels, and fixed creation-policy checks. |
| AC-12 | Active help and documentation explain default suppression, live verbose stderr, visible errors, guest argument boundaries, and preserved interactive I/O. | Source/help/documentation review against integration fixtures. |
| AC-13 | Existing lifecycle, listing, identity, replacement, purge, port, and baseline-independent regressions pass. | All required Rust checks in AGENTS.md and a creation-policy diff review. |

## Versioning and validation evidence

This specification-only change leaves `Cargo.toml` and `Cargo.lock` unchanged.
Implementation adds a feature and requires a MINOR version bump from the
implementation baseline in both files, following `AGENTS.md`. No machine
policy revision bump is required because creation policy does not change.

During implementation, create `FEATURES/feature-0014-validation.md` with the
tested source revision, native target, commands, exit codes, and `passed`,
`failed`, `not_run`, or `inconclusive` for every acceptance criterion. Run all
independent Rust checks listed in `AGENTS.md`. Record new results and pending
native or real guest checks in Feature 0010 validation while preserving
historical observations.

For real interactive validation, record the target, SmolVM version, profile,
terminal type, output mode, shell or harness interaction, authentication
observation, resize, Ctrl-C, guest exit status, and cleanup result. Include
native Windows console behavior in the target matrix. Mocked stream tests
do not establish real terminal behavior, authentication, or guest boot.

## Scope limits

This feature does not add a shell-access subcommand, persistent logging,
verbosity levels, log rotation, a runtime debugging flag, output filtering
inside interactive sessions, a new terminal proxy, or a change to command
selection or lifecycle policy. It does not implement the proposed remove
command or alter its selection rules.
