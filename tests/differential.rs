#![cfg(target_os = "linux")]

use codex_sandbox::{
    domain::{self, Category, Identity, Machine, PurgePolicy},
    ports::{self, PortAction},
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

const INVENTORY_ORACLE: &str = r#"
import argparse, contextlib, io, json, pathlib, runpy, sys, tempfile
module = runpy.run_path(sys.argv[1])
results = []
for case in json.load(sys.stdin):
    try:
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / 'inventory.json'
            path.write_text(case['raw'], encoding='utf-8')
            inventory = module['load_inventory'](path)
            action = case['action']
            if action == 'normalize':
                value = [{key: machine[key] for key in ('name', 'state', 'labels')} for machine in inventory]
            elif action == 'purge':
                args = argparse.Namespace(current_base=case['base'], artifacts_dir=pathlib.Path(case['artifacts']), policy=case['policy'])
                value = module['build_purge_plan'](args, inventory)
            else:
                args = argparse.Namespace(**case['identity']) if action != 'port' else argparse.Namespace(machine='target')
                output = io.StringIO()
                function = {'classify':'classify', 'readiness':'verify_readiness', 'port':'port_info'}[action]
                with contextlib.redirect_stdout(output):
                    module[function](args, inventory)
                value = output.getvalue()
            results.append({'ok': True, 'value': value})
    except (ValueError, OSError) as error:
        results.append({'ok': False, 'error': str(error)})
json.dump(results, sys.stdout)
"#;

const PORT_ORACLE: &str = r#"
import argparse, contextlib, io, json, pathlib, runpy, sys
module = runpy.run_path(sys.argv[1])
results = []
for case in json.load(sys.stdin):
    try:
        args = argparse.Namespace(state_root=pathlib.Path(case['root']), machine='target', created_at=1, workspace_sha256='a'*64, ports=case['requested'], state=case['state'], count=case['count'], dry_run=case['dry_run'])
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            module['prepare'](args)
        if case.get('commit') and output.getvalue().startswith('update'):
            module['commit'](args)
        value = output.getvalue()
        results.append({'ok':True, 'value':value})
    except (ValueError, OSError) as error:
        results.append({'ok':False, 'error':str(error)})
json.dump(results, sys.stdout)
"#;

fn oracle(code: &str, script: &str, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(code)
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join(script))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(requests).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
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
fn identity_json(identity: &Identity) -> Value {
    json!({"base_id":identity.base_id,"revision":identity.revision,"workspace_hash":identity.workspace_hash,"workspace_id":identity.workspace_id,"desired_name":identity.desired_name,"override":identity.override_name})
}
fn labels(kind: &str, base: &str, revision: &str, hash: &str, configuration: &str) -> Value {
    json!({"codex-sandbox.kind":kind,"codex-sandbox.base-id":base,"codex-sandbox.revision":revision,"codex-sandbox.workspace-sha256":hash,"codex-sandbox.workspace-path-sha256":"c".repeat(64),"codex-sandbox.configuration":configuration})
}

#[test]
fn inventory_validation_and_normalization_match_python_baseline() {
    let mut cases = vec![
        "null".into(),
        "{}".into(),
        "[".into(),
        "[1]".into(),
        "[{}]".into(),
        "[{\"name\":\"same\"},{\"name\":\"same\"}]".into(),
        "[]".into(),
    ];
    for value in [
        Value::Null,
        json!(3),
        json!(false),
        json!(""),
        json!("x"),
        json!("bad\nname"),
        json!("bad\rname"),
        json!("bad\u{1f}name"),
    ] {
        for field in ["name", "state", "labels"] {
            let mut machine = json!({"name":"target","state":"stopped","labels":{}});
            machine[field] = value.clone();
            cases.push(json!([machine]).to_string());
        }
        cases.push(json!([{"name":"target","labels":{"field":value}}]).to_string());
    }
    cases.extend([
        json!([{"name":"target"}]).to_string(),
        json!([{"name":"目标","state":"Running","labels":{"custom":"välue","nullable":null}}])
            .to_string(),
    ]);
    let requests: Vec<_> = cases
        .iter()
        .map(|raw| json!({"action":"normalize","raw":raw}))
        .collect();
    let results = oracle(INVENTORY_ORACLE, "sandbox-inventory.py", &requests);
    for (raw, expected) in cases.iter().zip(results) {
        let result = domain::parse_inventory(raw.as_bytes());
        assert_eq!(
            result.is_ok(),
            expected["ok"].as_bool().unwrap(),
            "{raw}: {expected}"
        );
        if let Ok(machines) = result {
            let actual: Vec<_> = machines
                .iter()
                .map(|m| json!({"name":m.name,"state":m.state,"labels":m.labels}))
                .collect();
            assert_eq!(json!(actual), expected["value"], "{raw}");
        }
    }
}

#[test]
fn public_port_metadata_validation_matches_python_baseline() {
    let mut requests = Vec::new();
    let mut cases = Vec::new();
    for state in ["created", "stopped", "running", "Running", "unknown", ""] {
        for count in [
            Value::Null,
            json!(-1),
            json!(0),
            json!(1),
            json!(64),
            json!(65),
            json!(true),
            json!(1.5),
            json!("1"),
        ] {
            for created_at in [
                Value::Null,
                json!(-1),
                json!(0),
                json!(1),
                json!(true),
                json!(1.5),
                json!("1"),
            ] {
                let raw =
                    json!([{"name":"target","state":state,"ports":count,"created_at":created_at}])
                        .to_string();
                requests.push(json!({"action":"port","raw":raw}));
                cases.push(raw);
            }
        }
    }
    let results = oracle(INVENTORY_ORACLE, "sandbox-inventory.py", &requests);
    for (raw, expected) in cases.iter().zip(results) {
        let machine = domain::parse_inventory(raw.as_bytes()).unwrap().remove(0);
        let actual = machine.port_info();
        assert_eq!(
            actual.is_ok(),
            expected["ok"].as_bool().unwrap(),
            "{raw}: {expected}"
        );
        if let Ok((state, count, created_at)) = actual {
            assert_eq!(
                format!("{state}\u{1f}{count}\u{1f}{created_at}\n"),
                expected["value"].as_str().unwrap()
            );
        }
    }
}

#[test]
fn ownership_classification_and_replacement_readiness_match_python_baseline() {
    let base = "aaaaaaaaaaaa";
    let hash = "b".repeat(64);
    let mut requests = Vec::new();
    let mut cases = Vec::new();
    for overridden in [false, true] {
        let identity = identity(overridden);
        for name in [
            &identity.desired_name,
            "codex-workspace-111111111111-r4-bbbbbbbbbbbb",
            "codex-workspace-aaaaaaaaaaaa-r5-bbbbbbbbbbbb",
            "codex-workspace-111111111111-r4-cccccccccccc",
            "unrelated",
            "explicit-name",
        ] {
            for state in ["created", "stopped", "running", "unknown"] {
                for metadata in [
                    json!({}),
                    labels("workspace", base, "5", &hash, "default"),
                    labels("workspace", "111111111111", "4", &hash, "default"),
                    labels("workspace", base, "bad", &hash, "default"),
                    labels("workspace", base, "5", &"c".repeat(64), "custom"),
                    labels("login", base, "5", &hash, "default"),
                    json!({"codex-sandbox.kind":"workspace"}),
                    json!({"custom":"value"}),
                ] {
                    let raw = json!([{"name":name,"state":state,"labels":metadata}]).to_string();
                    for action in ["classify", "readiness"] {
                        requests.push(
                            json!({"action":action,"raw":raw,"identity":identity_json(&identity)}),
                        );
                    }
                    cases.push((identity.clone(), raw));
                }
            }
        }
    }
    let results = oracle(INVENTORY_ORACLE, "sandbox-inventory.py", &requests);
    for ((identity, raw), expected) in cases.iter().zip(results.as_chunks::<2>().0.iter()) {
        let inventory = domain::parse_inventory(raw.as_bytes()).unwrap();
        let actual = domain::classify(&inventory, identity).remove(0);
        let category = match actual.category {
            Category::Desired => "desired",
            Category::Stale => "stale",
            Category::Ambiguous => "ambiguous",
            Category::Unrelated => "unrelated",
        };
        let expected_fields: Vec<_> = expected[0]["value"]
            .as_str()
            .unwrap()
            .trim_end_matches('\n')
            .split('\u{1f}')
            .collect();
        assert_eq!(
            (category, actual.source.as_str(), actual.reason.as_str()),
            (expected_fields[0], expected_fields[5], expected_fields[7]),
            "{raw} override {}",
            identity.override_name
        );
        assert_eq!(
            domain::verify_readiness(&inventory, identity).is_ok(),
            expected[1]["ok"].as_bool().unwrap(),
            "{raw} override {}",
            identity.override_name
        );
    }
}

#[test]
fn purge_plans_match_python_for_policies_running_protection_and_recovery() {
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
    let mut requests = Vec::new();
    let mut cases = Vec::new();
    for base in ["aaaaaaaaaaaa", "111111111111"] {
        for kind in ["builder", "login", "workspace"] {
            for state in ["created", "stopped", "running", "Running", "invalid"] {
                for configuration in ["default", "custom", ""] {
                    let hash = "b".repeat(64);
                    let name = match kind {
                        "builder" => format!("codex-builder-{base}"),
                        "login" => format!("codex-login-{base}-r5"),
                        _ => format!("codex-workspace-{base}-r5-bbbbbbbbbbbb"),
                    };
                    let metadata = match kind {
                        "builder" => {
                            json!({"codex-sandbox.kind":kind,"codex-sandbox.base-id":base})
                        }
                        "login" => {
                            json!({"codex-sandbox.kind":kind,"codex-sandbox.base-id":base,"codex-sandbox.revision":"5"})
                        }
                        _ => labels(kind, base, "5", &hash, configuration),
                    };
                    let raw = json!([{"name":name,"state":state,"labels":metadata}]).to_string();
                    for (policy, name) in [
                        (PurgePolicy::Conservative, "conservative"),
                        (PurgePolicy::Force, "force"),
                        (PurgePolicy::ForceAll, "force-all"),
                    ] {
                        requests.push(json!({"action":"purge","raw":raw,"base":"aaaaaaaaaaaa","artifacts":artifacts,"policy":name}));
                        cases.push((raw.clone(), policy));
                    }
                }
            }
        }
    }
    let results = oracle(INVENTORY_ORACLE, "sandbox-inventory.py", &requests);
    for ((raw, policy), expected) in cases.iter().zip(results) {
        let inventory = domain::parse_inventory(raw.as_bytes()).unwrap();
        let actual = domain::build_purge_plan(&inventory, "aaaaaaaaaaaa", &artifacts, *policy);
        assert_eq!(
            actual.is_ok(),
            expected["ok"].as_bool().unwrap(),
            "{raw} {policy:?}: {expected}"
        );
        if let Ok(plan) = actual {
            assert_eq!(
                json!(plan.has_alternate),
                expected["value"]["has_alternate"]
            );
            assert_eq!(
                json!(plan.protected_generations),
                expected["value"]["protected_generations"]
            );
            assert_eq!(
                json!(plan.managed_machines),
                expected["value"]["managed_machines"]
            );
            assert_eq!(
                json!(plan.selected_machines),
                expected["value"]["selected_machines"]
            );
            let selected: Vec<_> = plan
                .selected_artifacts
                .iter()
                .map(|a| json!({"base_id":a.base_id,"path":a.path}))
                .collect();
            assert_eq!(json!(selected), expected["value"]["selected_artifacts"]);
        }
    }
    fs::remove_file(artifacts.join("codex-devtools-aaaaaaaaaaaa.smolmachine")).unwrap();
    for state in ["created", "stopped", "running"] {
        let raw = json!([{"name":"codex-builder-aaaaaaaaaaaa","state":state}]).to_string();
        let request = json!({"action":"purge","raw":raw,"base":"aaaaaaaaaaaa","artifacts":artifacts,"policy":"force-all"});
        let expected = oracle(INVENTORY_ORACLE, "sandbox-inventory.py", &[request]).remove(0);
        let actual = domain::build_purge_plan(
            &domain::parse_inventory(raw.as_bytes()).unwrap(),
            "aaaaaaaaaaaa",
            &artifacts,
            PurgePolicy::ForceAll,
        );
        assert_eq!(actual.is_ok(), expected["ok"].as_bool().unwrap());
        if let Ok(plan) = actual {
            assert!(plan.recovery_only);
            assert_eq!(
                json!(plan.selected_machines),
                expected["value"]["selected_machines"]
            );
            assert!(plan.selected_artifacts.is_empty());
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
fn port_state(root: &Path, manifest: Option<&Value>, pending: Option<&Value>) {
    fs::create_dir_all(root.join("ports")).unwrap();
    for (suffix, value) in [(".json", manifest), (".pending.json", pending)] {
        if let Some(value) = value {
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
fn port_decisions_match_python_for_manifest_journal_drift_and_running_sets() {
    let root = tempfile::tempdir().unwrap();
    let mut empty = binding();
    empty["ports"] = json!([]);
    let mut one = binding();
    one["ports"] = json!([3000]);
    let mut mismatch = one.clone();
    mismatch["created_at"] = json!(2);
    let mut pending = binding();
    pending["removable_ports"] = json!([3000, 4000]);
    pending["target_ports"] = json!([4000]);
    let mut pending_empty = binding();
    pending_empty["removable_ports"] = json!([]);
    pending_empty["target_ports"] = json!([]);
    let mut pending_mismatch = pending.clone();
    pending_mismatch["workspace_sha256"] = json!("b".repeat(64));
    let invalid = json!({"version":1});
    let mut requests = Vec::new();
    let mut cases = Vec::new();
    for manifest in [
        None,
        Some(&empty),
        Some(&one),
        Some(&mismatch),
        Some(&invalid),
    ] {
        for journal in [
            None,
            Some(&pending),
            Some(&pending_empty),
            Some(&pending_mismatch),
            Some(&invalid),
        ] {
            for state in ["created", "stopped", "running"] {
                for count in [0, 1, 2] {
                    for requested in [vec![], vec![3000], vec![5000]] {
                        let directory = root.path().join(cases.len().to_string());
                        port_state(&directory, manifest, journal);
                        requests.push(json!({"root":directory,"requested":requested,"state":state,"count":count,"dry_run":true}));
                        cases.push((directory, port_machine(state, count), requested));
                    }
                }
            }
        }
    }
    let results = oracle(PORT_ORACLE, "sandbox-port-state.py", &requests);
    for ((directory, machine, requested), expected) in cases.iter().zip(results) {
        let before = read_state(directory);
        let actual = ports::prepare(directory, machine, &"a".repeat(64), requested, true);
        assert_eq!(
            actual.is_ok(),
            expected["ok"].as_bool().unwrap(),
            "{} {} {:?}: {expected}",
            machine.state,
            machine.ports.unwrap(),
            requested
        );
        if let Ok(action) = actual {
            let value = match action {
                PortAction::Running => "running\n".into(),
                PortAction::Skip => "skip\n".into(),
                PortAction::Update(ports) => format!(
                    "update\n{}",
                    ports.iter().map(|p| format!("{p}\n")).collect::<String>()
                ),
            };
            assert_eq!(value, expected["value"].as_str().unwrap());
        }
        assert_eq!(read_state(directory), before);
    }
}

#[test]
fn port_publication_commit_and_interrupted_recovery_match_python() {
    let root = tempfile::tempdir().unwrap();
    let python_root = root.path().join("python");
    let rust_root = root.path().join("rust");
    let hash = "a".repeat(64);
    for (requested, count, commit) in [
        (vec![3000], 0, true),
        (vec![4000], 1, false),
        (vec![5000], 1, true),
        (vec![], 1, true),
        (vec![], 0, true),
    ] {
        let request = json!({"root":python_root,"requested":requested,"state":"stopped","count":count,"dry_run":false,"commit":commit});
        let expected = oracle(PORT_ORACLE, "sandbox-port-state.py", &[request]).remove(0);
        let machine = port_machine("stopped", count);
        let actual = ports::prepare(&rust_root, &machine, &hash, &requested, false);
        assert_eq!(actual.is_ok(), expected["ok"].as_bool().unwrap());
        if commit && matches!(actual, Ok(PortAction::Update(_))) {
            ports::commit(&rust_root, &machine, &hash, &requested).unwrap();
        }
        assert_eq!(read_state(&rust_root), read_state(&python_root));
    }
    ports::remove(&rust_root, "target").unwrap();
    assert_eq!(read_state(&rust_root), vec![None, None]);
}
