# Feature 0008: Provisioning-download integrity

## Status

Proposed — deferred from Feature 0004

This feature is independent follow-up work. Its decisions and implementation
do not block the Rust script replacement in
[Feature 0004](feature-0004-replace-shell-scripts-with-cross-platform-rust-application.md).

## Summary

Define and implement a consistent integrity policy for downloads performed
while SmolVM provisions the sandbox guest. Cover the base image, operating
system packages, harness installers and packages, development tools, skills,
browser payloads, and other transitive downloads used by the supported
definitions.

The Rust migration preserves existing provisioning behavior and verification.
This feature separately decides which additional guarantees to require and
how to enforce them. It does not assume that version pins alone establish
content integrity or that every ecosystem needs the same mechanism.

## Motivation

Sandbox provisioning combines package managers, release binaries, installer
scripts, and other acquisition paths. Those paths have different versioning,
transport, and verification behavior. A written policy and direct failure
tests are needed before stronger guarantees can be claimed consistently.

Keeping this work separate lets the host lifecycle implementation move to
Rust without making an unrelated provisioning-policy redesign a prerequisite.

## Goals

- Inventory provisioning downloads and existing checks across the Claude,
  Codex, OpenCode, and Hermes Agent templates and their shared tools.
- Define the accepted integrity mechanism for each acquisition path.
- Make verification failures actionable and prevent failed builds from being
  published or reused as successful sandbox artifacts.
- Test verification, partial failures, retries, and cache publication.

## Non-goals

- Blocking or expanding Feature 0004's Rust migration.
- Rewriting lifecycle reconciliation, machine identity, or harness selection.
- Defining application release signing, application build provenance, or
  native installer policy.
- Verifying arbitrary downloads made later by a user, agent, or project.
- Managing harness credentials, models, or providers.
- Requiring offline initial provisioning or promising byte-identical builds
  without a separately established reproducibility policy.

## Decisions to resolve before implementation

### Download inventory and version policy

List each direct download, installer-triggered download, and relevant package
resolution path. Record the source, version selection, target architecture,
existing ecosystem checks, and responsible template or provisioning step.
Decide where exact versions, immutable revisions, image digests, or dependency
locks are required, including transitive inputs.

### Transport and content verification

Define transport requirements and allowed redirect behavior. Decide where
repository/package-manager verification is sufficient and where trusted
hashes, signatures, or other integrity metadata are required. Specify where
expected verification information comes from and how it is reviewed and
updated. A checksum obtained from the same untrusted payload location must
not be described as independent authentication without justification.

### Installer scripts

Define how installer scripts are selected, pinned, acquired, and checked
before execution, and how their downstream downloads are covered. Explicitly
address scripts that resolve a moving release or perform their own integrity
checks. Do not assume that verifying an installer automatically verifies every
payload it downloads.

### Missing verification information

Choose and document behavior when required hashes, signatures, or ecosystem
metadata are unavailable: refusal, an explicitly scoped exception, or another
reviewed policy. Define any exception's scope and user-visible diagnostics.
Do not silently choose fallback behavior during implementation.

### Failures, retries, and cache publication

Define handling of interrupted transfers, verification mismatches, failed
installers, and partial provisioning. Specify temporary-file cleanup, retry
behavior, and the verification boundary before publishing a reusable artifact.
An unsuccessful build must never become a successful cache entry or affect an
existing working machine.

### User-owned definitions and upgrades

Define which guarantees apply to application-generated templates and which
can be established for a user-edited or explicitly overridden `.smolfile`.
Do not claim to verify arbitrary provisioning commands without an enforcement
mechanism. Define how users obtain improved templates without silently
overwriting their existing `~/.smolsandbox/.smolfile` or credential state.

## Relationship to existing behavior

Feature 0004 supplies the portable executable, user-owned definition, fixed
harness profiles, and lifecycle implementation. It may be completed and
released before this feature. This work shall preserve its configuration
ownership and no-silent-overwrite rules.

Changes to generated definition bytes naturally produce a new base generation
under the existing identity rules. Provisioning failures do not authorize
stale-machine cleanup, replacement, or deletion. Existing artifact publication
and lifecycle safeguards remain in force throughout this work.

## Testing requirements

- Successful provisioning for each supported download/verification mechanism.
- Corrupted payloads and mismatched, invalid, or missing verification metadata.
- Installer failure, interrupted downloads, partial state, and safe retries.
- Allowed and refused redirects according to the selected transport policy.
- Failure to verify a transitive download where the policy requires it.
- No publication or reuse of an unsuccessful artifact, and preservation of
  existing machines and known-good artifacts after failure.
- Correct policy diagnostics for generated, edited, and overridden definitions.
- Template updates that preserve user-owned configuration and harness state.

## Acceptance criteria

1. The download inventory and each decision above are recorded with an explicit
   policy for every supported acquisition path and any documented exceptions.
2. Generated templates implement the selected policy for all four harnesses
   and shared tooling without removing existing lifecycle safeguards.
3. Required verification happens before the affected script or payload is
   executed or accepted, according to the documented ecosystem contract.
4. Missing metadata and verification failures follow the selected policy with
   actionable diagnostics and no silent weakening.
5. Failed or interrupted provisioning never publishes a reusable successful
   artifact or mutates an existing working machine.
6. Tests cover successful verification, corruption, missing metadata, partial
   failures, retry behavior, and cache publication boundaries.
7. Documentation states the policy's scope, limitations, and user-definition
   behavior, and explains how to adopt updated templates without data loss.
8. Feature 0004 remains independently implementable and releasable; no integrity
   decision is retroactively made a prerequisite for that migration.

## Implementation outline

1. Inventory acquisition paths and existing ecosystem protections.
2. Resolve and record the policy decisions and their acceptance cases.
3. Update provisioning templates and any necessary build-boundary integration.
4. Add failure injection and integration tests for the selected mechanisms.
5. Document guarantees, exceptions, and explicit template adoption.
