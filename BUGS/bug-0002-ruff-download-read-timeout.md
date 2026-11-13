# Bug 0002: Fresh sandbox provisioning aborts on a Ruff download read timeout

## Status

Fix implemented in `smolsandbox 0.8.2`; local validation is recorded below.
Validation on the affected machine remains pending. Report prepared on
2026-11-13. The user reported the failure on a
different machine while building a fresh sandbox from the current version.
The deployed application revision, SmolVM version, selected profile,
selected definition, host OS, and CPU architecture were not supplied.

Source inspection used revision
`583d5db794c77f91f5187495a3f34b4df525e3e9` (`smolsandbox 0.8.1`). That
revision is the analysis baseline; the affected machine's revision remains
unconfirmed. No software change accompanied the original report.

## Summary and impact

Fresh sandbox provisioning aborts during the Ruff installation after the
.NET tooling and the Python tools `uv`, `pytest`, and `debugpy` have installed
successfully. Pip reports a read timeout from `files.pythonhosted.org:443`.
The aborted build prevents the user from obtaining a usable fresh sandbox.

The report establishes a network read timeout during the Ruff installation.
The underlying cause and the effectiveness of a longer timeout remain
unconfirmed.

## Reported failure

The user reported that output was written to a log file on the affected
machine. Transferring the complete log to this session was unavailable.
Under a message described as "some possibly relevant errors", the user
transcribed these two lines:

```text
TimeoutError: The read operation timed out
pip._vendor.urllib3.exceptions.ReadTimeoutError: HTTPSConnectionPool(host='files.pythonhosted.org', port=443): Read time out.
```

The successful preceding Python installations were clarified by the user
after the initial report. Ruff was therefore not the first Python tooling
installation attempted in the reported build. The exact requested URL,
download filename, effective timeout, retry history, and exit code were not
supplied.

## Expected and actual behavior

Expected: provisioning completes when required package downloads succeed.
Recoverable download stalls receive a bounded opportunity to recover. If
downloads remain unavailable, provisioning fails with enough information to
identify the package and failing host.

Actual: the reported Ruff installation ends with a read timeout from the
package download host, and the fresh sandbox build aborts. The supplied
excerpt does not establish how many retries occurred before the abort.

## Relevant source and diagnostic limits

All four definitions at the analysis baseline in [`templates/`](../templates/) install the
Python tools through these commands, following the .NET tooling:

```text
pipx install uv==0.12.17
pipx install pytest==9.1.1
pipx install debugpy==1.8.22
pipx install ruff==0.16.8
```

The Ruff commands contain no explicit pip timeout or retry overrides.
The current definitions allow both `pypi.org` and `files.pythonhosted.org`
in `[network].allow_hosts`. The affected machine's selected definition and
effective pip configuration have not been inspected.

[Pip documents](https://pip.pypa.io/en/stable/cli/pip/) a default socket
timeout of 15 seconds and supports `PIP_DEFAULT_TIMEOUT` and `PIP_RETRIES`.
The exception excerpt does not report the effective timeout for this build.

The successful preceding installations establish that Python provisioning
and some package downloads succeeded during the same build. Those successes
do not establish uninterrupted access for subsequent downloads. The reported
read timeout alone does not identify a missing allowlist entry, an invalid
Ruff version, or a package compatibility defect. The requested resource may
have been package content, metadata, or an installation dependency; the
excerpt does not identify the resource.

## Reproduction status and next investigation

The user observed the following sequence on the affected machine:

1. Build a fresh sandbox using the selected definition.
2. Observe successful .NET tooling installation.
3. Observe successful `uv`, `pytest`, and `debugpy` installation.
4. Observe a build abort during Ruff installation with the exceptions above.

Local real guest reproduction: `not_run`. SmolVM was unavailable in the
analysis environment. At report preparation, no deterministic reproduction
or verified fix existed. Another build is not guaranteed to reproduce a
network stall. The later controlled reproduction is recorded below.

For a subsequent attempt, the maintainer should record the application and
SmolVM versions, selected definition, host OS and architecture, and outcome.
Enable `--verbose` on the existing build command to retain runtime output.
Inspect the pip log on the affected machine for the requested resource,
effective timeout, and retry history when the log is accessible. The user
has already supplied the exception needed to identify the failing host;
full-log transfer is not a prerequisite for trying the workaround below.

## Proposed workaround

In the selected user-owned `.smlbx` definition, replace the existing Ruff
installation entry with:

```toml
"PIP_DEFAULT_TIMEOUT=120 PIP_RETRIES=10 pipx install ruff==0.16.8",
```

Repeat the build using the same definition selection. Set the environment
variables inside the provisioning command so they apply to guest pip
processes. `PIP_RETRIES` controls connection retries; it does not guarantee
that every interrupted download will resume. The timeout is a socket timeout,
not an overall installation deadline.

Workaround result: `not_run`; the user has not reported trying the change.
A successful retry would support the workaround's usefulness without proving
the original stall's underlying cause.

As documented in [RUST-APPLICATION.md](../RUST-APPLICATION.md), existing
definitions remain user-owned when the executable is updated. Editing the
selected definition changes the base generation automatically and requires
no Rust rebuild. A future template fix must preserve existing user-owned
definitions and explain how to adopt the updated provisioning command.

## Resolution and acceptance

The maintainer should use the affected guest's result or a controlled network
stall to decide whether a provisioning timeout or retry policy change is
warranted. Keep the Ruff version pin and hostname policy unchanged unless
additional evidence identifies a defect in either setting.

- AC-1: Record the selected definition and effective pip settings for a build
  attempt after applying the proposed workaround. Record the Ruff installation
  and complete build outcomes separately. Result: `not_run`.
- AC-2: If a software fix changes timeout or retry policy, verify that a
  controlled stall exceeding the previous timeout but within the new allowance
  can recover through the actual provisioning download path. Result: `not_run`.
- AC-3: If a software fix changes timeout or retry policy, verify that a
  persistent download failure terminates within the documented failure bound
  and leaves no reusable successful base artifact. Result: `not_run`.
- AC-4: If templates change, verify all four profiles and document adoption
  for existing definitions. Run the Rust regression checks required by
  `AGENTS.md` for the software change. Result: `not_run`.

Rust regression checks were not run for the original report because that
change added only a bug report. The original report left the software version
unchanged under the repository's versioning policy.

## Implementation: 2026-11-13

The working-tree fix based on revision
`583d5db794c77f91f5187495a3f34b4df525e3e9` updates all four templates and
bumps the application to `0.8.2`. The preceding investigation describes the
original `0.8.1` report and its untested workaround; those observations remain
historical evidence.

The Ruff installation command is now:

```toml
"PIP_DEFAULT_TIMEOUT=120 PIP_RETRIES=10 timeout --verbose --kill-after=10s 300s pipx install ruff==0.16.8",
```

The socket timeout allows a stalled response 120 seconds to resume. Pip may
retry connection establishment 10 times. GNU `timeout` sends `TERM` after
300 seconds and `KILL` after a further 10 seconds if pipx has not exited.
Signals apply to the monitored process group.
This bounds the complete Ruff installation, including pipx setup, to
310 seconds under normal process scheduling. It does not bound the complete
build or guarantee recovery from every interrupted download. The package
pins and hostname policy remain unchanged.

The controlled reproduction uses real Debian pipx `1.1.0`, pip `23.0.1`,
and Python `3.11.2` on Linux. A local HTTP index serves a synthetic wheel
named `ruff-0.16.8-py3-none-any.whl`; this fixture exercises pipx's package
download and installation path without contacting PyPI or running real Ruff.
The server sends one payload byte, waits 18 seconds, then sends the remainder.
The original bare command failed twice after 15.5 seconds with `TimeoutError`
and pip's `ReadTimeoutError`. The revised Codex command installed the fixture
after 18.6 seconds. Removing the payload stall removes the timeout trigger.
This establishes a missing recovery allowance for that controlled stall;
the affected machine's underlying network fault remains unknown.

The retained check is
[`tests/provisioning/ruff_download.py`](../tests/provisioning/ruff_download.py).
Run the check explicitly on Linux with Python 3.11+, pipx, python3-venv, and
GNU timeout. The ordinary Rust checks do not require those host tools.

```text
python3 tests/provisioning/ruff_download.py recover codex
python3 tests/provisioning/ruff_download.py healthy codex --baseline
python3 tests/provisioning/ruff_download.py persistent codex
python3 tests/provisioning/ruff_download.py deadline codex
python3 tests/provisioning/ruff_download.py full-deadline codex
```

Repeat `recover` with `claude`, `opencode`, and `hermes` to exercise each
template. `persistent` never finishes the payload. `deadline` keeps the
connection active but shortens only GNU timeout's deadline to 8 seconds and
its termination grace to 2 seconds. `full-deadline` keeps the connection
active and uses the unmodified production deadline. Failure checks require
a nonzero exit and no installed fixture application.

`healthy codex --baseline` uses the original bare command without a stall.
`recover codex --baseline` uses the original command with the 18-second stall
and is expected to fail the recovery assertion. These commands distinguish
the timeout defect from fixture, pin, and installer failures.

The Rust regression
`every_template_bounds_ruff_downloads_and_preserves_python_pins` first failed
against the old templates, then passed after the fix. The independent
`failed_fresh_builder_never_publishes_an_artifact_or_generation_cache` check
verifies that simulated builder creation or startup failure returns nonzero,
never packs a base, and leaves no artifact or generation cache. The test also
checks that the selected definition is preserved. This mock establishes the
application's failure handling; real SmolVM provisioning remains unverified.

Existing definitions remain user-owned. Adopt the new command by editing
the selected definition's Ruff entry, or generate a separate updated
definition and compare the files. Follow
[template adoption](../RUST-APPLICATION.md#template-adoption) for the build
and replacement procedure. Changed definition bytes select a new generation;
no Rust lifecycle policy revision is needed.

Final command results and remaining affected-machine and real guest checks
are recorded in the Bug 0002 entry in
[Feature 0010 validation](../FEATURES/feature-0010-validation.md).
