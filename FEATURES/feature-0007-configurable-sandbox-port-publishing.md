# Feature 0007: Configurable sandbox port publishing

## Status

Implemented

## Summary

`start-codex.sh` and its `codex-sandbox.sh` compatibility entry point shall
accept a repeatable `--port PORT` option. Each value publishes the same TCP
port number from the workspace machine on the host loopback interface. A user
may request multiple ports by repeating the option, subject to SmolVM's limit
of 64 concrete mappings per machine.

Published ports are mutable stopped-machine settings in SmolVM. They shall not
be included in `WORKSPACE_SHA256`, the workspace ID, the deterministic machine
name, or the managed identity labels. Changing only the requested ports shall
reuse the existing workspace machine and its writable overlay.

Before starting a stopped workspace machine, the launcher shall reconcile its
stored SmolVM port mappings to the exact set requested by the current
invocation with `smolvm machine update`. Because SmolVM's JSON status exposes
only the number of mappings, the launcher shall keep a small atomic manifest
of the mappings it owns and a recovery journal for interrupted updates. The
next `smolvm machine start` shall activate the reconciled set. An invocation
with no `--port` options requests an empty set and therefore removes mappings
left by an earlier invocation.

## Motivation

Web applications, development servers, debuggers, notebooks, and other tools
started inside the workspace machine often need to be reached from the host.
The current launcher creates and starts the machine without any SmolVM port
mappings, so those services remain inaccessible across the VM boundary.

SmolVM accepts port mappings on `machine create`, but its current documented
lifecycle also supports adding and removing mappings from a stopped persistent
machine with `machine update`. Changes take effect on the next `machine start`.
Creating a separate persistent machine for every temporary set of development
ports would duplicate writable overlays and make installed tools and other
guest state appear to disappear when a user changes ports.

This feature treats port publishing as a launch-time profile for one persistent
workspace machine. Immutable inputs such as the artifact, mounts, resource
allocation, and egress allowlist continue to determine machine identity. The
requested inbound ports determine how that selected machine is configured for
its next start.

The supporting SmolVM research is recorded in
[`RESEARCH/2026-09-26-smolvm-port-publishing.md`](../RESEARCH/2026-09-26-smolvm-port-publishing.md).

## Goals

- Publish a guest TCP port on the same numbered host port through a launcher
  option.
- Accept as many distinct requested ports as SmolVM supports by allowing the
  option to be repeated.
- Make option order and exact duplicates irrelevant.
- Reuse the same persistent workspace machine when only the requested ports
  change.
- Preserve the machine's writable overlay across port changes.
- Make each invocation's requested set authoritative, including the empty set.
- Apply port changes only while the selected machine is stopped and before it
  starts.
- Keep port reconciliation inside the existing mutation locks.
- Recover safely when the launcher is interrupted around a port update.
- Fail without starting Codex when validation, inspection, update, or port
  binding fails.
- Document that published ports are host-local TCP listeners by default.

## Non-goals

- Publishing UDP ports.
- Mapping different host and guest port numbers.
- Accepting port ranges in one option value.
- Adding the short `-p` spelling.
- Selecting a host bind address or changing SmolVM's default loopback binding.
- Discovering ports automatically from guest processes, Docker Compose,
  Testcontainers, project files, or framework configuration.
- Keeping requested ports active while the workspace machine is stopped.
- Reserving a host port before SmolVM starts the machine.
- Applying a changed port set to a running machine.
- Automatically adopting mappings whose values are unknown because the local
  ownership manifest is missing or invalid.
- Falling back to machine recreation when the installed SmolVM lacks mutable
  port support.
- Adding port options to login, build, repair, clean, or purge commands.
- Treating a port set as part of the immutable workspace machine identity.

## SmolVM behavior and design decision

SmolVM's `--port`/`-p` option represents a host-to-guest TCP mapping. A single
number maps the host and guest port with the same value. SmolVM stores mappings
in the persistent machine record and uses them when the machine starts.

SmolVM documents `machine update` as the operation for changing ports on an
existing stopped machine. The command supports repeatable `--port` additions
and `--remove-port` removals, applies both to the stored record, and makes the
result effective on the next start. It rejects updates while the machine is
running. SmolVM also preserves the persistent filesystem across stop/start
cycles.

SmolVM's public `machine ls --json` and `machine status --json` output exposes
the number of stored mappings, but not their host and guest port values.
Therefore exact replacement cannot be implemented from JSON inventory alone.
The launcher's last-applied manifest supplies the mapping values needed by
`--remove-port`; the public mapping count provides a post-update consistency
check.

Therefore this feature makes the following decisions:

1. Requested ports do not influence `WORKSPACE_SHA256` or machine selection.
2. Changing only ports does not create or select another workspace machine.
3. A stopped existing machine is updated and then restarted with the requested
   set.
4. A running machine whose stored set differs is left running and startup
   fails with an actionable error; the launcher shall not interrupt another
   session to change ports.
5. A newly created machine follows the same update-before-start path as an
   existing stopped machine.

These decisions refine Feature 0005's phrase "complete set of immutable
`smolvm machine create` options." Although SmolVM also accepts ports during
creation, ports are not immutable because SmolVM provides a supported update
operation for them.

## SmolVM compatibility

The upstream behavior in this specification is verified against SmolVM
v1.19.2. This feature requires a compatible SmolVM that provides:

- repeatable `machine update --port` and `--remove-port` options;
- transactional updates for `created` and `stopped` machines;
- persisted mappings used by the next `machine start`;
- a numeric mapping count in machine JSON; and
- the 64-mapping validation contract.

Startup shall perform a read-only version or capability check before inventory
mutation. An incompatible SmolVM shall produce an actionable error and shall
not trigger machine creation, replacement, or a hash-based fallback. The
existing base ID may continue to include `smolvm --version`; this compatibility
check does not add requested ports to workspace identity.

## User interface

The canonical launcher accepts separated and equals forms:

```bash
./start-codex.sh --port 3000 /path/to/project
./start-codex.sh --port=3000 /path/to/project
```

The option is repeatable and may appear before or after the workspace:

```bash
./start-codex.sh \
  --port 3000 \
  /path/to/project \
  --port 5173 \
  --port 9229 \
  -- --full-auto
```

`codex-sandbox.sh` accepts and forwards the same forms. The first standalone
`--` ends sandbox option parsing. Every following argument belongs to Codex,
so this invocation does not publish a port:

```bash
./start-codex.sh -- --port 3000
```

The usage synopsis becomes:

```text
start-codex.sh [--help] [--allow-host HOST]... [--port PORT]... [WORKSPACE] [-- CODEX_ARG...]
```

The compatibility entry point uses the same synopsis under its own basename.
Help shall state that `--port` is repeatable, publishes TCP on the same host and
guest port, uses host loopback by default, and applies only to the current and
subsequent lifetime of the selected machine until another launch reconciles a
different set.

## Validation and canonicalization

Each `PORT` shall be an ASCII decimal integer from 1 through 65535 inclusive.
Signs, surrounding whitespace, separators, hostnames, IP addresses,
host-to-guest mappings, ranges, and protocol suffixes are invalid. A missing or
invalid value is a usage error and exits with status `2` before inventory,
locks, or lifecycle mutation.

The launcher shall convert accepted values to canonical base-10 integers,
deduplicate them, and sort them numerically. The canonical requested set is
used for comparison, diagnostics, and SmolVM update arguments. These requests
are equivalent:

```bash
./start-codex.sh --port 5173 --port 3000 /path/to/project
./start-codex.sh --port=3000 --port 05173 --port 3000 /path/to/project
```

SmolVM currently permits at most 64 concrete mappings per machine. More than
64 distinct requested ports is a usage error detected before lifecycle
mutation. Exact duplicates do not count toward the limit.

## Effective port set

The effective set is exactly the canonical set supplied to the current
`start-codex.sh` or `codex-sandbox.sh` invocation. There are no default ports
and no union with the machine's previous mappings.

This rule prevents ports from remaining published accidentally. For example:

```text
first launch:  --port 3000 --port 5173  -> stored set {3000, 5173}
second launch: --port 8000              -> stored set {8000}
third launch:  no --port options        -> stored set {}
```

Each port is translated to the SmolVM mapping `PORT:PORT`. The implementation
may use the single-number SmolVM spelling when adding it, but inventory and
comparison shall treat the result as the pair `(PORT, PORT)`.

## Machine identity

The canonical immutable workspace-machine specification defined by Feature
0005 remains:

```text
--from=<base-artifact-path>
--mem=<workspace-memory-in-MiB>
--cpus=<workspace-cpu-count>
--volume=<codex-home-host-path>:/root/.codex
--volume=<canonical-workspace-path>:/workspace/myrepo
--allow-host=<effective-host>  # one entry per host
```

No `--port` value is added to this list. Consequently, changing requested
ports leaves all of the following unchanged:

- `WORKSPACE_SHA256`;
- `WORKSPACE_ID`;
- `DESIRED_CODEX_MACHINE`;
- `codex-sandbox.workspace-sha256`;
- configuration classification; and
- stale-machine classification.

Port mappings also shall not be encoded in a new managed identity label. They
are mutable state represented by SmolVM's public mapping count and the
launcher's concrete ownership manifest.

## Port state and inventory contract

SmolVM's JSON machine inventory reports a mapping count rather than the
concrete host-to-guest pairs. The launcher shall validate the selected
machine's `ports` value as an integer from 0 through 64. A missing, negative,
out-of-range, or non-integer value is malformed inventory and shall fail
startup without lifecycle mutation.

The launcher shall keep its concrete mapping state below:

```text
$STATE_ROOT/ports/<machine-name>.json
$STATE_ROOT/ports/<machine-name>.pending.json
```

The committed manifest shall contain a format version, machine name, SmolVM
creation timestamp, complete workspace hash, and sorted unique list of
same-number ports last applied successfully by the launcher. The pending
journal shall bind to the same machine incarnation and contain both the known
removable set and the intended target set. Exact field names are an
implementation detail, but all values shall be validated before use.

The creation timestamp distinguishes a new machine that reused a deterministic
name from a deleted earlier incarnation. Neither file is ownership evidence:
the existing managed labels and inventory classification must select the
machine before either file may influence an update.

Both files shall be written with a temporary file and atomic rename while the
existing mutation locks are held. They shall not be sourced as shell, and
malformed content shall never become command arguments.

The launcher establishes or validates state as follows:

1. A newly created machine has a zero mapping count and may establish an empty
   committed manifest.
2. An existing selected machine with no matching manifest may establish an
   empty manifest only when SmolVM reports zero mappings.
3. Unless a valid matching pending journal contains the removable candidates,
   a missing, malformed, or wrong-incarnation manifest combined with a
   positive SmolVM mapping count is unrecoverable automatically because the
   public JSON does not reveal the mappings required by `--remove-port`.
   Startup shall fail without changing the machine and direct the user to
   inspect `smolvm machine ls --verbose`, remove the displayed mappings from
   the stopped machine, and retry.
4. Without a pending journal, the reported count shall equal the committed
   manifest length before reconciliation. A mismatch is unowned drift and
   shall fail without an update.
5. A matching pending journal means an earlier reconciliation may have been
   interrupted before or after SmolVM's transactional update. The launcher may
   recover it using the union of the committed ports and both journal sets.

Port state shall be removed after the corresponding machine is successfully
deleted. A stale manifest found for a new zero-port incarnation may be replaced
with an empty manifest; a stale manifest never authorizes modifying a
positive-count incarnation.

## Reconciliation and launch behavior

After immutable machine reconciliation selects or creates the desired
workspace machine, and before `smolvm machine start`, the launcher shall:

1. Read and validate the machine's lifecycle state and SmolVM mapping count.
2. Validate or bootstrap the committed manifest and recover any matching
   pending journal according to the preceding section.
3. Form the known removable set as the union of the committed ports, any
   pending journal's removable and target ports, and the current requested
   ports.
4. If the machine is `created` or `stopped`, atomically write a pending journal
   containing that removable set and the current requested target.
5. Invoke one `smolvm machine update --name NAME` command containing one
   `--remove-port PORT:PORT` argument for every port in the removable set and
   one `--port PORT` argument for every currently requested port. Supplying a
   mapping in both groups intentionally resets it in SmolVM's single database
   transaction. The update may be skipped only when both sets are empty and
   the reported count is zero.
6. Re-read inventory and require the reported mapping count to equal the
   number of requested ports. Because every known candidate was removed and
   every requested mapping was added, a larger count exposes an unknown extra
   mapping; a conflicting unknown mapping causes the update itself to fail.
7. Atomically replace the committed manifest with the requested set and then
   remove the pending journal. An interruption leaves enough journal state for
   the next invocation to remove either the old or new set safely.
8. Start the same named machine with the existing `smolvm machine start`
   command.
9. Continue with Codex-home seeding and guest Codex execution only after start
   succeeds.

Creation shall not bake the requested ports into
`sandbox_create_workspace_machine`. A new machine begins with no mappings and
uses the common update path. This keeps creation deterministic from the hashed
immutable specification and gives new and reused machines the same behavior.

Port inspection, update, and verification are mutating-startup work. They
shall run while the launcher still holds the existing purge and workspace
serialization guarantees. Another launcher shall not be able to change the
same machine's mappings between verification and start.

If the selected machine is already running and the stored mappings differ,
startup shall fail without stopping, updating, deleting, or replacing the
machine. The diagnostic shall identify the selected machine, its current
mapping count, the manifest and requested ports, and instruct the user to stop
the active session before retrying. A running machine may proceed without an
update only when it has a valid matching committed manifest, no pending
journal, a count equal to that manifest, and the manifest equals the requested
set. This feature otherwise adds no new behavior to the repository's handling
of an already-running selected machine.

`repair-sandbox.sh` does not accept `--port`. Repair continues to identify and
replace machines from immutable policy only. A newly repaired machine has no
published ports until the next launcher invocation applies its requested set.

## Failure behavior

- Invalid launcher input fails as usage before any SmolVM inventory or
  mutation call.
- Inventory failure or malformed selected-machine port data authorizes no
  update or start.
- Missing, malformed, wrong-incarnation, or count-inconsistent local port
  state authorizes no update when the machine has mappings that cannot be
  identified safely.
- `machine update` failure leaves the machine stopped, emits the SmolVM error,
  retains the pending journal for recovery, and prevents `machine start` and
  guest execution.
- Post-update verification failure prevents `machine start` and guest
  execution.
- A host port already in use causes `machine start` to fail. Codex shall not be
  executed, and the normal cleanup path shall attempt to leave the selected
  machine stopped.
- No failure in port handling authorizes machine deletion, stale replacement,
  or creation of a second machine solely for another port set.

## Network exposure

SmolVM publishes TCP mappings on IPv4 and, where available, IPv6 loopback by
default. This feature shall not set a wider bind address. Documentation shall
describe host access with loopback URLs such as `http://127.0.0.1:3000` and
shall not imply that a service is reachable from other hosts.

SmolVM can be configured outside this repository to widen its publish address.
That external override remains SmolVM behavior and is not a supported launcher
option in this feature.

The guest service must listen on the requested guest port. Publishing a port
does not start a service, alter its bind address, wait for readiness, or add an
application-level authentication layer.

## Relationship to other features

Feature 0001's ownership, inventory validation, locking, normal-start safety,
and explicit repair rules remain in force. Port reconciliation extends the
selected-machine startup path and does not change candidate ownership.

Feature 0002's create-before-delete replacement protocol remains in force.
Port update failures never authorize stale cleanup.

Feature 0005 remains authoritative for immutable identity and egress
allowlists. This feature clarifies that a SmolVM setting with a supported
stopped-machine update operation is mutable and is excluded from the immutable
hash. `--allow-host` remains immutable because `machine update` does not
provide corresponding egress-policy update options.

Feature 0006's help contract applies to the new option. The synopsis and
option descriptions for `start-codex.sh` and `codex-sandbox.sh` shall be
updated; every other entry point's help remains unchanged.

If Feature 0004's Rust migration proceeds, its `smolsandbox start` command
shall expose the same repeatable `--port PORT` behavior and preserve the
mutable update-before-start semantics. This feature supersedes Feature 0004's
current `start` synopsis to that extent.

## Documentation requirements

The README shall include:

- one example publishing a single port;
- one example publishing multiple ports;
- the valid range and 64-mapping limit;
- the same host/guest port rule;
- the exact-set behavior, including that omitting all options clears earlier
  mappings on the next launch;
- reuse of the same persistent machine across port changes;
- loopback-only access by default; and
- a note that the guest application must listen on the published port.

## Safety invariants

- Port values are validated before inventory access or mutation.
- Port changes never influence ownership or select a different machine.
- The exact requested set is verified before the machine starts.
- Local port state is bound to one positively identified machine incarnation
  and never substitutes for managed inventory labels.
- A pending journal is durable before SmolVM port mutation begins.
- A port update is attempted only for a positively selected managed workspace
  machine in `created` or `stopped` state.
- A running machine is never stopped solely to apply another port set.
- Update or verification failure never proceeds to machine start or Codex
  execution.
- Port reconciliation never deletes a machine or writable overlay.
- No published port is retained merely because an earlier invocation requested
  it.
- The launcher does not widen SmolVM's default loopback listener address.

## Acceptance criteria

1. `start-codex.sh` and `codex-sandbox.sh` accept repeatable `--port PORT` and
   `--port=PORT` options before or after `WORKSPACE` and before `--`.
2. `--port` after the pass-through delimiter is forwarded to Codex unchanged.
3. Missing, empty, signed, whitespace-containing, non-decimal, zero, and
   greater-than-65535 values fail with exit status `2` before lifecycle work.
4. Up to 64 distinct values are accepted; a 65th distinct value fails before
   lifecycle work.
5. Numerically equivalent values, duplicates, and different option orders
   produce one sorted canonical set.
6. Requested ports do not change the workspace hash, workspace ID, desired
   machine name, or managed identity labels.
7. Changing only the requested ports reuses the same machine and retains a
   file previously written to its persistent filesystem.
8. A fresh machine is created without baked port mappings, updated to the
   requested set, verified, and then started.
9. A stopped existing machine with committed `{3000, 5173}` requested as
   `{5173, 8000}` receives removals for the full known set, additions for the
   full requested set, and starts with exactly `{5173, 8000}`.
10. Launching without `--port` against a stopped machine with stored mappings
    removes every mapping before start.
11. A stopped nonempty set is reset transactionally even when its committed
    manifest already equals the requested set; an empty set with a zero count
    requires no update.
12. A running selected machine with a valid matching manifest and count may
    proceed without an update.
13. A running selected machine with a different, unknown, pending, or
    count-inconsistent set is not stopped or updated, and the launcher fails
    with an actionable diagnostic.
14. Malformed mapping-count data in SmolVM JSON causes no update, start,
    create, stop, delete, or guest execution.
15. A selected machine with a positive mapping count and no valid matching
    manifest fails without mutation and reports the manual recovery procedure.
16. Update failure and post-update verification failure both prevent machine
    start and guest execution.
17. An interruption before or after SmolVM's transactional update leaves a
    journal from which the next launch can remove both possible prior sets and
    establish the new requested set.
18. A start-time host-port collision prevents guest Codex execution and leaves
    the machine stopped after cleanup where SmolVM permits it.
19. Help for both launchers documents the option, repetition, delimiter,
    same-number TCP mapping, loopback default, and empty-set behavior.
20. Repair commands continue to address the same immutable machine regardless
    of the last or next requested port set.
21. Successful machine deletion removes its committed and pending port state;
    a new zero-port incarnation can safely replace stale local state.
22. Tests assert that port reconciliation and machine start remain serialized
    against concurrent mutating launcher operations.
23. An incompatible SmolVM fails a read-only capability check before lifecycle
    mutation and does not cause a second machine to be selected.
24. Every existing `tests/*-test.sh` script passes after the new focused tests
    are added.

## Implementation outline

1. Add a requested-port array and strict parser to `start-codex.sh`; update the
   early help scan, usage text, and `codex-sandbox.sh` compatibility help.
2. Canonicalize requested ports as sorted unique integers and enforce the
   SmolVM mapping limit before reconciliation.
3. Add a read-only SmolVM compatibility check for the required update and JSON
   capabilities.
4. Extend inventory normalization and validation to retain each machine's
   port count and creation timestamp.
5. Keep ports out of `sandbox_prepare_workspace_options` and
   `WORKSPACE_SHA256`.
6. Keep ports out of `sandbox_create_workspace_machine` so every machine uses
   the same post-selection reconciliation path.
7. Add validated, versioned, atomically written committed manifests and
   pending journals below `$STATE_ROOT/ports`, bound to machine incarnation.
8. Add a shared function that writes the journal, resets every known mapping
   with one stopped-machine `smolvm machine update`, checks the resulting
   public count, commits the manifest, and clears the journal.
9. Call that function for normal startup after machine selection or creation
   and before `smolvm machine start`, without releasing the applicable
   serialization locks.
10. Remove local port state only after the corresponding machine deletion is
   confirmed.
11. Extend the SmolVM test fixture to keep concrete internal mappings while
   reporting only their public count, and to simulate update, interruption,
   recovery, drift, running-state, malformed-inventory, and port-collision
   failures.
12. Add focused launcher and reconciliation tests, then update help assertions
   and README examples.
13. Run every `tests/*-test.sh` script.
