# smolsandbox lifecycle

This context defines lifecycle terms for the Rust `smolsandbox` application,
invoked with `smolsbx`. The Cargo package and library crate remain
`smolsandbox`; application state remains under `.smolsandbox`.
The [operating reference](RUST-APPLICATION.md) contains command and state rules.
The former shell/Python workflow was removed in commit
`28087885c2b84077c62ee92c5560a2cab1b68997` and remains available in Git history.

## Language

**Selected definition**:
The complete user-owned Smolfile chosen for an invocation. Definitions are
selected without merging. A definition identifies one harness profile and
contains its provisioning and default network policy.

**Harness profile**:
The selected coding-agent harness: Claude, Codex, OpenCode, or Hermes Agent.
Each profile has its own guest launch, authentication workflow, and persistent
harness home.

**Sandbox generation**:
A family of artifacts and machines produced from the exact selected definition
bytes and one SmolVM version.
_Avoid_: Version, base version

**Workspace configuration**:
One immutable machine policy for a workspace within a sandbox generation.
The policy includes the canonical workspace path, base artifact, resource
allocation, mounts, and effective network allowlist. Different immutable
policies select different workspace configurations. Published TCP port sets
are mutable and do not select a different workspace configuration.
_Avoid_: Workspace version, sandbox variant

**Default configuration**:
The workspace configuration whose effective network allowlist contains no
hosts beyond the selected definition's allowlist. Repeating an existing
definition host with `--allow-host` retains the default classification.
_Avoid_: Base machine, standard version

**Custom configuration**:
A workspace configuration whose effective policy includes a user-supplied
network host absent from the selected definition's allowlist.
_Avoid_: Old variant, temporary sandbox

**Legacy generation**:
A sandbox generation older than the current generation and represented in a
recognized earlier identity format.
_Avoid_: Old variant

**Protected generation**:
The current sandbox generation or a generation retained because it has a
running managed machine.
_Avoid_: In-use artifact
