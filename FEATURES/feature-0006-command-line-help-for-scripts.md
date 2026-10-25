# Feature 0006: Command-line help for lifecycle scripts

## Status

Implemented

## Summary

Every supported shell entry point shall accept `--help`, print a complete
description of its command-line options and parameters, and exit successfully
without performing lifecycle work.

Help shall use a consistent, human-readable structure while remaining specific
to each command. It shall identify the command, show its usage, explain every
accepted option and positional parameter, state relevant defaults and
constraints, and describe argument forwarding where applicable.

The compatibility entry points `build-login.sh` and `codex-sandbox.sh` shall
provide help under their own invoked names while documenting the same interface
as the canonical command to which they delegate.

## Motivation

The scripts currently expose their interfaces through a mixture of README
examples, source comments, terse usage lines printed after invalid input, and
implicit parser behavior. Some scripts have a `usage` function, some print only
a synopsis, and some accept no arguments without offering any discoverable way
to confirm that fact.

Users should be able to discover a command's interface at the command line
without reading source code or risking a build, machine start, cleanup, purge,
or repair operation. In particular, users need to be able to distinguish
sandbox launcher options from arguments forwarded to Codex.

## Goals

- Add a successful `--help` path to every supported shell entry point.
- Document every option, positional parameter, delimiter, default, and
  command-specific constraint accepted by each entry point.
- Use a consistent help layout across the script suite.
- Make help safe to request before the sandbox environment has been prepared.
- Preserve `--help` after the `start-codex.sh` pass-through delimiter as a
  Codex argument.
- Make compatibility entry-point help describe the invoked command name.
- Keep existing invalid-usage and operational-failure behavior unchanged.
- Add automated coverage for help text, exit status, stream selection, and
  absence of side effects.

## Non-goals

- Adding the short option `-h`.
- Adding shell completion, manual pages, generated Markdown, or
  machine-readable help.
- Making help output byte-for-byte compatible with an external CLI framework.
- Changing existing option names, parameter defaults, lifecycle behavior, or
  exit codes for errors.
- Tightening `start-login.sh`'s historical handling of extraneous non-help
  arguments as part of this feature.
- Treating test runners in `tests/` as supported user commands.
- Turning sourced implementation modules into user-facing commands.
- Promoting `sandbox-inventory.py` or any of its subcommands to a supported
  public interface.
- Resolving or implementing the Rust migration proposed by Feature 0004.

## Scope

The supported shell entry points covered by this feature are:

- `build-sandbox.sh`
- `build-login.sh`
- `start-codex.sh`
- `codex-sandbox.sh`
- `start-login.sh`
- `repair-sandbox.sh`
- `clean-sandbox.sh`
- `purge-sandbox.sh`

`sandbox-common.sh` is a sourced implementation module, not a command, and is
outside the command-line help surface. The executable scripts under `tests/`
are test runners and are also outside it.

`sandbox-inventory.py` is an internal helper. Its existing `argparse`-provided
top-level and subcommand help shall continue to work for development and
diagnostics, but its interface is not made public by this feature and need not
use the shell entry points' presentation format.

## General help contract

For each supported entry point, `--help` shall:

1. Write help text to standard output.
2. Write nothing to standard error.
3. Exit with status `0`.
4. Take precedence over validation of other script arguments when it appears
   anywhere the script itself is still parsing arguments.
5. Exit before preparing the sandbox environment or performing any lifecycle
   or state operation.

Requesting help shall not require `smolvm`, Python, an existing workspace,
artifacts, machines, credentials, network access, or writable repository state.
It shall not inspect SmolVM inventory, calculate sandbox identity, acquire a
lock, create a temporary file, start a subprocess for lifecycle work, or read,
write, create, stop, or delete sandbox state.

The first line shall be a `Usage:` synopsis. The remaining output shall include
a short command description followed by labeled `Arguments:` and `Options:`
sections. A command with no positional parameters shall say so or omit an empty
arguments section consistently across all such commands. Every help page shall
list `--help` itself.

Help shall be plain text without terminal-color escape sequences. Its content
shall not depend on the current machine inventory, state files, environment
credentials, or terminal width. The command in the synopsis shall use the
invoked entry point's basename rather than an environment-specific absolute
path.

The metavariables `WORKSPACE`, `HOST`, and `CODEX_ARG` shall be used
consistently. Optional parameters shall use square brackets, alternatives shall
use `|`, and repeatable values shall be marked with `...` and explained in
prose.

## Command-specific help

The wording may evolve, but each help page shall communicate at least the
following interface and semantics.

### `build-sandbox.sh`

```text
Usage: build-sandbox.sh [--help] [WORKSPACE]
```

- `WORKSPACE` is the workspace whose machine is prepared and defaults to the
  current directory.
- `--help` displays help and exits.

The description shall explain that the command builds or reuses the sandbox
base and prepares the login and workspace machines.

### `build-login.sh`

```text
Usage: build-login.sh [--help] [WORKSPACE]
```

The arguments and options are the same as `build-sandbox.sh`. The help text
shall identify this command as a compatibility entry point and state that the
unified build prepares both login and workspace machines.

### `start-codex.sh`

```text
Usage: start-codex.sh [--help] [--allow-host HOST]... [WORKSPACE] [-- CODEX_ARG...]
```

- `WORKSPACE` is the workspace mounted into the sandbox and defaults to the
  current directory.
- `--allow-host HOST` and `--allow-host=HOST` add a host to the workspace
  machine's default network allowlist. The option is repeatable and may appear
  before or after `WORKSPACE`, but not after the pass-through delimiter.
- `--help` displays launcher help and exits.
- `--` ends sandbox argument parsing and forwards every remaining argument to
  the guest Codex command without interpretation.
- `CODEX_ARG...` represents those forwarded arguments.

The help shall make clear that options after `--` belong to Codex. Therefore:

```bash
start-codex.sh --help
```

displays launcher help, while:

```bash
start-codex.sh -- --help
```

starts the sandbox and passes `--help` to Codex.

### `codex-sandbox.sh`

```text
Usage: codex-sandbox.sh [--help] [--allow-host HOST]... [WORKSPACE] [-- CODEX_ARG...]
```

The arguments, options, delimiter behavior, and defaults are the same as
`start-codex.sh`. The help text shall identify this command as a compatibility
entry point for `start-codex.sh`.

### `start-login.sh`

```text
Usage: start-login.sh [--help]
```

- The command has no supported positional parameters. For compatibility, this
  feature does not change its historical handling of extraneous non-help
  arguments, which remain undocumented and ignored.
- `--help` displays help and exits.

The description shall explain that the command starts the restricted login
machine and runs Codex device authentication.

### `repair-sandbox.sh`

```text
Usage: repair-sandbox.sh [--help] (--check | --replace-stale [--force]) [--allow-host HOST]... [WORKSPACE]
```

- Exactly one of `--check` and `--replace-stale` is required for an operation.
- `--check` inspects and reports reconciliation state without replacing stale
  machines.
- `--replace-stale` prepares and verifies the desired machine before removing
  eligible stale machines.
- `--force` is valid only with `--replace-stale` and permits a running stale
  machine to be stopped during replacement.
- `--allow-host HOST` and `--allow-host=HOST` are repeatable and select the same
  additional-host configuration used by `start-codex.sh`.
- `WORKSPACE` defaults to the current directory.
- `--help` displays help and exits; it does not require either operation mode.

### `clean-sandbox.sh`

```text
Usage: clean-sandbox.sh [--help]
```

- The command accepts no positional parameters.
- `--help` displays help and exits.

The description shall state that the command removes machines and cached
artifacts belonging to the authoritative current generation while preserving
the shared Codex state.

### `purge-sandbox.sh`

```text
Usage: purge-sandbox.sh [--help] [--force | --force-all]
```

- With neither force option, the conservative purge policy is used.
- `--force` removes eligible stopped custom and legacy machines while
  preserving the current base's stopped default configurations.
- `--force-all` removes every eligible stopped managed machine, including
  stopped default configurations.
- `--force` and `--force-all` are mutually exclusive.
- Every policy preserves running machines and protected artifacts.
- `--help` displays help and exits.

## Argument parsing and errors

For commands without a pass-through delimiter, a recognized `--help` shall
short-circuit parsing even if other supplied arguments would otherwise be
missing, conflicting, or excessive. For example, both
`repair-sandbox.sh --help` and `repair-sandbox.sh --check --help` shall display
help successfully without inspecting a workspace.

For `start-codex.sh` and `codex-sandbox.sh`, only `--help` before the first `--`
is the launcher's help option. Every token after the delimiter, including
`--help`, is forwarded unchanged to Codex.

`--help` takes no value. Forms such as `--help=value` remain unknown options and
shall follow the existing usage-error path.

Where they are currently usage errors, unknown options, missing option values,
conflicting operation modes, invalid option combinations, and excess positional
parameters shall continue to write an actionable diagnostic or usage text to
standard error and exit with status `2`. In particular, this feature does not
tighten `start-login.sh`'s historical argument handling. Operational failures
shall retain their existing non-zero statuses and shall not be relabeled as
help or usage success.

## Compatibility entry points

Compatibility entry points shall not merely expose help under the canonical
target's name. Invoking `build-login.sh --help` shall show `build-login.sh` in
the synopsis, and invoking `codex-sandbox.sh --help` shall show
`codex-sandbox.sh`.

Other than the entry-point name and compatibility description, their documented
arguments and behavior shall remain equivalent to their canonical targets.
Normal non-help invocations shall continue to delegate exactly as they do
before this feature.

## Relationship to other features

This feature documents the currently supported shell interface. It does not
change the reconciliation, replacement, purge, or configurable-network policy
defined by Features 0001, 0002, 0003, and 0005.

Feature 0004 proposes removing these scripts in favor of the Rust
`smolsandbox` command and already requires top-level help. If that migration
proceeds, its subcommands shall retain equivalent discoverability for their
then-current options, but implementing this feature neither resolves Feature
0004's blockers nor commits to its CLI design.

## Documentation requirements

The README shall mention that every supported shell entry point accepts
`--help` and shall use `start-codex.sh -- --help` when demonstrating how to ask
the guest Codex command for help. Documentation shall not imply that
`sandbox-common.sh`, test runners, or `sandbox-inventory.py` are supported user
commands.

## Safety invariants

- Help never builds, starts, stops, creates, replaces, cleans, or purges a
  machine or artifact.
- Help never mutates compatibility caches, shared Codex state, locks, or
  temporary files.
- Help never requires or queries external executables.
- Help before the pass-through delimiter is handled by the launcher; help after
  it is handled by Codex.
- Adding help does not broaden any network allowlist or alter machine identity.
- Compatibility aliases remain behaviorally equivalent to their canonical
  commands for non-help invocations.

## Acceptance criteria

1. Each of the eight supported shell entry points exits `0` for `--help`,
   writes non-empty help to standard output, and writes nothing to standard
   error.
2. Each help page starts with a `Usage:` synopsis naming the invoked entry
   point and lists `--help`.
3. Each help page documents every option and positional parameter accepted by
   that command, including defaults, repeatability, mutual exclusion, and mode
   restrictions described in this specification.
4. `start-codex.sh --help` and `codex-sandbox.sh --help` perform no lifecycle
   work, while `start-codex.sh -- --help` and
   `codex-sandbox.sh -- --help` forward `--help` to Codex.
5. Help succeeds when `smolvm` and Python are unavailable and when repository
   state and the requested workspace do not exist.
6. A help invocation records no SmolVM fixture operations and leaves sandbox
   files, locks, and artifacts unchanged.
7. `repair-sandbox.sh --help`, `repair-sandbox.sh --check --help`,
   `purge-sandbox.sh --force --help`, and a help request combined with excess
   positional parameters all return help success without mutation.
8. `build-login.sh --help` names `build-login.sh` and documents its
   compatibility role; `codex-sandbox.sh --help` does the same for
   `codex-sandbox.sh`.
9. Existing invalid invocations still exit `2`, and existing successful and
   operational-failure paths preserve their behavior.
10. The internal Python helper's existing top-level and subcommand `--help`
    behavior remains available without becoming a documented public API.
11. The README is updated with help discovery and pass-through guidance.
12. Every `tests/*-test.sh` script passes.

## Implementation outline

1. Add a side-effect-free help renderer and early `--help` recognition to each
   canonical entry point.
2. Add alias-aware help handling to `build-login.sh` and `codex-sandbox.sh`
   without changing their normal delegation paths.
3. Keep the start launcher's `--` boundary authoritative so forwarded Codex
   arguments are never consumed as launcher help.
4. Preserve the existing status-`2` usage path separately from the new
   status-`0` help path.
5. Add `tests/help-test.sh` covering content, streams, statuses, argument
   positions, aliases, pass-through behavior, missing prerequisites, and
   non-mutation.
6. Update the README and run every `tests/*-test.sh` script.
