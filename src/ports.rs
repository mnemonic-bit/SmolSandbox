use crate::domain::Machine;
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortAction {
    Running,
    Skip,
    Update(Vec<u16>),
}
#[derive(Debug)]
enum LocalState {
    Absent,
    Invalid,
    Mismatch,
    Valid(Value),
}
impl LocalState {
    fn name(&self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Invalid => "invalid",
            Self::Mismatch => "mismatch",
            Self::Valid(_) => "valid",
        }
    }
    fn ports(&self, field: &str) -> Vec<u16> {
        match self {
            Self::Valid(value) => value[field]
                .as_array()
                .unwrap()
                .iter()
                .map(|port| port.as_u64().unwrap() as u16)
                .collect(),
            _ => vec![],
        }
    }
    fn valid(&self) -> bool {
        matches!(self, Self::Valid(_))
    }
}
pub(crate) fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
}
fn validate_binding(machine: &Machine, hash: &str) -> Result<()> {
    ensure!(
        valid_name(&machine.name),
        "machine name {:?} cannot be used for local port state",
        machine.name
    );
    ensure!(
        machine.created_at.is_some(),
        "machine creation timestamp is invalid"
    );
    ensure!(
        hash.len() == 64
            && hash
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "workspace hash is invalid"
    );
    Ok(())
}
fn validate_ports(ports: &[u16]) -> Result<()> {
    ensure!(
        ports.len() <= 64
            && ports.iter().all(|port| *port > 0)
            && ports.windows(2).all(|pair| pair[0] < pair[1]),
        "ports must contain at most 64 sorted unique ports"
    );
    Ok(())
}
fn valid_json_ports(value: &Value) -> bool {
    let Some(array) = value.as_array() else {
        return false;
    };
    let Some(ports): Option<Vec<_>> = array
        .iter()
        .map(|port| {
            port.as_u64()
                .filter(|port| (1..=65535).contains(port))
                .map(|port| port as u16)
        })
        .collect()
    else {
        return false;
    };
    validate_ports(&ports).is_ok()
}
fn load(path: &Path, pending: bool, machine: &Machine, hash: &str) -> Result<LocalState> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LocalState::Absent)
        }
        Err(error) => return Err(error).with_context(|| format!("cannot read {}", path.display())),
    };
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return Ok(LocalState::Invalid),
    };
    if value
        .get("version")
        .and_then(Value::as_u64)
        .is_some_and(|version| version > 1)
    {
        bail!(
            "unsupported port state format version in {}; upgrade smolsbx before retrying",
            path.display()
        );
    }
    let fields = if pending {
        vec!["removable_ports", "target_ports"]
    } else {
        vec!["ports"]
    };
    if value.get("version").and_then(Value::as_u64) != Some(1)
        || fields.iter().any(|field| !valid_json_ports(&value[*field]))
        || value["created_at"].as_u64().is_none()
        || value["machine"].as_str().is_none()
        || value["workspace_sha256"].as_str().is_none()
    {
        return Ok(LocalState::Invalid);
    }
    if value["machine"].as_str() != Some(machine.name.as_str())
        || value["created_at"].as_u64() != machine.created_at
        || value["workspace_sha256"].as_str() != Some(hash)
    {
        return Ok(LocalState::Mismatch);
    }
    Ok(LocalState::Valid(value))
}
pub fn paths(root: &Path, machine: &str) -> (PathBuf, PathBuf) {
    (
        root.join("ports").join(format!("{machine}.json")),
        root.join("ports").join(format!("{machine}.pending.json")),
    )
}
fn atomic_write(path: &Path, value: &Value) -> Result<()> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    crate::platform::atomic_write(path, &bytes)
}
fn remove_file(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
fn binding(machine: &Machine, hash: &str) -> Value {
    json!({"version":1,"machine":machine.name,"created_at":machine.created_at,"workspace_sha256":hash})
}
pub fn prepare(
    root: &Path,
    machine: &Machine,
    hash: &str,
    requested: &[u16],
    dry_run: bool,
) -> Result<PortAction> {
    validate_binding(machine, hash)?;
    validate_ports(requested)?;
    let (state, count, _) = machine.port_info()?;
    let (manifest_path, pending_path) = paths(root, &machine.name);
    let manifest = load(&manifest_path, false, machine, hash)?;
    let pending = load(&pending_path, true, machine, hash)?;
    let manifest_ports = manifest.ports("ports");
    if state == "running" {
        ensure!(manifest.valid() && matches!(pending,LocalState::Absent) && count == manifest_ports.len() as u64 && manifest_ports == requested,
            "running machine {} has {count} stored mapping(s), manifest {}, pending journal {}, and requested ports {requested:?}; stop the active session before retrying",machine.name,manifest.name(),pending.name());
        return Ok(PortAction::Running);
    }
    if matches!(pending, LocalState::Absent) && manifest.valid() {
        ensure!(count == manifest_ports.len() as u64, "machine {} reports {count} mapping(s), but its port manifest records {}; refusing to overwrite unowned drift",machine.name,manifest_ports.len());
    } else if !pending.valid() && count > 0 {
        bail!("machine {} has {count} mapping(s) but no safe removable port set: manifest {}; pending journal {}. Inspect `smolvm machine ls --verbose`, remove the displayed mappings from the stopped machine, and retry",machine.name,manifest.name(),pending.name());
    }
    let removable: Vec<u16> = requested
        .iter()
        .copied()
        .chain(manifest_ports)
        .chain(pending.ports("removable_ports"))
        .chain(pending.ports("target_ports"))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    ensure!(!removable.is_empty() || count == 0, "machine {} has {count} mapping(s), but the recovery journal contains no removable candidates. Inspect `smolvm machine ls --verbose`, remove the displayed mappings from the stopped machine, and retry",machine.name);
    let mut value = binding(machine, hash);
    if removable.is_empty() {
        if !dry_run {
            value["ports"] = json!(requested);
            atomic_write(&manifest_path, &value)?;
            remove_file(&pending_path)?;
        }
        Ok(PortAction::Skip)
    } else {
        if !dry_run {
            value["removable_ports"] = json!(removable);
            value["target_ports"] = json!(requested);
            atomic_write(&pending_path, &value)?;
        }
        Ok(PortAction::Update(removable))
    }
}
pub fn commit(root: &Path, machine: &Machine, hash: &str, requested: &[u16]) -> Result<()> {
    validate_binding(machine, hash)?;
    validate_ports(requested)?;
    let (manifest_path, pending_path) = paths(root, &machine.name);
    let pending = load(&pending_path, true, machine, hash)?;
    ensure!(
        pending.valid(),
        "cannot commit port state: pending journal is {}",
        pending.name()
    );
    ensure!(
        pending.ports("target_ports") == requested,
        "cannot commit port state: pending target changed"
    );
    let mut value = binding(machine, hash);
    value["ports"] = json!(requested);
    atomic_write(&manifest_path, &value)?;
    remove_file(&pending_path)
}
pub fn remove(root: &Path, machine: &str) -> Result<()> {
    ensure!(
        valid_name(machine),
        "machine name {machine:?} cannot be used for local port state"
    );
    let (manifest, pending) = paths(root, machine);
    remove_file(&manifest)?;
    remove_file(&pending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn machine() -> Machine {
        Machine {
            name: "codex-workspace-test".into(),
            state: "stopped".into(),
            labels: BTreeMap::new(),
            ports: Some(0),
            created_at: Some(1),
        }
    }
    fn hash() -> String {
        "a".repeat(64)
    }
    #[test]
    fn unknown_positive_mapping_refuses_adoption() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine();
        m.ports = Some(1);
        assert!(prepare(dir.path(), &m, &hash(), &[4444], false)
            .unwrap_err()
            .to_string()
            .contains("no safe removable"));
        assert!(!dir.path().join("ports").exists());
    }
    #[test]
    fn interrupted_update_recovers_union_and_commits_exact_set() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine();
        assert_eq!(
            prepare(dir.path(), &m, &hash(), &[5000], false).unwrap(),
            PortAction::Update(vec![5000])
        );
        m.ports = Some(1);
        assert_eq!(
            prepare(dir.path(), &m, &hash(), &[6000], false).unwrap(),
            PortAction::Update(vec![5000, 6000])
        );
        commit(dir.path(), &m, &hash(), &[6000]).unwrap();
        m.state = "running".into();
        assert_eq!(
            prepare(dir.path(), &m, &hash(), &[6000], false).unwrap(),
            PortAction::Running
        );
        assert!(prepare(dir.path(), &m, &hash(), &[7000], false).is_err());
        m.state = "stopped".into();
        assert_eq!(
            prepare(dir.path(), &m, &hash(), &[], false).unwrap(),
            PortAction::Update(vec![6000])
        );
    }
    #[test]
    fn dry_run_has_no_side_effects_and_new_incarnation_resets_zero_ports() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine();
        prepare(dir.path(), &m, &hash(), &[5000], true).unwrap();
        assert!(!dir.path().join("ports").exists());
        prepare(dir.path(), &m, &hash(), &[5000], false).unwrap();
        commit(dir.path(), &m, &hash(), &[5000]).unwrap();
        m.created_at = Some(2);
        assert_eq!(
            prepare(dir.path(), &m, &hash(), &[], false).unwrap(),
            PortAction::Skip
        );
    }
    #[test]
    fn future_formats_are_never_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        let m = machine();
        let (path, _) = paths(dir.path(), &m.name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{\"version\":2}").unwrap();
        assert!(prepare(dir.path(), &m, &hash(), &[], false).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{\"version\":2}");
    }
    #[test]
    fn malformed_bindings_and_ports_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let m = machine();
        for ports in [vec![0], vec![2, 1], vec![1, 1]] {
            assert!(prepare(dir.path(), &m, &hash(), &ports, false).is_err());
        }
        assert!(prepare(dir.path(), &m, "invalid", &[], false).is_err());
        assert!(remove(dir.path(), "../escape").is_err());
    }
}

#[cfg(test)]
mod parity_tests {
    use super::*;
    use std::collections::BTreeMap;
    fn machine() -> Machine {
        Machine {
            name: "test-machine".into(),
            state: "stopped".into(),
            labels: BTreeMap::new(),
            ports: Some(0),
            created_at: Some(1),
        }
    }
    fn hash() -> String {
        "a".repeat(64)
    }
    fn publish(path: &Path, value: &Value) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
    }
    #[test]
    fn running_requires_matching_manifest_count_and_absent_journal() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine();
        m.state = "running".into();
        assert!(prepare(dir.path(), &m, &hash(), &[], false).is_err());
        m.state = "stopped".into();
        prepare(dir.path(), &m, &hash(), &[], false).unwrap();
        m.state = "running".into();
        assert_eq!(
            prepare(dir.path(), &m, &hash(), &[], false).unwrap(),
            PortAction::Running
        );
        m.ports = Some(1);
        assert!(prepare(dir.path(), &m, &hash(), &[], false).is_err());
        m.ports = Some(0);
        let (_, pending) = paths(dir.path(), &m.name);
        fs::write(pending, b"broken").unwrap();
        assert!(prepare(dir.path(), &m, &hash(), &[], false).is_err());
    }
    #[test]
    fn manifest_count_drift_is_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine();
        prepare(dir.path(), &m, &hash(), &[3000], false).unwrap();
        commit(dir.path(), &m, &hash(), &[3000]).unwrap();
        assert!(prepare(dir.path(), &m, &hash(), &[], false).is_err());
        m.ports = Some(2);
        assert!(prepare(dir.path(), &m, &hash(), &[3000], false).is_err());
    }
    #[test]
    fn malformed_journal_cannot_authorize_positive_mapping_removal() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine();
        m.ports = Some(1);
        let (_, pending) = paths(dir.path(), &m.name);
        let mut value = binding(&m, &hash());
        value["removable_ports"] = json!([3000]);
        value["target_ports"] = json!([4000]);
        for (field, invalid) in [
            ("version", json!(true)),
            ("created_at", json!(true)),
            ("removable_ports", json!([0])),
            ("target_ports", json!([2, 1])),
            ("machine", json!(false)),
            ("workspace_sha256", Value::Null),
        ] {
            let mut bad = value.clone();
            bad[field] = invalid;
            publish(&pending, &bad);
            assert!(
                prepare(dir.path(), &m, &hash(), &[5000], false).is_err(),
                "{field}"
            );
        }
        value["created_at"] = json!(2);
        publish(&pending, &value);
        assert!(prepare(dir.path(), &m, &hash(), &[5000], false).is_err());
    }
    #[test]
    fn empty_recovery_set_does_not_authorize_unknown_positive_count() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine();
        m.ports = Some(1);
        let (_, pending) = paths(dir.path(), &m.name);
        let mut value = binding(&m, &hash());
        value["removable_ports"] = json!([]);
        value["target_ports"] = json!([]);
        publish(&pending, &value);
        assert!(prepare(dir.path(), &m, &hash(), &[], false)
            .unwrap_err()
            .to_string()
            .contains("no removable candidates"));
    }
    #[test]
    fn commit_requires_valid_binding_and_unchanged_target() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine();
        assert!(commit(dir.path(), &m, &hash(), &[],).is_err());
        prepare(dir.path(), &m, &hash(), &[3000], false).unwrap();
        assert!(commit(dir.path(), &m, &hash(), &[4000]).is_err());
        m.created_at = Some(2);
        assert!(commit(dir.path(), &m, &hash(), &[3000]).is_err());
        m.created_at = Some(1);
        commit(dir.path(), &m, &hash(), &[3000]).unwrap();
        let (manifest, pending) = paths(dir.path(), &m.name);
        assert!(manifest.exists());
        assert!(!pending.exists());
        remove(dir.path(), &m.name).unwrap();
        remove(dir.path(), &m.name).unwrap();
        assert!(!manifest.exists());
    }
    #[test]
    fn zero_port_invalid_manifest_can_be_replaced_but_future_journal_cannot() {
        let dir = tempfile::tempdir().unwrap();
        let m = machine();
        let (manifest, pending) = paths(dir.path(), &m.name);
        publish(&manifest, &json!({"invalid":"old state"}));
        assert_eq!(
            prepare(dir.path(), &m, &hash(), &[], false).unwrap(),
            PortAction::Skip
        );
        publish(&pending, &json!({"version":3}));
        assert!(prepare(dir.path(), &m, &hash(), &[], false).is_err());
        assert_eq!(fs::read(pending).unwrap(), b"{\"version\":3}");
    }
    #[test]
    fn invalid_public_metadata_and_unsupported_state_fail_before_publication() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = machine();
        for count in [None, Some(65)] {
            m.ports = count;
            assert!(prepare(dir.path(), &m, &hash(), &[], false).is_err());
        }
        m.ports = Some(0);
        m.created_at = None;
        assert!(prepare(dir.path(), &m, &hash(), &[], false).is_err());
        m.created_at = Some(1);
        m.state = "unknown".into();
        assert!(prepare(dir.path(), &m, &hash(), &[], false).is_err());
        assert!(!dir.path().join("ports").exists());
    }
    #[cfg(unix)]
    #[test]
    fn port_state_is_user_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let m = machine();
        prepare(dir.path(), &m, &hash(), &[], false).unwrap();
        let (manifest, _) = paths(dir.path(), &m.name);
        assert_eq!(
            fs::metadata(manifest).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(dir.path().join("ports"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
}
