# smolsandbox

The Rust application supports Claude, Codex, OpenCode, and Hermes Agent. Its
executable was renamed from `codex-sandbox` to `smolsandbox`; update callers to
use `smolsandbox`. Existing native definitions, credentials, application state,
and machines are retained. Codex remains a supported harness.

Features 0004 and 0009 are marked implemented following the user's CI
confirmation and completion request. The Rust implementation and its validation
commands are documented in
[smolsandbox: operation and validation](RUST-APPLICATION.md). The shell workflow
remains in `src-old/`. Feature 0010's documentation cutover is still pending.

The [retained shell workflow](src-old/README.md) includes the old scripts,
Python helpers, Smolfile, shell tests, and test fixture. Use the Rust application
for current commands.
