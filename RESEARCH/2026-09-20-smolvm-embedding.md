# SmolVM embedding assessment

Date: 2026-09-20  
Decision scope: whether the planned cross-platform Rust rewrite should link SmolVM as a Rust library or drive a SmolVM executable.

## Recommendation

For the first Rust migration, **ship a pinned SmolVM distribution with the application and invoke its executable as a subprocess**. Do not require a separately installed SmolVM, and do not require Codex on the host. Treat replacing that subprocess boundary with direct Rust linking as a later optimization, gated on upstream support for every operation this application needs.

SmolVM really does contain a Rust library and an embedded runtime, so direct linking is more than a hypothetical. It is not presently a good compatibility boundary for this project, however:

- the root `smolvm` engine crate is not published to crates.io;
- its high-level `EmbeddedRuntime` does not expose pack creation, create-from-`.smolmachine`, complete machine inventory, or interactive TTY execution;
- the CLI command modules implementing those workflows are private to the `smolvm` binary;
- the documented first-party Rust SDK deliberately uses the SmolVM executable rather than linking the unpublished engine;
- linking the engine would not remove native runtime assets or the VM boot subprocess, and the documented embedded SDK platform packages do not currently establish Windows support.

This conclusion is based on SmolVM **v1.16.2**, released 2026-09-18 at commit [`ebffebe0`](https://github.com/smol-machines/smolvm/commit/ebffebe0ca924f9f59c3c0c79e551916319c8da5), plus the first-party `smol` repository at commit [`65aecb2b`](https://github.com/smol-machines/smol/commit/65aecb2b2783db1d25202167b8feab92fe6027b9) dated 2026-09-19. The release is identified as the latest on the [v1.16.2 release page](https://github.com/smol-machines/smolvm/releases/tag/v1.16.2).

## What is directly supported

### A real Rust library exists

The v1.16.2 root manifest defines both `[lib] name = "smolvm"` and a `smolvm` binary, and labels the package Apache-2.0 ([manifest](https://github.com/smol-machines/smolvm/blob/v1.16.2/Cargo.toml)). The library describes itself as a library and CLI, exports low-level VM types such as `VmConfig`, `VmBackend`, and `VmHandle`, and publicly exports `embedded` ([library root](https://github.com/smol-machines/smolvm/blob/v1.16.2/src/lib.rs)). The official FAQ likewise says SmolVM is “library-first” and that its Rust crate is the native engine ([FAQ, Platforms and interfaces](https://www.smolmachines.com/faq#platforms-and-interfaces)).

The high-level [`EmbeddedRuntime`](https://github.com/smol-machines/smolvm/blob/v1.16.2/src/embedded/runtime.rs) and [`MachineSpec`](https://github.com/smol-machines/smolvm/blob/v1.16.2/src/embedded/control.rs) provide source-level APIs for creating image/bare machines, starting, stopping, deleting, non-interactive exec, streamed output, files, images, forks, checkpoints, labels, mounts, ports, and lifecycle state. The repository includes direct Rust examples using this API, such as [`examples/reattach_test.rs`](https://github.com/smol-machines/smolvm/blob/v1.16.2/examples/reattach_test.rs).

### The library boundary does not match this application's workflows

The application's current lifecycle depends on `pack create`, `machine create --from`, `machine ls --json`, and `machine exec -it`. Those paths are implemented by CLI types in `src/cli`, while [`src/main.rs`](https://github.com/smol-machines/smolvm/blob/v1.16.2/src/main.rs) declares `mod cli` inside the binary target; external crates cannot call those command modules as a public library API.

At v1.16.2, `EmbeddedRuntime` has no corresponding public high-level method for:

- building a `.smolmachine` from a Smolfile or prepared VM;
- creating a persistent machine from a `.smolmachine`;
- listing the full machine inventory with records/labels;
- attaching host stdin/stdout to an interactive pseudo-TTY.

Some lower-level pieces are public: `smolvm-pack` can read/build/extract pack files, `SmolvmDb` can read records, and `AgentClient` has interactive methods. Combining those pieces is technically possible, but it reproduces orchestration currently housed in private CLI modules. That is coupling to implementation details, not use of a complete supported facade.

### Official Rust SDK behavior confirms the intended boundary

The first-party `smolmachines` Rust SDK currently states that local machines are driven by a `smolvm` engine process because the engine crate is unpublished; its manifest intentionally has no engine dependency ([Rust SDK README](https://github.com/smol-machines/smol/blob/65aecb2b2783db1d25202167b8feab92fe6027b9/sdk/rust/README.md#the-local-engine), [Rust SDK manifest](https://github.com/smol-machines/smol/blob/65aecb2b2783db1d25202167b8feab92fe6027b9/sdk/rust/Cargo.toml)). Its local transport executes the CLI with `std::process::Command` ([local transport](https://github.com/smol-machines/smol/blob/65aecb2b2783db1d25202167b8feab92fe6027b9/sdk/rust/src/transport/local.rs)). If no engine is installed, its bootstrap code downloads and checksum-verifies an engine release ([bootstrap](https://github.com/smol-machines/smol/blob/65aecb2b2783db1d25202167b8feab92fe6027b9/sdk/rust/src/bootstrap.rs)).

SmolVM's own crates.io workflow corroborates that distinction: it publishes `smolvm-protocol`, `smolvm-network`, `smolvm-pack`, `smolfile`, and `smolvm-registry`, but not the root `smolvm` engine ([publish workflow](https://github.com/smol-machines/smolvm/blob/v1.16.2/.github/workflows/publish-crates.yml)). No first-party engine-wide API stability policy or SemVer compatibility commitment was found. There are localized compatibility comments in source, but they are not a guarantee for the entire public module graph. A direct dependency should therefore be pinned to an exact tag and commit, not a loose version range.

## Runtime and platform implications

“Embedded” does not mean “one process” or automatically mean “one file.” SmolVM launches each VM by re-executing a `_boot-vm`-capable binary. The library deliberately exports `internal_boot::run` so another Rust binary can implement that hidden entry point ([library documentation](https://github.com/smol-machines/smolvm/blob/v1.16.2/src/lib.rs), [boot dispatch in the CLI](https://github.com/smol-machines/smolvm/blob/v1.16.2/src/main.rs)). Alternatively an embedder points `SMOLVM_BOOT_BINARY` at a separate helper. The VM monitor remains a child process either way, which is also a desirable isolation boundary.

Direct linking also still requires distribution of SmolVM's runtime assets:

- patched `libkrun` and `libkrunfw` dynamic libraries;
- the Linux guest agent rootfs;
- preformatted storage/overlay templates, especially on Windows;
- optional GPU libraries where supported.

The official release build packages these assets beside the executable rather than relying on the Rust binary alone ([Unix/macOS distribution script](https://github.com/smol-machines/smolvm/blob/v1.16.2/scripts/build-dist.sh), [Windows distribution script](https://github.com/smol-machines/smolvm/blob/v1.16.2/scripts/build-dist-windows.sh)). macOS binaries must be code-signed with the Hypervisor.framework entitlement; Windows requires `krun.dll`, `libkrunfw.dll`, the agent-rootfs archive, disk templates, and WHP. The [release workflow](https://github.com/smol-machines/smolvm/blob/v1.16.2/.github/workflows/release.yml) currently builds macOS arm64, Linux x86-64/arm64, and Windows x86-64 artifacts. The main SmolVM CLI documents native Windows support, but the first-party embedded SDK package list currently covers macOS and Linux, not Windows ([SmolVM README platform table](https://github.com/smol-machines/smolvm/blob/v1.16.2/README.md#platform-support), [embedded SDK notes](https://github.com/smol-machines/smolvm/blob/v1.16.2/sdks/README.md)). Therefore the CLI distribution is the verified three-OS integration path today.

Bundling the official distribution inside this application's release archive is preferable to downloading it on first run: it preserves ready-to-run/offline installation, lets CI test the exact pinned engine and assets, and allows the application to resolve the private engine path without adding SmolVM to the user's `PATH`.

## Licensing and redistribution

- SmolVM itself is Apache-2.0 ([license](https://github.com/smol-machines/smolvm/blob/v1.16.2/LICENSE), [manifest](https://github.com/smol-machines/smolvm/blob/v1.16.2/Cargo.toml)).
- v1.16.2 pins `libkrun` commit [`d2b7c30f`](https://github.com/smol-machines/libkrun/commit/d2b7c30f83382849b17c47df87862b56322c2bd4), whose code is Apache-2.0 ([license](https://github.com/smol-machines/libkrun/blob/d2b7c30f83382849b17c47df87862b56322c2bd4/LICENSE)).
- v1.16.2 pins `libkrunfw` commit [`6ec329e1`](https://github.com/smol-machines/libkrunfw/commit/6ec329e11154814a4df9963a3f94f2a430f55723). Its first-party license section says the bundled Linux kernel and patches are GPL-2.0-only, library/generated code is LGPL-2.1-only, and binary distributions must be accompanied by the source for both the bundled kernel and library ([libkrunfw license statement](https://github.com/smol-machines/libkrunfw/blob/6ec329e11154814a4df9963a3f94f2a430f55723/README.md#license)). It also says programs linking the library need not themselves be GPL/LGPL.

Consequently, both subprocess bundling and direct linking require a release compliance plan: preserve notices/licenses and satisfy `libkrunfw` source-offer/source-accompaniment obligations for the exact binaries shipped. The guest rootfs contains additional third-party components whose notices and redistribution terms also need to accompany releases. This is an engineering finding, not legal advice.

## Codex dependency boundary

Codex is not a host dependency. This repository's [`devtools-base.smolfile`](../devtools-base.smolfile) installs the pinned Codex CLI inside the guest with npm while preparing the reusable artifact. [`start-codex.sh`](../start-codex.sh) and [`start-login.sh`](../start-login.sh) then execute `codex` and `codex login --device-auth` through `smolvm machine exec` inside that VM. The Rust application's required host-side external runtime is therefore SmolVM only—and even that can be privately bundled—while Codex remains part of the built guest artifact.

## Decision gate for future direct linking

Reconsider direct SmolVM linking only when a pinned upstream release provides and tests, on Linux, macOS, and Windows, public high-level APIs for all of the following:

1. create a pack from the embedded/default Smolfile;
2. create/reconcile a named persistent machine from that pack;
3. enumerate complete, labeled machine records safely;
4. interactive TTY exec with resize, Ctrl-C, and exit-status propagation;
5. runtime asset discovery/packaging and `_boot-vm` dispatch for third-party Rust binaries;
6. an explicit compatibility policy for those APIs.

Until then, the subprocess interface is not a concession in portability: it is the boundary the official Rust SDK itself uses, and it is the only first-party path that already covers the project's full lifecycle and all three required host operating systems.
