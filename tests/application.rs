#![cfg(any(not(windows), feature = "test-adapter"))]

#[cfg(feature = "test-adapter")]
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

struct Fixture {
    root: TempDir,
    home: PathBuf,
    workspace: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let workspace = root.path().join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(root.path().join("inventory.json"), "[]").unwrap();
        Self {
            root,
            home,
            workspace,
        }
    }
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_codex-sandbox"));
        c.current_dir(&self.workspace)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("SMOLVM_TEST_HOME", &self.home);
        for name in [
            "BASE_SMOLFILE",
            "ARTIFACTS_DIR",
            "STATE_ROOT",
            "CODEX_HOME_DIR",
            "BUILDER_MACHINE",
            "LOGIN_MACHINE",
            "CODEX_MACHINE",
            "OPENAI_API_KEY",
            "SMOLVM_BIN",
        ] {
            c.env_remove(name);
        }
        c.env("SMOLVM_BIN", env!("CARGO_BIN_EXE_smolvm-test-double"))
            .env("SMOLVM_TEST_VERSION", "smolvm test 1.19.2")
            .env("SMOLVM_TEST_LOG", self.root.path().join("smolvm.log"))
            .env("SMOLVM_TEST_ARGV_LOG", self.root.path().join("argv.jsonl"))
            .env(
                "SMOLVM_TEST_INVENTORY_FILE",
                self.root.path().join("inventory.json"),
            )
            .env("SMOLVM_TEST_ALLOW_CREATE", "1")
            .env("SMOLVM_TEST_ALLOW_LIFECYCLE", "1")
            .env("SMOLVM_TEST_ALLOW_CLEANUP", "1");
        c
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }
    fn configure(&self, harness: &str) {
        assert_ok(&self.run(&["create-config", "--harness", harness]));
    }
    fn definition(&self) -> PathBuf {
        self.home.join(".smolsandbox/default.smlbx")
    }
    #[cfg(feature = "test-adapter")]
    fn inventory(&self) -> Vec<Value> {
        serde_json::from_slice(&fs::read(self.root.path().join("inventory.json")).unwrap()).unwrap()
    }
    #[cfg(feature = "test-adapter")]
    fn set_inventory(&self, inventory: &[Value]) {
        fs::write(
            self.root.path().join("inventory.json"),
            serde_json::to_vec(inventory).unwrap(),
        )
        .unwrap();
    }
    fn log(&self) -> String {
        fs::read_to_string(self.root.path().join("smolvm.log"))
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.ends_with("--help") || line.contains(" -- "))
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[cfg(feature = "test-adapter")]
    fn clear_log(&self) {
        fs::write(self.root.path().join("smolvm.log"), "").unwrap();
    }
}
fn assert_ok(output: &Output) {
    assert!(
        output.status.success(),
        "status {:?}\nstdout {}\nstderr {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn copy_executable(from: &Path, to: &Path) {
    use std::io::Write;
    let mut file = fs::File::create(to).unwrap();
    file.write_all(&fs::read(from).unwrap()).unwrap();
    file.sync_all().unwrap();
    fs::set_permissions(to, fs::metadata(from).unwrap().permissions()).unwrap();
}

#[test]
fn help_and_version_are_independent_of_configuration() {
    let f = Fixture::new();
    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["build", "--help"],
        vec!["start", "--help"],
        vec!["repair", "--help"],
        vec!["clean", "--help"],
        vec!["purge", "--help"],
        vec!["login", "--help"],
        vec!["create-config", "--help"],
    ] {
        assert_ok(
            &f.command()
                .env("SMOLVM_BIN", f.root.path().join("missing"))
                .args(args)
                .output()
                .unwrap(),
        );
    }
    assert!(!f.home.join(".smolsandbox").exists());
    assert!(f.log().is_empty());
}

#[test]
fn usage_errors_do_not_create_state_or_call_smolvm() {
    let f = Fixture::new();
    for args in [
        vec!["login", "extra"],
        vec!["repair"],
        vec!["repair", "--check", "--force"],
        vec!["purge", "--force", "--force-all"],
        vec!["start", "--port", "0"],
        vec!["start", "--port", "65536"],
        vec!["create-config", "--harness", "other"],
    ] {
        assert_eq!(f.run(&args).status.code(), Some(2), "{args:?}");
    }
    let mut args = vec!["start".to_owned()];
    for port in 1..=65 {
        args.extend(["--port".into(), port.to_string()]);
    }
    assert_eq!(
        f.command().args(args).output().unwrap().status.code(),
        Some(2)
    );
    assert!(!f.home.join(".smolsandbox").exists());
    assert!(f.log().is_empty());
}

#[test]
fn every_template_is_offline_and_no_clobber() {
    for harness in ["claude", "codex", "opencode", "hermes"] {
        let f = Fixture::new();
        assert_ok(
            &f.command()
                .env("PATH", "")
                .env("SMOLVM_BIN", f.root.path().join("missing"))
                .args(["create-config", "--harness", harness])
                .output()
                .unwrap(),
        );
        let before = fs::read(f.definition()).unwrap();
        let parsed: toml::Value = toml::from_str(std::str::from_utf8(&before).unwrap()).unwrap();
        assert!(parsed.is_table());
        assert!(String::from_utf8_lossy(&before).contains(harness));
        assert!(!f
            .run(&["create-config", "--harness", "codex"])
            .status
            .success());
        assert_eq!(fs::read(f.definition()).unwrap(), before);
        assert!(f.log().is_empty());
    }
}

#[test]
fn concurrent_configuration_creation_publishes_one_complete_file() {
    let f = Fixture::new();
    let mut a = f
        .command()
        .args(["create-config", "--harness", "codex"])
        .spawn()
        .unwrap();
    let mut b = f
        .command()
        .args(["create-config", "--harness", "claude"])
        .spawn()
        .unwrap();
    let successes = [a.wait().unwrap(), b.wait().unwrap()]
        .iter()
        .filter(|s| s.success())
        .count();
    assert_eq!(successes, 1);
    toml::from_str::<toml::Value>(&fs::read_to_string(f.definition()).unwrap()).unwrap();
}

#[test]
fn copied_binary_generates_configuration_without_adjacent_resources() {
    let f = Fixture::new();
    let destination = f
        .root
        .path()
        .join(format!("relocated{}", std::env::consts::EXE_SUFFIX));
    copy_executable(Path::new(env!("CARGO_BIN_EXE_codex-sandbox")), &destination);
    let mut command = Command::new(destination);
    command
        .env("HOME", &f.home)
        .env("USERPROFILE", &f.home)
        .env("SMOLVM_TEST_HOME", &f.home)
        .env("PATH", "")
        .current_dir(f.root.path())
        .args(["create-config", "--harness", "codex"]);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    let output = loop {
        match command.output() {
            Ok(output) => break output,
            // Parallel test subprocesses may briefly inherit the copy's writable
            // descriptor between fork and exec, despite close-on-exec.
            Err(error)
                if cfg!(target_os = "linux")
                    && error.raw_os_error() == Some(26)
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => panic!("cannot execute copied binary: {error}"),
        }
    };
    assert_ok(&output);
    fs::write(f.definition(), "user edits\n").unwrap();
    assert_ok(&f.run(&["--version"]));
    assert_eq!(fs::read_to_string(f.definition()).unwrap(), "user edits\n");
}

#[test]
fn invalid_and_missing_definitions_fail_without_lifecycle_mutation() {
    let f = Fixture::new();
    let missing = f.run(&["build"]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("create-config"));
    f.configure("codex");
    for definition in [
        "garbage",
        "[broken",
        "# codex-sandbox-harness: invalid\n[vm]\nmemory=1024\n",
    ] {
        fs::write(f.definition(), definition).unwrap();
        assert!(!f.run(&["build"]).status.success());
    }
    assert!(f.log().is_empty());
}

#[test]
fn discovery_uses_workspace_repository_then_home_and_exact_names() {
    let f = Fixture::new();
    f.configure("codex");
    fs::create_dir_all(f.workspace.join(".git")).unwrap();
    fs::create_dir_all(f.workspace.join(".smolsandbox")).unwrap();
    let repo_definition = f.workspace.join(".smolsandbox/default.smlbx");
    let mut bytes = fs::read(f.definition()).unwrap();
    bytes.extend_from_slice(b"\n# repository definition\n");
    fs::write(&repo_definition, &bytes).unwrap();
    let selected = f
        .command()
        .current_dir(f.root.path())
        .args(["repair", "--check"])
        .arg(&f.workspace)
        .output()
        .unwrap();
    assert_ok(&selected);
    assert!(
        String::from_utf8_lossy(&selected.stdout).contains(&repo_definition.display().to_string())
    );
    fs::write(&repo_definition, "invalid nearer definition").unwrap();
    assert!(!f.run(&["repair", "--check"]).status.success());
    fs::remove_file(&repo_definition).unwrap();
    let selected = f.run(&["repair", "--check"]);
    assert_ok(&selected);
    assert!(
        String::from_utf8_lossy(&selected.stdout).contains(&f.definition().display().to_string())
    );
    fs::copy(f.definition(), f.home.join(".smolsandbox/custom")).unwrap();
    assert_ok(&f.run(&["repair", "--check", "--smolfile", "custom"]));
    assert!(!f
        .run(&["repair", "--check", "--smolfile", "custom.smlbx"])
        .status
        .success());
    assert!(!f
        .run(&["repair", "--check", "--smolfile", "absent"])
        .status
        .success());
}

#[test]
fn explicit_paths_use_invocation_directory_and_flag_precedes_environment() {
    let f = Fixture::new();
    f.configure("codex");
    fs::copy(f.definition(), f.root.path().join("explicit.smlbx")).unwrap();
    let output = f
        .command()
        .current_dir(f.root.path())
        .env("BASE_SMOLFILE", "missing")
        .args(["repair", "--check", "--smolfile", "./explicit.smlbx"])
        .arg(&f.workspace)
        .output()
        .unwrap();
    assert_ok(&output);
    let output = f
        .command()
        .current_dir(f.root.path())
        .env("BASE_SMOLFILE", "explicit.smlbx")
        .args(["repair", "--check"])
        .arg(&f.workspace)
        .output()
        .unwrap();
    assert_ok(&output);
    let output = f
        .command()
        .env("BASE_SMOLFILE", "default.smlbx")
        .args(["repair", "--check"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_ok(
        &f.command()
            .env("BASE_SMOLFILE", "missing")
            .args(["repair", "--check", "--smolfile", "default.smlbx"])
            .output()
            .unwrap(),
    );
}

#[test]
fn identity_uses_selected_complete_bytes_and_ignores_definition_location() {
    let f = Fixture::new();
    f.configure("codex");
    let identity = |output: Output| {
        assert_ok(&output);
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .find_map(|line| {
                line.strip_prefix("Desired workspace machine: ")
                    .map(str::to_owned)
            })
            .unwrap()
    };
    let first = identity(f.run(&["repair", "--check"]));
    fs::copy(f.definition(), f.workspace.join("other.smlbx")).unwrap();
    assert_eq!(
        identity(f.run(&["repair", "--check", "--smolfile", "./other.smlbx"])),
        first
    );
    fs::write(
        f.home.join(".smolsandbox/unused.smlbx"),
        "invalid unselected file",
    )
    .unwrap();
    assert_eq!(identity(f.run(&["repair", "--check"])), first);
    let mut bytes = fs::read(f.definition()).unwrap();
    bytes.extend_from_slice(b"\n# user edit\n");
    fs::write(f.definition(), bytes).unwrap();
    assert_ne!(identity(f.run(&["repair", "--check"])), first);
}

#[cfg(feature = "test-adapter")]
mod lifecycle {
    use super::*;
    fn workspace_machine(f: &Fixture) -> Value {
        f.inventory()
            .into_iter()
            .find(|m| m["labels"]["codex-sandbox.kind"] == "workspace")
            .unwrap()
    }

    #[test]
    fn start_recovers_machine_and_reconciles_exact_ports_without_changing_identity() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&[
            "start", "--port", "00080", "--port", "80", "--port", "3000", "--", "--help", "a b", "",
        ]));
        let first = workspace_machine(&f);
        assert_eq!(first["_test_ports"], json!([80, 3000]));
        f.clear_log();
        assert_ok(&f.run(&["start", "--port", "5173", "--", "--help"]));
        let next = workspace_machine(&f);
        assert_eq!(next["name"], first["name"]);
        assert_eq!(next["_test_ports"], json!([5173]));
        assert!(!f.log().contains("machine create"));
        assert_ok(&f.run(&["start", "--", "--help"]));
        assert_eq!(workspace_machine(&f)["_test_ports"], json!([]));
        let argv = fs::read_to_string(f.root.path().join("argv.jsonl")).unwrap();
        let invocation: Vec<String> = argv
            .lines()
            .map(|l| serde_json::from_str::<Vec<String>>(l).unwrap())
            .find(|a| a.iter().any(|v| v == "a b"))
            .unwrap();
        assert_eq!(&invocation[invocation.len() - 3..], ["--help", "a b", ""]);
    }

    #[test]
    fn allowlist_is_a_set_and_ports_do_not_change_immutable_hash() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&[
            "start",
            "--allow-host",
            "z.example",
            "--allow-host",
            "a.example",
        ]));
        let first = workspace_machine(&f);
        assert_ok(&f.run(&[
            "start",
            "--allow-host",
            "a.example",
            "--allow-host",
            "z.example",
            "--allow-host",
            "z.example",
            "--port",
            "3000",
        ]));
        assert_eq!(workspace_machine(&f)["name"], first["name"]);
        assert_eq!(
            workspace_machine(&f)["labels"]["codex-sandbox.configuration"],
            "custom"
        );
    }

    #[test]
    fn malformed_inventory_never_authorizes_creation_or_deletion() {
        for raw in [
            "{}",
            "null",
            "[{\"name\":\"x\",\"labels\":3}]",
            "[{\"name\":\"x\"},{\"name\":\"x\"}]",
            "[",
            "[{\"name\":\"x\",\"state\":false}]",
        ] {
            let f = Fixture::new();
            f.configure("codex");
            fs::write(f.root.path().join("inventory.json"), raw).unwrap();
            assert!(!f.run(&["start"]).status.success(), "{raw}");
            assert!(!f.log().contains("machine create"));
            assert!(!f.log().contains("machine delete"));
        }
    }

    #[test]
    fn replacement_prepares_and_verifies_before_deleting_stale_machine() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let stale = workspace_machine(&f)["name"].as_str().unwrap().to_owned();
        let mut inv = f.inventory();
        for machine in &mut inv {
            if machine["name"] == stale {
                machine["name"] = json!(stale.replace("-r5-", "-r4-"));
                machine["labels"]["codex-sandbox.revision"] = json!("4");
            }
        }
        f.set_inventory(&inv);
        let stale = stale.replace("-r5-", "-r4-");
        f.clear_log();
        let refused = f.run(&["start"]);
        assert!(!refused.status.success());
        assert!(String::from_utf8_lossy(&refused.stderr).contains(&stale));
        assert!(!f.log().contains("machine create"));
        f.clear_log();
        assert_ok(&f.run(&["repair", "--replace-stale"]));
        let log = f.log();
        let create = log.find("machine create").unwrap();
        let delete = log.find(&format!("machine delete --name {stale}")).unwrap();
        assert!(create < delete);
        assert!(log[create..delete].contains("machine ls --json"));
        assert!(!f.inventory().iter().any(|m| m["name"] == stale));
    }

    #[test]
    fn failed_replacement_preserves_stale_machine_and_caches() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let before = f.inventory();
        let cache = f.home.join(".smolsandbox/state/base-id");
        let before_cache = fs::read(&cache).unwrap();
        let mut definition = fs::read_to_string(f.definition()).unwrap();
        definition.push_str("\n# changed\n");
        fs::write(f.definition(), definition).unwrap();
        f.clear_log();
        let output = f
            .command()
            .env("SMOLVM_TEST_PACK_FAIL", "1")
            .args(["repair", "--replace-stale"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        for machine in before {
            assert!(f.inventory().iter().any(|m| m["name"] == machine["name"]));
        }
        assert_eq!(fs::read(cache).unwrap(), before_cache);
        assert!(!f.log().contains("machine delete"));
    }

    #[test]
    fn failed_registered_replacement_is_preserved_for_inspection() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let desired = workspace_machine(&f)["name"].as_str().unwrap().to_owned();
        let stale = desired.replace("-r5-", "-r4-");
        let mut inventory = f.inventory();
        for machine in &mut inventory {
            if machine["name"] == desired {
                machine["name"] = json!(stale);
                machine["labels"]["codex-sandbox.revision"] = json!("4");
            }
        }
        f.set_inventory(&inventory);
        let cache = f.home.join(".smolsandbox/state/generation.json");
        let before = fs::read(&cache).unwrap();
        f.clear_log();
        let output = f
            .command()
            .env("SMOLVM_TEST_CREATE_FAIL_AFTER", "1")
            .args(["repair", "--replace-stale"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("preserved for inspection"));
        let inventory = f.inventory();
        assert!(inventory.iter().any(|machine| machine["name"] == desired));
        assert!(inventory.iter().any(|machine| machine["name"] == stale));
        assert_eq!(fs::read(cache).unwrap(), before);
        let log = f.log();
        assert!(!log.contains("machine delete --name"));
        let creation = log.find("machine create --name").unwrap();
        assert!(log[creation..].contains("machine ls --json"));
    }

    #[test]
    fn running_port_mismatch_is_refused_without_stopping_machine() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start", "--port", "3000"]));
        let mut inv = f.inventory();
        for m in &mut inv {
            if m["labels"]["codex-sandbox.kind"] == "workspace" {
                m["state"] = json!("running");
            }
        }
        f.set_inventory(&inv);
        f.clear_log();
        assert!(!f.run(&["start", "--port", "4000"]).status.success());
        assert!(!f.log().contains("machine stop"));
        assert!(!f.log().contains("machine update --name"));
    }

    #[test]
    fn interrupted_port_update_is_recovered_before_next_start() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start", "--port", "3000"]));
        let output = f
            .command()
            .env("SMOLVM_TEST_UPDATE_FAIL_AFTER", "1")
            .args(["start", "--port", "4000"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_ok(&f.run(&["start", "--port", "5000"]));
        assert_eq!(workspace_machine(&f)["_test_ports"], json!([5000]));
    }

    #[test]
    fn guest_status_and_cleanup_failure_precedence() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let output = f
            .command()
            .env("SMOLVM_TEST_EXEC_STATUS", "23")
            .env("SMOLVM_TEST_STOP_FAIL", "1")
            .arg("start")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(23));
        let mut inv = f.inventory();
        for m in &mut inv {
            m["state"] = json!("stopped");
        }
        f.set_inventory(&inv);
        let output = f
            .command()
            .env("SMOLVM_TEST_STOP_FAIL", "1")
            .arg("start")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
    }

    #[test]
    fn profiles_mount_only_selected_state_and_forward_only_codex_secret_reference() {
        for harness in ["codex", "claude", "opencode", "hermes"] {
            let f = Fixture::new();
            f.configure(harness);
            let output = f
                .command()
                .env("OPENAI_API_KEY", "private-never-print-this")
                .args(["start", "--", "--help"])
                .output()
                .unwrap();
            assert_ok(&output);
            let log = f.log();
            assert!(!log.contains("private-never-print-this"));
            assert!(!String::from_utf8_lossy(&output.stdout).contains("private-never-print-this"));
            assert_eq!(
                log.contains("--secret-env OPENAI_API_KEY=OPENAI_API_KEY"),
                harness == "codex"
            );
            assert_eq!(log.contains(":/root/.codex"), harness == "codex");
            assert_eq!(log.contains("--no-daemon"), harness == "codex");
        }
    }

    #[test]
    fn check_is_lock_free_and_does_not_write_application_state() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["repair", "--check"]));
        assert!(!f.home.join(".smolsandbox/state").exists());
        assert!(!f.home.join(".smolsandbox/locks").exists());
        assert!(!f.log().contains("machine create"));
    }

    #[test]
    fn clean_workspace_preserves_artifacts_definitions_and_harness_home() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["build"]));
        let artifact_dir = f.home.join(".smolsandbox/artifacts");
        let before = fs::read_dir(&artifact_dir).unwrap().count();
        assert_ok(&f.run(&["clean", "--workspace"]));
        assert!(!f
            .inventory()
            .iter()
            .any(|m| m["labels"]["codex-sandbox.kind"] == "workspace"));
        assert!(f.definition().exists());
        assert!(f.home.join(".smolsandbox/codex-home").exists());
        assert_eq!(fs::read_dir(artifact_dir).unwrap().count(), before);
    }

    #[test]
    fn purge_policies_preserve_running_machines_and_current_artifact() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let default = workspace_machine(&f)["name"].as_str().unwrap().to_owned();
        assert_ok(&f.run(&["start", "--allow-host", "custom.example"]));
        f.clear_log();
        assert_eq!(f.run(&["purge"]).status.code(), Some(3));
        assert!(!f.log().contains("machine delete --name"));
        assert_ok(&f.run(&["purge", "--force"]));
        assert_eq!(
            f.inventory()
                .iter()
                .filter(|m| m["labels"]["codex-sandbox.kind"] == "workspace")
                .count(),
            1
        );
        assert!(f.inventory().iter().any(|m| m["name"] == default));
        let mut inventory = f.inventory();
        for m in &mut inventory {
            if m["name"] == default {
                m["state"] = json!("running");
            }
        }
        f.set_inventory(&inventory);
        f.clear_log();
        assert_ok(&f.run(&["purge", "--force-all"]));
        assert!(f.inventory().iter().any(|m| m["name"] == default));
        assert!(!f.log().contains("machine stop --name"));
        assert!(fs::read_dir(f.home.join(".smolsandbox/artifacts"))
            .unwrap()
            .next()
            .is_some());
    }

    #[test]
    fn failed_delete_retains_port_state_and_definition() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start", "--port", "3000"]));
        let machine = workspace_machine(&f)["name"].as_str().unwrap().to_owned();
        let manifest = f
            .home
            .join(".smolsandbox/state/ports")
            .join(format!("{machine}.json"));
        assert!(manifest.exists());
        let output = f
            .command()
            .env("SMOLVM_TEST_DELETE_FAIL_NAME", &machine)
            .args(["clean", "--workspace"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(manifest.exists());
        assert!(f.definition().exists());
        assert!(f.inventory().iter().any(|m| m["name"] == machine));
        assert_ok(&f.run(&["clean", "--workspace"]));
        assert!(!manifest.exists());
    }

    #[test]
    fn login_requires_existing_machine_and_uses_selected_authentication() {
        for harness in ["codex", "claude", "opencode", "hermes"] {
            let f = Fixture::new();
            f.configure(harness);
            assert!(!f.run(&["login"]).status.success());
            assert!(!f.log().contains("machine create --name"));
            assert_ok(&f.run(&["build"]));
            f.clear_log();
            assert_ok(&f.run(&["login"]));
            let expected = match harness {
                "codex" => "codex login --device-auth",
                "claude" => "claude auth login",
                "opencode" => "opencode auth login",
                _ => "hermes setup",
            };
            assert!(f.log().contains(expected), "{}", f.log());
            assert!(f.log().contains("machine stop --name"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn termination_during_start_seed_and_guest_stops_the_started_machine() {
        use std::{
            process::Stdio,
            time::{Duration, Instant},
        };
        unsafe extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
        }
        for stage in ["START", "SEED", "EXEC"] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start"]));
            f.clear_log();
            let ready = f.root.path().join("ready");
            let child = f
                .command()
                .env(format!("SMOLVM_TEST_{stage}_SLEEP_MS"), "10000")
                .env(format!("SMOLVM_TEST_{stage}_READY"), &ready)
                .arg("start")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let start = Instant::now();
            while !ready.exists() {
                assert!(
                    start.elapsed() < Duration::from_secs(10),
                    "{stage} never became ready"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
            assert_eq!(unsafe { kill(child.id() as i32, 15) }, 0);
            let output = child.wait_with_output().unwrap();
            assert!(!output.status.success(), "{stage}");
            assert!(
                f.log().contains("machine stop --name"),
                "stage {stage}: {}",
                f.log()
            );
            assert_eq!(workspace_machine(&f)["state"], "stopped", "{stage}");
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn differential_start_matches_shell_creation_labels_order_and_guest_status() {
        use codex_sandbox::domain::hash_bytes;
        let f = Fixture::new();
        f.configure("codex");
        let artifact_dir = f.home.join(".smolsandbox/artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        let mut bytes = b"smolvm test 1.19.2\n".to_vec();
        bytes.extend_from_slice(&fs::read(f.definition()).unwrap());
        let base = hash_bytes(&bytes)[..12].to_owned();
        fs::write(
            artifact_dir.join(format!("codex-devtools-{base}.smolmachine")),
            "prepared artifact",
        )
        .unwrap();
        let bin = f.root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_smolvm-test-double"), bin.join("smolvm"))
            .unwrap();
        let path = std::env::join_paths(
            std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let base_command = f.command();
        let mut old = Command::new("bash");
        for (key, value) in base_command.get_envs() {
            match value {
                Some(value) => {
                    old.env(key, value);
                }
                None => {
                    old.env_remove(key);
                }
            }
        }
        old.current_dir(&f.workspace)
            .env("PATH", path)
            .env("ARTIFACTS_DIR", &artifact_dir)
            .env("STATE_ROOT", f.home.join(".smolsandbox/state"))
            .env("CODEX_HOME_DIR", f.home.join(".smolsandbox/codex-home"))
            .env("BASE_SMOLFILE", f.definition())
            .env("SMOLVM_TEST_EXEC_STATUS", "17");
        let output = old
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("start-codex.sh"))
            .args(["--port", "3000", "--", "--help"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(17),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mutating = |log: &str| -> Vec<Vec<String>> {
            log.lines()
                .map(|line| serde_json::from_str::<Vec<String>>(line).unwrap())
                .filter(|argv| {
                    argv.first().is_some_and(|s| s == "machine")
                        && !matches!(argv.get(1).map(String::as_str), Some("ls"))
                        && !argv.iter().any(|s| s == "--help")
                        || argv.iter().any(|s| s == "--no-daemon")
                })
                .collect()
        };
        let old_calls = mutating(&fs::read_to_string(f.root.path().join("argv.jsonl")).unwrap());
        f.set_inventory(&[]);
        fs::remove_dir_all(f.home.join(".smolsandbox/state")).unwrap();
        fs::write(f.root.path().join("argv.jsonl"), "").unwrap();
        let output = f
            .command()
            .env("SMOLVM_TEST_EXEC_STATUS", "17")
            .args(["start", "--port", "3000", "--", "--help"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(17));
        let new_calls = mutating(&fs::read_to_string(f.root.path().join("argv.jsonl")).unwrap());
        assert_eq!(new_calls, old_calls);
    }
}
