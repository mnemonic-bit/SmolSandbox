# Feature 0011 validation

## Result and tested revision

The executable rename is implemented and all local checks passed, including
the baseline-independence example and the fresh build and installation.
Feature 0011 is completed. On 2026-10-25, the user confirmed that CI is green
and authorized marking the feature completed. That confirmation closes AC-06
for the four-target native build and artifact-upload matrix. CI results were
reported by the user; no CI run URL was supplied or independently inspected.

The baseline is `4624e7a8088c05f26e8137365a836fb009a276cc`. The implementation
was tested as an uncommitted working tree on 2026-10-25 (Europe/Berlin), with
Rust `1.99.1`, Cargo `1.99.1`, and host `x86_64-unknown-linux-gnu`.
The SHA-256 of `git diff --binary` for the tested tracked changes is
`9d950f8eb9a0e65d91d444fba5ba5a2f103be60da7124dd1de506219e51364bb`.
The feature specification was already untracked at implementation start;
this validation record is also untracked and excluded from that diff hash.
The implementation baseline matches the specification's baseline; no scope
change was required.

The package, library, production binary, command guidance, CI artifacts, and
active Rust documentation now use `smolsandbox`. Package version `0.4.0`,
dependency resolutions, command syntax, persisted metadata, and the retained
shell baseline are preserved. Features 0004 and 0010 retain their cutover gates.

## Commands and results

Logs are available in the local evidence directory
`/workspace/feature0011-checks-fi4pyj8t`. These paths identify workspace evidence;
they are not published CI results.

| Command | Result | Exit | Evidence |
| --- | --- | --- | --- |
| `cargo metadata --locked --no-deps --format-version 1` | passed | 0 | `00.log`: package, library, and production target are `smolsandbox`; the other binary remains `smolvm-test-double`; no old target or crate alias |
| `cargo fmt --all -- --check` | passed | 0 | `01.log` |
| `cargo clippy --locked --all-targets -- -D warnings` | passed | 0 | `02.log` |
| `cargo clippy --locked --features test-adapter --all-targets -- -D warnings` | passed | 0 | `03.log` |
| `cargo test --locked --all-targets` | passed | 0 | `04.log`: 55 library, 9 application, 7 CLI, and 8 differential tests |
| `cargo test --locked --features test-adapter --all-targets` | passed | 0 | `05.log`: 55 library, 54 application, 7 CLI, and 8 differential tests |
| `cargo build --locked --release --bin smolsandbox` | passed | 0 | `06.log` |
| `cargo run --locked --example check_baseline_independence` | passed | 0 | `baseline-independence.log`: isolated source without the retained baseline; formatting, both Clippy modes, both full test modes, release build, and seven compiled test executables with empty `PATH` in each feature mode |
| `git diff --check` | passed | 0 | No whitespace errors |
| Tracked old-name audit | passed | 0 | Remaining occurrences reviewed below; matches are expected |

The tracked old-name audit command was:

```text
git grep -n -I -E 'codex[-_]sandbox|Codex Sandbox'
```

Two intermediate checks failed and were resolved before the full checks above:

- An initial CLI assertion expected every parser error to include a usage line.
  Clap omits usage for an invalid harness value. The test now preserves that
  behavior while requiring application identity for errors that include usage.
- An initial adapter integration run shared a target directory with a concurrent
  build in the default feature mode. Lifecycle tests then encountered a binary
  without the adapter. An isolated target run passed all 54 application and 8
  differential tests; the sequential full runs above also passed. The isolated
  log is `compatibility/feature0011-owned-isolated-tests.log`.

### Fresh build and local installation

The temporary artifact root is
`/workspace/feature0011-artifacts-yt1y3sxe`. The following commands each exited 0:

```text
cargo build --locked --release --bin smolsandbox --target-dir /workspace/feature0011-artifacts-yt1y3sxe/target
/workspace/feature0011-artifacts-yt1y3sxe/target/release/smolsandbox --version
cargo install --locked --path . --bin smolsandbox --root /workspace/feature0011-artifacts-yt1y3sxe/install --target-dir /workspace/feature0011-artifacts-yt1y3sxe/target
/workspace/feature0011-artifacts-yt1y3sxe/install/bin/smolsandbox --version
```

Both executables printed exactly `smolsandbox 0.4.0` and a newline. The fresh
release directory and fresh installation directory contain no `codex-sandbox`
or `codex-sandbox.exe`. The installation's `bin` directory contains only
`smolsandbox`. Logs `00.log` through `03.log` are under the artifact root.
Existing outputs and installed executables were not deleted.

### Native targets

The [workflow](../.github/workflows/rust.yml) preserves the four-target matrix,
both feature-mode checks, the baseline-independence example, and
`if-no-files-found: error`. Its build target is `smolsandbox`; its artifact is
`smolsandbox-${{ matrix.target }}` with `smolsandbox`/`smolsandbox.exe` paths.
Static workflow inspection passed. The user subsequently confirmed on
2026-10-25 that CI is green, including the required build and upload matrix.

| Target | Native validation result | Evidence |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | passed | Local checks in this record; user-confirmed green CI on 2026-10-25 |
| `aarch64-unknown-linux-gnu` | passed | User-confirmed green CI on 2026-10-25 |
| `aarch64-apple-darwin` | passed | User-confirmed green CI on 2026-10-25 |
| `x86_64-pc-windows-msvc` | passed | User-confirmed green CI on 2026-10-25 |

AC-06 passed based on the user's confirmation. No CI result links were supplied;
the native CI results above record that confirmation rather than new local runs.

## Compatibility comparison

The baseline source was extracted with `git archive HEAD` at the revision above.
Its build command exited 0:

```text
cargo build --locked --features test-adapter --bin codex-sandbox --bin smolvm-test-double --manifest-path /tmp/feature0011-baseline.7xII6S/Cargo.toml --target-dir /tmp/feature0011-baseline.7xII6S/target
```

The renamed executable was built and tested with this command, which exited 0:

```text
cargo test --locked --features test-adapter --test application --test differential --target-dir /tmp/feature0011-compat-target
```

`python3 /tmp/feature0011-compare.py` exited 0. Copies of the comparison script,
result JSON, baseline build log, and isolated test log are under
`/workspace/feature0011-checks-fi4pyj8t/compatibility`.

Both binaries used the same baseline-built SmolVM test double, fixed version
`smolvm test 1.19.2`, definition bytes, workspace, environment, inventory, and
saved native home. The baseline created a Codex definition, built the machines,
and started port 3000 before the home and inventory were saved. A credential
fixture was included at `.smolsandbox/codex-home/auth.json`. Each binary then
received a restoration of the same home and inventory at the same paths.

| Input or identity | Observed value |
| --- | --- |
| Definition SHA-256 | `1e0e1b601ccb4c4d9a1544eafe7e66f1fde0fb1299916d750dcd8e84fcf76b4d` |
| Workspace | `/tmp/feature0011-compare.7gkadgpg/project` |
| Native home | `/tmp/feature0011-compare.7gkadgpg/home/.smolsandbox` |
| Base ID | `bc8d49d82eb3` |
| Workspace SHA-256 | `4c596e39eb06dd5549ae11a13e6b75ed166f4dbcdd33d007e4fbc144cec40eae` |
| Workspace-path SHA-256 | `3c60d9cb2f1748e98013a737fb86b4f202577e5a2c6b1f86b742466f53ee034f` |
| Desired workspace machine | `codex-workspace-bc8d49d82eb3-r5-4c596e39eb06` |
| Login machine | `codex-login-bc8d49d82eb3-r5` |

`repair --check` outputs and exit codes matched exactly. Reuse with
`start --port 3000 -- --help` produced equal outputs, SmolVM argument arrays,
inventories, credentials, and every persisted file's bytes. Neither binary
created or deleted a machine during reuse; capability probes using
`machine create --help` were excluded from mutation counting.

Separate equivalent fixtures verified stale revision 4 and contradictory
workspace metadata refusals: both binaries exited 1 without create/delete
operations or inventory changes. An unrelated machine produced equal
`repair --check` plans without mutations. Refusal comparison normalized only
the executable spelling in messages; reuse comparison required exact output
equality. No shell state was imported into the native home.

### Preserved bytes and identities

All four template files equal their baseline bytes:

| Template | SHA-256 |
| --- | --- |
| `templates/claude.smlbx` | `753a1a48cff706b4ef7d31348f560bbf122411ef571beb8270910d4f3ee1ffeb` |
| `templates/codex.smlbx` | `1e0e1b601ccb4c4d9a1544eafe7e66f1fde0fb1299916d750dcd8e84fcf76b4d` |
| `templates/hermes.smlbx` | `0d38bf2171262c6ad64776d47aeb5b9bf9d9a0da3ca1c9a636cfc38d5354fff4` |
| `templates/opencode.smlbx` | `e9699459ce5d5e8298c021eb3765f8dfa1a5daeb85362fb1681f1114b516837d` |

Byte comparisons against `git show HEAD:<path>` also passed for
`devtools-base.smolfile`, `sandbox-common.sh`, `src/domain.rs`,
`src/platform.rs`, `src/smolvm.rs`, and both historical validation records.
Semantic lockfile comparison passed after normalizing the root package's name
and package ordering: the version and dependency resolutions are unchanged.
Diff review found no template, label namespace, marker, native state location,
credential location, machine revision, allowlist, shell baseline, or fixture
protocol changes. Retained shell tests are `not_run`: no retained baseline file,
shell test, or network allowlist changed, so the specification's shell-test
condition was not triggered.

## Old-name occurrence audit

The tracked-file audit found 708 matching lines in 31 files before this new
record was added. `old-name-occurrences.json` in the evidence directory records
every matching line number. All occurrences fall into the categories below.

| Locations | Matching lines | Classification |
| --- | --- | --- |
| Features 0001, 0003, 0005 | 10 | Persisted managed-label keys and their documentation |
| Feature 0004 | 2 | Retained `codex-sandbox.sh` alias and explicitly historical prior-draft application home |
| Feature 0006 | 12 | Retained shell filename, shell usage, and shell tests |
| Feature 0007 | 7 | Six retained shell references and one managed-label key |
| Features 0004-validation and 0009-validation | 3 | Unchanged commands and paths recorded as executed historically |
| `README.md` | 1 | Explicit executable migration note |
| `RUST-APPLICATION.md` | 4 | Migration note, retained marker and label namespace, retained shell mapping |
| `devtools-base.smolfile` | 1 | Unchanged retained baseline comment |
| `examples/check_baseline_independence.rs` | 1 | Exclusion of the actual retained `codex-sandbox.sh` file |
| `sandbox-common.sh`, `sandbox-inventory.py` | 19 | Retained shell temporary path and metadata protocol |
| `src/config.rs` | 5 | Metadata marker reader, diagnostic quoting that marker, and marker fixtures; command recommendation uses `smolsandbox` |
| `src/domain.rs`, `src/lifecycle.rs` | 10 | Managed-label keys, readers, writers, and unknown-key validation fixture |
| Four `templates/*.smlbx` files | 5 | Four retained markers and one unchanged embedded comment |
| `tests/application.rs` | 37 | Managed-label and marker compatibility fixtures and assertions |
| `tests/data/ownership-cases.json`, `tests/data/purge-cases.json` | 558 | Persisted metadata fixtures |
| `tests/differential.rs` | 2 | Explicit assertion that a user state path containing the old spelling is preserved while upgrade guidance uses `smolsandbox` |
| Shell tests: allow-hosts, clean-sandbox, help, purge-sandbox, reconcile-workspace, start-codex | 31 | Retained shell entry points and shell metadata assertions |

The untracked Feature 0011 specification was reviewed separately: its old-name
references describe the rename source, compatibility boundaries, audit pattern,
or baseline observations. This new validation record uses old names only in
historical baseline commands, compatibility descriptions, migration assertions,
and the audit pattern. Neither file adds an active old-name application command.
Product-heading review found no remaining old application title; the retained
shell-command heading in Feature 0006 is intentional. No old Rust import remains.
Documentation link checks passed for all 12 local links in the eight changed
documents.

## Acceptance outcomes

| Criterion | Outcome | Evidence |
| --- | --- | --- |
| AC-01 | passed | Cargo metadata and manifest inspection; no compatibility target or crate alias |
| AC-02 | passed | Fresh release build and local install; exact version output; old executable absent from both fresh directories |
| AC-03 | passed | CLI contract tests: top-level and all seven subcommand usage identities, version and short version flags, representative usage errors, preserved streams/exit codes, offline filesystem snapshots |
| AC-04 | passed | Configuration, build, login, repair, authentication, and future port-state guidance assertions; message literal audit |
| AC-05 | passed | Formatting, both Clippy modes, both full test modes, and baseline-independence example passed |
| AC-06 | passed | Workflow inspection and local Linux checks passed; user confirmed green native CI on 2026-10-25 |
| AC-07 | passed | Baseline-versus-renamed fixed-input lifecycle comparison described above |
| AC-08 | passed | Template hashes, byte comparisons, unchanged identity code, fixture tests, and persisted state/credential comparison |
| AC-09 | passed | Existing copied-binary relocation test passed in both feature modes with the renamed binary and no adjacent resources or host script tools |
| AC-10 | passed | Documentation/specification review, migration notes, preserved cutover status, executed new-name build/version/help commands |
| AC-11 | passed | Complete tracked occurrence classification and separate untracked-file review |

Every mandatory criterion has passed based on the local validation above and
the user's CI confirmation. Feature 0011 is completed. Features 0004 and 0010
retain their separate cutover requirements.
