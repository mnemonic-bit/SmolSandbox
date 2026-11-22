# Feature 0021: Add a Windows ARM64 release target

## Status

Proposed. This change adds a specification and supporting research only.
Implementation, native Windows ARM64 execution, and hosted publication checks
are pending. No workflow, release asset, runtime policy, or package version
changes during specification preparation.

## Outcome and baseline

Add Windows ARM64 as the fifth native build and binary distribution target.
Users can download `smolsbx-aarch64-pc-windows-msvc.zip`, extract its sole
`smolsbx.exe` entry, and run the ARM64 executable on Windows 11 ARM64.
Retain the existing Linux x86-64, Linux ARM64, macOS ARM64, and Windows
x86-64 targets. The result remains three operating systems, with five
operating-system/architecture combinations distributed as ZIPs.

Specification preparation observed commit
`e9957bbee4816dc09ff5549fcbe3d5ae80793324`, branch `develop`, and application
version `0.13.0` on 2026-11-22. The working tree already contained the
untracked `feature-0020-list-output-formats.md`; preserve that document.

The baseline has four entries in `.github/workflows/rust.yml::native`,
`.github/scripts/package_release.py::TARGETS`, and
`.github/scripts/release.py::BINARY_TARGETS`. Packaging checks Windows
x86-64 PE headers and runs the extracted executable with `--version`.
Publication requires exactly the configured binary ZIP set. The workflow's
inline eligibility program duplicates `release.py`, and
`test_inline_eligibility_program_matches_canonical_script` checks equality.
`README.md` has four download buttons, and `RUST-APPLICATION.md` explicitly
excludes Windows ARM64 from the release matrix.

Structural exploration used the installed `codebase-memory` skill and
project `workspace-myrepo`, graph generation `2026-11-22T12:25:45Z`.
Symbol snippets, caller tracing, and direct source reads confirmed the
packaging, publication, and virtualization paths. Coverage checks reported
no recorded issues for the cited workflow, release scripts and tests,
`src/platform.rs`, `src/smolvm.rs`, `tests/application.rs`, `README.md`,
and `RUST-APPLICATION.md`. Coverage is a best-effort signal.

## Feasibility and runtime boundary

GitHub documents the native ARM64 label `windows-11-arm` for both public
and private repositories. Standard public runners are free; private runners
consume included usage and then incur charges. A larger or self-hosted
runner is not required solely to add this architecture. These are documented
capabilities, not evidence of a successful run for this repository.
See [GitHub's runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners#supported-runners-and-hardware-resources).

Use Rust target `aarch64-pc-windows-msvc`. Verify the toolchain host as
well as the output architecture: installing an ARM64 compilation target
into an x86-64 toolchain does not establish native ARM64 test execution.
The [research record](feature-0021-windows-arm-research.md) records current
Rust support, runner tools, action compatibility, and upstream runtime
limitations with primary-source links.

The existing application rejects `windows/aarch64` in
`src/platform.rs::virtualization_preflight`, reached through
`Smolvm::preflight` before lifecycle mutation. A new release binary alone
does not enable sandbox creation or guest execution. This feature covers
native CLI distribution and the existing tests using the SmolVM test
double. Keep the unsupported-host rejection for Windows ARM64. Document
that limitation next to the new download and in the host requirements.
Help, version, and configuration generation remain usable without SmolVM.

Enabling Windows ARM64 virtualization requires a separate feature backed
by a compatible upstream SmolVM runtime and real guest evidence. Do not
treat runner availability, x86-64 emulation, mocked lifecycle tests, or a
successful `--version` invocation as that evidence.

## Scope and ownership

The implementer owns `.github/workflows/rust.yml`, release and packaging
scripts and regressions under `.github/scripts/`, the new download button
under `assets/badges/`, `README.md`, `RUST-APPLICATION.md`, matching package
versions, and implementation evidence. The repository owner owns Actions
access and account-level runner availability and billing settings.

Preserve the existing commands, template bytes, user-owned definitions,
state paths, machine identity, network policy, lifecycle policy revision,
and release permissions. No installer, package-manager distribution,
Windows x86 target, ARM64EC binary, runtime bundling, or self-hosted runner
provisioning is included.

Specification preparation leaves `0.13.0` unchanged. Implementation adds a
feature: increment MINOR in `Cargo.toml` and the matching `smolsandbox`
entry in `Cargo.lock`, resetting PATCH to zero. From the observed baseline
the implementation version would be `0.14.0`; derive the bump from the
actual implementation baseline if other software changes land first.
This release-target addition requires no lifecycle `REVISION` increment.

## Required behavior

### Native validation

1. Add exactly one matrix entry pairing `runner: windows-11-arm` with
   `target: aarch64-pc-windows-msvc`. Retain the existing four entries.
2. Install a stable ARM64-host Rust toolchain with `rustfmt` and `clippy`.
   Record runner OS/architecture, runner image version, and `rustc -vV`.
   Require host `aarch64-pc-windows-msvc`; fail an architecture mismatch.
3. Run the same formatting, Clippy, tests, production build, and baseline
   independence checks as the other native jobs. Run Clippy and tests both
   with default features and with `test-adapter`. Preserve native Windows
   credential-permission tests and existing test-double coverage.
4. Verify that the baseline-independence example's internally invoked Cargo
   commands also use the native ARM64 host toolchain. Record its execution
   of compiled tests with an empty `PATH`.
5. Use runner-provided Bash and Python for packaging when available. Resolve
   any toolchain, shell, or action incompatibility in the new job without
   weakening the other jobs or introducing a host script dependency into
   the distributed executable. Verify the pinned checkout and upload
   actions on Windows ARM64; retain release download on Ubuntu.
6. Preserve branch/event guards: build only pushes or permitted manual runs
   on `main` and `develop`; pull requests into `main` run the existing
   read-only eligibility check. Only an eligible push to `main` publishes.
   Failure, cancellation, or skipping of the Windows ARM64 job prevents
   publication just as failure of another required native job does.

No hosted guest boot or nested virtualization is required by this feature.
Keep `test-adapter` limited to checks; build the distributed `smolsbx.exe`
without `test-adapter`.

### Packaging and publication

1. Add `aarch64-pc-windows-msvc` to packaging and publication target lists.
   Keep the inline workflow eligibility program equal to `release.py`.
   Update diagnostics that describe four required targets to describe five
   or derive the count from the required target set.
2. Read the production binary from
   `target/aarch64-pc-windows-msvc/release/smolsbx.exe`. Validate the DOS
   header, PE signature, and COFF machine value `0xAA64` for ARM64. Reject
   an x86-64 binary, an x86 binary, a malformed header, or a truncated
   header even if a renamed executable can run through emulation. Retain
   the existing x86-64 check using `0x8664`. Machine values come from
   [Microsoft's PE specification](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#machine-types).
3. Create `smolsbx-aarch64-pc-windows-msvc.zip` containing only the regular
   file `smolsbx.exe` at the archive root. Retain deterministic packaging
   and the existing executable-only contract. Extract the ZIP and require
   `--version` to exit `0` with `smolsbx <package-version>` on the ARM64 job.
4. Upload the ZIP as Actions artifact
   `smolsbx-aarch64-pc-windows-msvc`. Retain distinct artifact names for all
   five targets. Download the complete set from the same workflow run and
   immutable commit into the release job.
5. Require exactly five uploaded binary ZIP assets for releases produced
   by the implementation workflow. Validate every required local ZIP
   before publication writes. A missing ARM64 ZIP fails with a diagnostic
   naming `aarch64-pc-windows-msvc`; retain the other missing-target checks.
6. Preserve tag identity, generated notes, upload verification, partial
   recovery, and idempotency. A complete rerun adds no duplicate assets;
   recovery retains verified asset IDs and bytes. Conflicting assets fail
   without replacement or deletion.
7. Preserve existing releases, tags, and four-target workflow revisions.
   Do not backfill older releases or rerun old publication using the new
   target requirements. Recovery of an older release uses its original
   workflow revision and required asset set.

The new stable download URL is:

```text
https://github.com/mnemonic-bit/SmolSandbox/releases/latest/download/smolsbx-aarch64-pc-windows-msvc.zip
```

### Download and operating documentation

Add `assets/badges/download-windows-arm64.svg` using the established SVG
button style. Add a README download button with visible label
`Windows ARM64` and alternative text
`Download smolsbx for Windows ARM64 (ZIP)`. Link the button to the stable
URL above. Retain the existing four buttons and all status badges.

Update the release-target and release-asset tables in
`RUST-APPLICATION.md`, along with active references to four required assets.
Separate CLI distribution targets from supported virtualization hosts.
Place an explicit Windows ARM64 sandbox-runtime limitation next to the
README download guidance and in the host requirements. Explain that the
new button requires the first normal release containing the fifth asset;
before publication, the latest release may have only four ZIPs. Keep the
release-page fallback and historical versioned-filename guidance.

Preserve previous specifications and validation observations as historical
records. For releases using the implementation workflow, this feature
supersedes the four-target counts in Feature 0017 and the README-badge
Feature 0019 without changing their previous acceptance evidence.

## Acceptance criteria

| ID | Required observable result | Verification |
| --- | --- | --- |
| AC-01 | The native matrix has the existing four targets plus exactly one `windows-11-arm` / `aarch64-pc-windows-msvc` entry. | Workflow regression with an independent expected five-target set and runner mapping; workflow syntax validation. |
| AC-02 | Windows ARM64 runs the full native check sequence using an ARM64-host toolchain, including both feature configurations, credential permissions, and baseline independence. | Hosted run logs with runner architecture, image version, `rustc -vV`, commands, and exit codes. |
| AC-03 | Packaging accepts ARM64 PE machine `0xAA64` and rejects substituted x86-64/x86 binaries, malformed signatures, and truncated headers; x86-64 packaging still accepts `0x8664`. | Independent byte fixtures in `test_package_release.py`, including architecture substitutions in both Windows directions. |
| AC-04 | The new ZIP contains only root `smolsbx.exe` from a production build without `test-adapter`, and the extracted ARM64 executable reports the expected version. | ZIP entry and byte checks plus hosted architecture validation and native `--version` execution. |
| AC-05 | Publication requires exactly the five expected ZIPs from the same validated commit/run; a missing ARM64 artifact or failed ARM64 job prevents publication. | Fake-API missing-package regressions, workflow dependency/guard checks, and a disposable hosted publication run. |
| AC-06 | Five-target upload recovery and complete reruns preserve verified assets, notes, and tag identity; conflicting contents fail without replacement. | Extend `test_release_assets.py` and `test_release.py` with independent five-target expectations and recovery fixtures. Verify asset IDs and bytes in a disposable repository. |
| AC-07 | Existing release triggers, read-only eligibility, permission boundaries, and inline/canonical script equality remain valid. | Existing workflow regressions, including `test_inline_eligibility_program_matches_canonical_script`, with the new target present. |
| AC-08 | The rendered README has five download buttons, and the new link downloads an ARM64 ZIP after the first eligible release. Operating guidance distinguishes binary support from unsupported Windows ARM64 virtualization. | Rendered README review, release-table review, and public asset download with PE and ZIP inspection. |
| AC-09 | Windows ARM64 help, version, and configuration generation work without SmolVM; production virtualization preflight retains its unsupported-host rejection. | Native offline smoke checks and an ARM64 production-path rejection check with no lifecycle mutation. Mocked lifecycle results remain labelled as mocked. |
| AC-10 | The manifest and lockfile have a matching minor version bump, the original four targets retain passing checks, and historical releases remain unchanged. | Version diff review, full five-target hosted run, release regressions, and comparison of historical tags and assets. |

## Implementation evidence

During implementation, create `FEATURES/feature-0021-validation.md` with
the tested revision, package version, toolchain and runner image versions,
commands, exit codes, workflow URLs, artifact and release asset IDs, and
AC-01 through AC-10 results. Use `passed`, `failed`, `not_run`, or
`inconclusive`. Run the seven independent Rust checks in
[AGENTS.md](../AGENTS.md), the complete release regression suite
(`python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v`),
and workflow syntax validation, recording the validator and version.

Use a disposable repository for publication, interrupted-upload, and
failure-gate checks. Do not create test releases in the production
repository. Record unavailable hosted checks as `not_run`; local fixtures
do not establish hosted action compatibility or native Windows execution.

Append new validation observations and pending native or real guest checks
to [Feature 0010 validation](feature-0010-validation.md), preserving prior
observations and source revisions. Distribution acceptance does not close
the existing real guest cutover gates or establish Windows ARM64 runtime
support. Keep implementation acceptance pending until all mandatory
criteria have passing evidence.

Specification preparation: software, native Windows ARM64, and hosted
publication checks are `not_run` because this change adds only documents.
The [research record](feature-0021-windows-arm-research.md) contains the
documentation findings and their limits.
