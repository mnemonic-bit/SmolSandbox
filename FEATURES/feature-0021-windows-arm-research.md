# Feature 0021 research: Windows ARM64 release target

Research date: 2026-11-22.

## Decision

GitHub Actions supports a native Windows ARM64 build, test, and packaging
job using `runs-on: windows-11-arm`. Rust supports the required
`aarch64-pc-windows-msvc` target. Adding a fifth native CLI release target
is feasible based on current documentation. The proposed behavior and
acceptance criteria are in the
[feature specification](feature-0021-windows-arm64-release-target.md).

A native CLI release does not establish Windows ARM64 sandbox lifecycle
support. Upstream SmolVM currently documents Windows x86-64 support and
publishes a Windows x86-64 runtime. Retain SmolSandbox's Windows ARM64
virtualization rejection until a compatible runtime and real guest
validation establish support.

## GitHub runner availability

GitHub's current [hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
lists `windows-11-arm` for both public and private repositories. Public
jobs receive four processors and 16 GB RAM; private jobs receive two
processors and 8 GB RAM. Both configurations list 14 GB storage. Standard
runners are free for public repositories. Private usage consumes included
usage before additional charges apply.

Windows and Linux ARM64 standard runners
[became generally available for public repositories on 2025-08-07](https://github.blog/changelog/2025-08-07-arm64-hosted-runners-for-public-repositories-are-now-generally-available/).
GitHub subsequently
[added private repository support on 2026-01-29](https://github.blog/changelog/2026-01-29-arm64-standard-runners-are-now-available-in-private-repositories/).
The public-only restriction in the 2025 announcement is obsolete. A paid
larger runner is not required merely because the repository is private.
Repository-specific Actions access and billing settings were not audited.

## Toolchain and Actions compatibility

The [Rust platform reference](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
lists `aarch64-pc-windows-msvc` as Tier 1 with host tools. The target
supports the standard library and native Windows PE/COFF executables.
Native compilation and execution are supported; cross-compilation alone
would not establish native test execution.

The current [Windows ARM64 runner image inventory](https://github.com/actions/runner-images/blob/main/images/windows/Windows11-VS2026-Arm64-Readme.md)
includes Rust, Cargo, rustup, Clippy, rustfmt, Python, Bash, ARM64 Visual
C++ tools, and Windows SDK components. Record the exact image and tool
versions from each workflow run because runner images change.

The repository's existing action versions use the Node 24 runtime:
[checkout v7.0.1](https://github.com/actions/checkout/blob/v7.0.1/action.yml),
[upload-artifact v7.0.2](https://github.com/actions/upload-artifact/blob/v7.0.2/action.yml),
and [download-artifact v8.0.2](https://github.com/actions/download-artifact/blob/v8.0.2/action.yml).
The [GitHub runner source](https://github.com/actions/runner/blob/main/src/Misc/externals.sh)
provisions Node 24 for Windows ARM64. The
[rust-toolchain action](https://github.com/dtolnay/rust-toolchain/blob/master/action.yml)
selects an ARM64 rustup installer when `runner.arch` is `ARM64`.

Inference: the existing action choices provide a viable implementation
path. A successful workflow must still demonstrate checkout, toolchain
setup, native tests, ZIP creation, upload, and release-job download.

Microsoft's [PE machine-type reference](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#machine-types)
assigns `0xAA64` to ARM64 and `0x8664` to x86-64. Packaging must distinguish
those values rather than infer architecture from an executable's filename
or successful execution under emulation.

## Sandbox runtime limitations

SmolVM's [platform support table](https://github.com/smol-machines/smolvm#platform-support)
lists Windows x86-64 with Windows Hypervisor Platform (WHP). The table
does not advertise Windows ARM64. At upstream revision
[`bedc0c8affa5b68662849c34c95ee091ff43f28f`](https://github.com/smol-machines/smolvm/tree/bedc0c8affa5b68662849c34c95ee091ff43f28f),
the [release workflow](https://github.com/smol-machines/smolvm/blob/bedc0c8affa5b68662849c34c95ee091ff43f28f/.github/workflows/release.yml#L432)
builds the Windows distribution for `x86_64-pc-windows-gnu`, using
`lib/windows-x86_64`.

A read-only query of GitHub's release API found Darwin ARM64, Linux ARM64,
Linux x86-64, and Windows x86-64 distributions in
[SmolVM v1.36.1](https://github.com/smol-machines/smolvm/releases/tag/v1.36.1).
No Windows ARM64 distribution appeared in that release's asset list.
This observation is bounded to the named release and upstream revision.

WHP itself is not categorically unavailable on ARM64. Microsoft's
[WHvGetCapability reference](https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/funcs/whvgetcapability)
documents ARM64 support from Windows 11 24H2 build 26100.3915. Applications
must check hypervisor presence and the `Arm64Support` capability. That
operating-system API support does not establish SmolVM backend support.

GitHub documents [nested virtualization as unsupported and experimental](https://docs.github.com/en/actions/concepts/runners/github-hosted-runners#runner-images).
Hosted native CLI tests and tests using the SmolVM test double therefore
cannot establish passing real guest behavior.

## Verification status

| Check | Result | Evidence or limit |
| --- | --- | --- |
| Primary-source documentation review | `passed` | Linked GitHub, Rust, Microsoft, action, and SmolVM sources support a native CLI distribution target. |
| GitHub Windows ARM64 workflow execution | `not_run` | Specification preparation adds no CI implementation. |
| ARM64 release ZIP validation and publication | `not_run` | Implementation is pending. |
| Real Windows ARM64 SmolVM guest lifecycle | `not_run` | A compatible upstream runtime and suitable host remain prerequisites. |

The implementer must distinguish the Windows ARM64 CLI release from
supported sandbox hosts in download documentation. Full sandbox support
requires a separate runtime compatibility decision and real guest evidence.
