use crate::{
    cli::{Command, Options},
    config::{Config, Profile},
    domain::{self, Category, Identity, Inventory, Machine, PurgePolicy},
    platform::{atomic_write, private_dir, FileLock},
    ports::{self, PortAction},
    smolvm::Smolvm,
};
use anyhow::{bail, Context, Result};
use regex::Regex;
use std::{
    collections::BTreeSet,
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

const REVISION: &str = "5";
const MEMORY: &str = "12288";
const CPUS: &str = "4";

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn pair(values: &mut Vec<OsString>, flag: &str, value: impl Into<OsString>) {
    values.push(flag.into());
    values.push(value.into());
}

fn override_value(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn regular_nonempty(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() || metadata.len() == 0 {
                bail!(
                    "artifact is empty or not a regular file: {}",
                    path.display()
                );
            }
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            Err(error).with_context(|| format!("could not inspect artifact {}", path.display()))
        }
    }
}

fn regular_artifact_candidate(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error)
            .with_context(|| format!("could not inspect artifact candidate {}", path.display())),
    }
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("could not remove {}", path.display())),
    }
}

struct Lifecycle<'a> {
    config: &'a Config,
    smolvm: &'a Smolvm,
    identity: Identity,
    artifact_base: PathBuf,
    artifact: PathBuf,
    builder: String,
    login: String,
    path_hash: String,
    hosts: Vec<String>,
    configuration: &'static str,
}

impl<'a> Lifecycle<'a> {
    fn new(config: &'a Config, smolvm: &'a Smolvm, options: &Options) -> Result<Self> {
        let base_id = base_id(&smolvm.version, &config.definition.bytes);
        let artifact_base = config
            .artifacts_dir
            .join(format!("codex-devtools-{base_id}"));
        let artifact = artifact_base.with_extension("smolmachine");
        let mut hosts: BTreeSet<String> = config.definition.allow_hosts.iter().cloned().collect();
        let configuration = if options.allow_hosts.iter().any(|host| !hosts.contains(host)) {
            "custom"
        } else {
            "default"
        };
        hosts.extend(options.allow_hosts.iter().cloned());
        let hosts: Vec<String> = hosts.into_iter().collect();
        let workspace = config
            .workspace
            .to_str()
            .context("workspace path is not UTF-8")?;
        let home = config
            .harness_home
            .to_str()
            .context("harness home path is not UTF-8")?;
        let mut immutable = vec![
            format!("--from={}", artifact.display()),
            format!("--mem={MEMORY}"),
            format!("--cpus={CPUS}"),
            format!("--volume={home}:{}", config.definition.profile.guest_home()),
            format!("--volume={workspace}:/workspace/myrepo"),
        ];
        immutable.extend(hosts.iter().map(|host| format!("--allow-host={host}")));
        let workspace_hash = domain::hash_set(&immutable);
        let workspace_id = workspace_hash[..12].to_owned();
        let explicit = override_value("CODEX_MACHINE");
        let desired_name = explicit
            .clone()
            .unwrap_or_else(|| format!("codex-workspace-{base_id}-r{REVISION}-{workspace_id}"));
        let builder =
            override_value("BUILDER_MACHINE").unwrap_or_else(|| format!("codex-builder-{base_id}"));
        let login = override_value("LOGIN_MACHINE")
            .unwrap_or_else(|| format!("codex-login-{base_id}-r{REVISION}"));
        Ok(Self {
            config,
            smolvm,
            artifact_base,
            artifact,
            builder,
            login,
            path_hash: domain::hash_bytes(workspace.as_bytes()),
            hosts,
            configuration,
            identity: Identity {
                base_id,
                revision: REVISION.into(),
                workspace_hash,
                workspace_id,
                desired_name,
                override_name: explicit.is_some(),
            },
        })
    }

    fn command(&self, values: Vec<OsString>) -> Result<()> {
        self.smolvm.run(&values).map(|_| ())
    }

    fn machine(&self, operation: &str, name: &str) -> Result<()> {
        let mut values = args(&["machine", operation, "--name", name]);
        if operation == "delete" {
            values.push("--force".into());
        }
        self.command(values)
    }

    fn unchanged_base(&self) -> Result<()> {
        let bytes = fs::read(&self.config.definition.path).with_context(|| {
            format!(
                "could not reread definition {}",
                self.config.definition.path.display()
            )
        })?;
        let output = self.smolvm.capture(&args(&["--version"]))?;
        if !output.status.success() {
            bail!("could not refresh SmolVM version: exit status {}. Use --verbose to display runtime output.", crate::smolvm::exit_code(output.status));
        }
        let version = std::str::from_utf8(&output.stdout)
            .context("SmolVM version is not UTF-8. Use --verbose to display runtime output.")?;
        if base_id(version, &bytes) != self.identity.base_id {
            bail!("the sandbox definition or SmolVM version changed; retry the command");
        }
        Ok(())
    }

    fn locks(&self) -> Result<(FileLock, FileLock)> {
        let purge = FileLock::acquire(&self.config.locks_dir.join("artifacts.lock"))?;
        validate_generation(&self.config.state_root)?;
        self.unchanged_base()?;
        let workspace = FileLock::acquire(
            &self
                .config
                .locks_dir
                .join(format!("workspace-{}.lock", self.path_hash)),
        )?;
        Ok((purge, workspace))
    }

    fn label(values: &mut Vec<OsString>, key: &str, value: &str) {
        pair(values, "--label", format!("codex-sandbox.{key}={value}"));
    }

    fn ensure_base(&self, inventory: &Inventory) -> Result<()> {
        private_dir(&self.config.harness_home)?;
        fs::create_dir_all(&self.config.artifacts_dir)?;
        if regular_nonempty(&self.artifact)? {
            println!("Reusing cached base generation {}.", self.identity.base_id);
            return Ok(());
        }
        println!(
            "Building base generation {} ({}).",
            self.identity.base_id,
            self.smolvm.version.trim()
        );
        if !inventory.iter().any(|machine| machine.name == self.builder) {
            let mut values = args(&["machine", "create", "--name", &self.builder]);
            Self::label(&mut values, "kind", "builder");
            Self::label(&mut values, "base-id", &self.identity.base_id);
            pair(
                &mut values,
                "--smolfile",
                self.config.definition.path.as_os_str(),
            );
            self.command(values)?;
        }
        if let Err(error) = self.machine("stop", &self.builder) {
            eprintln!(
                "WARNING: could not stop builder {}: {error:#}",
                self.builder
            );
        }
        self.machine("start", &self.builder)?;
        self.machine("stop", &self.builder)?;
        let staging = tempfile::Builder::new()
            .prefix(".codex-build-")
            .tempdir_in(&self.config.artifacts_dir)?;
        let temporary_base = staging.path().join("artifact");
        let mut values = args(&["pack", "create", "--from-vm", &self.builder, "-o"]);
        values.push(temporary_base.as_os_str().to_owned());
        self.command(values)?;
        let temporary_artifact = temporary_base.with_extension("smolmachine");
        if !regular_nonempty(&temporary_artifact)? {
            bail!("SmolVM pack produced no artifact; the previous cache was preserved");
        }
        let has_manifest = temporary_base.exists();
        if has_manifest && !regular_nonempty(&temporary_base)? {
            bail!("SmolVM produced an invalid artifact manifest");
        }
        fs::rename(&temporary_artifact, &self.artifact)
            .context("could not publish prepared artifact")?;
        if has_manifest {
            fs::rename(&temporary_base, &self.artifact_base)
                .context("could not publish artifact manifest")?;
        }
        Ok(())
    }

    fn ensure_login(&self, inventory: &Inventory) -> Result<()> {
        if self.config.definition.profile.login().is_none() {
            return Ok(());
        }
        if inventory.iter().any(|machine| machine.name == self.login) {
            return Ok(());
        }
        let mut values = args(&["machine", "create", "--name", &self.login]);
        Self::label(&mut values, "kind", "login");
        Self::label(&mut values, "base-id", &self.identity.base_id);
        Self::label(&mut values, "revision", REVISION);
        pair(&mut values, "--from", self.artifact.as_os_str());
        pair(
            &mut values,
            "--volume",
            format!(
                "{}:{}",
                self.config.harness_home.display(),
                self.config.definition.profile.guest_home()
            ),
        );
        for host in self.config.definition.profile.login_hosts() {
            pair(&mut values, "--allow-host", host);
        }
        self.command(values)
    }

    fn create_workspace(&self) -> Result<()> {
        let mut values = args(&["machine", "create", "--name", &self.identity.desired_name]);
        for (key, value) in [
            ("kind", "workspace"),
            ("base-id", &self.identity.base_id),
            ("revision", REVISION),
            ("workspace-sha256", &self.identity.workspace_hash),
            ("workspace-path-sha256", &self.path_hash),
            ("configuration", self.configuration),
        ] {
            Self::label(&mut values, key, value);
        }
        pair(&mut values, "--from", self.artifact.as_os_str());
        pair(&mut values, "--mem", MEMORY);
        pair(&mut values, "--cpus", CPUS);
        pair(
            &mut values,
            "--volume",
            format!(
                "{}:{}",
                self.config.harness_home.display(),
                self.config.definition.profile.guest_home()
            ),
        );
        pair(
            &mut values,
            "--volume",
            format!("{}:/workspace/myrepo", self.config.workspace.display()),
        );
        for host in &self.hosts {
            pair(&mut values, "--allow-host", host.as_str());
        }
        self.command(values)
    }

    fn caches(&self) -> Result<()> {
        fs::create_dir_all(&self.config.state_root)?;
        let generation = serde_json::json!({"version": 1, "base_id": self.identity.base_id, "revision": REVISION});
        atomic_write(
            &self.config.state_root.join("generation.json"),
            &serde_json::to_vec(&generation)?,
        )?;
        atomic_write(
            &self.config.state_root.join("base-id"),
            format!("{}\n", self.identity.base_id).as_bytes(),
        )?;
        atomic_write(
            &self.config.state_root.join("machine-revision"),
            format!("{REVISION}\n").as_bytes(),
        )?;
        Ok(())
    }

    fn remove_builders(&self) -> Result<()> {
        let inventory = self.smolvm.inventory()?;
        let pattern = Regex::new(r"^codex-builder-([0-9a-f]{12})$")?;
        for machine in inventory {
            let artifact = if machine.name == self.builder {
                self.artifact.clone()
            } else if let Some(found) = pattern.captures(&machine.name) {
                self.config
                    .artifacts_dir
                    .join(format!("codex-devtools-{}.smolmachine", &found[1]))
            } else {
                continue;
            };
            if machine.state.eq_ignore_ascii_case("running") {
                continue;
            }
            if !regular_nonempty(&artifact)? {
                continue;
            }
            if let Err(error) = self.machine("stop", &machine.name) {
                eprintln!(
                    "WARNING: could not stop builder {} before deletion: {error:#}",
                    machine.name
                );
            }
            self.machine("delete", &machine.name)?;
            ports::remove(&self.config.state_root, &machine.name)?;
        }
        Ok(())
    }

    fn reconcile(&self, options: &Options, requested_ports: &[u16]) -> Result<()> {
        let inventory = self.smolvm.inventory()?;
        let classification = domain::classify(&inventory, &self.identity);
        let desired = classification
            .iter()
            .find(|item| item.category == Category::Desired);
        let stale: Vec<_> = classification
            .iter()
            .filter(|item| item.category == Category::Stale)
            .collect();
        let ambiguous: Vec<_> = classification
            .iter()
            .filter(|item| item.category == Category::Ambiguous)
            .collect();
        if options.check {
            println!("Desired workspace machine: {}", self.identity.desired_name);
            println!(
                "Selected: {}",
                desired
                    .map(|item| item.machine.name.as_str())
                    .unwrap_or("none")
            );
            for item in stale.iter().chain(ambiguous.iter()) {
                println!(
                    "  {} ({}, {}): {}",
                    item.machine.name, item.machine.state, item.source, item.reason
                );
            }
            return Ok(());
        }
        for item in stale.iter().chain(ambiguous.iter()) {
            eprintln!(
                "Workspace candidate: {} ({}, {}): {}",
                item.machine.name, item.machine.state, item.source, item.reason
            );
        }
        if !options.replace_stale {
            if desired.is_none() && (!stale.is_empty() || !ambiguous.is_empty()) {
                bail!("no machine matches the current sandbox definition; run smolsandbox repair --check, then repair --replace-stale with the same workspace and additional hosts; no machines were created or deleted");
            }
            if let Some(desired) = desired {
                if options.command == Command::Start {
                    ports::prepare(
                        &self.config.state_root,
                        &desired.machine,
                        &self.identity.workspace_hash,
                        requested_ports,
                        true,
                    )?;
                }
                if !stale.is_empty() || !ambiguous.is_empty() {
                    eprintln!("WARNING: selected the desired machine, but other workspace candidates remain.");
                }
                println!("Recovered workspace machine {}.", desired.machine.name);
            } else {
                self.ensure_base(&inventory)?;
                self.create_workspace()?;
            }
            if !regular_nonempty(&self.artifact)? {
                self.ensure_base(&inventory)?;
            }
            if options.command == Command::Build {
                self.ensure_login(&inventory)?;
            }
        } else {
            if !ambiguous.is_empty() {
                bail!("refusing replacement because workspace ownership is ambiguous");
            }
            for item in &stale {
                if item.machine.state.eq_ignore_ascii_case("running") && !options.force {
                    bail!(
                        "stale machine {} is running; rerun with --force",
                        item.machine.name
                    );
                }
            }
            self.ensure_base(&inventory).context(
                "could not prepare desired base; stale workspace machines were left unchanged",
            )?;
            if desired.is_none() {
                if let Err(error) = self.create_workspace() {
                    match self.smolvm.inventory() {
                        Ok(fresh)
                            if fresh
                                .iter()
                                .any(|machine| machine.name == self.identity.desired_name) =>
                        {
                            eprintln!("WARNING: desired machine {} was registered despite creation failure and was preserved for inspection.", self.identity.desired_name);
                        }
                        Err(refresh) => {
                            eprintln!("WARNING: could not inspect the failed creation: {refresh:#}")
                        }
                        _ => {}
                    }
                    return Err(error).context("could not create desired replacement; stale workspace machines were left unchanged");
                }
            }
            let fresh = self.smolvm.inventory().context(
                "could not refresh inventory; stale workspace machines were left unchanged",
            )?;
            let fresh_classification = domain::classify(&fresh, &self.identity);
            if fresh_classification
                .iter()
                .any(|item| item.category == Category::Ambiguous)
            {
                bail!("refusing cleanup because refreshed workspace ownership is ambiguous");
            }
            self.unchanged_base()?;
            if !regular_nonempty(&self.artifact)? {
                bail!("desired replacement artifact disappeared; stale machines were preserved");
            }
            let replacement = domain::verify_readiness(&fresh, &self.identity)
                .context("desired replacement is not ready; stale machines were preserved")?;
            println!(
                "Verified desired workspace machine {} ({}).",
                replacement.name, replacement.state
            );
            let mut eligible = Vec::new();
            for selected in stale {
                let refreshed = fresh_classification
                    .iter()
                    .find(|item| {
                        item.machine.name == selected.machine.name
                            && item.category == Category::Stale
                    })
                    .with_context(|| {
                        format!(
                            "stale machine {} is no longer eligible for cleanup",
                            selected.machine.name
                        )
                    })?;
                if refreshed.machine.state.eq_ignore_ascii_case("running") && !options.force {
                    bail!(
                        "stale machine {} is now running; rerun with --force",
                        refreshed.machine.name
                    );
                }
                eligible.push(refreshed);
            }
            let count = eligible.len();
            for (removed, item) in eligible.into_iter().enumerate() {
                if item.machine.state.eq_ignore_ascii_case("running") {
                    self.machine("stop", &item.machine.name).with_context(|| format!("could not stop stale workspace machine {}; {removed} deletion(s) completed, {} machine(s) remain unprocessed; compatibility caches were not changed", item.machine.name, count - removed))?;
                }
                self.machine("delete", &item.machine.name).with_context(|| format!("could not delete stale workspace machine {}; {removed} deletion(s) completed, {} machine(s) remain unprocessed; compatibility caches were not changed", item.machine.name, count - removed))?;
                ports::remove(&self.config.state_root, &item.machine.name).with_context(|| format!("stale machine {} was deleted but its local port state could not be removed; {} deletion(s) completed, {} machine(s) remain unprocessed; compatibility caches were not changed", item.machine.name, removed + 1, count - removed - 1))?;
            }
        }
        if let Err(error) = self.remove_builders() {
            eprintln!("WARNING: temporary builder cleanup failed: {error:#}");
        }
        self.caches()
    }

    fn selected(&self) -> Result<Machine> {
        let inventory = self.smolvm.inventory()?;
        let classified = domain::classify(&inventory, &self.identity);
        let machine = classified
            .into_iter()
            .find(|item| {
                item.category == Category::Desired
                    && item.machine.name == self.identity.desired_name
            })
            .context("machine no longer matches the selected managed workspace identity")?
            .machine;
        machine.port_info()?;
        Ok(machine)
    }

    fn reconcile_ports(&self, requested: &[u16]) -> Result<bool> {
        let selected = self.selected()?;
        let action = ports::prepare(
            &self.config.state_root,
            &selected,
            &self.identity.workspace_hash,
            requested,
            false,
        )?;
        match action {
            PortAction::Running => Ok(true),
            PortAction::Skip => Ok(false),
            PortAction::Update(removable) => {
                let mut values = args(&["machine", "update", "--name", &selected.name]);
                for port in removable {
                    pair(&mut values, "--remove-port", format!("{port}:{port}"));
                }
                for port in requested {
                    pair(&mut values, "--port", port.to_string());
                }
                self.command(values)
                    .context("could not reconcile ports; the recovery journal was retained")?;
                let fresh = self.selected()?;
                let (state, count, created_at) = fresh.port_info()?;
                if Some(created_at) != selected.created_at {
                    bail!("machine was replaced during port reconciliation");
                }
                if !matches!(state.as_str(), "created" | "stopped") {
                    bail!("machine changed lifecycle state during port reconciliation");
                }
                if count != requested.len() as u64 {
                    bail!(
                        "machine reports {count} port mappings after reconciliation; expected {}",
                        requested.len()
                    );
                }
                ports::commit(
                    &self.config.state_root,
                    &fresh,
                    &self.identity.workspace_hash,
                    requested,
                )?;
                Ok(false)
            }
        }
    }

    fn seed(&self, machine: &str) -> Result<()> {
        let profile = self.config.definition.profile;
        let seed = profile.seed_path();
        let command = format!(
            "if [ -d {seed} ]; then cp -r --update=none {seed}/. {}/; fi",
            profile.guest_home()
        );
        self.command(args(&[
            "machine", "exec", "--name", machine, "--", "sh", "-lc", &command,
        ]))
    }

    fn guest(
        &self,
        machine: &str,
        command: Vec<OsString>,
        extra: &[OsString],
        secrets: &[&str],
    ) -> Result<i32> {
        let mut values = args(&["machine", "exec", "--name", machine, "-it"]);
        values.push("--".into());
        values.extend(command);
        values.extend_from_slice(extra);
        self.smolvm.interactive(&values, secrets)
    }

    fn clean(&self, workspace_only: bool) -> Result<()> {
        let inventory = self.smolvm.inventory()?;
        let base = &self.identity.base_id;
        let builder = Regex::new(&format!(r"^codex-builder-{base}$"))?;
        let login = Regex::new(&format!(r"^codex-login-{base}-r[0-9]+$"))?;
        let workspace = Regex::new(&format!(r"^codex-workspace-{base}-r[0-9]+-[0-9a-f]{{12}}$"))?;
        let mut selected = Vec::new();
        for machine in inventory {
            let label = |name: &str| {
                machine
                    .labels
                    .get(&format!("codex-sandbox.{name}"))
                    .map(String::as_str)
                    .unwrap_or("")
            };
            let kind = label("kind");
            let label_base = label("base-id");
            let revision = label("revision");
            let hash = label("workspace-sha256");
            let path = label("workspace-path-sha256");
            let configuration = label("configuration");
            let managed = machine
                .labels
                .keys()
                .any(|key| domain::MANAGED_KEYS.contains(&key.as_str()));
            let valid_revision =
                !revision.is_empty() && revision.bytes().all(|byte| byte.is_ascii_digit());
            let valid_hash = is_hex(hash, 64);
            let chosen = if workspace_only {
                if path != self.path_hash {
                    false
                } else {
                    if kind != "workspace"
                        || !is_hex(label_base, 12)
                        || !valid_revision
                        || !valid_hash
                        || !matches!(configuration, "default" | "custom")
                        || machine.name
                            != format!(
                                "codex-workspace-{label_base}-r{revision}-{}",
                                hash.get(..12).unwrap_or("")
                            )
                    {
                        bail!(
                            "refusing to clean {} because its managed labels are contradictory",
                            machine.name
                        );
                    }
                    true
                }
            } else if builder.is_match(&machine.name)
                || login.is_match(&machine.name)
                || (kind == "workspace" && label_base == base && valid_revision && valid_hash)
            {
                true
            } else if workspace.is_match(&machine.name) {
                if managed {
                    bail!(
                        "refusing to clean {} because its managed labels are contradictory",
                        machine.name
                    );
                }
                true
            } else {
                if machine
                    .name
                    .starts_with(&format!("codex-workspace-{base}-r"))
                    && managed
                {
                    bail!(
                        "refusing to clean {} because its managed labels are contradictory",
                        machine.name
                    );
                }
                false
            };
            if chosen {
                selected.push(machine.name);
            }
        }
        let mut removed = 0;
        for machine in &selected {
            println!("Deleting VM: {machine}");
            if let Err(error) = self.machine("stop", machine) {
                eprintln!("WARNING: could not stop machine {machine} before deletion: {error:#}");
            }
            self.machine("delete", machine).with_context(|| format!("cleanup stopped after {removed} deletion(s); later machines and artifacts were preserved"))?;
            removed += 1;
            ports::remove(&self.config.state_root, machine)?;
        }
        if !workspace_only {
            remove_if_exists(&self.artifact_base)?;
            remove_if_exists(&self.artifact)?;
            remove_if_exists(&self.config.state_root.join("base-id"))?;
            remove_if_exists(&self.config.state_root.join("machine-revision"))?;
            remove_if_exists(&self.config.state_root.join("generation.json"))?;
            println!("Removed {removed} VM(s) and the cached artifact.");
        } else {
            println!(
                "Removed {removed} workspace VM(s) for {}.",
                self.config.workspace.display()
            );
        }
        println!(
            "Preserved harness state: {}",
            self.config.harness_home.display()
        );
        Ok(())
    }

    fn purge(&self, policy: PurgePolicy, after_build: bool) -> Result<i32> {
        let _lock = FileLock::acquire(&self.config.locks_dir.join("artifacts.lock"))?;
        validate_generation(&self.config.state_root)?;
        self.unchanged_base()?;
        let inventory = self.smolvm.inventory()?;
        let plan = domain::build_purge_plan(
            &inventory,
            &self.identity.base_id,
            &self.config.artifacts_dir,
            policy,
        )?;
        if policy == PurgePolicy::Conservative && plan.has_alternate {
            eprintln!("WARNING: parallel configurations exist for the current generation; no machines or artifacts were removed. Use purge --force or purge --force-all to select a wider policy.");
            return Ok(if after_build { 0 } else { 3 });
        }
        for machine in &plan.running_machines {
            println!(
                "Preserving running machine and artifact generation {}: {}",
                machine.base_id, machine.name
            );
        }
        self.unchanged_base()?;
        let fresh = self.smolvm.inventory()?;
        let refreshed = domain::build_purge_plan(
            &fresh,
            &self.identity.base_id,
            &self.config.artifacts_dir,
            policy,
        )?;
        if plan != refreshed {
            bail!("managed inventory or artifacts changed before deletion; nothing was removed");
        }
        let mut removed = 0;
        for machine in &plan.selected_machines {
            self.machine("delete", &machine.name).with_context(|| {
                format!(
                    "purge stopped after {removed} machine deletion(s); no artifacts were removed"
                )
            })?;
            removed += 1;
            ports::remove(&self.config.state_root, &machine.name)?;
        }
        let after = self.smolvm.inventory()?;
        let managed = domain::managed_inventory(&after, &self.identity.base_id)?;
        if managed.iter().any(|machine| {
            machine.state == "running" && plan.selected_generations.contains(&machine.base_id)
        }) {
            bail!("running machines now protect selected artifact generations; artifacts were preserved");
        }
        self.unchanged_base()?;
        if !plan.recovery_only && !regular_nonempty(&self.artifact)? {
            bail!("current artifact disappeared; superseded artifacts were preserved");
        }
        let mut removed_artifacts = 0;
        for artifact in &plan.selected_artifacts {
            let path = &artifact.path;
            let parent = path.parent().context("artifact candidate has no parent")?;
            if parent != self.config.artifacts_dir.canonicalize()?
                || artifact.base_id == self.identity.base_id
                || !regular_artifact_candidate(path)?
            {
                bail!(
                    "artifact candidate changed or escaped managed scope: {}",
                    path.display()
                );
            }
            fs::remove_file(path).with_context(|| format!("purge stopped after {removed_artifacts} artifact deletion(s); later artifacts were preserved"))?;
            removed_artifacts += 1;
        }
        println!("Removed {removed} machine(s), {} generation(s), and {removed_artifacts} artifact file(s).", plan.selected_generations.len());
        Ok(0)
    }
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn base_id(version: &str, bytes: &[u8]) -> String {
    let mut input = version.trim_end_matches(['\r', '\n']).as_bytes().to_vec();
    input.push(b'\n');
    input.extend_from_slice(bytes);
    domain::hash_bytes(&input)[..12].to_owned()
}

fn validate_generation(state_root: &Path) -> Result<()> {
    let path = state_root.join("generation.json");
    match fs::read(&path) {
        Ok(bytes) => {
            let state: serde_json::Value = serde_json::from_slice(&bytes).with_context(|| {
                format!("invalid application generation state in {}", path.display())
            })?;
            if state.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
                bail!(
                    "unsupported application state version in {}; state was preserved",
                    path.display()
                );
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("could not inspect application generation state"),
    }
    Ok(())
}

struct StopGuard<'a, 'b> {
    lifecycle: &'a Lifecycle<'b>,
    name: String,
    active: bool,
}

impl StopGuard<'_, '_> {
    fn finish(mut self, result: Result<i32>) -> Result<i32> {
        let cleanup = if self.active {
            self.lifecycle.machine("stop", &self.name)
        } else {
            Ok(())
        };
        self.active = false;
        if let Err(error) = cleanup {
            eprintln!("WARNING: could not stop machine {}: {error:#}", self.name);
            return match result {
                Ok(0) => Ok(1),
                other => other,
            };
        }
        result
    }
}

impl Drop for StopGuard<'_, '_> {
    fn drop(&mut self) {
        if self.active {
            if let Err(error) = self.lifecycle.machine("stop", &self.name) {
                eprintln!(
                    "WARNING: cleanup could not stop machine {}: {error:#}",
                    self.name
                );
            }
        }
    }
}

pub fn run(options: &Options) -> Result<i32> {
    if options.command == Command::List {
        return list(options);
    }
    if options.command == Command::CreateConfig {
        let harness = options
            .harness
            .as_deref()
            .context("create-config requires --harness")?;
        crate::config::create_config(harness, options.file.as_deref())?;
        return Ok(0);
    }
    let config = Config::resolve(options)?;
    validate_generation(&config.state_root)?;
    println!(
        "Selected definition: {} ({})",
        config.definition.path.display(),
        config.definition.profile.display_name()
    );
    if options.command == Command::Login && config.definition.profile.login().is_none() {
        bail!(
            "{} has no separate login workflow. {}",
            config.definition.profile.display_name(),
            config.definition.profile.authentication_hint()
        );
    }
    let smolvm = Smolvm::discover(options.smolvm.as_deref(), options.verbose)?;
    let lifecycle = Lifecycle::new(&config, &smolvm, options)?;
    if options.command == Command::Repair && options.check {
        lifecycle.reconcile(options, &[])?;
        return Ok(0);
    }
    smolvm.preflight()?;
    match options.command {
        Command::Build | Command::Repair | Command::Start => {
            let requested: BTreeSet<u16> = options
                .ports
                .iter()
                .map(|port| port.parse::<u16>())
                .collect::<std::result::Result<_, _>>()?;
            if requested.len() > 64 {
                bail!("at most 64 distinct ports may be requested");
            }
            let requested: Vec<u16> = requested.into_iter().collect();
            let locks = lifecycle.locks()?;
            private_dir(&config.harness_home)?;
            lifecycle.reconcile(options, &requested)?;
            if options.command == Command::Start {
                let running = lifecycle.reconcile_ports(&requested)?;
                let guard = StopGuard {
                    lifecycle: &lifecycle,
                    name: lifecycle.identity.desired_name.clone(),
                    active: !running,
                };
                if !running {
                    lifecycle.machine("start", &lifecycle.identity.desired_name)?;
                }
                drop(locks);
                if let Err(error) = lifecycle.seed(&lifecycle.identity.desired_name) {
                    eprintln!("WARNING: could not seed persistent harness state; continuing with existing contents: {error:#}");
                }
                let secrets: &[&str] = if config.definition.profile == Profile::Codex
                    && env::var_os("OPENAI_API_KEY").is_some_and(|key| !key.is_empty())
                {
                    &["OPENAI_API_KEY"]
                } else {
                    &[]
                };
                let result = lifecycle.guest(
                    &lifecycle.identity.desired_name,
                    config.definition.profile.launch(),
                    &options.harness_args,
                    secrets,
                );
                return guard.finish(result);
            }
            drop(locks);
            if options.command == Command::Build {
                if let Err(error) = lifecycle.purge(PurgePolicy::Conservative, true) {
                    eprintln!("WARNING: build completed; conservative purge failed: {error:#}");
                }
                println!(
                    "Ready: {}\nNext: smolsandbox login\n      smolsandbox start {}",
                    lifecycle.artifact.display(),
                    config.workspace.display()
                );
            } else {
                println!(
                    "Workspace machine ready: {}",
                    lifecycle.identity.desired_name
                );
            }
            Ok(0)
        }
        Command::Login => {
            let lock = FileLock::acquire(&config.locks_dir.join("artifacts.lock"))?;
            validate_generation(&config.state_root)?;
            lifecycle.unchanged_base()?;
            let inventory = smolvm.inventory()?;
            let machine = inventory
                .iter()
                .find(|machine| machine.name == lifecycle.login)
                .with_context(|| {
                    format!(
                        "login machine {} does not exist; run smolsandbox build first",
                        lifecycle.login
                    )
                })?;
            if machine
                .labels
                .keys()
                .any(|key| domain::MANAGED_KEYS.contains(&key.as_str()))
                && (machine.label(0) != "login"
                    || machine.label(1) != lifecycle.identity.base_id
                    || machine.label(2) != REVISION)
            {
                bail!(
                    "login machine {} has contradictory managed labels; no machine was started",
                    machine.name
                );
            }
            private_dir(&config.harness_home)?;
            let guard = StopGuard {
                lifecycle: &lifecycle,
                name: lifecycle.login.clone(),
                active: true,
            };
            lifecycle.machine("start", &lifecycle.login)?;
            drop(lock);
            if let Err(error) = lifecycle.seed(&lifecycle.login) {
                eprintln!("WARNING: could not seed persistent harness state: {error:#}");
            }
            let result = lifecycle.guest(
                &lifecycle.login,
                config
                    .definition
                    .profile
                    .login()
                    .context("profile has no login workflow")?,
                &[],
                &[],
            );
            guard.finish(result)
        }
        Command::Clean => {
            let _lock = FileLock::acquire(&config.locks_dir.join("artifacts.lock"))?;
            validate_generation(&config.state_root)?;
            lifecycle.unchanged_base()?;
            lifecycle.clean(options.workspace_only)?;
            Ok(0)
        }
        Command::Purge => lifecycle.purge(
            if options.force_all {
                PurgePolicy::ForceAll
            } else if options.force {
                PurgePolicy::Force
            } else {
                PurgePolicy::Conservative
            },
            false,
        ),
        Command::CreateConfig | Command::List => unreachable!(),
    }
}

fn list(options: &Options) -> Result<i32> {
    let (workspace, config) = Config::resolve_for_listing(options)?;
    let smolvm = Smolvm::discover(options.smolvm.as_deref(), options.verbose)?;
    let mut reference = crate::listing::Reference {
        definition: None,
        base_id: None,
        revision: REVISION.into(),
        workspace_sha256: None,
        desired_machine: None,
    };
    if let Some(config) = &config {
        let lifecycle = Lifecycle::new(config, &smolvm, options)?;
        reference.definition = Some(config.definition.path.clone());
        reference.base_id = Some(lifecycle.identity.base_id);
        reference.workspace_sha256 = Some(lifecycle.identity.workspace_hash);
        reference.desired_machine = Some(lifecycle.identity.desired_name);
    }
    let inventory = smolvm.inventory()?;
    crate::listing::report(&inventory, &workspace, reference, options.all).print(options.json)?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_bytes_and_version_define_base_identity() {
        assert_eq!(
            base_id("smolvm 1\n", b"definition"),
            base_id("smolvm 1", b"definition")
        );
        assert_ne!(
            base_id("smolvm 1", b"definition"),
            base_id("smolvm 1", b"edited definition")
        );
        assert_ne!(
            base_id("smolvm 1", b"definition"),
            base_id("smolvm 2", b"definition")
        );
    }

    #[test]
    fn cache_reuse_rejects_empty_artifacts() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("artifact");
        assert!(!regular_nonempty(&path).unwrap());
        fs::write(&path, b"").unwrap();
        assert!(regular_nonempty(&path).is_err());
        fs::write(&path, b"artifact").unwrap();
        assert!(regular_nonempty(&path).unwrap());
    }

    #[test]
    fn generation_validation_preserves_unknown_versions_and_corruption() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("generation.json");
        for bytes in [
            br#"{"version":2,"future_field":"preserve"}"#.as_slice(),
            br#"{"version":"1"}"#.as_slice(),
            b"{incomplete".as_slice(),
            b"[]".as_slice(),
        ] {
            fs::write(&path, bytes).unwrap();
            assert!(validate_generation(directory.path()).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        }
        fs::write(
            &path,
            br#"{"version":1,"base_id":"abcdef123456","revision":"5"}"#,
        )
        .unwrap();
        validate_generation(directory.path()).unwrap();
    }

    #[test]
    fn generation_validation_is_read_only_when_state_is_absent() {
        let directory = tempfile::tempdir().unwrap();
        let state_root = directory.path().join("absent-state");
        validate_generation(&state_root).unwrap();
        assert!(!state_root.exists());
    }

    #[test]
    fn superseded_empty_artifact_is_eligible_but_not_reusable() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .join("codex-devtools-abcdef123456.smolmachine");
        fs::write(&path, b"").unwrap();
        assert!(regular_artifact_candidate(&path).unwrap());
        assert!(regular_nonempty(&path).is_err());
        assert!(!regular_artifact_candidate(directory.path()).unwrap());
        assert!(!regular_artifact_candidate(&directory.path().join("missing")).unwrap());
    }
}
