# Bug 0001: Named-configuration tests reject CRLF template headers

## Status

Resolved in commit `7a8f67a7be392bcf95304771f6c6f751893cf34f`
(`smolsandbox 0.8.1`). Native Windows verification after the fix is pending.
Report prepared on 2026-11-11 against current revision
`583d5db794c77f91f5187495a3f34b4df525e3e9`.

The user clarified that the supplied error came from an earlier commit and
that the later pipeline encountered other issues. Those other issues were
not supplied for analysis. The old assertion output is historical evidence
for this resolved bug; it does not establish a recurrence after the fix.

## Summary and impact

Two application tests require an LF newline immediately after the harness
metadata marker. A template containing a CRLF header makes both assertions
fail even though `create-config` succeeds and preserves the template bytes.
The defect blocks CI when templates are built with CRLF line endings. Local
reproduction establishes an assertion defect; the supplied log does not
establish a configuration-creation, path-resolution, or permissions defect.

The affected assertions are present in `smolsandbox 0.8.0` at revision
`4edabfda24dfd22f115a482805d158693603c334`. The original Actions run URL,
tested commit, feature configuration, and checkout line-ending settings were
not supplied. Windows is inferred from the `tests\application.rs` paths in
the log; the workflow configures a `windows-2025` job targeting
`x86_64-pc-windows-msvc`. CRLF bytes in that specific CI run remain unobserved.

## Reported failure

The supplied Actions output reports 60 passing tests, two failing tests,
and process exit code 1. Cargo reports test-process exit code 101 in local
reproduction. The failed assertions are:

```text
named_configuration_uses_invocation_directory_without_changing_default_or_permissions
tests\application.rs:225:9
assertion failed: text.starts_with(&format!("# codex-sandbox-harness: {harness}\n"))

named_configuration_accepts_absolute_paths_and_creates_parent_directories
tests\application.rs:269:5
assertion failed: fs::read_to_string(destination).unwrap().starts_with("# codex-sandbox-harness: claude\n")
```

## Expected and actual behavior

Expected: both tests accept the exact harness marker followed by either LF
or CRLF. Configuration creation continues to preserve template bytes, reject
an existing destination, and satisfy the existing path and permission checks.

Actual on the affected source: the assertions require the bytes
`# codex-sandbox-harness: claude\n`. A generated file beginning with
`# codex-sandbox-harness: claude\r\n` fails that comparison. The relative-path
test encounters Claude first in its profile loop, so the failure also prevents
that test from reaching Codex, OpenCode, and Hermes.

## Root cause and existing fix

[`Profile::template`](../src/config.rs) embeds `templates/*.smlbx` through
`include_str!`. [`create_config`](../src/config.rs) passes the embedded
template bytes to file publication. Template line endings therefore reach
the generated definition unchanged.

The two original assertions in [`tests/application.rs`](../tests/application.rs)
include a literal `\n` in their expected prefix. CRLF introduces `\r` before
that byte. Changing only the first newline in `templates/claude.smlbx` to
CRLF reproduces both reported failures on Linux, without Windows path or
permission behavior.

Commit `7a8f67a` replaces both prefix checks with comparisons against
`text.lines().next().unwrap()`. The comparisons still require the exact
marker and profile while accepting LF and CRLF. The commit also bumps the
package to `0.8.1`. No additional software change is proposed by this report.

## Reproduction

Use a disposable checkout of `4edabfda24dfd22f115a482805d158693603c334`
and a separate Cargo target directory to avoid reusing unrelated build
artifacts. With LF templates, run:

```text
cargo test --locked --test application named_configuration_ -- --nocapture
```

The focused run passes three tests. In that disposable checkout, run this
Python snippet to change only the Claude header newline:

```python
from pathlib import Path

path = Path("templates/claude.smlbx")
data = path.read_bytes().replace(b"\r\n", b"\n")
path.write_bytes(data.replace(b"\n", b"\r\n", 1))
```

Rerun the Cargo command. Both reported assertions fail; the third focused
test passes; Cargo exits 101. Removing the single added carriage return
restores the LF case. Replacing only the two assertions with the versions
from `7a8f67a` makes the same CRLF case pass.

## Verification on 2026-11-11

Local execution used Rust `1.99.1` on native `x86_64-unknown-linux-gnu`.
Isolated experiments used the `0.8.0` baseline above, a fresh target
directory, controlled template bytes, and then the corrected test file.
All focused runs used the command above; adapter runs added
`--features test-adapter`.

| Source and template condition | Result | Observation |
| --- | --- | --- |
| Current `583d5db`, LF templates, default and adapter features | passed | Three focused tests per feature configuration; exit 0 |
| Baseline `4edabfd`, LF templates | passed | Three focused tests; exit 0 |
| Baseline, all four templates converted to CRLF | failed | Both reported assertions failed on two consecutive runs; exit 101 |
| Baseline, only Claude header changed to CRLF | failed | Same two assertions failed; exit 101 |
| Corrected assertions, all four templates CRLF, default and adapter features | passed | Three focused tests per feature configuration; exit 0 |
| Corrected assertions, only Claude header CRLF | passed | Three focused tests; exit 0 |
| CLI creation with the Claude CRLF header | passed | Exit 0; generated header was `b'# codex-sandbox-harness: claude\r\n'` |
| Native Windows execution after the fix | not_run | Requires the corrected Windows Actions job and its run URL |

These results supplement the earlier
[Windows regression verification](../FEATURES/feature-0010-validation.md#windows-configuration-header-regression-verification-2026-11-07).
Full lint, release, and baseline-independence checks were not rerun for this
report because this change adds documentation only. Earlier results remain
recorded separately. Native real guest checks were not run.

## Remaining acceptance and follow-up

- AC-1: Both named-configuration assertions accept LF and CRLF while requiring
  the exact marker and harness. Local result: passed, as recorded above.
- AC-2: Configuration generation preserves template bytes and the existing
  destination, path, and permission checks remain effective. Local result:
  passed for the focused Linux tests; native Windows result: not_run.
- AC-3: The Windows Actions job passes the application suite in both feature
  configurations on a commit containing `7a8f67a`. Result: not_run.

The user confirmed that the supplied run predates the current pipeline
issues. No further software fix is required for the reported assertions.
When the separate pipeline issues are resolved, the maintainer should record
the corrected Windows run URL, tested revision, and outcome in Feature 0010
validation. The pending native verification does not establish a recurrence
of this bug. Analysis of the separate pipeline issues requires their own
failure output and is outside this report's scope.
