# Codex in a SmolVM sandbox

This repository builds a reusable, all-batteries-included `.smolmachine` for
Codex and creates persistent VMs from it. The prepared Debian 12 slim environment
contains Node.js 22, Python, .NET 8, Git, common diagnostics, Podman/Buildah
with a Docker-compatible command, Playwright/Chromium, Codex, selected Codex
skills, and codebase-memory-mcp.

The workflow deliberately uses `smolvm machine create --from`, not
`smolvm pack run`. SmolVM extracts an artifact's base layers once and shares
them between named machines; each machine then keeps its own writable overlay.
That avoids copying the complete pack on every Codex invocation.

## Prerequisite

Install SmolVM and ensure Python 3 is available on the host (the lifecycle
scripts use it to validate SmolVM's JSON inventory):

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

After a successful build, the script runs the conservative purge policy. It
removes unprotected legacy generations, but refuses that cleanup when a
current-generation custom workspace configuration exists. That refusal does
not turn an otherwise successful build into a failure; the output shows the
standalone force commands instead.

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

Add hosts required by a project with repeatable `--allow-host` options. They
may appear before or after the workspace (but before the `--` that starts Codex
arguments):

```bash
/path/to/this/repo/start-codex.sh \
  --allow-host packages.example.com \
  /path/to/project \
  --allow-host downloads.example.com
```

These hosts extend, rather than replace, the default allowlist. The effective
host set is sorted and deduplicated, so option order and repeated hosts do not
change the selected machine or add duplicate `--allow-host` arguments.

Each effective workspace configuration gets a deterministic named machine
created from the same cached artifact. Its workspace hash covers the canonical
set of immutable `smolvm machine create` options: the base artifact, CPU and
memory allocation, host-to-guest mounts (including the canonical workspace
path), and the complete default-plus-user allowlist. Changing any of those
baked options selects a different machine; reopening the same configuration
reuses its machine and writable overlay. The launcher stops the VM when Codex
exits while preserving the machine's disk state.

The workspace VM has allowlisted access to OpenAI, GitHub, npm, NuGet, PyPI,
Docker Hub, GitHub Container Registry (GHCR), and Microsoft Container Registry.

## Clean and rebuild the current generation

Remove every builder, login, and workspace VM belonging to the current base
generation, together with its cached artifact:

```bash
./clean-sandbox.sh
```

The cleanup leaves `.smolvm/codex-home` intact, so authentication, Codex
configuration, skills, and sessions survive. Machines and artifacts belonging
to other base generations are not changed. Build a fresh copy of the cleaned
generation with:

```bash
./build-sandbox.sh [workspace]
```

## Purge superseded generations and configurations

Remove stopped machines from superseded generations and their unprotected
artifact files with:

```bash
./purge-sandbox.sh
```

The conservative command is an all-or-nothing operation. If it finds a
current-generation workspace created with additional `--allow-host` values,
it prints every current workspace configuration and removes nothing. Use
`--force` to remove stopped custom configurations and legacy machines while
preserving stopped default configurations:

```bash
./purge-sandbox.sh --force
```

Use `--force-all` to remove every stopped managed builder, login, and workspace
machine in scope:

```bash
./purge-sandbox.sh --force-all
```

All policies preserve running machines, any artifact generation needed by a
running machine, and both forms of the current artifact. Purge validates the
complete machine inventory and exact managed artifact names before deletion;
ambiguous metadata or an inventory change stops the operation. If base
initialization was aborted before the current artifact was published, purge
may remove the exact stopped `codex-builder-<base-id>` machine so the next
build can start cleanly; that recovery run preserves every other machine and
artifact. A missing artifact without that stopped builder still stops the
operation. Unrelated machines and files are never selected.

## Inspect and repair workspace machines

The launcher reconstructs the desired base, revision, and workspace identity
from the current Smolfile, SmolVM version, and canonical machine-creation
options. The files under `.smolvm/` are compatibility caches; deleting or
corrupting them does not prevent an existing matching workspace machine from
being recovered.

If an older machine exists for a workspace but no machine matches the current
sandbox definition, normal startup stops without creating or deleting
anything. Inspect the candidates with:

```bash
./repair-sandbox.sh --check /path/to/project
```

For a configuration with additional hosts, repeat the same `--allow-host`
options on inspection or replacement commands.

Replace stopped stale machines explicitly with:

```bash
./repair-sandbox.sh --replace-stale /path/to/project
```

Replacement is a create-before-delete handoff. The repair command prepares the
current base, creates the desired workspace machine when needed, and verifies
that machine from a fresh inventory before it stops or deletes any stale
machine. If preparation or verification fails, stale machines and compatibility
caches are left unchanged; a newly registered desired machine is preserved for
inspection.

The repair command refuses to replace a running stale machine without explicit
authorization. After inspecting the report, `--force` permits that machine to
be stopped during cleanup, after the desired machine has passed verification:

```bash
./repair-sandbox.sh --replace-stale --force /path/to/project
```

Only machines positively associated with the workspace by managed labels or a
strict legacy name are eligible for removal. If cleanup fails, the verified
desired machine and every unprocessed stale machine remain available, and the
compatibility caches are not rewritten.

## Cache and state layout

- `artifacts/codex-devtools-<base-id>.smolmachine`: reusable prepared base.
- `.smolvm/base-id`: compatibility cache of the derived base generation.
- `.smolvm/machine-revision`: compatibility cache of the runtime-policy revision.
- `.smolvm/codex-home`: shared login, configuration, skills, and sessions.
- `codex-builder-<base-id>`: temporary build VM, deleted after the reusable
  artifact has been created (stopped builders backed by this repo's cached
  artifacts are cleaned on the next start).
- `codex-login-<base-id>-r<revision>`: login-only VM.
- `codex-workspace-<base-id>-r<revision>-<workspace-id>`: one persistent VM
  per immutable workspace configuration. New workspace machines carry both a
  path-only workspace digest and a `default` or `custom` configuration label,
  while the workspace ID continues to cover the complete immutable creation
  specification. Bump `SANDBOX_MACHINE_REVISION` in `sandbox-common.sh` when
  changing its runtime policy.

Both `artifacts/` and `.smolvm/` are excluded from Git.
