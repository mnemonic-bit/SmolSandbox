# Codex Sandbox Lifecycle

This context describes the identities and retained state of reusable Codex
sandboxes and their workspace-specific variants.

## Language

**Sandbox generation**:
A family of artifacts and machines produced from one base sandbox definition
and one SmolVM version.
_Avoid_: Version, base version

**Workspace configuration**:
One immutable machine policy for a workspace within a sandbox generation.
Different network allowlists are different workspace configurations.
_Avoid_: Workspace version, sandbox variant

**Default configuration**:
The workspace configuration that uses the repository's standard policy without
user-supplied network hosts.
_Avoid_: Base machine, standard version

**Custom configuration**:
A workspace configuration whose effective policy includes a user-supplied
network host beyond the default policy.
_Avoid_: Old variant, temporary sandbox

**Legacy generation**:
A sandbox generation older than the current generation and represented in a
recognized earlier identity format.
_Avoid_: Old variant

**Protected generation**:
The current sandbox generation or a generation retained because it has a
running managed machine.
_Avoid_: In-use artifact
