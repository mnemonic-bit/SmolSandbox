# Feature 0010: Finish the Rust documentation cutover

## Status

Proposed

## Summary

Make the Rust `smolsandbox` application the documented user and contributor
interface when Feature 0004's cutover gates pass. Replace active shell-based
instructions with Rust commands, document the application home and definition
selection rules, and update repository maintenance guidance to reference the
Rust implementation and independent tests.

Documentation may be prepared before cutover. Until the required validation is
recorded and baseline removal is included in the cutover changeset, retain an
accurate pre-cutover status. Documentation changes alone do not establish a
supported release or complete Feature 0004.

## Problem

`README.md` still teaches the shell entry points, requires host Python for
lifecycle operations, and describes repository-local `.smolvm/` state as the
current layout. `RUST-APPLICATION.md` contains the Rust operating instructions
but also records the retained baseline and outstanding cutover gates.

`AGENTS.md` directs maintainers to synchronize allowlists in
`devtools-base.smolfile` and `sandbox-common.sh`, update shell assertions, and
run shell test scripts. Those instructions will point at retired files after
cutover. Active guidance needs to describe the Rust templates, identity rules,
and replacement regression tests.

## Goals

- Provide a complete Rust first-use path in the README.
- Make active operating and maintenance instructions match the final file tree.
- Explain configuration ownership, state locations, and shell-era state handling.
- Preserve lifecycle safety guidance and distinguish the four harness profiles.
- Keep historical specifications and validation evidence identifiable as history.
- Record cutover status from observed validation results.

## Non-goals

- Changing command behavior, configuration discovery, or lifecycle policy.
- Implementing the regression transfer in
  [Feature 0009](feature-0009-transfer-shell-regressions-to-independent-rust-tests.md).
- Waiving native execution, real guest, authentication, or terminal gates.
- Adding shell wrappers, compatibility aliases, or automatic state migration.
- Adding installers, release signing, or download-integrity policy.
- Rewriting every historical document to remove references to old interfaces.

## Scope and dependencies

Update `README.md`, `RUST-APPLICATION.md`, and `AGENTS.md`. Review `CONTEXT.md`,
active contributor instructions, CI references, and the Feature 0004 status and
validation record for conflicting commands, paths, or completion claims.

Use Feature 0004 as the behavior contract and the current executable help as
the command syntax reference. Use Feature 0009's coverage map and final Rust
test symbols when naming replacement maintenance checks. Apply `AGENT-LANG.md`
to authored prose. Apply the installed `writing-for-agents` skill when editing
`AGENTS.md` during implementation.

The final documentation cutover depends on:

1. Feature 0009's independent regression suite and recorded results.
2. Passing native mocked runs on the release targets in
   `.github/workflows/rust.yml`, including native Windows credential permissions.
3. Passing real SmolVM smoke tests for Claude, Codex, OpenCode, and Hermes Agent
   on every release target required by Feature 0004, including authentication,
   terminal behavior, lifecycle cleanup, and executable relocation.
4. Removal of the old production entry points, host lifecycle helpers, and
   shell test fixtures in the same Feature 0004 cutover changeset or an already
   completed cutover changeset.

Physical baseline removal belongs to Feature 0004. This feature owns the
documentation that accompanies removal. If a prerequisite is `not_run`,
`failed`, or `inconclusive`, keep the affected cutover claim pending and name
the missing evidence.

## User documentation

### README and operating reference

Make the README the entry point for the Rust application. Include a first-use
sequence for configuration generation, build, login, and start. Identify
SmolVM as the separately installed runtime prerequisite. Distinguish building
the application, which needs a Rust toolchain, from running a copied executable.

Document `create-config --harness <claude|codex|opencode|hermes>` and the commands
`build`, `start`, `login`, `repair`, `clean`, and `purge`. Include examples for
`--smolfile`, `--smolvm`, repeated `--allow-host`, repeated `--port`, workspace
selection, and argument pass-through after `--`. Distinguish application help
from guest harness help.

Describe supported host/architecture combinations and native virtualization
prerequisites consistently with the tested release matrix. Do not describe
cross-compilation as proof of native execution. Explain that the copied
executable needs no checkout, adjacent templates, host Bash/Python, host harness
installation, or Rust toolchain at runtime. Guest provisioning can still use
shell commands and Python.

Keep `RUST-APPLICATION.md` as the detailed operating and validation reference,
or consolidate its content into an explicitly linked replacement. Choose one
authoritative location for detailed rules and link to that location from
shorter guidance. Avoid conflicting duplicate instructions.

### Configuration and retained state

Document the current application home, `~/.smolsandbox`, including
`default.smlbx`, artifacts, generation caches, port manifests and journals,
locks, and the separate harness homes. Explain the guest workspace mount at
`/workspace/myrepo` and the profile-specific guest state mounts.

Describe whole-file selection without merging: `--smolfile` takes precedence
over `BASE_SMOLFILE`; bare names search the selected workspace's repository
`.smolsandbox` directory before the native user's directory; explicit paths
resolve according to Feature 0004. Invalid nearer files fail rather than
selecting another definition. The executable directory is not a search path.

Explain offline, atomic, no-clobber configuration generation. Updating the
executable does not refresh user definitions. Template adoption requires an
explicit user action and preserves existing edits.

State that Rust starts with fresh application state and neither imports nor
removes shell-era `artifacts/` or `.smolvm/` contents. Describe the old paths
only as legacy state. Do not prescribe automatic migration or destructive
cleanup. Document supported environment overrides and distinguish them from
retired internal shell controls.

### Lifecycle and failure behavior

Preserve guidance for non-destructive reconciliation, stale-only refusal,
read-only repair inspection, and create-before-delete replacement. Explain
that forced replacement may stop eligible running stale machines only after
the desired machine passes readiness verification.

Explain exact-set TCP port publishing, loopback binding, limits, omission
clearing old mappings, and refusal to change a running machine's port set.
Distinguish mutable ports from immutable identity inputs and additional hosts.

Explain generation clean, workspace-only clean, and each purge policy.
Identify preserved definitions, harness state, unrelated resources, running
machines, and protected artifacts. State that port state is removed only
after successful machine deletion. Preserve guest exit-status and cleanup
failure precedence, secret forwarding by reference, and profile-specific
authentication guidance.

## Contributor and agent documentation

Replace `AGENTS.md` instructions that reference retired files with instructions
that name the final Rust templates, policy implementation, and independent
regression tests. Preserve the requirement to use the codebase-memory skill
for structural exploration and to read `AGENT-LANG.md` before authoring prose.

Document the actual sources of network policy: generated definitions come
from `templates/*.smlbx`, while lifecycle operations use the complete selected
user-owned definition. Definition-byte changes select a new base generation;
additional immutable creation options participate in workspace identity.
Explain when changes to Rust runtime creation policy require a runtime revision
increment, using the final implementation's constant and regression checks.
Do not retain the obsolete rule to synchronize two retired allowlists.

Replace shell test commands with the independent Rust formatting, lint,
default-feature test, `test-adapter` test, and release-build commands. Name
the checks introduced by Feature 0009 for template policy, revision, and
lifecycle changes. Explain that distributed builds exclude `test-adapter`.

## Historical documents and status

Retain earlier feature specifications and research as historical records.
Old command names may remain in explicitly historical sections or a command
mapping for returning users. Do not present the mapping as available wrappers.
Correct active cross-references that would otherwise point at deleted files.

Preserve previous validation observations, their dates, and their source
revisions. Add new cutover evidence instead of relabeling historical results.
For each new result, record the command or smoke procedure, revision, native
target, profile where applicable, and `passed`, `failed`, `not_run`, or
`inconclusive`. Include CI run references when CI provides the evidence.

Update Feature 0004's status only when all of its acceptance criteria are met.
Remove pre-cutover warnings only after the warnings no longer describe the
repository and release state. Keep Feature 0008 independent of migration
completion.

## Acceptance criteria

- **AC-1:** The README provides a complete Rust first-use sequence and links to
  the authoritative operating reference. Every example matches executable help.
- **AC-2:** Active documentation accurately describes supported hosts, runtime
  prerequisites, copied-executable operation, configuration selection, all four
  profiles, environment overrides, and the application-home layout.
- **AC-3:** Active instructions use Rust lifecycle commands and current state
  paths. Remaining shell names and shell-era paths are explicitly historical;
  no active instruction depends on a removed file.
- **AC-4:** Lifecycle guidance retains the documented identity, replacement,
  ports, cleanup, purge, credentials, and failure guarantees without introducing
  automatic state migration or unsupported compatibility commands.
- **AC-5:** `AGENTS.md` names the current network-policy sources, runtime
  revision rule, and independent Rust checks. Existing exploration and agent
  language requirements remain present.
- **AC-6:** Validation instructions run without the retired baseline and
  distinguish ordinary release builds from `test-adapter` builds.
- **AC-7:** Local document links resolve, examples agree with command help,
  and a reference audit finds no active dependency on a retired entry point,
  helper, fixture, or definition. Historical exceptions are listed in the
  implementation's validation record.
- **AC-8:** Cutover completion claims cite passing prerequisite evidence and
  match actual baseline removal. Feature 0004 remains incomplete while any
  required validation or removal step is outstanding.

## Validation and implementation outline

1. Inventory active shell commands, paths, maintenance rules, and status claims.
2. Prepare the Rust README and operating-reference updates with links to the
   authoritative rules.
3. Rewrite agent maintenance guidance against the final Rust tests and policy.
4. Check help and version examples without configuration or SmolVM. Check
   lifecycle examples against the native mocked suite and recorded real guest
   procedures; do not execute destructive examples against user state.
5. Audit Markdown links and retired-file references. Record historical
   exceptions and review the final file tree against active instructions.
6. Confirm the Feature 0004 cutover evidence and baseline removal.
7. Publish the final documentation state with the cutover changeset and record
   validation results. Leave completion claims pending if a gate is unmet.
