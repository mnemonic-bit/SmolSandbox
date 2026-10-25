# Repository instructions

- Use the installed `codebase-memory` skill for structural codebase exploration.
- Before writing natural-language text to any file, read AGENT-LANG.md and apply its rules to that text.

## Versioning

- Include a version bump in `Cargo.toml` and the matching `smolsandbox` package
  entry in `Cargo.lock` with every software change, including changes to tests,
  dependencies, and build configuration.
- Use `MAJOR.MINOR.PATCH`: increment `PATCH` for bug fixes, `MINOR` for features,
  and `MAJOR` for breaking changes. Reset lower components to zero when
  incrementing `MINOR` or `MAJOR`.
- If a change only adds feature specifications or bug reports, leave the
  software version unchanged.

## Sandbox network policy

- Keep the `[network].allow_hosts` list in `src-old/devtools-base.smolfile` synchronized
  with the workspace machine's `--allow-host` arguments in `src-old/sandbox-common.sh`.
- Whenever either URL allowlist changes, increment
  `SANDBOX_MACHINE_REVISION` in `src-old/sandbox-common.sh`. Existing machines retain
  their creation-time network policy, so the revision change is required to
  replace them with machines that use the new allowlist.
- Update the allowlist and revision assertions in
  `src-old/tests/devtools-base-config-test.sh` and
  `src-old/tests/reconcile-workspace-test.sh`, then run every `src-old/tests/*-test.sh` script.
