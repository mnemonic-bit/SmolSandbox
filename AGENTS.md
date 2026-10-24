# Repository instructions

- Use the installed `codebase-memory` skill for structural codebase exploration.
- Before writing natural-language text to any file, read AGENT-LANG.md and apply its rules to that text.

## Sandbox network policy

- Keep the `[network].allow_hosts` list in `devtools-base.smolfile` synchronized
  with the workspace machine's `--allow-host` arguments in `sandbox-common.sh`.
- Whenever either URL allowlist changes, increment
  `SANDBOX_MACHINE_REVISION` in `sandbox-common.sh`. Existing machines retain
  their creation-time network policy, so the revision change is required to
  replace them with machines that use the new allowlist.
- Update the allowlist and revision assertions in
  `tests/devtools-base-config-test.sh` and
  `tests/reconcile-workspace-test.sh`, then run every `tests/*-test.sh` script.
