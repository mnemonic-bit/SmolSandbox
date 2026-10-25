use serde_json::{json, Value};
use smolsandbox::{
    domain::{self, Category, Identity, Machine, PurgePolicy},
    ports::{self, PortAction},
};
use std::{collections::BTreeMap, fs, path::Path};

fn cases(data: &str, expected_count: usize) -> Vec<Value> {
    let cases: Vec<Value> = serde_json::from_str(data).unwrap();
    assert_eq!(cases.len(), expected_count);
    cases
}

fn identity(override_name: bool) -> Identity {
    Identity {
        base_id: "aaaaaaaaaaaa".into(),
        revision: "5".into(),
        workspace_hash: "b".repeat(64),
        workspace_id: "bbbbbbbbbbbb".into(),
        desired_name: if override_name {
            "explicit-name".into()
        } else {
            "codex-workspace-aaaaaaaaaaaa-r5-bbbbbbbbbbbb".into()
        },
        override_name,
    }
}

#[test]
fn inventory_validation_and_normalization_have_fixed_expectations() {
    for case in cases(include_str!("data/inventory-cases.json"), 41) {
        let raw = case["raw"].as_str().unwrap();
        let expected = &case["expected"];
        let actual = domain::parse_inventory(raw.as_bytes());
        assert_eq!(actual.is_ok(), !expected.is_null(), "{raw}");
        if let Ok(machines) = actual {
            let normalized: Vec<_> = machines
                .iter()
                .map(|m| json!({"name":m.name,"state":m.state,"labels":m.labels}))
                .collect();
            assert_eq!(json!(normalized), *expected, "{raw}");
        }
    }
}

#[test]
fn public_port_metadata_has_fixed_validation_expectations() {
    for case in cases(include_str!("data/port-metadata-cases.json"), 378) {
        let raw = case["raw"].as_str().unwrap();
        let expected = &case["expected"];
        let machine = domain::parse_inventory(raw.as_bytes()).unwrap().remove(0);
        let actual = machine.port_info();
        assert_eq!(actual.is_ok(), !expected.is_null(), "{raw}");
        if let Ok((state, count, created_at)) = actual {
            assert_eq!(
                format!("{state}\u{1f}{count}\u{1f}{created_at}\n"),
                expected.as_str().unwrap(),
                "{raw}"
            );
        }
    }
}

#[test]
fn ownership_classification_and_replacement_readiness_have_fixed_expectations() {
    for case in cases(include_str!("data/ownership-cases.json"), 384) {
        let identity = identity(case["override"].as_bool().unwrap());
        let raw = case["raw"].as_str().unwrap();
        let inventory = domain::parse_inventory(raw.as_bytes()).unwrap();
        let actual = domain::classify(&inventory, &identity).remove(0);
        let category = match actual.category {
            Category::Desired => "desired",
            Category::Stale => "stale",
            Category::Ambiguous => "ambiguous",
            Category::Unrelated => "unrelated",
        };
        assert_eq!(
            json!([category, actual.source, actual.reason]),
            case["classification"],
            "{case}"
        );
        assert_eq!(
            domain::verify_readiness(&inventory, &identity).is_ok(),
            case["ready"].as_bool().unwrap(),
            "{case}"
        );
    }
}

#[test]
fn purge_plans_have_fixed_policy_running_protection_and_recovery_expectations() {
    let root = tempfile::tempdir().unwrap();
    let artifacts = root.path().join("artifacts");
    fs::create_dir(&artifacts).unwrap();
    for base in ["aaaaaaaaaaaa", "111111111111", "222222222222"] {
        for extension in ["", ".smolmachine"] {
            fs::write(
                artifacts.join(format!("codex-devtools-{base}{extension}")),
                "artifact",
            )
            .unwrap();
        }
    }
    fs::write(artifacts.join("unrelated"), "keep").unwrap();
    for case in cases(include_str!("data/purge-cases.json"), 273) {
        let current_artifact = artifacts.join("codex-devtools-aaaaaaaaaaaa.smolmachine");
        if case["missing_current_artifact"].as_bool().unwrap() && current_artifact.exists() {
            fs::remove_file(current_artifact).unwrap();
        }
        let policy = match case["policy"].as_str().unwrap() {
            "conservative" => PurgePolicy::Conservative,
            "force" => PurgePolicy::Force,
            "force-all" => PurgePolicy::ForceAll,
            value => panic!("unknown fixture policy {value}"),
        };
        let raw = case["raw"].as_str().unwrap();
        let inventory = domain::parse_inventory(raw.as_bytes()).unwrap();
        let actual = domain::build_purge_plan(&inventory, "aaaaaaaaaaaa", &artifacts, policy);
        let expected = &case["expected"];
        assert_eq!(actual.is_ok(), !expected.is_null(), "{case}");
        if let Ok(plan) = actual {
            let selected_artifacts: Vec<_> = plan
                .selected_artifacts
                .iter()
                .map(|a| json!({"base_id":a.base_id,"filename":a.path.file_name().unwrap().to_str().unwrap()}))
                .collect();
            assert_eq!(
                json!({
                    "has_alternate":plan.has_alternate,
                    "protected_generations":plan.protected_generations,
                    "managed_machines":plan.managed_machines,
                    "selected_machines":plan.selected_machines,
                    "selected_artifacts":selected_artifacts,
                    "recovery_only":plan.recovery_only
                }),
                *expected,
                "{case}"
            );
        }
    }
}

fn port_machine(state: &str, count: u64) -> Machine {
    Machine {
        name: "target".into(),
        state: state.into(),
        labels: BTreeMap::new(),
        ports: Some(count),
        created_at: Some(1),
    }
}
fn binding() -> Value {
    json!({"version":1,"machine":"target","created_at":1,"workspace_sha256":"a".repeat(64)})
}
fn port_state(root: &Path, manifest: &Value, pending: &Value) {
    fs::create_dir_all(root.join("ports")).unwrap();
    for (suffix, value) in [(".json", manifest), (".pending.json", pending)] {
        if !value.is_null() {
            fs::write(
                root.join("ports").join(format!("target{suffix}")),
                value.to_string(),
            )
            .unwrap();
        }
    }
}
fn read_state(root: &Path) -> Vec<Option<Value>> {
    ["target.json", "target.pending.json"]
        .iter()
        .map(|name| {
            fs::read(root.join("ports").join(name))
                .ok()
                .map(|bytes| serde_json::from_slice(&bytes).unwrap())
        })
        .collect()
}

#[test]
fn port_decisions_have_fixed_manifest_journal_drift_and_running_set_expectations() {
    let root = tempfile::tempdir().unwrap();
    for (index, case) in cases(include_str!("data/port-state-cases.json"), 675)
        .into_iter()
        .enumerate()
    {
        let directory = root.path().join(index.to_string());
        port_state(&directory, &case["manifest"], &case["journal"]);
        let before = read_state(&directory);
        let machine = port_machine(
            case["state"].as_str().unwrap(),
            case["count"].as_u64().unwrap(),
        );
        let requested: Vec<u16> = serde_json::from_value(case["requested"].clone()).unwrap();
        let actual = ports::prepare(&directory, &machine, &"a".repeat(64), &requested, true);
        let expected = &case["expected"];
        assert_eq!(actual.is_ok(), !expected.is_null(), "{case}");
        if let Ok(action) = actual {
            let value = match action {
                PortAction::Running => "running\n".into(),
                PortAction::Skip => "skip\n".into(),
                PortAction::Update(ports) => format!(
                    "update\n{}",
                    ports.iter().map(|p| format!("{p}\n")).collect::<String>()
                ),
            };
            assert_eq!(value, expected.as_str().unwrap(), "{case}");
        }
        assert_eq!(read_state(&directory), before, "{case}");
    }
}

#[test]
fn port_publication_commit_and_interrupted_recovery_have_fixed_expectations() {
    let root = tempfile::tempdir().unwrap();
    let hash = "a".repeat(64);
    let mut manifest_3000 = binding();
    manifest_3000["ports"] = json!([3000]);
    let mut manifest_5000 = binding();
    manifest_5000["ports"] = json!([5000]);
    let mut manifest_empty = binding();
    manifest_empty["ports"] = json!([]);
    let mut interrupted = binding();
    interrupted["removable_ports"] = json!([3000, 4000]);
    interrupted["target_ports"] = json!([4000]);
    for (requested, count, commit, expected_action, expected_state) in [
        (
            vec![3000],
            0,
            true,
            PortAction::Update(vec![3000]),
            vec![Some(manifest_3000.clone()), None],
        ),
        (
            vec![4000],
            1,
            false,
            PortAction::Update(vec![3000, 4000]),
            vec![Some(manifest_3000), Some(interrupted)],
        ),
        (
            vec![5000],
            1,
            true,
            PortAction::Update(vec![3000, 4000, 5000]),
            vec![Some(manifest_5000), None],
        ),
        (
            vec![],
            1,
            true,
            PortAction::Update(vec![5000]),
            vec![Some(manifest_empty.clone()), None],
        ),
        (
            vec![],
            0,
            true,
            PortAction::Skip,
            vec![Some(manifest_empty), None],
        ),
    ] {
        let machine = port_machine("stopped", count);
        let actual = ports::prepare(root.path(), &machine, &hash, &requested, false).unwrap();
        assert_eq!(actual, expected_action);
        if commit && matches!(actual, PortAction::Update(_)) {
            ports::commit(root.path(), &machine, &hash, &requested).unwrap();
        }
        assert_eq!(read_state(root.path()), expected_state);
    }
    ports::remove(root.path(), "target").unwrap();
    assert_eq!(read_state(root.path()), vec![None, None]);
}

#[test]
fn future_port_state_preserves_compatibility_path_and_recommends_smolsandbox_upgrade() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("codex-sandbox-existing-state");
    let future = json!({"version":2,"future_field":"preserve"});
    port_state(&directory, &future, &Value::Null);
    let before = read_state(&directory);
    let error = ports::prepare(
        &directory,
        &port_machine("stopped", 0),
        &"a".repeat(64),
        &[],
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("codex-sandbox-existing-state"));
    assert!(error.contains("upgrade smolsandbox before retrying"));
    assert_eq!(read_state(&directory), before);
}

#[test]
fn authentication_guidance_uses_smolsandbox_for_every_profile() {
    use smolsandbox::config::Profile;

    for (profile, expected) in [
        (
            Profile::Claude,
            "Run smolsandbox login for Claude authentication, or use /login inside Claude.",
        ),
        (
            Profile::Codex,
            "Run smolsandbox login for Codex device authentication.",
        ),
        (
            Profile::OpenCode,
            "Run smolsandbox login to select and authenticate a provider.",
        ),
        (
            Profile::Hermes,
            "Run smolsandbox login to configure Hermes Agent with hermes setup.",
        ),
    ] {
        assert_eq!(profile.authentication_hint(), expected);
    }
}
