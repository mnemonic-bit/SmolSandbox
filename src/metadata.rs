use crate::domain::{hash_bytes, hash_set, Machine};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const BASE_INPUTS: &str = "SmolSandbox.base-inputs";
pub const MACHINE_INPUTS: &str = "SmolSandbox.machine-inputs";

#[derive(Debug, Deserialize, Serialize)]
pub struct BaseInputs {
    pub version: u64,
    pub smolvm_version: String,
    pub definition_sha256: String,
    pub definition_path: String,
    pub harness: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MachineInputs {
    pub version: u64,
    pub workspace_path: Option<String>,
    pub options: Vec<String>,
}

pub struct Metadata {
    pub base: Option<BaseInputs>,
    pub machine: Option<MachineInputs>,
    pub status: &'static str,
    pub reasons: Vec<String>,
}

enum Component<T> {
    Absent,
    Valid(T),
    Invalid(&'static str),
    Unsupported,
}

fn decode_label<T: serde::de::DeserializeOwned>(
    machine: &Machine,
    key: &str,
    legacy_key: &str,
    require_workspace_path: bool,
) -> Component<T> {
    let current = machine.labels.get(key);
    let legacy = machine.labels.get(legacy_key);
    if current
        .zip(legacy)
        .is_some_and(|(current, old)| current != old)
    {
        return Component::Invalid("descriptive labels contradict across namespaces");
    }
    decode(current.or(legacy), require_workspace_path)
}

fn decode<T: serde::de::DeserializeOwned>(
    value: Option<&String>,
    require_workspace_path: bool,
) -> Component<T> {
    let Some(value) = value else {
        return Component::Absent;
    };
    let Ok(value) = serde_json::from_str::<Value>(value) else {
        return Component::Invalid("malformed JSON");
    };
    let Some(object) = value.as_object() else {
        return Component::Invalid("expected a JSON object");
    };
    let Some(version) = object.get("version").and_then(Value::as_u64) else {
        return Component::Invalid("missing or invalid schema version");
    };
    if version != 1 {
        return Component::Unsupported;
    }
    // Option fields still have to be present in the versioned wire format.
    if require_workspace_path && !object.contains_key("workspace_path") {
        return Component::Invalid("missing workspace_path field");
    }
    match serde_json::from_value(value) {
        Ok(value) => Component::Valid(value),
        Err(_) => Component::Invalid("missing or wrongly typed fields"),
    }
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn absolute(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with(r"\\")
        || (value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
            && value.as_bytes().get(1) == Some(&b':')
            && matches!(value.as_bytes().get(2), Some(b'/' | b'\\')))
}

fn guest_home(harness: &str) -> Option<&'static str> {
    match harness {
        "codex" => Some("/root/.codex"),
        "claude" => Some("/root/.claude"),
        "opencode" => Some("/root/.local/share/opencode"),
        "hermes" => Some("/root/.hermes"),
        _ => None,
    }
}

fn validate_base(value: &BaseInputs) -> Result<(), &'static str> {
    if value.smolvm_version.is_empty() || value.smolvm_version.ends_with(['\r', '\n']) {
        return Err("invalid normalized SmolVM version");
    }
    if !hex(&value.definition_sha256, 64) {
        return Err("invalid definition fingerprint");
    }
    if !absolute(&value.definition_path) {
        return Err("definition path is not absolute");
    }
    if guest_home(&value.harness).is_none() {
        return Err("invalid harness identifier");
    }
    Ok(())
}

fn validate_machine(
    value: &MachineInputs,
    machine: &Machine,
    kind: Option<&str>,
    base: Option<&BaseInputs>,
) -> Result<(), &'static str> {
    if value.options.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("options are not sorted and exact-deduplicated");
    }
    let mut options = Vec::new();
    for option in &value.options {
        let Some((flag, argument)) = option.split_once('=') else {
            return Err("invalid creation option structure");
        };
        if !flag.starts_with("--") || flag.len() <= 2 || argument.is_empty() {
            return Err("invalid creation option structure");
        }
        options.push((flag, argument));
    }
    let values = |flag| {
        options
            .iter()
            .filter_map(move |(key, value)| (*key == flag).then_some(*value))
            .collect::<Vec<_>>()
    };
    let Some(kind) = kind else {
        return Err("machine kind is unavailable");
    };
    let allowed: &[&str] = match kind {
        "workspace" => &["--from", "--mem", "--cpus", "--volume", "--allow-host"],
        "login" => &["--from", "--volume", "--allow-host"],
        "builder" => &["--smolfile"],
        _ => return Err("machine kind is unavailable"),
    };
    if options.iter().any(|(flag, _)| !allowed.contains(flag)) {
        return Err("creation option contradicts machine kind");
    }
    if values("--allow-host")
        .iter()
        .any(|host| host.starts_with('-') || host.contains(['\r', '\n']))
    {
        return Err("allow-host option is invalid");
    }
    if kind == "builder" {
        let definitions = values("--smolfile");
        if value.workspace_path.is_some() || definitions.len() != 1 || !absolute(definitions[0]) {
            return Err("builder requires one definition and no workspace path");
        }
        if base.is_some_and(|base| definitions[0] != base.definition_path) {
            return Err("builder definition contradicts base-inputs");
        }
        return Ok(());
    }
    let artifacts = values("--from");
    if artifacts.len() != 1 || !absolute(artifacts[0]) {
        return Err("machine requires one absolute artifact path");
    }
    let base_id = machine.label(1);
    if hex(base_id, 12)
        && artifacts[0].rsplit(['/', '\\']).next()
            != Some(format!("codex-devtools-{base_id}.smolmachine").as_str())
    {
        return Err("artifact filename contradicts base-id");
    }
    let volumes = values("--volume");
    if kind == "login" && value.workspace_path.is_some() {
        return Err("shared machine has a workspace path");
    }
    let harness_volume = if kind == "workspace" {
        let Some(path) = value.workspace_path.as_deref() else {
            return Err("workspace path is missing");
        };
        if !absolute(path) {
            return Err("workspace path is not absolute");
        }
        let path_hash = machine.label(4);
        if !hex(path_hash, 64) || hash_bytes(path.as_bytes()) != path_hash {
            return Err("workspace path contradicts workspace-path-sha256");
        }
        let hash = machine.label(3);
        if !hex(hash, 64) || hash_set(&value.options) != hash {
            return Err("creation options contradict workspace-sha256");
        }
        for flag in ["--mem", "--cpus"] {
            let resources = values(flag);
            if resources.len() != 1
                || !resources[0].bytes().all(|byte| byte.is_ascii_digit())
                || !resources[0].parse::<u64>().is_ok_and(|value| value > 0)
            {
                return Err("workspace resource option is missing or invalid");
            }
        }
        let workspace_volume = format!("{path}:/workspace/myrepo");
        if volumes.len() != 2 || !volumes.contains(&workspace_volume.as_str()) {
            return Err("workspace volume contradicts workspace path");
        }
        volumes
            .iter()
            .copied()
            .find(|volume| *volume != workspace_volume)
    } else if volumes.len() == 1 {
        volumes.first().copied()
    } else {
        None
    };
    let Some((host, target)) = harness_volume.and_then(|volume| volume.rsplit_once(':')) else {
        return Err("harness volume is missing or invalid");
    };
    if !absolute(host)
        || !["codex", "claude", "opencode", "hermes"]
            .iter()
            .any(|harness| guest_home(harness) == Some(target))
    {
        return Err("harness volume is invalid");
    }
    if base.is_some_and(|base| guest_home(&base.harness) != Some(target)) {
        return Err("harness volume contradicts base-inputs");
    }
    Ok(())
}

impl Metadata {
    pub fn inspect(machine: &Machine, kind: Option<&str>) -> Self {
        let mut base =
            decode_label::<BaseInputs>(machine, BASE_INPUTS, "codex-sandbox.base-inputs", false);
        if let Component::Valid(value) = &base {
            if let Err(reason) = validate_base(value) {
                base = Component::Invalid(reason);
            }
        }
        let mut inputs = decode_label::<MachineInputs>(
            machine,
            MACHINE_INPUTS,
            "codex-sandbox.machine-inputs",
            true,
        );
        if let Component::Valid(value) = &inputs {
            let valid_base = match &base {
                Component::Valid(value) => Some(value),
                _ => None,
            };
            if let Err(reason) = validate_machine(value, machine, kind, valid_base) {
                inputs = Component::Invalid(reason);
            }
        }
        let invalid =
            matches!(base, Component::Invalid(_)) || matches!(inputs, Component::Invalid(_));
        let unsupported =
            matches!(base, Component::Unsupported) || matches!(inputs, Component::Unsupported);
        let count = usize::from(matches!(base, Component::Valid(_)))
            + usize::from(matches!(inputs, Component::Valid(_)));
        let status = if invalid {
            "invalid"
        } else if unsupported {
            "unsupported"
        } else {
            match count {
                2 => "complete",
                1 => "partial",
                _ => "unavailable",
            }
        };
        let mut reasons = Vec::new();
        fn component<T>(
            value: Component<T>,
            name: &str,
            key: &str,
            reasons: &mut Vec<String>,
        ) -> Option<T> {
            match value {
                Component::Valid(value) => Some(value),
                Component::Absent => {
                    reasons.push(format!("Machine {name:?}: {key} is absent."));
                    None
                }
                Component::Invalid(reason) => {
                    reasons.push(format!("Machine {name:?}: {key} is invalid: {reason}."));
                    None
                }
                Component::Unsupported => {
                    reasons.push(format!(
                        "Machine {name:?}: {key} schema version is unsupported."
                    ));
                    None
                }
            }
        }
        let base = component(base, &machine.name, BASE_INPUTS, &mut reasons);
        let machine = component(inputs, &machine.name, MACHINE_INPUTS, &mut reasons);
        Self {
            base,
            machine,
            status,
            reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn windows_workspace() -> Machine {
        let path = "C:\\Projects\\a=β\twork";
        let mut options = vec![
            "--from=E:\\artifacts\\codex-devtools-123456789abc.smolmachine".to_owned(),
            "--mem=8192".into(),
            "--cpus=2".into(),
            "--volume=D:\\agent home:/root/.codex".into(),
            format!("--volume={path}:/workspace/myrepo"),
            "--allow-host=host=例.example".into(),
        ];
        options.sort();
        Machine {
            name: "windows-workspace".into(),
            state: "stopped".into(),
            labels: BTreeMap::from([
                ("SmolSandbox.kind".into(), "workspace".into()),
                ("SmolSandbox.base-id".into(), "123456789abc".into()),
                (
                    "SmolSandbox.workspace-path-sha256".into(),
                    "72756f14626225c584893bc09cfb5d9122a1f53829f6c7e8766b6d469a16c69d".into(),
                ),
                (
                    "SmolSandbox.workspace-sha256".into(),
                    "2e9431a3e4003b22636b160643ab28ae0edb59e636f7042e72f5d6e500946dd7".into(),
                ),
                (
                    BASE_INPUTS.into(),
                    json!({
                        "version":1,"smolvm_version":"smolvm 1.19.2+build=example",
                        "definition_sha256":"a".repeat(64),
                        "definition_path":"C:\\definitions\\selected=β.smlbx",
                        "harness":"codex", "options":"unknown extra field"
                    })
                    .to_string(),
                ),
                (
                    MACHINE_INPUTS.into(),
                    json!({
                        "version":1,"workspace_path":path,"options":options,
                        "future_field":{"ignored":true}
                    })
                    .to_string(),
                ),
            ]),
            ports: None,
            created_at: None,
        }
    }

    #[test]
    fn windows_paths_unicode_equals_and_controls_preserve_fixed_identity() {
        let machine = windows_workspace();
        let metadata = Metadata::inspect(&machine, Some("workspace"));
        assert_eq!(metadata.status, "complete", "{:?}", metadata.reasons);
        assert!(metadata.reasons.is_empty());
        let inputs = metadata.machine.unwrap();
        assert_eq!(
            inputs.workspace_path.as_deref(),
            Some("C:\\Projects\\a=β\twork")
        );
        assert_eq!(inputs.options.len(), 6);
        assert_eq!(
            serde_json::to_value(inputs).unwrap()["options"][0],
            "--allow-host=host=例.example"
        );
    }

    #[test]
    fn descriptive_namespaces_accept_old_and_agreeing_values_and_reject_conflicts() {
        for mode in ["old", "dual", "split"] {
            let mut machine = windows_workspace();
            let current = machine.labels.clone();
            for (index, (key, value)) in current.into_iter().enumerate() {
                if mode == "old" || mode == "dual" || index % 2 == 0 {
                    let suffix = key.strip_prefix("SmolSandbox.").unwrap();
                    machine
                        .labels
                        .insert(format!("codex-sandbox.{suffix}"), value);
                    if mode != "dual" {
                        machine.labels.remove(&key);
                    }
                }
            }
            let metadata = Metadata::inspect(&machine, Some("workspace"));
            assert_eq!(
                metadata.status, "complete",
                "{mode}: {:?}",
                metadata.reasons
            );
        }
        for (key, legacy) in [
            (BASE_INPUTS, "codex-sandbox.base-inputs"),
            (MACHINE_INPUTS, "codex-sandbox.machine-inputs"),
        ] {
            let mut machine = windows_workspace();
            machine
                .labels
                .insert(legacy.into(), "{different JSON".into());
            let metadata = Metadata::inspect(&machine, Some("workspace"));
            assert_eq!(metadata.status, "invalid");
            assert_eq!(metadata.base.is_none(), key == BASE_INPUTS);
            assert_eq!(metadata.machine.is_none(), key == MACHINE_INPUTS);
            assert!(metadata
                .reasons
                .iter()
                .any(|reason| reason.contains("contradict across namespaces")));
            assert!(!machine.ownership_conflicts());
        }
    }

    #[test]
    fn invalid_component_precedes_future_version_and_preserves_independent_component() {
        let mut machine = windows_workspace();
        machine
            .labels
            .insert(BASE_INPUTS.into(), json!({"version":9}).to_string());
        machine.labels.insert(
            MACHINE_INPUTS.into(),
            "{arbitrary malformed secret payload".into(),
        );
        let metadata = Metadata::inspect(&machine, Some("workspace"));
        assert_eq!(metadata.status, "invalid");
        assert!(metadata.base.is_none());
        assert!(metadata.machine.is_none());
        assert!(!metadata.reasons.join(" ").contains("secret payload"));

        let mut machine = windows_workspace();
        machine
            .labels
            .insert(BASE_INPUTS.into(), json!({"version":9}).to_string());
        let metadata = Metadata::inspect(&machine, Some("workspace"));
        assert_eq!(metadata.status, "unsupported");
        assert!(metadata.base.is_none());
        assert!(metadata.machine.is_some());
    }

    #[test]
    fn required_nullable_field_and_cross_component_harness_are_checked() {
        let mut machine = windows_workspace();
        let mut value: Value = serde_json::from_str(&machine.labels[MACHINE_INPUTS]).unwrap();
        value.as_object_mut().unwrap().remove("workspace_path");
        machine
            .labels
            .insert(MACHINE_INPUTS.into(), value.to_string());
        let metadata = Metadata::inspect(&machine, Some("workspace"));
        assert_eq!(metadata.status, "invalid");
        assert!(metadata.base.is_some());
        assert!(metadata.machine.is_none());

        let mut machine = windows_workspace();
        let mut value: Value = serde_json::from_str(&machine.labels[BASE_INPUTS]).unwrap();
        value["harness"] = json!("hermes");
        machine.labels.insert(BASE_INPUTS.into(), value.to_string());
        let metadata = Metadata::inspect(&machine, Some("workspace"));
        assert_eq!(metadata.status, "invalid");
        assert!(metadata.base.is_some());
        assert!(metadata.machine.is_none());
        assert!(metadata
            .reasons
            .iter()
            .any(|reason| reason.contains("harness volume contradicts")));
    }

    #[test]
    fn shared_inputs_use_only_their_own_options_and_accept_unc_paths() {
        for (kind, mut options) in [
            (
                "builder",
                vec!["--smolfile=C:\\definitions\\selected=β.smlbx".to_owned()],
            ),
            (
                "login",
                vec![
                    r"--from=\\server\artifacts\codex-devtools-123456789abc.smolmachine".into(),
                    r"--volume=\\server\state\agent home:/root/.codex".into(),
                    "--allow-host=login.example.com".into(),
                ],
            ),
        ] {
            let mut machine = windows_workspace();
            machine
                .labels
                .insert("SmolSandbox.kind".into(), kind.into());
            machine.labels.remove("SmolSandbox.workspace-sha256");
            machine.labels.remove("SmolSandbox.workspace-path-sha256");
            options.sort();
            machine.labels.insert(
                MACHINE_INPUTS.into(),
                json!({"version":1,"workspace_path":null,"options":options}).to_string(),
            );
            let metadata = Metadata::inspect(&machine, Some(kind));
            assert_eq!(metadata.status, "complete", "{:?}", metadata.reasons);
            assert!(metadata.machine.unwrap().workspace_path.is_none());
        }
    }

    #[test]
    fn consistent_hashes_do_not_hide_invalid_mounts_or_resources() {
        for (old, replacement, reason) in [
            ("--mem=8192", "--mem=0", "resource option"),
            ("--cpus=2", "--cpus=invalid", "resource option"),
            (
                "--allow-host=host=例.example",
                "--allow-host=-invalid.example",
                "allow-host option",
            ),
            (
                "--allow-host=host=例.example",
                "--allow-host=invalid\n.example",
                "allow-host option",
            ),
            (
                "--volume=C:\\Projects\\a=β\twork:/workspace/myrepo",
                "--volume=C:\\Projects\\a=β\twork:/workspace/elsewhere",
                "workspace volume contradicts",
            ),
        ] {
            let mut machine = windows_workspace();
            let mut value: Value = serde_json::from_str(&machine.labels[MACHINE_INPUTS]).unwrap();
            let options = value["options"].as_array_mut().unwrap();
            let position = options.iter().position(|option| option == old).unwrap();
            options[position] = json!(replacement);
            let mut strings: Vec<String> = options
                .iter()
                .map(|value| value.as_str().unwrap().to_owned())
                .collect();
            strings.sort();
            machine
                .labels
                .insert("SmolSandbox.workspace-sha256".into(), hash_set(&strings));
            value["options"] = json!(strings);
            machine
                .labels
                .insert(MACHINE_INPUTS.into(), value.to_string());
            let metadata = Metadata::inspect(&machine, Some("workspace"));
            assert_eq!(metadata.status, "invalid");
            assert!(metadata.base.is_some());
            assert!(metadata.machine.is_none());
            assert!(
                metadata.reasons.iter().any(|value| value.contains(reason)),
                "{:?}",
                metadata.reasons
            );
        }
    }
}
