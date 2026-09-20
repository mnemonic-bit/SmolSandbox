# Codex in a SmolVM sandbox

This repository builds a reusable, all-batteries-included `.smolmachine` for
Codex and creates persistent VMs from it. The prepared Ubuntu 24.04 environment
contains Node.js 22, Python, .NET 8, Git, common diagnostics, Podman/Buildah,
Playwright/Chromium, Codex, selected Codex skills, and codebase-memory-mcp.

The workflow deliberately uses `smolvm machine create --from`, not
`smolvm pack run`. SmolVM extracts an artifact's base layers once and shares
them between named machines; each machine then keeps its own writable overlay.
That avoids copying the complete pack on every Codex invocation.

## Prerequisite

Install SmolVM:

```bash
curl -sSL https://smolmachines.com/install.sh | bash
smolvm --version
```

On Linux, SmolVM also requires KVM access at `/dev/kvm`.

## Build and create the machines

From this repository, run:

```bash
./build-sandbox.sh
```

The current directory is used as the first workspace. Pass another directory
to prepare its workspace VM instead:

```bash
./build-sandbox.sh /path/to/project
```

The build performs three operations:

1. It creates and provisions a persistent builder from
   `devtools-base.smolfile`.
2. It snapshots the stopped builder into a definition-addressed `.smolmachine`
   under `artifacts/`.
3. It creates, but does not start, a login VM and a workspace VM from that
   artifact.

The generation hash includes both the Smolfile and `smolvm --version`. Running the
build again with the same inputs reuses the builder, artifact, and machines.
Changing the base definition or SmolVM version creates a new generation and
leaves the old one intact. Major application-level tools are version-pinned in
the Smolfile; changing a pin naturally produces a new generation.

## Authenticate

For ChatGPT subscription authentication, run once:

```bash
./start-login.sh
```

This VM can reach only `auth.openai.com`, `chatgpt.com`, and `api.openai.com`.
It stops after the login command finishes. Authentication and Codex state live
in the ignored host directory `.smolvm/codex-home`, shared with workspace VMs.

Alternatively, export `OPENAI_API_KEY`; `start-codex.sh` passes it to the Codex
process as a SmolVM secret reference without persisting the value in the
machine configuration:

```bash
export OPENAI_API_KEY=...
```

## Run Codex

Start Codex in the current directory:

```bash
/path/to/this/repo/start-codex.sh
```

Or specify a workspace and optional Codex arguments:

```bash
/path/to/this/repo/start-codex.sh /path/to/project -- --help
```

Each absolute workspace path gets a deterministic named machine created from
the same cached artifact. Reopening that workspace reuses its machine and
writable overlay. Common development ports (3000, 5000, 5173, 8000, and 8080)
are forwarded to the host. The launcher stops the VM when Codex exits, so the
ports are released while the machine's disk state remains available.

The workspace VM has allowlisted access to OpenAI, GitHub, npm, NuGet, PyPI,
Docker Hub, and Microsoft Container Registry. Add more `--allow-host` entries
in `build-sandbox.sh` when a project needs another service; changing an
existing machine's policy requires recreating or updating that stopped VM.

## Cache and state layout

- `artifacts/codex-devtools-<base-id>.smolmachine`: reusable prepared base.
- `.smolvm/base-id`: the generation used by the launcher scripts.
- `.smolvm/machine-revision`: the current mounts/ports/network-policy revision.
- `.smolvm/codex-home`: shared login, configuration, skills, and sessions.
- `codex-builder-<base-id>`: stopped builder retained for inexpensive repacks.
- `codex-login-<base-id>-r<revision>`: login-only VM.
- `codex-workspace-<base-id>-r<revision>-<workspace-id>`: one persistent VM
  per workspace. Bump `MACHINE_REVISION` when changing its runtime policy.

Both `artifacts/` and `.smolvm/` are excluded from Git.
