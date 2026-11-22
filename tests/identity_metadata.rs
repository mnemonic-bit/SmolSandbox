#![cfg(feature = "test-adapter")]

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

const VERSION: &str = "smolvm test 1.19.2 build=λ\\native";
const SECRET: &str = "definition-secret-sentinel-must-not-be-copied";

struct Fixture {
    _temporary: TempDir,
    root: PathBuf,
    home: PathBuf,
    workspace: PathBuf,
    definition: PathBuf,
}

impl Fixture {
    fn new(profile: &str) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = native_path(temporary.path());
        let home = root.join("home = λ");
        let workspace = root.join("workspace = λ");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        let definition = root.join("definition = λ.smlbx");
        fs::write(&definition, format!("# codex-sandbox-harness: {profile}\n# {SECRET}\nimage = 'debian:bookworm-slim'\n[network]\nallow_hosts = ['z.default.example', 'a.default.example', 'z.default.example']\n")).unwrap();
        fs::write(root.join("inventory.json"), "[]").unwrap();
        Self {
            _temporary: temporary,
            root,
            home,
            workspace,
            definition,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_smolsbx"));
        command
            .current_dir(&self.workspace)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("SMOLVM_TEST_HOME", &self.home)
            .env("SMOLVM_BIN", env!("CARGO_BIN_EXE_smolvm-test-double"))
            .env("SMOLVM_TEST_VERSION", format!("{VERSION}\r\n"))
            .env("SMOLVM_TEST_LOG", self.root.join("calls.log"))
            .env("SMOLVM_TEST_ARGV_LOG", self.root.join("argv.jsonl"))
            .env(
                "SMOLVM_TEST_INVENTORY_FILE",
                self.root.join("inventory.json"),
            )
            .env("SMOLVM_TEST_ALLOW_CREATE", "1")
            .env("SMOLVM_TEST_ALLOW_LIFECYCLE", "1")
            .env("SMOLVM_TEST_ALLOW_CLEANUP", "1");
        for variable in [
            "BASE_SMOLFILE",
            "ARTIFACTS_DIR",
            "STATE_ROOT",
            "CODEX_HOME_DIR",
            "CODEX_MACHINE",
            "BUILDER_MACHINE",
            "LOGIN_MACHINE",
            "OPENAI_API_KEY",
        ] {
            command.env_remove(variable);
        }
        command
    }

    fn run(&self, action: &str, additions: &[&str]) -> Output {
        let output = self
            .command()
            .arg(action)
            .arg("--smolfile")
            .arg(&self.definition)
            .args(additions)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "status {:?}, stdout {}, stderr {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn calls(&self) -> Vec<Vec<String>> {
        fs::read_to_string(self.root.join("argv.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    fn inventory(&self) -> Vec<Value> {
        serde_json::from_slice(&fs::read(self.root.join("inventory.json")).unwrap()).unwrap()
    }
}

fn native_path(path: &Path) -> PathBuf {
    let path = fs::canonicalize(path).unwrap();
    #[cfg(windows)]
    {
        let text = path.to_str().unwrap();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text))
    }
    #[cfg(not(windows))]
    path
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn canonical(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn labels(args: &[String]) -> BTreeMap<String, String> {
    args.windows(2)
        .filter(|pair| pair[0] == "--label")
        .map(|pair| {
            let (key, value) = pair[1].split_once('=').unwrap();
            (key.to_string(), value.to_string())
        })
        .collect()
}

fn creation_options(args: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut index = 2;
    while index < args.len() {
        let flag = &args[index];
        if let Some((flag, value)) = flag.split_once('=') {
            if !["--name", "--label"].contains(&flag) {
                result.push(format!("{flag}={value}"));
            }
            index += 1;
        } else {
            let value = &args[index + 1];
            if !["--name", "--label"].contains(&flag.as_str()) {
                result.push(format!("{flag}={value}"));
            }
            index += 2;
        }
    }
    canonical(result)
}

#[test]
fn every_profile_records_exact_inputs_for_all_machine_kinds() {
    for (profile, target, login_hosts) in [
        (
            "codex",
            "/root/.codex",
            vec!["auth.openai.com", "chatgpt.com", "api.openai.com"],
        ),
        (
            "claude",
            "/root/.claude",
            vec![
                "api.anthropic.com",
                "claude.ai",
                "console.anthropic.com",
                "platform.claude.com",
            ],
        ),
        (
            "opencode",
            "/root/.local/share/opencode",
            vec![
                "opencode.ai",
                "models.dev",
                "api.anthropic.com",
                "api.openai.com",
                "auth.openai.com",
                "chatgpt.com",
                "github.com",
                "api.github.com",
            ],
        ),
        (
            "hermes",
            "/root/.hermes",
            vec![
                "hermes-agent.nousresearch.com",
                "portal.nousresearch.com",
                "openrouter.ai",
                "api.openai.com",
                "api.anthropic.com",
            ],
        ),
    ] {
        let f = Fixture::new(profile);
        f.run("build", &[]);
        let bytes = fs::read(&f.definition).unwrap();
        let mut identity_bytes = format!("{VERSION}\n").into_bytes();
        identity_bytes.extend(&bytes);
        let base = digest(&identity_bytes)[..12].to_string();
        let artifact = f
            .home
            .join(".smolsandbox")
            .join("artifacts")
            .join(format!("codex-devtools-{base}.smolmachine"));
        let harness_home = if profile == "codex" {
            f.home.join(".smolsandbox").join("codex-home")
        } else {
            f.home
                .join(".smolsandbox")
                .join("harness-home")
                .join(profile)
        };
        let mount = format!("--volume={}:{target}", harness_home.display());
        let workspace_options = canonical(vec![
            format!("--from={}", artifact.display()),
            "--mem=12288".into(),
            "--cpus=4".into(),
            mount.clone(),
            format!("--volume={}:/workspace/myrepo", f.workspace.display()),
            "--allow-host=a.default.example".into(),
            "--allow-host=z.default.example".into(),
        ]);
        let workspace_hash = digest(&serde_json::to_vec(&workspace_options).unwrap());
        let expected_base = json!({"version":1,"smolvm_version":VERSION,"definition_sha256":digest(&bytes),"definition_path":f.definition.to_str().unwrap(),"harness":profile});
        let creates: Vec<_> = f
            .calls()
            .into_iter()
            .filter(|args| {
                args.starts_with(&["machine".into(), "create".into()])
                    && !args.iter().any(|arg| arg == "--help")
            })
            .collect();
        assert_eq!(creates.len(), 3, "{profile}");
        for args in creates {
            let actual_labels = labels(&args);
            assert!(
                actual_labels
                    .keys()
                    .all(|key| key.starts_with("SmolSandbox.")),
                "{profile}: {actual_labels:?}"
            );
            assert!(
                !actual_labels
                    .keys()
                    .any(|key| key.starts_with("codex-sandbox.")),
                "{profile}: {actual_labels:?}"
            );
            let kind = actual_labels["SmolSandbox.kind"].as_str();
            let mut expected_labels = BTreeMap::from([
                ("SmolSandbox.kind".into(), kind.into()),
                ("SmolSandbox.base-id".into(), base.clone()),
                (
                    "SmolSandbox.base-inputs".into(),
                    serde_json::to_string(&expected_base).unwrap(),
                ),
            ]);
            let options = match kind {
                "builder" => vec![format!("--smolfile={}", f.definition.display())],
                "login" => {
                    expected_labels.insert("SmolSandbox.revision".into(), "7".into());
                    let mut options = vec![format!("--from={}", artifact.display()), mount.clone()];
                    options.extend(
                        login_hosts
                            .iter()
                            .map(|host| format!("--allow-host={host}")),
                    );
                    canonical(options)
                }
                "workspace" => {
                    for (key, value) in [
                        ("revision", "7".to_string()),
                        ("workspace-sha256", workspace_hash.clone()),
                        (
                            "workspace-path-sha256",
                            digest(f.workspace.to_str().unwrap().as_bytes()),
                        ),
                        ("configuration", "default".to_string()),
                    ] {
                        expected_labels.insert(format!("SmolSandbox.{key}"), value);
                    }
                    workspace_options.clone()
                }
                other => panic!("unexpected kind {other}"),
            };
            let machine_inputs = json!({"version":1,"workspace_path":if kind=="workspace" {json!(f.workspace.to_str().unwrap())} else {Value::Null},"options":options});
            expected_labels.insert(
                "SmolSandbox.machine-inputs".into(),
                serde_json::to_string(&machine_inputs).unwrap(),
            );
            assert_eq!(
                actual_labels.keys().collect::<Vec<_>>(),
                expected_labels.keys().collect::<Vec<_>>(),
                "{profile}/{kind}"
            );
            for key in expected_labels.keys() {
                if key.ends_with("inputs") {
                    assert_eq!(
                        serde_json::from_str::<Value>(&actual_labels[key]).unwrap(),
                        serde_json::from_str::<Value>(&expected_labels[key]).unwrap(),
                        "{profile}/{kind}: {key}"
                    );
                } else {
                    assert_eq!(
                        actual_labels[key], expected_labels[key],
                        "{profile}/{kind}: {key}"
                    );
                }
            }
            assert_eq!(creation_options(&args), options, "{profile}/{kind}");
            assert!(!args.join(" ").contains(SECRET));
            let name = &args[3];
            match kind {
                "builder" => assert_eq!(name, &format!("codex-builder-{base}")),
                "login" => assert_eq!(name, &format!("codex-login-{base}-r7")),
                _ => assert_eq!(
                    name,
                    &format!("codex-workspace-{base}-r7-{}", &workspace_hash[..12])
                ),
            }
            if let Some(machine) = f
                .inventory()
                .iter()
                .find(|machine| machine["name"] == *name)
            {
                let raw: BTreeMap<String, String> =
                    serde_json::from_value(machine["labels"].clone()).unwrap();
                assert_eq!(raw, actual_labels);
            }
        }
        let listing: Value =
            serde_json::from_slice(&f.run("list", &["--all", "--json"]).stdout).unwrap();
        for format in ["yaml", "json"] {
            let output = f.run("list", &["--all", "--format", format]);
            let formatted: Value = if format == "yaml" {
                serde_saphyr::from_slice(&output.stdout).unwrap()
            } else {
                serde_json::from_slice(&output.stdout).unwrap()
            };
            assert_eq!(formatted, listing, "{profile}: {format}");
        }
        assert_eq!(listing["machines"].as_array().unwrap().len(), 2);
        for machine in listing["machines"].as_array().unwrap() {
            assert_eq!(machine["metadata_status"], "complete", "{machine}");
        }
    }
}

#[test]
fn byte_edits_change_fingerprint_and_reused_machines_keep_creation_provenance() {
    let f = Fixture::new("codex");
    f.run("build", &[]);
    let original = f.inventory();
    let original_workspace = original
        .iter()
        .find(|machine| machine["labels"]["SmolSandbox.kind"] == "workspace")
        .unwrap();
    let copy = f.root.join("moved definition.smlbx");
    fs::copy(&f.definition, &copy).unwrap();
    let output = f
        .command()
        .env(
            "CODEX_MACHINE",
            original_workspace["name"].as_str().unwrap(),
        )
        .env("STATE_ROOT", f.root.join("alternate-state"))
        .env("OPENAI_API_KEY", "credential-sentinel")
        .args(["start", "--verbose", "--port", "3000", "--smolfile"])
        .arg(&copy)
        .args(["--", "guest-sentinel"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reused = f.inventory();
    let workspace = reused
        .iter()
        .find(|machine| machine["name"] == original_workspace["name"])
        .unwrap();
    assert_eq!(workspace["labels"], original_workspace["labels"]);
    let inputs = workspace["labels"]["SmolSandbox.machine-inputs"]
        .as_str()
        .unwrap();
    for excluded in [
        "alternate-state",
        "credential-sentinel",
        "guest-sentinel",
        "--port",
        "--verbose",
        "--name",
        "--label",
    ] {
        assert!(!inputs.contains(excluded), "{excluded}: {inputs}");
    }
    let original_bytes = fs::read(&f.definition).unwrap();
    let mut changed = original_bytes.clone();
    changed.extend_from_slice(b"\n# byte-only change\n");
    fs::write(&f.definition, &changed).unwrap();
    f.run("build", &[]);
    let current = f.inventory();
    let new_workspace = current
        .iter()
        .find(|machine| {
            machine["labels"]["SmolSandbox.kind"] == "workspace"
                && machine["name"] != original_workspace["name"]
        })
        .unwrap();
    let original_base: Value = serde_json::from_str(
        original_workspace["labels"]["SmolSandbox.base-inputs"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let changed_base: Value = serde_json::from_str(
        new_workspace["labels"]["SmolSandbox.base-inputs"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(original_base["definition_sha256"], digest(&original_bytes));
    assert_eq!(changed_base["definition_sha256"], digest(&changed));
    assert_ne!(
        new_workspace["labels"]["SmolSandbox.base-id"],
        original_workspace["labels"]["SmolSandbox.base-id"]
    );
    assert!(!fs::read_to_string(f.root.join("argv.jsonl"))
        .unwrap()
        .contains(SECRET));
}

#[test]
fn resolved_input_variants_record_exact_identity_changes_and_exclude_mutable_settings() {
    for variant in [
        "artifacts",
        "harness-home",
        "workspace",
        "runtime-version",
        "mutable",
    ] {
        let f = Fixture::new("codex");
        f.run("build", &[]);
        let original = f.inventory();
        let original_workspace = original
            .iter()
            .find(|machine| machine["labels"]["SmolSandbox.kind"] == "workspace")
            .unwrap();
        let original_inputs: Value = serde_json::from_str(
            original_workspace["labels"]["SmolSandbox.machine-inputs"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let original_base: Value = serde_json::from_str(
            original_workspace["labels"]["SmolSandbox.base-inputs"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let workspace = if variant == "workspace" {
            f.root.join("other workspace = λ")
        } else {
            f.workspace.clone()
        };
        fs::create_dir_all(&workspace).unwrap();
        let artifacts = if variant == "artifacts" {
            f.root.join("alternate artifacts = λ")
        } else {
            f.home.join(".smolsandbox").join("artifacts")
        };
        let harness_home = if variant == "harness-home" {
            f.root.join("alternate harness home = λ")
        } else {
            f.home.join(".smolsandbox").join("codex-home")
        };
        let runtime = if variant == "runtime-version" {
            "smolvm test 1.19.3 build=λ\\native"
        } else {
            VERSION
        };
        if variant == "mutable" {
            let remaining: Vec<_> = original
                .iter()
                .filter(|machine| machine["name"] != original_workspace["name"])
                .collect();
            fs::write(
                f.root.join("inventory.json"),
                serde_json::to_vec(&remaining).unwrap(),
            )
            .unwrap();
        }
        let output = f
            .command()
            .env("ARTIFACTS_DIR", &artifacts)
            .env("CODEX_HOME_DIR", &harness_home)
            .env("STATE_ROOT", f.root.join("mutable-state-sentinel"))
            .env("SMOLVM_TEST_VERSION", format!("{runtime}\r\n"))
            .env("OPENAI_API_KEY", "mutable-credential-sentinel")
            .args(["start", "--verbose", "--port", "3000", "--smolfile"])
            .arg(&f.definition)
            .arg(&workspace)
            .args(["--", "mutable-guest-sentinel"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{variant}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let create = f
            .calls()
            .into_iter()
            .rfind(|args| args.iter().any(|arg| arg == "SmolSandbox.kind=workspace"))
            .unwrap();
        let actual = labels(&create);
        let base: Value = serde_json::from_str(&actual["SmolSandbox.base-inputs"]).unwrap();
        let inputs: Value = serde_json::from_str(&actual["SmolSandbox.machine-inputs"]).unwrap();
        let bytes = fs::read(&f.definition).unwrap();
        let mut base_bytes = format!("{runtime}\n").into_bytes();
        base_bytes.extend(&bytes);
        let base_id = digest(&base_bytes)[..12].to_string();
        let expected_options = canonical(vec![
            format!(
                "--from={}",
                artifacts
                    .join(format!("codex-devtools-{base_id}.smolmachine"))
                    .display()
            ),
            "--mem=12288".into(),
            "--cpus=4".into(),
            format!("--volume={}:/root/.codex", harness_home.display()),
            format!("--volume={}:/workspace/myrepo", workspace.display()),
            "--allow-host=a.default.example".into(),
            "--allow-host=z.default.example".into(),
        ]);
        let expected_hash = digest(&serde_json::to_vec(&expected_options).unwrap());
        assert_eq!(
            inputs,
            json!({"version":1,"workspace_path":workspace.to_str().unwrap(),"options":expected_options}),
            "{variant}"
        );
        assert_eq!(creation_options(&create), expected_options, "{variant}");
        assert_eq!(
            actual["SmolSandbox.workspace-sha256"], expected_hash,
            "{variant}"
        );
        assert_eq!(actual["SmolSandbox.base-id"], base_id, "{variant}");
        assert_eq!(base["smolvm_version"], runtime, "{variant}");
        for field in ["definition_sha256", "definition_path", "harness"] {
            assert_eq!(base[field], original_base[field], "{variant}: {field}");
        }
        if variant == "mutable" {
            assert_eq!(inputs, original_inputs);
            assert_eq!(
                actual["SmolSandbox.workspace-sha256"],
                original_workspace["labels"]["SmolSandbox.workspace-sha256"]
                    .as_str()
                    .unwrap()
            );
        } else {
            assert_ne!(inputs, original_inputs, "{variant}");
            assert_ne!(
                actual["SmolSandbox.workspace-sha256"],
                original_workspace["labels"]["SmolSandbox.workspace-sha256"]
                    .as_str()
                    .unwrap(),
                "{variant}"
            );
        }
        for excluded in [
            "mutable-state-sentinel",
            "mutable-credential-sentinel",
            "mutable-guest-sentinel",
            "--port",
            "--verbose",
            "--name",
            "--label",
        ] {
            assert!(
                !actual["SmolSandbox.machine-inputs"].contains(excluded),
                "{variant}: {excluded}"
            );
        }
    }
}
