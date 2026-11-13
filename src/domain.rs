use anyhow::{bail, ensure, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const MANAGED_KEYS: [&str; 6] = [
    "codex-sandbox.kind",
    "codex-sandbox.base-id",
    "codex-sandbox.revision",
    "codex-sandbox.workspace-sha256",
    "codex-sandbox.workspace-path-sha256",
    "codex-sandbox.configuration",
];
pub fn hash_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}
pub fn hash_set(values: &[String]) -> String {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    hash_bytes(&serde_json::to_vec(&values).expect("string arrays serialize"))
}
fn matches(pattern: &str, value: &str) -> bool {
    Regex::new(pattern).unwrap().is_match(value)
}
fn base_valid(value: &str) -> bool {
    matches(r"^[0-9a-f]{12}$", value)
}
fn hash_valid(value: &str) -> bool {
    matches(r"^[0-9a-f]{64}$", value)
}
fn revision_valid(value: &str) -> bool {
    matches(r"^[0-9]+$", value)
}
fn clean(value: Option<&Value>, field: &str) -> Result<String> {
    let text = match value {
        None | Some(Value::Null) => "",
        Some(Value::String(text)) => text,
        _ => bail!("machine {field} must be a string"),
    };
    ensure!(
        !text.contains(['\n', '\r', '\u{1f}']),
        "machine {field} contains an unsupported control character"
    );
    Ok(text.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Machine {
    pub name: String,
    pub state: String,
    pub labels: BTreeMap<String, String>,
    pub ports: Option<u64>,
    pub created_at: Option<u64>,
}
pub type Inventory = Vec<Machine>;
impl Machine {
    pub fn label(&self, index: usize) -> &str {
        self.labels
            .get(MANAGED_KEYS[index])
            .map(String::as_str)
            .unwrap_or("")
    }
    pub fn running(&self) -> bool {
        self.state.eq_ignore_ascii_case("running")
    }
    pub fn normalized_state(&self) -> Option<String> {
        let state = self.state.to_ascii_lowercase();
        ["running", "stopped", "created"]
            .contains(&state.as_str())
            .then_some(state)
    }
    pub fn port_info(&self) -> Result<(String, u64, u64)> {
        let state = self.normalized_state().with_context(|| {
            format!("machine {:?} has an unsupported lifecycle state", self.name)
        })?;
        let count = self
            .ports
            .filter(|count| *count <= 64)
            .with_context(|| format!("machine {:?} has an invalid ports count", self.name))?;
        let created = self.created_at.with_context(|| {
            format!("machine {:?} has an invalid creation timestamp", self.name)
        })?;
        Ok((state, count, created))
    }
}
pub fn parse_inventory(bytes: &[u8]) -> Result<Inventory> {
    let value: Value = serde_json::from_slice(bytes).context("invalid inventory JSON")?;
    let entries = value
        .as_array()
        .context("inventory root must be a JSON array")?;
    let mut names = BTreeSet::new();
    let mut result = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let item = entry
            .as_object()
            .with_context(|| format!("inventory entry {index} must be an object"))?;
        let name = clean(item.get("name"), "name")?;
        ensure!(!name.is_empty(), "inventory entry {index} has no name");
        ensure!(
            names.insert(name.clone()),
            "inventory contains duplicate machine name {name:?}"
        );
        let mut state = match item.get("state") {
            None => "unknown".into(),
            value => clean(value, "state")?,
        };
        if state.is_empty() {
            state = "unknown".into();
        }
        let mut labels = BTreeMap::new();
        match item.get("labels") {
            None | Some(Value::Null) => {}
            Some(Value::Object(map)) => {
                for (key, value) in map {
                    let key = clean(Some(&Value::String(key.clone())), "label key")?;
                    labels.insert(key, clean(Some(value), "label value")?);
                }
            }
            _ => bail!("machine {name:?} labels must be an object"),
        }
        result.push(Machine {
            name,
            state,
            labels,
            ports: item.get("ports").and_then(Value::as_u64),
            created_at: item.get("created_at").and_then(Value::as_u64),
        });
    }
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub base_id: String,
    pub revision: String,
    pub workspace_hash: String,
    pub workspace_id: String,
    pub desired_name: String,
    pub override_name: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Desired,
    Stale,
    Ambiguous,
    Unrelated,
}
#[derive(Debug, Clone)]
pub struct Classified {
    pub machine: Machine,
    pub category: Category,
    pub source: String,
    pub reason: String,
}
pub fn classify(inventory: &Inventory, identity: &Identity) -> Vec<Classified> {
    let legacy = Regex::new(&format!(
        r"^codex-workspace-([0-9a-f]{{12}})-r([0-9]+)-{}$",
        regex::escape(&identity.workspace_id)
    ))
    .unwrap();
    inventory
        .iter()
        .map(|machine| {
            let has_labels = MANAGED_KEYS
                .iter()
                .any(|key| machine.labels.contains_key(*key));
            let owned =
                machine.label(0) == "workspace" && machine.label(3) == identity.workspace_hash;
            let legacy_match = legacy.is_match(&machine.name);
            let mut source = if has_labels { "labels" } else { "legacy-name" };
            let (category, reason) =
                if identity.override_name && machine.name == identity.desired_name {
                    source = "override";
                    (Category::Desired, "")
                } else if owned {
                    if !base_valid(machine.label(1)) || !revision_valid(machine.label(2)) {
                        (
                            Category::Ambiguous,
                            "managed workspace labels are incomplete or malformed",
                        )
                    } else if identity.override_name {
                        (Category::Stale, "")
                    } else if machine.name == identity.desired_name
                        && machine.label(1) == identity.base_id
                        && machine.label(2) == identity.revision
                    {
                        (Category::Desired, "")
                    } else if machine.label(1) != identity.base_id
                        || machine.label(2) != identity.revision
                    {
                        (Category::Stale, "")
                    } else {
                        (
                            Category::Ambiguous,
                            "managed identity matches but the deterministic name does not",
                        )
                    }
                } else if has_labels {
                    if machine.name == identity.desired_name
                        || legacy_match
                        || machine.label(3) == identity.workspace_hash
                    {
                        (
                            Category::Ambiguous,
                            "managed labels are missing or contradict the requested workspace",
                        )
                    } else {
                        (Category::Unrelated, "")
                    }
                } else if identity.override_name {
                    (Category::Unrelated, "")
                } else if machine.name == identity.desired_name {
                    (Category::Desired, "")
                } else if legacy_match {
                    (Category::Stale, "")
                } else {
                    (Category::Unrelated, "")
                };
            Classified {
                machine: machine.clone(),
                category,
                source: source.into(),
                reason: reason.into(),
            }
        })
        .collect()
}
pub fn verify_readiness<'a>(inventory: &'a Inventory, identity: &Identity) -> Result<&'a Machine> {
    let matches: Vec<_> = inventory
        .iter()
        .filter(|machine| machine.name == identity.desired_name)
        .collect();
    ensure!(
        !matches.is_empty(),
        "the desired machine is missing from the refreshed inventory"
    );
    ensure!(
        matches.len() == 1,
        "multiple machines match the desired identity"
    );
    let machine = matches[0];
    if MANAGED_KEYS
        .iter()
        .any(|key| machine.labels.contains_key(*key))
    {
        ensure!(
            machine.label(0) == "workspace"
                && machine.label(1) == identity.base_id
                && machine.label(2) == identity.revision
                && machine.label(3) == identity.workspace_hash,
            "the desired machine has incomplete or contradictory managed labels"
        );
    } else if !identity.override_name {
        ensure!(
            identity.desired_name
                == format!(
                    "codex-workspace-{}-r{}-{}",
                    identity.base_id, identity.revision, identity.workspace_id
                ),
            "the unlabelled desired machine is not an allowed legacy machine"
        );
    }
    ensure!(
        machine.normalized_state().is_some(),
        "the desired machine state is {:?}",
        machine.state
    );
    Ok(machine)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagedMachine {
    pub name: String,
    pub state: String,
    pub kind: String,
    pub base_id: String,
    pub revision: String,
    pub workspace_sha256: String,
    pub workspace_path_sha256: String,
    pub configuration: String,
    pub source: String,
}
pub fn managed_inventory(inventory: &Inventory, current_base: &str) -> Result<Vec<ManagedMachine>> {
    ensure!(base_valid(current_base), "current base ID is malformed");
    let builder = Regex::new(r"^codex-builder-([0-9a-f]{12})$").unwrap();
    let login = Regex::new(r"^codex-login-([0-9a-f]{12})-r([0-9]+)$").unwrap();
    let workspace =
        Regex::new(r"^codex-workspace-([0-9a-f]{12})-r([0-9]+)-([0-9a-f]{12})$").unwrap();
    let mut result = Vec::new();
    let mut ambiguous = Vec::new();
    for machine in inventory {
        let has_labels = machine
            .labels
            .keys()
            .any(|key| key.starts_with("codex-sandbox."));
        let b = builder.captures(&machine.name);
        let l = login.captures(&machine.name);
        let w = workspace.captures(&machine.name);
        if b.is_none() && l.is_none() && w.is_none() && !has_labels {
            continue;
        }
        let Some(state) = machine.normalized_state() else {
            ambiguous.push(format!(
                "{}: managed machine state {:?} is unsupported",
                machine.name, machine.state
            ));
            continue;
        };
        let mut managed = ManagedMachine {
            name: machine.name.clone(),
            state,
            kind: String::new(),
            base_id: String::new(),
            revision: String::new(),
            workspace_sha256: String::new(),
            workspace_path_sha256: String::new(),
            configuration: String::new(),
            source: if has_labels {
                "labels".into()
            } else {
                "legacy-name".into()
            },
        };
        let problem = if let Some(capture) = b {
            managed.kind = "builder".into();
            managed.base_id = capture[1].into();
            (has_labels
                && !(machine.label(0) == "builder"
                    && machine.label(1) == managed.base_id
                    && !MANAGED_KEYS[2..]
                        .iter()
                        .any(|key| machine.labels.contains_key(*key))))
            .then_some("builder labels are incomplete or contradictory")
        } else if let Some(capture) = l {
            managed.kind = "login".into();
            managed.base_id = capture[1].into();
            managed.revision = capture[2].into();
            (has_labels
                && !(machine.label(0) == "login"
                    && machine.label(1) == managed.base_id
                    && machine.label(2) == managed.revision
                    && !MANAGED_KEYS[3..]
                        .iter()
                        .any(|key| machine.labels.contains_key(*key))))
            .then_some("login labels are incomplete or contradictory")
        } else if let Some(capture) = w {
            managed.kind = "workspace".into();
            managed.base_id = capture[1].into();
            managed.revision = capture[2].into();
            if !has_labels {
                (managed.base_id == current_base)
                    .then_some("current-generation workspace metadata is missing")
            } else {
                let old_valid = machine.label(0) == "workspace"
                    && base_valid(machine.label(1))
                    && revision_valid(machine.label(2))
                    && hash_valid(machine.label(3))
                    && machine.label(1) == &capture[1]
                    && machine.label(2) == &capture[2]
                    && machine.label(3).starts_with(&capture[3]);
                managed.workspace_sha256 = machine.label(3).into();
                managed.workspace_path_sha256 = machine.label(4).into();
                managed.configuration = machine.label(5).into();
                if old_valid
                    && hash_valid(machine.label(4))
                    && ["default", "custom"].contains(&machine.label(5))
                {
                    None
                } else if old_valid
                    && managed.base_id != current_base
                    && machine.label(4).is_empty()
                    && machine.label(5).is_empty()
                {
                    managed.source = "legacy-labels".into();
                    None
                } else {
                    Some("workspace labels are incomplete or contradictory")
                }
            }
        } else {
            Some("managed labels are attached to an unrecognized machine name")
        };
        if let Some(problem) = problem {
            ambiguous.push(format!("{}: {problem}", machine.name));
        } else {
            result.push(managed);
        }
    }
    ambiguous.sort();
    ensure!(
        ambiguous.is_empty(),
        "ambiguous managed machine inventory: {}",
        ambiguous.join("; ")
    );
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurgePolicy {
    Conservative,
    Force,
    ForceAll,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub base_id: String,
    pub path: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgePlan {
    pub current_base: String,
    pub current_artifact_available: bool,
    pub recovery_only: bool,
    pub policy: PurgePolicy,
    pub has_alternate: bool,
    pub managed_machines: Vec<ManagedMachine>,
    pub current_workspaces: Vec<ManagedMachine>,
    pub running_machines: Vec<ManagedMachine>,
    pub retained_machines: Vec<ManagedMachine>,
    pub protected_generations: BTreeSet<String>,
    pub artifacts: Vec<Artifact>,
    pub selected_machines: Vec<ManagedMachine>,
    pub selected_artifacts: Vec<Artifact>,
    pub selected_generations: BTreeSet<String>,
}
pub fn artifact_inventory(directory: &Path) -> Result<Vec<Artifact>> {
    let pattern = Regex::new(r"^codex-devtools-([0-9a-f]{12})(\.smolmachine)?$").unwrap();
    let mut result = Vec::new();
    for entry in fs::read_dir(directory).context("could not enumerate artifact directory")? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(capture) = pattern.captures(name) else {
            continue;
        };
        if entry.file_type()?.is_file() {
            result.push(Artifact {
                base_id: capture[1].into(),
                path: entry.path(),
            });
        }
    }
    result.sort_by(|a, b| (&a.base_id, &a.path).cmp(&(&b.base_id, &b.path)));
    Ok(result)
}
pub fn build_purge_plan(
    inventory: &Inventory,
    current_base: &str,
    artifacts_dir: &Path,
    policy: PurgePolicy,
) -> Result<PurgePlan> {
    let directory =
        fs::canonicalize(artifacts_dir).context("could not resolve artifact directory")?;
    let current = directory.join(format!("codex-devtools-{current_base}.smolmachine"));
    let available = match fs::symlink_metadata(&current) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && metadata.len() > 0,
                "current artifact is empty or not a regular file: {}",
                current.display()
            );
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error).context("could not inspect current artifact"),
    };
    let machines = managed_inventory(inventory, current_base)?;
    let recovery: Vec<_> = machines
        .iter()
        .filter(|machine| {
            machine.kind == "builder"
                && machine.base_id == current_base
                && machine.state != "running"
        })
        .cloned()
        .collect();
    ensure!(
        available || !recovery.is_empty(),
        "current artifact is missing, empty, or not a regular file: {}",
        current.display()
    );
    let artifacts = artifact_inventory(&directory)?;
    let running: Vec<_> = machines
        .iter()
        .filter(|machine| machine.state == "running")
        .cloned()
        .collect();
    let protected: BTreeSet<_> = running
        .iter()
        .map(|machine| machine.base_id.clone())
        .chain(std::iter::once(current_base.to_owned()))
        .collect();
    let workspaces: Vec<_> = machines
        .iter()
        .filter(|machine| machine.kind == "workspace" && machine.base_id == current_base)
        .cloned()
        .collect();
    let selected_machines: Vec<_> = if !available {
        recovery
    } else {
        machines
            .iter()
            .filter(|machine| {
                machine.state != "running"
                    && match policy {
                        PurgePolicy::Conservative => machine.base_id != current_base,
                        PurgePolicy::Force => {
                            machine.base_id != current_base
                                || (machine.kind == "workspace"
                                    && machine.configuration == "custom")
                        }
                        PurgePolicy::ForceAll => true,
                    }
            })
            .cloned()
            .collect()
    };
    let selected_artifacts: Vec<_> = artifacts
        .iter()
        .filter(|artifact| available && !protected.contains(&artifact.base_id))
        .cloned()
        .collect();
    let names: BTreeSet<_> = selected_machines
        .iter()
        .map(|machine| machine.name.as_str())
        .collect();
    let retained = machines
        .iter()
        .filter(|machine| machine.state != "running" && !names.contains(machine.name.as_str()))
        .cloned()
        .collect();
    Ok(PurgePlan {
        current_base: current_base.into(),
        current_artifact_available: available,
        recovery_only: !available,
        policy,
        has_alternate: workspaces
            .iter()
            .any(|machine| machine.configuration == "custom"),
        managed_machines: machines,
        current_workspaces: workspaces,
        running_machines: running,
        retained_machines: retained,
        protected_generations: protected,
        artifacts,
        selected_machines,
        selected_generations: selected_artifacts
            .iter()
            .map(|artifact| artifact.base_id.clone())
            .collect(),
        selected_artifacts,
    })
}
pub fn verify_artifact_protection(inventory: &Inventory, plan: &PurgePlan) -> Result<()> {
    let machines = managed_inventory(inventory, &plan.current_base)?;
    let conflicts: Vec<_> = machines
        .iter()
        .filter(|machine| {
            machine.state == "running" && plan.selected_generations.contains(&machine.base_id)
        })
        .map(|machine| machine.name.as_str())
        .collect();
    ensure!(
        conflicts.is_empty(),
        "artifact deletion is no longer safe; running machine(s) protect a selected generation: {}",
        conflicts.join(", ")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn identity() -> Identity {
        Identity {
            base_id: "aaaaaaaaaaaa".into(),
            revision: "7".into(),
            workspace_hash: "b".repeat(64),
            workspace_id: "bbbbbbbbbbbb".into(),
            desired_name: "codex-workspace-aaaaaaaaaaaa-r7-bbbbbbbbbbbb".into(),
            override_name: false,
        }
    }
    fn workspace(base: &str, hash: &str, configuration: &str, state: &str) -> Machine {
        let name = format!("codex-workspace-{base}-r7-{}", &hash[..12]);
        parse_inventory(&serde_json::to_vec(&json!([{"name":name,"state":state,"ports":0,"created_at":1,"labels":{
            MANAGED_KEYS[0]:"workspace",MANAGED_KEYS[1]:base,MANAGED_KEYS[2]:"7",MANAGED_KEYS[3]:hash,MANAGED_KEYS[4]:"c".repeat(64),MANAGED_KEYS[5]:configuration
        }}])).unwrap()).unwrap().remove(0)
    }
    #[test]
    fn malformed_inventory_never_becomes_empty() {
        for bytes in [
            b"null".as_slice(),
            b"{}",
            b"[null]",
            b"[{\"name\":1}]",
            b"[{\"name\":\"a\"},{\"name\":\"a\"}]",
            b"[{\"name\":\"a\",\"labels\":[]}]",
            b"[{\"name\":\"a\",\"labels\":{\"x\":false}}]",
            b"[{\"name\":\"a\\n\"}]",
            &[255],
        ] {
            assert!(parse_inventory(bytes).is_err(), "{bytes:?}");
        }
        assert!(parse_inventory(b"[]").unwrap().is_empty());
    }
    #[test]
    fn mapping_metadata_requires_integer_count_and_timestamp() {
        for (ports, created) in [
            (json!(true), json!(1)),
            (json!(65), json!(1)),
            (json!(0), json!(-1)),
            (json!(0), json!(true)),
            (json!(0.0), json!(1)),
        ] {
            let inventory = parse_inventory(
                &serde_json::to_vec(
                    &json!([{"name":"a","state":"stopped","ports":ports,"created_at":created}]),
                )
                .unwrap(),
            )
            .unwrap();
            assert!(inventory[0].port_info().is_err());
        }
    }
    #[test]
    fn identity_hash_matches_compact_sorted_json() {
        assert_eq!(
            hash_set(&["b".into(), "a".into(), "a".into()]),
            hash_bytes(b"[\"a\",\"b\"]")
        );
    }
    #[test]
    fn persisted_hashes_match_pre_upgrade_fixtures() {
        for (bytes, expected) in [
            (
                b"".as_slice(),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc".as_slice(),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                b"/workspace/myrepo".as_slice(),
                "f633fbbea9bf4c2d7a6ab142ecde026223f9df3db906522efa8c0c29f3e3588c",
            ),
            (
                br"C:\Users\Example\project".as_slice(),
                "e4d34c482b74221740f915f478e9c85d29e2d7d8f8fcd1a240eeef239b25768a",
            ),
            (
                &[0, 1, 127, 128, 255],
                "0150a92bb1212cd00516b65fde0704614760000963874fcbb11eaa734ee87809",
            ),
        ] {
            assert_eq!(hash_bytes(bytes), expected);
        }
        let immutable = [
            "--from=/home/example/.smolsandbox/artifacts/codex-devtools-0123456789ab.smolmachine",
            "--mem=12288",
            "--cpus=4",
            "--volume=/home/example/.smolsandbox/codex-home:/root/.codex",
            "--volume=/workspace/myrepo:/workspace/myrepo",
            "--allow-host=registry.npmjs.org",
            "--allow-host=api.openai.com",
        ]
        .map(str::to_owned);
        assert_eq!(
            hash_set(&immutable),
            "250340614fbd7a971c189012869487cd6d7634f40cd3f3ede8ba0b740ed3713d"
        );
    }
    #[test]
    fn ownership_labels_override_legacy_looking_name() {
        let id = identity();
        let mut m = workspace(&id.base_id, &id.workspace_hash, "default", "stopped");
        assert_eq!(
            classify(&vec![m.clone()], &id)[0].category,
            Category::Desired
        );
        m.labels.insert(MANAGED_KEYS[3].into(), "d".repeat(64));
        assert_eq!(
            classify(&vec![m.clone()], &id)[0].category,
            Category::Ambiguous
        );
        assert!(verify_readiness(&vec![m], &id).is_err());
    }
    #[test]
    fn incomplete_owned_metadata_is_ambiguous() {
        let id = identity();
        let mut m = workspace(&id.base_id, &id.workspace_hash, "default", "stopped");
        m.labels.remove(MANAGED_KEYS[2]);
        assert_eq!(classify(&vec![m], &id)[0].category, Category::Ambiguous);
    }
    #[test]
    fn strict_legacy_recognition_and_override_scope() {
        let mut id = identity();
        let mut m = workspace("111111111111", &id.workspace_hash, "default", "stopped");
        m.labels.clear();
        assert_eq!(classify(&vec![m.clone()], &id)[0].category, Category::Stale);
        m.name.push_str("-extra");
        assert_eq!(
            classify(&vec![m.clone()], &id)[0].category,
            Category::Unrelated
        );
        id.override_name = true;
        id.desired_name = m.name.clone();
        assert_eq!(classify(&vec![m], &id)[0].category, Category::Desired);
    }
    #[test]
    fn readiness_requires_fresh_machine_and_supported_state() {
        let id = identity();
        assert!(verify_readiness(&vec![], &id).is_err());
        let m = workspace(&id.base_id, &id.workspace_hash, "default", "broken");
        assert!(verify_readiness(&vec![m], &id).is_err());
    }
    #[test]
    fn purge_protects_running_generations_and_current_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let current = "aaaaaaaaaaaa";
        let old = "111111111111";
        fs::write(
            dir.path()
                .join(format!("codex-devtools-{current}.smolmachine")),
            b"current",
        )
        .unwrap();
        fs::write(
            dir.path().join(format!("codex-devtools-{old}.smolmachine")),
            b"old",
        )
        .unwrap();
        let default = workspace(current, &"b".repeat(64), "default", "stopped");
        let custom = workspace(current, &"d".repeat(64), "custom", "stopped");
        let running = workspace(old, &"e".repeat(64), "default", "running");
        let inventory = vec![default.clone(), custom.clone(), running];
        let conservative =
            build_purge_plan(&inventory, current, dir.path(), PurgePolicy::Conservative).unwrap();
        assert!(conservative.has_alternate);
        assert!(conservative.selected_artifacts.is_empty());
        assert!(conservative.selected_machines.is_empty());
        let force = build_purge_plan(&inventory, current, dir.path(), PurgePolicy::Force).unwrap();
        assert_eq!(
            force
                .selected_machines
                .iter()
                .map(|m| m.name.clone())
                .collect::<Vec<_>>(),
            vec![custom.name]
        );
        let all = build_purge_plan(&inventory, current, dir.path(), PurgePolicy::ForceAll).unwrap();
        assert_eq!(all.selected_machines.len(), 2);
        assert!(all.selected_artifacts.is_empty());
        assert_eq!(all.running_machines.len(), 1);
    }
    #[test]
    fn purge_without_current_artifact_only_recovers_stopped_builder() {
        let dir = tempfile::tempdir().unwrap();
        let current = "aaaaaaaaaaaa";
        let inventory = parse_inventory(br#"[{"name":"codex-builder-aaaaaaaaaaaa","state":"stopped"},{"name":"codex-builder-111111111111","state":"stopped"}]"#).unwrap();
        let plan =
            build_purge_plan(&inventory, current, dir.path(), PurgePolicy::ForceAll).unwrap();
        assert!(plan.recovery_only);
        assert_eq!(plan.selected_machines.len(), 1);
        assert_eq!(plan.selected_machines[0].base_id, current);
        assert!(plan.selected_artifacts.is_empty());
        fs::write(
            dir.path()
                .join(format!("codex-devtools-{current}.smolmachine")),
            b"",
        )
        .unwrap();
        assert!(build_purge_plan(&inventory, current, dir.path(), PurgePolicy::ForceAll).is_err());
    }
    #[test]
    fn purge_refuses_current_generation_incomplete_metadata() {
        let id = identity();
        let mut m = workspace(&id.base_id, &id.workspace_hash, "default", "stopped");
        m.labels.remove(MANAGED_KEYS[4]);
        assert!(managed_inventory(&vec![m], &id.base_id).is_err());
    }
    #[test]
    fn artifact_inventory_ignores_directories_and_nonmatching_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("codex-devtools-aaaaaaaaaaaa.tmp"), b"keep").unwrap();
        fs::create_dir(dir.path().join("codex-devtools-bbbbbbbbbbbb.smolmachine")).unwrap();
        fs::write(dir.path().join("codex-devtools-cccccccccccc"), b"old").unwrap();
        assert_eq!(artifact_inventory(dir.path()).unwrap().len(), 1);
    }
}

#[cfg(test)]
mod parity_tests {
    use super::*;
    use serde_json::json;
    fn machine(name: &str, state: &str, labels: Value) -> Machine {
        parse_inventory(
            &serde_json::to_vec(&json!([{"name":name,"state":state,"labels":labels}])).unwrap(),
        )
        .unwrap()
        .remove(0)
    }
    #[test]
    fn absent_optional_metadata_has_baseline_defaults() {
        let inventory =
            parse_inventory(br#"[{"name":"one"},{"name":"two","state":null,"labels":null}]"#)
                .unwrap();
        for m in inventory {
            assert_eq!(m.state, "unknown");
            assert!(m.labels.is_empty());
            assert!(m.port_info().is_err());
        }
    }
    #[test]
    fn managed_inventory_rejects_all_contradictory_ownership_shapes() {
        for (name, state, labels) in [
            (
                "codex-builder-aaaaaaaaaaaa",
                "stopped",
                json!({MANAGED_KEYS[0]:"builder"}),
            ),
            (
                "codex-builder-aaaaaaaaaaaa",
                "stopped",
                json!({MANAGED_KEYS[0]:"builder",MANAGED_KEYS[1]:"aaaaaaaaaaaa",MANAGED_KEYS[2]:"7"}),
            ),
            (
                "codex-login-aaaaaaaaaaaa-r7",
                "stopped",
                json!({MANAGED_KEYS[0]:"login",MANAGED_KEYS[1]:"aaaaaaaaaaaa",MANAGED_KEYS[2]:"8"}),
            ),
            (
                "codex-login-aaaaaaaaaaaa-r7",
                "stopped",
                json!({MANAGED_KEYS[0]:"login",MANAGED_KEYS[1]:"aaaaaaaaaaaa",MANAGED_KEYS[2]:"7",MANAGED_KEYS[3]:"b".repeat(64)}),
            ),
            (
                "custom-name",
                "stopped",
                json!({MANAGED_KEYS[0]:"workspace"}),
            ),
            ("codex-builder-aaaaaaaaaaaa", "failed", json!({})),
            (
                "codex-workspace-aaaaaaaaaaaa-r7-bbbbbbbbbbbb",
                "stopped",
                json!({}),
            ),
        ] {
            assert!(
                managed_inventory(&vec![machine(name, state, labels)], "aaaaaaaaaaaa").is_err(),
                "{name} {state}"
            );
        }
        assert!(managed_inventory(&vec![], "INVALID").is_err());
    }
    #[test]
    fn obsolete_legacy_labels_are_accepted_only_without_partial_new_metadata() {
        let labels = json!({MANAGED_KEYS[0]:"workspace",MANAGED_KEYS[1]:"111111111111",MANAGED_KEYS[2]:"7",MANAGED_KEYS[3]:"b".repeat(64)});
        let mut m = machine(
            "codex-workspace-111111111111-r7-bbbbbbbbbbbb",
            "stopped",
            labels,
        );
        assert_eq!(
            managed_inventory(&vec![m.clone()], "aaaaaaaaaaaa").unwrap()[0].source,
            "legacy-labels"
        );
        m.labels.insert(MANAGED_KEYS[4].into(), "c".repeat(64));
        assert!(managed_inventory(&vec![m], "aaaaaaaaaaaa").is_err());
    }
    #[test]
    fn managed_prefix_prevents_unknown_named_resource_deletion() {
        let m = machine(
            "unrelated",
            "stopped",
            json!({"codex-sandbox.future":"value"}),
        );
        assert!(managed_inventory(&vec![m], "aaaaaaaaaaaa").is_err());
        let m = machine(
            "codex-workspace-aaaaaaaaaaaa-r7-bbbbbbbbbbbb-extra",
            "stopped",
            json!({}),
        );
        assert!(managed_inventory(&vec![m], "aaaaaaaaaaaa")
            .unwrap()
            .is_empty());
    }
    #[test]
    fn running_state_change_protects_previously_selected_artifact() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("codex-devtools-aaaaaaaaaaaa.smolmachine"),
            b"current",
        )
        .unwrap();
        fs::write(
            dir.path().join("codex-devtools-111111111111.smolmachine"),
            b"old",
        )
        .unwrap();
        fs::write(dir.path().join("codex-devtools-111111111111"), b"companion").unwrap();
        let mut m = machine(
            "codex-workspace-111111111111-r7-bbbbbbbbbbbb",
            "stopped",
            json!({}),
        );
        let plan = build_purge_plan(
            &vec![m.clone()],
            "aaaaaaaaaaaa",
            dir.path(),
            PurgePolicy::Conservative,
        )
        .unwrap();
        assert_eq!(plan.selected_artifacts.len(), 2);
        assert_eq!(plan.selected_generations.len(), 1);
        verify_artifact_protection(&vec![m.clone()], &plan).unwrap();
        m.state = "RUNNING".into();
        assert!(verify_artifact_protection(&vec![m], &plan).is_err());
    }
    #[test]
    fn missing_current_artifact_requires_stopped_current_builder() {
        let dir = tempfile::tempdir().unwrap();
        for inventory in [
            vec![],
            vec![machine("codex-builder-aaaaaaaaaaaa", "running", json!({}))],
            vec![machine("codex-builder-111111111111", "stopped", json!({}))],
        ] {
            assert!(build_purge_plan(
                &inventory,
                "aaaaaaaaaaaa",
                dir.path(),
                PurgePolicy::ForceAll
            )
            .is_err());
        }
    }
    #[cfg(unix)]
    #[test]
    fn symlink_artifacts_are_never_selected_or_accepted_as_current() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("real"), b"data").unwrap();
        symlink(
            dir.path().join("real"),
            dir.path().join("codex-devtools-111111111111.smolmachine"),
        )
        .unwrap();
        assert!(artifact_inventory(dir.path()).unwrap().is_empty());
        symlink(
            dir.path().join("real"),
            dir.path().join("codex-devtools-aaaaaaaaaaaa.smolmachine"),
        )
        .unwrap();
        assert!(build_purge_plan(
            &vec![],
            "aaaaaaaaaaaa",
            dir.path(),
            PurgePolicy::Conservative
        )
        .is_err());
    }
}
