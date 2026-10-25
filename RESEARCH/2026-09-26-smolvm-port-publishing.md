# SmolVM port-publishing lifecycle and sandbox identity

Historical research: this record describes the 2026-09-26 shell workflow.
Use the [operating reference](../RUST-APPLICATION.md) for current Rust commands.
Local source links point to the retained shell baseline in `src-old/`.

Date: 2026-09-26

Decision scope: how a repeatable published-port option in `start-codex.sh` should map to SmolVM, and whether requested ports belong in this repository's workspace identity hash.

## Conclusion

SmolVM supports repeatable `-p` / `--port` options when a machine is created. It also supports adding and removing mappings on an existing **stopped** machine with `smolvm machine update`; those changes are stored in the machine record and used on the next `machine start`. A different port set therefore does **not** require a new SmolVM machine. The supported transition is:

```bash
smolvm machine stop --name "$machine"
smolvm machine update --name "$machine" \
  --remove-port 3000 \
  --port 8080
smolvm machine start --name "$machine"
```

This conclusion is based on the first-party SmolVM v1.19.2 sources and documentation at commit [`4daf5e91`](https://github.com/smol-machines/smolvm/commit/4daf5e91a1af0ef83b831d9e96215638d013201d).

For this repository, the recommended design is to **exclude requested ports from `WORKSPACE_SHA256`** and reuse the existing persistent workspace machine. Ports are mutable launch configuration in SmolVM, while this repository defines the hash as the canonical set of immutable machine-creation options. Adding ports to the hash would make every distinct port set select another machine and writable overlay even though SmolVM can update the original machine safely while it is stopped.

That recommendation has one important implementation requirement: each invocation must reconcile the machine's persisted port mappings to the exact requested set before starting it. `machine update --port` only adds mappings and `--remove-port` removes named mappings; omitting ports on a later invocation does not clear earlier mappings automatically. Treating the CLI values as additive would leave old host listeners enabled on later starts.

## Supported syntax and limits

The create command declares `-p` / `--port` as a vector and describes it as repeatable. Its accepted value is `PORT[-END]` or `HOST[-END]:GUEST[-END]` ([create CLI definition](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/cli/machine.rs#L3563-L3565)). The official Smolfile documentation gives the same three forms ([Smolfile port documentation](https://github.com/smol-machines/smolvm/blob/v1.19.2/docs/smolfile.md#L24-L29)):

- `8080` maps host TCP 8080 to guest TCP 8080.
- `8080:80` maps host TCP 8080 to guest TCP 80.
- `5173-5180:5173-5180` expands to equal-length one-to-one mappings.

Examples for more than one mapping are therefore:

```bash
smolvm machine create --name sandbox \
  --port 3000 \
  --port 8080:80

smolvm machine update --name sandbox \
  --port 3000 \
  --port 8080:80
```

SmolVM's implementation is TCP-specific. The forwarding module identifies the option as `-p HOST:GUEST`, accepts host TCP connections, and relays them to guest TCP ([TCP listener design](https://github.com/smol-machines/smolvm/blob/v1.19.2/crates/smolvm-network/src/tcp_listeners.rs#L1-L31)). UDP publishing is not represented by this option.

The request cannot literally support an unlimited number of ports. SmolVM caps a machine at **64 concrete mappings**, including entries produced by expanding ranges ([mapping limit and range expansion](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/data/network.rs#L54-L59), [parser and cap enforcement](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/data/network.rs#L137-L177)). Port zero is rejected, the range endpoints must be valid `u16` values, and each host port may be mapped only once ([port validation](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/data/network.rs#L181-L217), [duplicate-host check](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/data/network.rs#L244-L255)). The feature specification should expose the repeatable option while documenting and testing the upstream 64-mapping ceiling.

By default, SmolVM binds published TCP ports on host loopback (`127.0.0.1`, with an IPv6 loopback listener attempted as well). `SMOLVM_PUBLISH_ADDR` can widen the bind address, but this repository does not currently set it ([binding behavior](https://github.com/smol-machines/smolvm/blob/v1.19.2/crates/smolvm-network/src/tcp_listeners.rs#L97-L134)). A host-port collision makes the machine start fail when SmolVM binds the listener; the same source deliberately binds before listener threads start so launch can fail cleanly ([binding behavior](https://github.com/smol-machines/smolvm/blob/v1.19.2/crates/smolvm-network/src/tcp_listeners.rs#L97-L105)).

## Port mappings are mutable machine configuration

The first-party update command is explicit about lifecycle: it modifies a stopped machine, writes changes to the database record, and applies them on the next start ([`UpdateCmd` contract](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/cli/machine.rs#L5241-L5250)). It accepts repeatable `--port` values and repeatable `--remove-port` values in the same syntax ([update port arguments](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/cli/machine.rs#L5271-L5277)). The command rejects states other than `stopped` or `created` ([state check](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/cli/machine.rs#L5340-L5360)) and applies additions and removals to the persisted `ports` list in a database transaction ([record mutation](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/cli/machine.rs#L5467-L5513)).

On start, SmolVM loads the named machine's record, converts its stored ports to mappings, checks conflicts, and starts with those mappings ([start reads stored mappings](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/cli/vm_common.rs#L1573-L1607), [mapping load](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/cli/vm_common.rs#L1711-L1714)). `machine start` itself has no `--port` argument ([`StartCmd` arguments](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/cli/machine.rs#L4455-L4508)). Consequently:

1. Port changes cannot be applied live to a running workspace machine.
2. They cannot be supplied as a one-start override.
3. They can be applied to the same persistent machine through stop, update, and start; recreation is unnecessary.

SmolVM's documented persistence model says machine filesystem changes survive stop/start cycles ([persistence model](https://github.com/smol-machines/smolvm/blob/v1.19.2/AGENTS.md#L81-L89)). Reconfiguring mappings on the stopped machine therefore preserves its writable overlay.

## Implications for `WORKSPACE_SHA256`

Today [`sandbox_prepare_workspace_options`](../src-old/sandbox-common.sh) hashes the artifact, memory, CPUs, two volume mounts, and the complete allow-host set. [`sandbox_create_workspace_machine`](../src-old/sandbox-common.sh) passes that same configuration to `smolvm machine create`. The repository documentation describes these as the immutable options that determine machine identity and says changing one selects a different persistent machine ([current identity contract](../src-old/README.md#run-codex)).

SmolVM's lifecycle means published ports do not have to join that identity:

- **Exclude ports from the hash:** the existing machine name and overlay are reused; reconcile the requested mappings on the stopped record before `machine start`. This matches invocation-specific port publishing and is the recommended behavior.
- **Include ports in the hash:** each canonical port set selects a separate machine and overlay. This is valid, but it is a repository product choice rather than a SmolVM requirement. It would also make harmless option-order differences create identities unless the inputs are sorted and deduplicated before hashing.

If the recommended reusable-machine design is selected, the creation and reuse paths must converge on the same exact port set. A newly created machine may receive the canonical mappings through `machine create --port`, or all selected machines may go through a common `machine update` step while still in the `created`/`stopped` state. Existing machines require a delta consisting of every obsolete mapping as `--remove-port` and every missing mapping as `--port`.

There is an upstream observability constraint to account for in the spec: v1.19.2's `machine ls --json` / `machine status --json` output reports only the **count** of stored mappings, not the mapping values ([JSON construction](https://github.com/smol-machines/smolvm/blob/v1.19.2/src/cli/vm_common.rs#L3045-L3068)). A reliable exact-set reconciliation therefore needs an explicit source of the previously applied mappings, a cleanup protocol that restores the record to a known empty set after stopping, or a SmolVM interface that exposes the concrete mappings. Merely issuing the newly requested `--port` options is insufficient because the update command is additive.

## Specification consequences

The feature specification should require all of the following:

1. Parse a repeatable sandbox option before the `--` Codex-argument delimiter and validate it before reconciliation.
2. Define whether the user-facing value is a same-number `PORT` only or exposes SmolVM's full `HOST:GUEST` and range syntax. Pass the selected semantics through without inventing UDP support.
3. Canonicalize exact duplicates and ordering for predictable commands; reject conflicting mappings of one host port.
4. Enforce or clearly surface SmolVM's maximum of 64 expanded mappings.
5. Hold the workspace lifecycle lock while stopping, updating, and starting so another launcher cannot observe or modify the intermediate stopped configuration.
6. Reconcile an exact set, including removal when a later invocation requests fewer ports or none.
7. Preserve `WORKSPACE_SHA256` and the selected machine name when only the requested port set changes.
8. Surface update failures and host-port bind conflicts without starting Codex under a different port policy.

The decisive upstream fact is that ports are persisted but mutable configuration. Reusing and restarting the existing machine is supported; choosing a new machine per port set is unnecessary unless the project deliberately wants separate writable overlays for separate exposure policies.
