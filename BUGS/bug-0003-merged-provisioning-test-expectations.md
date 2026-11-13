# Bug 0003: Merged provisioning tests disagree on Python pins and Ruff timeout policy

## Status

Open. Both reported failures were reproduced on 2026-11-13 against merge
commit `e0c5ea6ad89a3402c6840a7129da7186051527a4`, application `0.9.0`,
on remote branch `feature/0016-bump-versions`. The root cause is confirmed
by controlled changes in a disposable source copy. This report does not
apply a software fix or change the application version.

The workspace remains at `15b6c5444f8817da63737fc29f942b1d47ec17e5`.
That earlier revision passes its 65 library tests and lacks one of the
reported failing tests. The remote merge was fetched over HTTPS and
exported to a separate directory for reproduction. Source references below
identify the affected merge because workspace files differ from that merge.

## Summary and impact

Merging the Ruff download timeout fix from `develop` into Feature 0016
combines updated template versions with incompatible test expectations.
All four templates retain the bounded Ruff installation and select Ruff
`0.17.5`, uv `0.13.8`, pytest `9.2.0`, and debugpy `1.8.22`.
Two tests still require different commands from those templates.

The user reports that all GitHub builds failed. Local execution of the
affected merge reproduces the exact two assertions, source lines, and
64-passed/2-failed library result supplied by the user. These assertions
inspect parsed template strings; they do not run pipx or contact PyPI.

The [native workflow](https://github.com/mnemonic-bit/SmolSandbox/blob/e0c5ea6ad89a3402c6840a7129da7186051527a4/.github/workflows/rust.yml)
runs the library tests on Linux x64, Linux ARM64, macOS ARM64, and Windows
x64. The same contradictory expectations explain failures across that
matrix. A failing test step prevents subsequent release-build and artifact
steps from running. The supplied excerpt establishes a test failure; it
does not establish a Rust compilation or real guest provisioning failure.
The original Actions run URL and per-job logs were not supplied or inspected.

## Reported failure

```text
config::tests::all_templates_preserve_shared_provisioning_tools_pins_and_verification
src/config.rs:673:17
missing init command: pipx install ruff==0.17.5

config::tests::every_template_bounds_ruff_downloads_and_preserves_python_pins
src/config.rs:477:17
Codex: missing pipx install uv==0.12.17

test result: FAILED. 64 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
error: test failed, to rerun pass `--lib`
Error: Process completed with exit code 101.
```

Expected: the provisioning tests accept the Feature 0016 pins together with
the bounded Ruff download policy, while continuing to reject missing tools,
incorrect versions, duplicate Ruff installations, and lost timeout settings.

Actual: the merged tests require incompatible Ruff commands and disagree
with the selected uv and pytest versions. Each test stops at its first
mismatch, hiding additional stale expectations.

## Confirmed root cause

The merge has parents `15b6c5444f8817da63737fc29f942b1d47ec17e5`
(Feature 0016 version updates) and
`ce1814c7a0052f9be6d1c472bb7ec2d062c507da` (Bug 0002 Ruff timeout fix).
The merge retains both changes in the templates but does not reconcile
the independent tests introduced by those parents.

| Surface at the affected merge | Expected or selected command |
| --- | --- |
| All four templates | `pipx install uv==0.13.8` |
| Timeout regression test | `pipx install uv==0.12.17` |
| All four templates | `pipx install pytest==9.2.0` |
| Timeout regression test | `pipx install pytest==9.1.1` |
| All four templates | `PIP_DEFAULT_TIMEOUT=120 PIP_RETRIES=10 timeout --verbose --kill-after=10s 300s pipx install ruff==0.17.5` |
| Shared provisioning test | `pipx install ruff==0.17.5` |
| Timeout regression test | `PIP_DEFAULT_TIMEOUT=120 PIP_RETRIES=10 timeout --verbose --kill-after=10s 300s pipx install ruff==0.16.8` |

Both tests parse `[dev].init` into command strings and use
`commands.contains(&command)`. This compares complete array entries for
equality. The bare Ruff command is absent because the actual array entry
includes environment variables and the GNU timeout wrapper. Ruff `0.17.5`
is present; the shared test's error message describes a command mismatch.

The timeout test first fails on uv. Updating only that expectation would
expose the stale pytest expectation, followed by the stale Ruff version.
The unchanged debugpy expectation already matches the templates.

Affected source at the merge:

- [Timeout regression test, lines 457–500](https://github.com/mnemonic-bit/SmolSandbox/blob/e0c5ea6ad89a3402c6840a7129da7186051527a4/src/config.rs#L457-L500).
- [Shared provisioning expectations, lines 654–674](https://github.com/mnemonic-bit/SmolSandbox/blob/e0c5ea6ad89a3402c6840a7129da7186051527a4/src/config.rs#L654-L674).
- [Codex template](https://github.com/mnemonic-bit/SmolSandbox/blob/e0c5ea6ad89a3402c6840a7129da7186051527a4/templates/codex.smlbx#L186-L190), [Claude template](https://github.com/mnemonic-bit/SmolSandbox/blob/e0c5ea6ad89a3402c6840a7129da7186051527a4/templates/claude.smlbx#L75-L79), [OpenCode template](https://github.com/mnemonic-bit/SmolSandbox/blob/e0c5ea6ad89a3402c6840a7129da7186051527a4/templates/opencode.smlbx#L73-L77), and [Hermes template](https://github.com/mnemonic-bit/SmolSandbox/blob/e0c5ea6ad89a3402c6840a7129da7186051527a4/templates/hermes.smlbx#L76-L80).

## Reproduction and diagnostic results

Use a disposable source copy of
`e0c5ea6ad89a3402c6840a7129da7186051527a4` with its own Cargo target
directory. Run from that copy's root:

```text
cargo test --locked --lib
cargo test --locked --lib config::tests:: -- --nocapture
```

The full library command returns exit 101 with 64 passing and two failing
tests. The focused command returns exit 101 with nine passing and the same
two failing tests. Repeated runs reproduce both messages. The observed host
is Linux x86-64 with Rust/Cargo `1.100.0`.

To isolate the cause, only four expected strings in the disposable
`src/config.rs` were changed: uv to `0.13.8`, pytest to `9.2.0`, wrapped
Ruff to `0.17.5`, and bare Ruff to the complete wrapped command. Templates
and production Rust code were unchanged. The following results belong to
that diagnostic copy, not to an implemented repository fix:

| Check | Result | Evidence |
| --- | --- | --- |
| Original full library suite | failed | `cargo test --locked --lib`: exit 101, 64 passed, 2 failed |
| Original focused configuration tests | failed | `cargo test --locked --lib config::tests:: -- --nocapture`: exit 101, 9 passed, 2 failed |
| Four corrected expectations | passed | `cargo test --locked --lib`: exit 0, 66 passed |
| Four corrected expectations with adapter | passed | `cargo test --locked --features test-adapter --lib`: exit 0, 65 passed |

These results confirm stale test expectations as the cause of the supplied
failures. No download, host allowlist, guest image, or runner change was
required to make the library tests pass. The disposable edits were restored
after diagnosis.

## Related Ruff fixture mismatch

The merged [download regression script](https://github.com/mnemonic-bit/SmolSandbox/blob/e0c5ea6ad89a3402c6840a7129da7186051527a4/tests/provisioning/ruff_download.py)
also retains a `0.16.8` wheel filename, package metadata, and fixture module.
The script extracts the current template command, which requests `0.17.5`,
but its local package index offers only `0.16.8`.

Running `python3 tests/provisioning/ruff_download.py healthy codex` against
the affected merge returns exit 1. Pip reports
`No matching distribution found for ruff==0.17.5`, and the script fails
with `pipx did not request the wheel payload`. This is a local fixture
version mismatch; the result says nothing about public PyPI availability.

Changing only the fixture version fields to `0.17.5` in the disposable copy
allows the same healthy case to pass, exit 0, with the wheel payload
requested and the fixture CLI executed. The historical `--baseline` command
was left unchanged during that probe. Baseline mode needs a matching
fixture for its selected version when the eventual fix supports both modes.

The script explicitly runs outside the Rust suite, and the CI workflow does
not invoke it. Its mismatch is related maintenance work, not the source of
the two reported CI assertions. Recovery and deadline cases were `not_run`
during this investigation.

## Proposed correction and acceptance

The maintainer should reconcile the tests on top of the affected merge:

1. Update the timeout test's uv, pytest, and Ruff expectations to the
   Feature 0016 versions.
2. Update the shared provisioning test to accept the complete bounded Ruff
   command while preserving independent version and timeout assertions.
3. Update the download fixture to serve the version requested by each
   supported mode, including its wheel filename and package metadata.
4. Preserve the timeout policy, one Ruff installation per profile, package
   host checks, and `coreutils` requirement. Preserve user-owned definitions.
5. Bump the application PATCH version in both `Cargo.toml` and the matching
   `smolsandbox` entry in `Cargo.lock` when applying the test fix. On the
   affected `0.9.0` baseline, the next bug-fix version is `0.9.1`.

| Criterion | Verification required for resolution |
| --- | --- |
| AC-01 | Both named tests pass for all four templates with uv `0.13.8`, pytest `9.2.0`, debugpy `1.8.22`, and exactly one wrapped Ruff `0.17.5` installation. |
| AC-02 | The corrected tests still fail when a required pin, Ruff timeout/retry setting, deadline, or required package host is removed or changed. |
| AC-03 | The local download fixture reaches the wheel payload for each profile and supported mode; healthy/recovery cases pass and persistent/deadline cases establish bounded failure. |
| AC-04 | All seven Rust checks in [AGENTS.md](../AGENTS.md) pass on the final software revision; both manifest versions match. |
| AC-05 | The four native CI jobs pass and their release artifacts are retained; record the tested revision and Actions run URLs. |

Resolution remains pending. Full integration, Clippy, release,
baseline-independence, and native CI checks for a proposed fix are `not_run`
because no software fix is included in this report. Real SmolVM checks are
also `not_run`; earlier native and guest acceptance requirements remain in
effect. Historical Feature 0016 passing observations concern an earlier
source state and do not establish that the later merge passes.

## Implementation and validation: 2026-11-13

The software correction is implemented locally as application `0.9.1`.
Bug resolution remains pending AC-05. The observations above describe the
original investigation and retain their original source revisions.

The working tree is based on `15b6c5444f8817da63737fc29f942b1d47ec17e5`.
The relevant software changes from affected merge
`e0c5ea6ad89a3402c6840a7129da7186051527a4` were restored before applying
the correction: the four bounded Ruff templates, the timeout regression
test, the failed-builder regression test, and the download fixture.
The focused configuration suite then reproduced both reported failures:
`cargo test --locked --lib config::tests:: -- --nocapture` returned exit
101 with nine passing tests and the two reported assertion messages.

Four expected strings in `src/config.rs` now agree on uv `0.13.8`, pytest
`9.2.0`, debugpy `1.8.22`, and the complete bounded Ruff `0.17.5` command.
The independent fixed expectations still require one Ruff installation,
both Python package hosts, and `coreutils`. The four templates are
byte-identical to the affected merge. Existing user-owned definitions
remain unchanged. Both application manifest versions are `0.9.1`.

The download fixture derives its wheel filename, metadata, and CLI output
from the selected command's Ruff version. Template mode selects `0.17.5`;
historical baseline mode retains `0.16.8`. Successful installs verify the
fixture CLI output. Baseline recovery now validates the historical failure
comparison: the fixture check passes only after the bare command returns
nonzero with `ReadTimeoutError` and leaves no installed application.
Baseline deadline cases are explicitly unsupported because the bare command
has no GNU timeout wrapper; the CLI rejects those combinations with exit 2.

| Criterion | Result | Evidence |
| --- | --- | --- |
| AC-01 | passed | Corrected focused configuration suite: exit 0, 11 tests passed, covering all four profiles |
| AC-02 | passed | All 92 deliberate template mutations were detected by the corrected test assertions; pristine assertions passed before and after the probe |
| AC-03 | passed | All 32 supported real-pipx fixture combinations passed; each requested its selected-version wheel payload |
| AC-04 | passed | All seven required Rust commands returned exit 0; default suite: 118 tests; adapter suite: 185 tests; release executable reports `smolsandbox 0.9.1` |
| AC-05 | not_run | The local software correction has not been published to a tested CI revision; four native jobs, Actions URLs, and retained release artifacts remain pending |

The mutation probe copied both corrected test function bodies byte-for-byte
into a disposable Rust test harness. A `Profile::template` replacement read
disposable template files instead of embedded strings. For each profile,
23 mutations removed or changed each Python pin, Ruff timeout, retries,
kill grace, deadline, package host, or `coreutils`, or added a duplicate
Ruff install. Every mutation changed exactly one occurrence and caused
exit 101 with an assertion panic. Both pristine tests passed before and
after the probe. The actual focused Cargo suite separately verified the
embedded templates. The temporary mutation harness was removed.

The [Feature 0010 implementation audit](../FEATURES/feature-0010-validation.md#bug-0003-implementation-audit-2026-11-13)
records the seven Rust commands, fixture case results, software snapshot,
and remaining native and real guest checks. Independent agent review found
no actionable implementation findings. Local passing results do not
establish native CI or real SmolVM provisioning results.
