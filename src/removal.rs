use crate::{
    cli::Options,
    config::application_home,
    domain::{self, Inventory, Machine, MANAGED_KEYS},
    lifecycle::REVISION,
    platform::{canonical_workspace, FileLock},
    ports,
    smolvm::Smolvm,
};
use anyhow::{bail, ensure, Context, Result};
use regex::Regex;
use serde_json::Value;
use std::{collections::BTreeMap, env, ffi::OsString, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Ownership {
    kind: String,
    base: String,
    revision: String,
    path_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Delete,
    StopAndDelete,
    SkipRunning,
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::StopAndDelete => "stop-and-delete",
            Self::SkipRunning => "skip-running",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    machine: Machine,
    owner: Ownership,
    action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Plan {
    relevant: BTreeMap<String, Machine>,
    targets: Vec<Target>,
    protected: usize,
    builders: usize,
    unassociated: usize,
}

struct ContextSelection {
    workspace: Option<PathBuf>,
    path_hash: Option<String>,
    state_root: PathBuf,
    lock: PathBuf,
}
impl ContextSelection {
    fn resolve(options: &Options) -> Result<Self> {
        ensure!(
            !options.global || options.workspace.is_none(),
            "WORKSPACE cannot be used with --global"
        );
        let cwd = env::current_dir().context("cannot read invocation directory")?;
        let workspace = if options.global {
            None
        } else {
            Some(canonical_workspace(
                options.workspace.as_deref().unwrap_or(&cwd),
            )?)
        };
        let path_hash = workspace.as_ref().map(|path| {
            domain::hash_bytes(path.to_str().expect("canonical UTF-8 workspace").as_bytes())
        });
        let home = application_home()?;
        let state_root = match env::var_os("STATE_ROOT").filter(|value| !value.is_empty()) {
            Some(value) => {
                let path = PathBuf::from(value);
                if path.is_absolute() {
                    path
                } else {
                    cwd.join(path)
                }
            }
            None => home.join("state"),
        };
        Ok(Self {
            workspace,
            path_hash,
            state_root,
            lock: home.join("locks/artifacts.lock"),
        })
    }
}

struct Names {
    builder: Regex,
    login: Regex,
    workspace: Regex,
}
impl Names {
    fn new() -> Self {
        Self {
            builder: Regex::new(r"^codex-builder-([0-9a-f]{12})$").unwrap(),
            login: Regex::new(r"^codex-login-([0-9a-f]{12})-r([0-9]+)$").unwrap(),
            workspace: Regex::new(r"^codex-workspace-([0-9a-f]{12})-r([0-9]+)-([0-9a-f]{12})$")
                .unwrap(),
        }
    }

    fn legacy(&self, name: &str) -> Option<Ownership> {
        if let Some(c) = self.builder.captures(name) {
            Some(Ownership {
                kind: "builder".into(),
                base: c[1].into(),
                revision: String::new(),
                path_hash: String::new(),
            })
        } else if let Some(c) = self.login.captures(name) {
            Some(Ownership {
                kind: "login".into(),
                base: c[1].into(),
                revision: c[2].into(),
                path_hash: String::new(),
            })
        } else {
            self.workspace.captures(name).map(|c| Ownership {
                kind: "workspace".into(),
                base: c[1].into(),
                revision: c[2].into(),
                path_hash: String::new(),
            })
        }
    }

    fn ownership(&self, machine: &Machine) -> Result<Option<Ownership>> {
        let legacy = self.legacy(&machine.name);
        if !machine
            .labels
            .keys()
            .any(|key| key.starts_with("codex-sandbox."))
        {
            return Ok(legacy);
        }
        let kind = machine.label(0);
        ensure!(
            hex(machine.label(1), 12),
            "{}: managed base ID is missing or malformed",
            machine.name
        );
        let no_workspace_fields = !MANAGED_KEYS[3..]
            .iter()
            .any(|key| machine.labels.contains_key(*key));
        match kind {
            "workspace" => {
                ensure!(
                    decimal(machine.label(2))
                        && hex(machine.label(3), 64)
                        && hex(machine.label(4), 64)
                        && ["default", "custom"].contains(&machine.label(5)),
                    "{}: workspace labels are incomplete or contradictory",
                    machine.name
                );
            }
            "login" => ensure!(
                decimal(machine.label(2)) && no_workspace_fields,
                "{}: login labels are incomplete or contradictory",
                machine.name
            ),
            "builder" => ensure!(
                !machine.labels.contains_key(MANAGED_KEYS[2]) && no_workspace_fields,
                "{}: builder labels are incomplete or contradictory",
                machine.name
            ),
            _ => bail!("{}: managed kind is missing or unsupported", machine.name),
        }
        let owner = Ownership {
            kind: kind.into(),
            base: machine.label(1).into(),
            revision: machine.label(2).into(),
            path_hash: machine.label(4).into(),
        };
        if let Some(legacy) = legacy {
            ensure!(
                owner.kind == legacy.kind
                    && owner.base == legacy.base
                    && owner.revision == legacy.revision,
                "{}: deterministic name contradicts managed labels",
                machine.name
            );
            if let Some(c) = self.workspace.captures(&machine.name) {
                ensure!(
                    machine.label(3).starts_with(&c[3]),
                    "{}: deterministic name contradicts workspace configuration hash",
                    machine.name
                );
            }
        }
        Ok(Some(owner))
    }
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn decimal(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}
fn older(revision: &str, current: &str) -> bool {
    let revision = revision.trim_start_matches('0');
    let current = current.trim_start_matches('0');
    (revision.len(), revision) < (current.len(), current)
}
fn plan(inventory: &Inventory, options: &Options, path_hash: Option<&str>) -> Result<Plan> {
    let names = Names::new();
    let mut plan = Plan {
        relevant: BTreeMap::new(),
        targets: Vec::new(),
        protected: 0,
        builders: 0,
        unassociated: 0,
    };
    for machine in inventory {
        let workspace_candidate =
            machine.label(0) == "workspace" || names.workspace.is_match(&machine.name);
        let valid_path = hex(machine.label(4), 64);
        if workspace_candidate && !valid_path {
            plan.unassociated += 1;
        }
        if let Some(path_hash) = path_hash {
            if !valid_path || machine.label(4) != path_hash {
                continue;
            }
        }
        let Some(owner) = names.ownership(machine)? else {
            continue;
        };
        if path_hash.is_some() {
            ensure!(
                owner.kind == "workspace",
                "{}: matching workspace path has contradictory managed kind",
                machine.name
            );
        }
        plan.relevant.insert(machine.name.clone(), machine.clone());
        if !options.all {
            if owner.kind == "builder" {
                plan.builders += 1;
                continue;
            }
            if !older(&owner.revision, REVISION) {
                plan.protected += 1;
                continue;
            }
        }
        ensure!(
            ports::valid_name(&machine.name),
            "machine name {:?} cannot be used for local port state",
            machine.name
        );
        ensure!(
            machine.created_at.is_some(),
            "{}: invalid or missing creation timestamp",
            machine.name
        );
        let state = machine.normalized_state().with_context(|| {
            format!(
                "{}: selected machine has unsupported state {:?}",
                machine.name, machine.state
            )
        })?;
        let action = match state.as_str() {
            "running" if options.force => Action::StopAndDelete,
            "running" => Action::SkipRunning,
            _ => Action::Delete,
        };
        plan.targets.push(Target {
            machine: machine.clone(),
            owner,
            action,
        });
    }
    plan.targets
        .sort_by(|a, b| a.machine.name.cmp(&b.machine.name));
    Ok(plan)
}

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn inventory(runtime: &Smolvm) -> Result<Inventory> {
    let output = runtime.capture(&args(&["machine", "ls", "--json"]))?;
    ensure!(output.status.success(), "SmolVM inventory failed; no further mutation is authorized. Use --verbose to display runtime output.");
    let machines = domain::parse_inventory(&output.stdout)
        .context("invalid SmolVM inventory; no further mutation is authorized")?;
    let value: Value = serde_json::from_slice(&output.stdout)?;
    for row in value.as_array().expect("validated inventory array") {
        for field in ["ports", "created_at"] {
            if let Some(value) = row.get(field).filter(|value| !value.is_null()) {
                ensure!(
                    value.as_u64().is_some(),
                    "machine {} {field} must be an unsigned integer",
                    row["name"]
                );
            }
        }
    }
    Ok(machines)
}

fn capabilities(runtime: &Smolvm, plan: &Plan) -> Result<()> {
    let operations = if plan
        .targets
        .iter()
        .any(|target| target.action == Action::StopAndDelete)
    {
        vec![
            ("delete", vec!["--name", "--force"]),
            ("stop", vec!["--name"]),
        ]
    } else {
        vec![("delete", vec!["--name", "--force"])]
    };
    for (operation, required) in operations {
        let output = runtime.capture(&args(&["machine", operation, "--help"]))?;
        ensure!(output.status.success(), "SmolVM machine {operation} capability inspection failed. Use --verbose to display runtime output.");
        let mut help =
            String::from_utf8(output.stdout).context("SmolVM capability output is not UTF-8")?;
        help.push_str(std::str::from_utf8(&output.stderr)?);
        let flags: Vec<&str> = help
            .split_whitespace()
            .map(|token| {
                token
                    .trim_matches([',', '[', ']'])
                    .split('=')
                    .next()
                    .unwrap_or("")
            })
            .collect();
        for flag in required {
            ensure!(flags.contains(&flag), "incompatible SmolVM: machine {operation} must support {flag}. Use --verbose to display runtime output.");
        }
    }
    Ok(())
}

fn print_plan(plan: &Plan, context: &ContextSelection, options: &Options) {
    match &context.workspace {
        Some(workspace) => println!("Removal scope: canonical workspace {}", workspace.display()),
        None => println!("Removal scope: global (selected runtime and user registry)"),
    }
    println!(
        "Current policy revision: {REVISION}; mode: {}; running policy: {}; dry-run: {}",
        if options.all {
            "all-revisions"
        } else {
            "stale-only"
        },
        if options.force {
            "stop-and-delete"
        } else {
            "skip-running (use --force to include)"
        },
        options.dry_run
    );
    for target in &plan.targets {
        let association = if target.owner.kind != "workspace" {
            "shared"
        } else if target.owner.path_hash.is_empty() {
            "unknown"
        } else {
            &target.owner.path_hash
        };
        println!(
            "Planned {}: {} kind={} base={} revision={} state={} association={} reason={}",
            target.action.name(),
            target.machine.name,
            target.owner.kind,
            target.owner.base,
            if target.owner.revision.is_empty() {
                "none"
            } else {
                &target.owner.revision
            },
            target.machine.state,
            association,
            if options.all {
                "all revisions requested"
            } else {
                "older policy revision"
            }
        );
    }
    if plan
        .targets
        .iter()
        .all(|target| target.action == Action::SkipRunning)
    {
        println!("No eligible machines to remove under the running-machine policy.");
    }
    println!("Runtime queries and deletion are not an atomic transaction; external SmolVM clients do not honor application locks.");
}

fn totals(plan: &Plan, removed: usize) {
    let skipped = plan
        .targets
        .iter()
        .filter(|target| target.action == Action::SkipRunning)
        .count();
    println!("Totals: removed={removed}; planned removals={}; skipped running={skipped}; protected current/newer={}; protected revisionless builders={}; unassociated workspace candidates={} (association unavailable)", plan.targets.len() - skipped, plan.protected, plan.builders, plan.unassociated);
    if skipped > 0 {
        println!("Use --force to include the {skipped} skipped running machine(s).");
    }
}

fn verify_snapshot(
    runtime: &Smolvm,
    expected: &Plan,
    options: &Options,
    path_hash: Option<&str>,
    skipped: &[String],
) -> Result<Inventory> {
    let current = inventory(runtime)?;
    let mut actual = plan(&current, options, path_hash)?;
    let mut expected = expected.clone();
    for name in skipped {
        if let (Some(actual), Some(expected)) = (
            actual.relevant.get_mut(name),
            expected.relevant.get_mut(name),
        ) {
            actual.state = expected.state.clone();
        }
    }
    ensure!(
        actual.relevant == expected.relevant,
        "relevant removal inventory changed after planning; no further mutation is authorized"
    );
    Ok(current)
}

pub fn run(options: &Options) -> Result<i32> {
    let context = ContextSelection::resolve(options)?;
    let runtime = Smolvm::discover(options.smolvm.as_deref(), options.verbose)?;
    let mut planned = plan(&inventory(&runtime)?, options, context.path_hash.as_deref())?;
    print_plan(&planned, &context, options);
    if options.dry_run
        || planned
            .targets
            .iter()
            .all(|target| target.action == Action::SkipRunning)
    {
        totals(&planned, 0);
        return Ok(0);
    }
    let targets = planned.targets.clone();
    let skipped: Vec<_> = targets
        .iter()
        .filter(|target| target.action == Action::SkipRunning)
        .map(|target| target.machine.name.clone())
        .collect();
    capabilities(&runtime, &planned)?;
    let _lock = FileLock::acquire(&context.lock)?;
    verify_snapshot(&runtime, &planned, options, context.path_hash.as_deref(), &skipped)
        .context("removal plan changed while acquiring the application mutation lock; no machines were changed")?;
    let mut removed = Vec::new();
    let mut stopped = Vec::new();
    let mut delete_operations = Vec::new();
    let mut stop_operations = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        if target.action == Action::SkipRunning {
            continue;
        }
        let result = (|| -> Result<()> {
            verify_snapshot(
                &runtime,
                &planned,
                options,
                context.path_hash.as_deref(),
                &skipped,
            )?;
            if target.action == Action::StopAndDelete {
                runtime.run(&args(&["machine", "stop", "--name", &target.machine.name]))?;
                stop_operations.push(target.machine.name.clone());
                let after = inventory(&runtime).with_context(|| format!("{}: stop operation succeeded but stop verification failed; deletion is not authorized", target.machine.name))?;
                let machine = after
                    .iter()
                    .find(|machine| machine.name == target.machine.name)
                    .context("machine disappeared after successful stop")?;
                let mut expected = target.machine.clone();
                expected.state = machine.state.clone();
                ensure!(*machine == expected && machine.normalized_state().as_deref() == Some("stopped"), "{}: stop verification failed: ownership, incarnation, state, or reported ports changed", target.machine.name);
                stopped.push(target.machine.name.clone());
                planned
                    .relevant
                    .insert(machine.name.clone(), machine.clone());
                verify_snapshot(
                    &runtime,
                    &planned,
                    options,
                    context.path_hash.as_deref(),
                    &skipped,
                )?;
                println!("Stopped: {}", target.machine.name);
            }
            runtime.run(&args(&[
                "machine",
                "delete",
                "--name",
                &target.machine.name,
                "--force",
            ]))?;
            delete_operations.push(target.machine.name.clone());
            let after = inventory(&runtime).with_context(|| format!("{}: delete operation succeeded but absence verification failed; local port state was preserved", target.machine.name))?;
            ensure!(
                !after
                    .iter()
                    .any(|machine| machine.name == target.machine.name),
                "{}: name remains present after deletion; local port state was preserved",
                target.machine.name
            );
            removed.push(target.machine.name.clone());
            println!("Removed: {}", target.machine.name);
            ports::remove(&context.state_root, &target.machine.name).with_context(|| {
                format!(
                    "{} was deleted but local port-state removal failed",
                    target.machine.name
                )
            })?;
            planned.relevant.remove(&target.machine.name);
            Ok(())
        })();
        if let Err(error) = result {
            let unprocessed: Vec<_> = targets[index + 1..]
                .iter()
                .filter(|target| target.action != Action::SkipRunning)
                .map(|target| target.machine.name.as_str())
                .collect();
            eprintln!("Removal partial results: removed={} [{}]; successful delete operations={} [{}]; stopped={} [{}]; successful stop operations={} [{}]; failed target={}; unprocessed={} [{}]. Completed operations were not rolled back.", removed.len(), removed.join(", "), delete_operations.len(), delete_operations.join(", "), stopped.len(), stopped.join(", "), stop_operations.len(), stop_operations.join(", "), target.machine.name, unprocessed.len(), unprocessed.join(", "));
            return Err(error)
                .with_context(|| format!("removal failed for {}", target.machine.name));
        }
    }
    totals(&planned, removed.len());
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_revisions_compare_without_overflow_and_normalize_zeroes() {
        assert!(older("0004", "5"));
        assert!(!older("0005", "5"));
        assert!(!older("0006", "5"));
        assert!(!older(&"9".repeat(200), "5"));
        assert!(older("0", "5"));
        assert!(!older("000", "0"));
    }
}
