#![cfg(any(not(windows), feature = "test-adapter"))]

#[cfg(feature = "test-adapter")]
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

#[cfg(feature = "test-adapter")]
#[path = "support/verbose.rs"]
mod verbose;

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
        let mut c = Command::new(env!("CARGO_BIN_EXE_smolsandbox"));
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
        self.home.join(".smolsandbox").join("default.smlbx")
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
        let output = f
            .command()
            .env("PATH", "")
            .env("SMOLVM_BIN", f.root.path().join("missing"))
            .args(["create-config", "--harness", harness])
            .output()
            .unwrap();
        assert_ok(&output);
        assert!(String::from_utf8_lossy(&output.stdout)
            .contains("Next: smolsandbox build\nThen: smolsandbox start"));
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
fn named_configuration_uses_invocation_directory_without_changing_default_or_permissions() {
    for harness in ["claude", "codex", "opencode", "hermes"] {
        let f = Fixture::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&f.workspace, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let output = f
            .command()
            .env("PATH", "")
            .env("SMOLVM_BIN", f.root.path().join("missing"))
            .env("BASE_SMOLFILE", f.root.path().join("ignored.smlbx"))
            .args([
                "create-config",
                "--harness",
                harness,
                "--smolfile",
                "named.smlbx",
            ])
            .output()
            .unwrap();
        assert_ok(&output);
        assert!(String::from_utf8_lossy(&output.stdout).contains("--smolfile"));
        let destination = f.workspace.join("named.smlbx");
        let bytes = fs::read(&destination).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(
            text.lines().next().unwrap(),
            format!("# codex-sandbox-harness: {harness}")
        );
        toml::from_str::<toml::Value>(text).unwrap();
        assert!(!f.home.join(".smolsandbox").exists());
        assert!(!f
            .run(&[
                "create-config",
                "--harness",
                "codex",
                "--smolfile",
                "named.smlbx"
            ])
            .status
            .success());
        assert_eq!(fs::read(&destination).unwrap(), bytes);
        assert_eq!(fs::read_dir(&f.workspace).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&f.workspace).unwrap().permissions().mode() & 0o777,
                0o755
            );
            assert_eq!(
                fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(f.log().is_empty());
    }
}

#[test]
fn named_configuration_accepts_absolute_paths_and_creates_parent_directories() {
    let f = Fixture::new();
    f.configure("codex");
    let default = fs::read(f.definition()).unwrap();
    let destination = f.root.path().join("custom configs/claude.smlbx");
    assert_ok(
        &f.command()
            .args(["create-config", "--harness", "claude", "--smolfile"])
            .arg(&destination)
            .output()
            .unwrap(),
    );
    assert_eq!(
        fs::read_to_string(destination)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
        "# codex-sandbox-harness: claude"
    );
    assert_eq!(fs::read(f.definition()).unwrap(), default);
    assert_eq!(fs::read_dir(&f.workspace).unwrap().count(), 0);
    assert!(f.log().is_empty());
}

#[test]
fn named_configuration_preserves_existing_destination_directories() {
    let f = Fixture::new();
    let destination = f.workspace.join("claude.smlbx");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("sentinel"), "preserve").unwrap();
    assert!(!f
        .run(&[
            "create-config",
            "--harness",
            "claude",
            "--smolfile",
            "claude.smlbx"
        ])
        .status
        .success());
    assert_eq!(
        fs::read_to_string(destination.join("sentinel")).unwrap(),
        "preserve"
    );
    assert_eq!(fs::read_dir(&f.workspace).unwrap().count(), 1);
    assert!(!f.home.join(".smolsandbox").exists());
    assert!(f.log().is_empty());
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
    copy_executable(Path::new(env!("CARGO_BIN_EXE_smolsandbox")), &destination);
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
    assert!(String::from_utf8_lossy(&missing.stderr)
        .contains("Run smolsandbox create-config --harness codex"));
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
    let assert_selected_definition = |output: &Output, expected: &Path| {
        assert_ok(output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let selected = stdout
            .lines()
            .find_map(|line| line.strip_prefix("Selected definition: "))
            .and_then(|line| line.strip_suffix(" (Codex)"))
            .unwrap_or_else(|| panic!("missing selected Codex definition: {stdout}"));
        assert_eq!(
            Path::new(selected).canonicalize().unwrap(),
            expected.canonicalize().unwrap(),
            "{stdout}"
        );
    };
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
    assert_selected_definition(&selected, &repo_definition);
    fs::write(&repo_definition, "invalid nearer definition").unwrap();
    assert!(!f.run(&["repair", "--check"]).status.success());
    fs::remove_file(&repo_definition).unwrap();
    let selected = f.run(&["repair", "--check"]);
    assert_selected_definition(&selected, &f.definition());
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

    fn calls(f: &Fixture) -> Vec<Vec<String>> {
        fs::read_to_string(f.root.path().join("argv.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
    fn reset_calls(f: &Fixture) {
        f.clear_log();
        fs::write(f.root.path().join("argv.jsonl"), "").unwrap();
    }
    fn assert_no_mutations(f: &Fixture) {
        assert!(
            !calls(f)
                .iter()
                .any(|a| a.first().is_some_and(|s| s == "machine")
                    && !a.iter().any(|s| s == "--help")
                    && matches!(
                        a.get(1).map(String::as_str),
                        Some("create" | "update" | "start" | "stop" | "delete" | "exec")
                    )),
            "{}",
            f.log()
        );
    }
    fn stale_machine(machine: &Value, revision: &str) -> Value {
        let mut stale = machine.clone();
        let base = stale["labels"]["codex-sandbox.base-id"].as_str().unwrap();
        let hash = stale["labels"]["codex-sandbox.workspace-sha256"]
            .as_str()
            .unwrap();
        stale["name"] = json!(format!(
            "codex-workspace-{base}-r{revision}-{}",
            &hash[..12]
        ));
        stale["labels"]["codex-sandbox.revision"] = json!(revision);
        stale
    }
    fn cache_files(f: &Fixture) -> Vec<(PathBuf, Vec<u8>)> {
        ["base-id", "machine-revision", "generation.json"]
            .into_iter()
            .map(|name| {
                let path = f.home.join(".smolsandbox/state").join(name);
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect()
    }
    fn assert_caches_unchanged(files: &[(PathBuf, Vec<u8>)]) {
        for (path, bytes) in files {
            assert_eq!(&fs::read(path).unwrap(), bytes, "{}", path.display());
        }
    }
    fn sequence(f: &Fixture, responses: &[String]) -> PathBuf {
        let path = f.root.path().join("sequence.jsonl");
        fs::write(
            &path,
            std::iter::once(responses[0].clone())
                .chain(responses.iter().cloned())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        path
    }
    struct ChildGuard {
        child: Option<std::process::Child>,
        release: Option<PathBuf>,
    }
    impl ChildGuard {
        fn spawn(command: &mut Command, release: Option<PathBuf>) -> Self {
            Self {
                child: Some(
                    command
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .spawn()
                        .unwrap(),
                ),
                release,
            }
        }
        fn wait_for(&mut self, ready: &Path) {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while !ready.exists() {
                assert!(
                    self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                    "child exited before readiness"
                );
                assert!(
                    std::time::Instant::now() < deadline,
                    "readiness timed out: {}",
                    ready.display()
                );
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        fn release(&self) {
            if let Some(path) = &self.release {
                fs::write(path, "release").unwrap();
            }
        }
        fn finish(mut self) -> Output {
            self.release();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            while self.child.as_mut().unwrap().try_wait().unwrap().is_none() {
                assert!(std::time::Instant::now() < deadline, "child did not finish");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            self.child.take().unwrap().wait_with_output().unwrap()
        }
    }
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            self.release();
            if let Some(child) = &mut self.child {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
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

        let latest_name = || {
            calls(&f)
                .into_iter()
                .rfind(|args| {
                    args.get(1).is_some_and(|arg| arg == "create")
                        && args.iter().any(|arg| arg == "codex-sandbox.kind=workspace")
                })
                .unwrap()[3]
                .clone()
        };
        assert_ok(&f.run(&["start", "--allow-host", "different.example"]));
        assert_ne!(latest_name(), first["name"].as_str().unwrap());
        let output = f
            .command()
            .env("CODEX_HOME_DIR", f.home.join("other-codex-home"))
            .args([
                "start",
                "--allow-host",
                "a.example",
                "--allow-host",
                "z.example",
            ])
            .output()
            .unwrap();
        assert_ok(&output);
        assert_ne!(latest_name(), first["name"].as_str().unwrap());
        let defaults = Fixture::new();
        defaults.configure("codex");
        assert_ok(&defaults.run(&["start"]));
        let before = workspace_machine(&defaults);
        let document: toml::Value =
            toml::from_str(&fs::read_to_string(defaults.definition()).unwrap()).unwrap();
        let host = document["network"]["allow_hosts"][0].as_str().unwrap();
        assert_ok(&defaults.run(&["start", "--allow-host", host, "--allow-host", host]));
        assert_eq!(workspace_machine(&defaults)["name"], before["name"]);
        assert_eq!(
            workspace_machine(&defaults)["labels"]["codex-sandbox.configuration"],
            "default"
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
        assert!(String::from_utf8_lossy(&refused.stderr).contains("run smolsandbox repair --check"));
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
        for verbose in [false, true] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start"]));
            let mut command = f.command();
            if verbose {
                command.arg("--verbose");
            }
            let output = command
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
            let mut command = f.command();
            if verbose {
                command.arg("--verbose");
            }
            let output = command
                .env("SMOLVM_TEST_STOP_FAIL", "1")
                .arg("start")
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(1));
        }
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
            let missing = f.run(&["login"]);
            assert!(!missing.status.success());
            assert!(
                String::from_utf8_lossy(&missing.stderr).contains("run smolsandbox build first")
            );
            assert!(!f.log().contains("machine create --name"));
            let built = f.run(&["build"]);
            assert_ok(&built);
            let stdout = String::from_utf8_lossy(&built.stdout);
            assert!(stdout.contains("Next: smolsandbox login"));
            assert!(stdout.contains("smolsandbox start "));
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
        for (stage, verbose) in ["START", "SEED", "EXEC"]
            .into_iter()
            .flat_map(|stage| [false, true].map(|verbose| (stage, verbose)))
        {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start"]));
            f.clear_log();
            let ready = f.root.path().join("ready");
            let mut command = f.command();
            command.env("SMOLVM_TEST_OUTPUT", "1");
            if verbose {
                command.arg("--verbose");
            }
            let child = command
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

    #[test]
    fn startup_has_independent_names_labels_order_arguments_and_exit_status() {
        use sha2::{Digest, Sha256};
        let digest = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
        let f = Fixture::new();
        f.configure("codex");
        let workspace = f
            .workspace
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        #[cfg(windows)]
        let workspace = if let Some(unc) = workspace.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else {
            workspace
                .strip_prefix(r"\\?\")
                .unwrap_or(&workspace)
                .to_owned()
        };
        let definition = fs::read(f.definition()).unwrap();
        let mut input = b"smolvm test 1.19.2\n".to_vec();
        input.extend_from_slice(&definition);
        let base = digest(&input)[..12].to_owned();
        let application_home = f.home.join(".smolsandbox");
        let artifact_dir = application_home.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        let artifact = artifact_dir.join(format!("codex-devtools-{base}.smolmachine"));
        fs::write(&artifact, "prepared artifact").unwrap();
        let text: toml::Value = toml::from_str(std::str::from_utf8(&definition).unwrap()).unwrap();
        let mut immutable = vec![
            format!("--from={}", artifact.display()),
            "--mem=12288".into(),
            "--cpus=4".into(),
            format!(
                "--volume={}:/root/.codex",
                application_home.join("codex-home").display()
            ),
            format!("--volume={}:/workspace/myrepo", workspace),
        ];
        immutable.extend(
            text["network"]["allow_hosts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|host| format!("--allow-host={}", host.as_str().unwrap())),
        );
        immutable.sort();
        immutable.dedup();
        let hash = digest(&serde_json::to_vec(&immutable).unwrap());
        let name = format!("codex-workspace-{base}-r5-{}", &hash[..12]);
        let output = f
            .command()
            .env("SMOLVM_TEST_EXEC_STATUS", "17")
            .args([
                "start", "--port", "3000", "--", "--help", "a b", "", "--port", "9000",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(17));
        let machine = workspace_machine(&f);
        assert_eq!(machine["name"], name);
        assert_eq!(
            machine["labels"],
            json!({
                "codex-sandbox.kind":"workspace", "codex-sandbox.base-id":base,
                "codex-sandbox.revision":"5", "codex-sandbox.workspace-sha256":hash,
                "codex-sandbox.workspace-path-sha256":digest(workspace.as_bytes()),
                "codex-sandbox.configuration":"default"
            })
        );
        let calls = calls(&f);
        let mutations: Vec<_> = calls
            .iter()
            .filter(|args| {
                args.first().is_some_and(|a| a == "machine")
                    && !matches!(args.get(1).map(String::as_str), Some("ls"))
                    && !args.iter().any(|a| a == "--help")
                    || args.iter().any(|a| a == "--no-daemon")
            })
            .collect();
        assert_eq!(
            mutations.iter().map(|a| a[1].as_str()).collect::<Vec<_>>(),
            ["create", "update", "start", "exec", "exec", "stop"]
        );
        for args in &mutations {
            assert_eq!(&args[..4], ["machine", args[1].as_str(), "--name", &name]);
        }
        let hosts: Vec<_> = mutations[0]
            .windows(2)
            .filter(|pair| pair[0] == "--allow-host")
            .map(|pair| pair[1].clone())
            .collect();
        let mut expected_hosts: Vec<_> = text["network"]["allow_hosts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|host| host.as_str().unwrap().to_owned())
            .collect();
        expected_hosts.sort();
        expected_hosts.dedup();
        assert_eq!(hosts, expected_hosts);
        for value in &immutable {
            let (flag, value) = value.split_once('=').unwrap();
            assert!(
                mutations[0].windows(2).any(|pair| pair == [flag, value]),
                "missing creation option {flag}={value}"
            );
        }
        assert_eq!(
            mutations[1].as_slice(),
            [
                "machine",
                "update",
                "--name",
                &name,
                "--remove-port",
                "3000:3000",
                "--port",
                "3000"
            ]
        );
        assert!(!mutations[0].iter().any(|a| a == "--port"));
        assert_eq!(&mutations[1][mutations[1].len() - 2..], ["--port", "3000"]);
        assert_eq!(
            &mutations[4][4..],
            [
                "-it",
                "--",
                "codex",
                "--no-daemon",
                "--help",
                "a b",
                "",
                "--port",
                "9000"
            ]
        );
        assert_eq!(machine["state"], "stopped");
    }
    #[test]
    fn concurrent_starts_serialize_port_update_verification_and_start() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start", "--port", "2000"]));
        reset_calls(&f);
        let trace = f.root.path().join("callers.jsonl");
        let first_ready = f.root.path().join("update.ready");
        let first_release = f.root.path().join("update.release");
        let mut first = ChildGuard::spawn(
            f.command()
                .env("SMOLVM_TEST_CALLER", "first")
                .env("SMOLVM_TEST_CALLER_LOG", &trace)
                .env("SMOLVM_TEST_UPDATE_GATE_READY", &first_ready)
                .env("SMOLVM_TEST_UPDATE_GATE_RELEASE", &first_release)
                .args(["start", "--port", "3000"]),
            Some(first_release),
        );
        first.wait_for(&first_ready);
        let second_ready = f.root.path().join("version.ready");
        let second_release = f.root.path().join("version.release");
        let contended = f.root.path().join("lock.contended");
        let mut second = ChildGuard::spawn(
            f.command()
                .env("SMOLVM_TEST_CALLER", "second")
                .env("SMOLVM_TEST_LOCK_CONTENDED", &contended)
                .env("SMOLVM_TEST_CALLER_LOG", &trace)
                .env("SMOLVM_TEST_VERSION_GATE_READY", &second_ready)
                .env("SMOLVM_TEST_VERSION_GATE_RELEASE", &second_release)
                .args(["start", "--port", "4000"]),
            Some(second_release),
        );
        second.wait_for(&second_ready);
        second.release();
        second.wait_for(&contended);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while fs::read_to_string(&contended).unwrap_or_default()
            != f.home
                .join(".smolsandbox")
                .join("locks")
                .join("artifacts.lock")
                .to_string_lossy()
        {
            assert!(
                std::time::Instant::now() < deadline,
                "second invocation did not contend for the lifecycle lock"
            );
            std::thread::yield_now();
        }
        let records: Vec<Value> = fs::read_to_string(&trace)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(!records.iter().any(|v| v["caller"] == "second"
            && matches!(v["args"][1].as_str(), Some("update" | "start"))
            && v["args"][2] == "--name"));
        assert_eq!(
            records
                .iter()
                .filter(|v| v["caller"] == "second"
                    && v["args"][1] == "ls"
                    && v["args"][2] == "--json")
                .count(),
            1
        );
        first.release();
        assert_ok(&first.finish());
        let output = second.finish();
        assert!(matches!(output.status.code(), Some(0 | 1)));
        let records: Vec<Value> = fs::read_to_string(trace)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let first_update = records
            .iter()
            .position(|v| {
                v["caller"] == "first" && v["args"][1] == "update" && v["args"][2] == "--name"
            })
            .unwrap();
        let first_verify = records
            .iter()
            .enumerate()
            .skip(first_update + 1)
            .find(|(_, v)| v["caller"] == "first" && v["args"][1] == "ls")
            .unwrap()
            .0;
        let first_start = records
            .iter()
            .position(|v| {
                v["caller"] == "first" && v["args"][1] == "start" && v["args"][2] == "--name"
            })
            .unwrap();
        let second_inventory = records
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                v["caller"] == "second" && v["args"][1] == "ls" && v["args"][2] == "--json"
            })
            .nth(1)
            .unwrap()
            .0;
        assert!(
            first_update < first_verify
                && first_verify < first_start
                && first_start < second_inventory
        );
        assert!(records
            .iter()
            .enumerate()
            .filter(|(_, v)| v["caller"] == "second"
                && matches!(v["args"][1].as_str(), Some("update" | "start"))
                && v["args"][2] == "--name")
            .all(|(index, _)| index > first_start));
    }

    #[test]
    fn port_update_and_verification_failures_preserve_recovery_and_never_start() {
        for flag in [
            "SMOLVM_TEST_UPDATE_FAIL",
            "SMOLVM_TEST_UPDATE_WRONG_COUNT",
            "SMOLVM_TEST_UPDATE_FAIL_AFTER",
        ] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start", "--port", "3000"]));
            let machine = workspace_machine(&f);
            let (manifest, journal) = smolsandbox::ports::paths(
                &f.home.join(".smolsandbox/state"),
                machine["name"].as_str().unwrap(),
            );
            let before = fs::read(&manifest).unwrap();
            reset_calls(&f);
            let output = f
                .command()
                .env(flag, "1")
                .args(["start", "--port", "4000"])
                .output()
                .unwrap();
            assert!(!output.status.success(), "{flag}");
            assert!(
                !calls(&f)
                    .iter()
                    .any(|a| a.get(1).is_some_and(|s| s == "start" || s == "exec")
                        && !a.iter().any(|s| s == "--help")),
                "{flag}: {}",
                f.log()
            );
            assert_eq!(fs::read(&manifest).unwrap(), before);
            assert!(journal.exists(), "{flag}");
            assert_eq!(workspace_machine(&f)["state"], "stopped");
            if flag == "SMOLVM_TEST_UPDATE_WRONG_COUNT" {
                // Restore the public mapping count to the retained journal's known set.
                let mut inv = f.inventory();
                for m in &mut inv {
                    if m["name"] == machine["name"] {
                        m["_test_ports"] = json!([4000]);
                        m["ports"] = json!(1);
                    }
                }
                f.set_inventory(&inv);
            }
            assert_ok(&f.run(&["start", "--port", "5000"]));
            assert_eq!(workspace_machine(&f)["_test_ports"], json!([5000]));
            assert!(!journal.exists());
        }
    }

    #[test]
    fn start_failure_skips_seed_and_harness_and_attempts_owned_stop() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        reset_calls(&f);
        let output = f
            .command()
            .env("SMOLVM_TEST_START_FAIL", "1")
            .arg("start")
            .output()
            .unwrap();
        assert!(!output.status.success());
        let calls = calls(&f);
        assert!(!calls
            .iter()
            .any(|a| a.get(1).is_some_and(|s| s == "exec") && !a.iter().any(|s| s == "--help")));
        assert!(calls
            .iter()
            .any(|a| a.get(1).is_some_and(|s| s == "stop")
                && a.get(2).is_some_and(|s| s == "--name")));
        assert_eq!(workspace_machine(&f)["state"], "stopped");
    }

    #[test]
    fn seed_commands_are_non_clobbering_and_failures_only_warn() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        reset_calls(&f);
        for fail in [false, true] {
            let output = f
                .command()
                .env("SMOLVM_TEST_SEED_FAIL", if fail { "1" } else { "0" })
                .env("SMOLVM_TEST_EXEC_STATUS", "19")
                .args(["start", "--", "--help"])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(19));
            assert_eq!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("could not seed persistent harness state; continuing"),
                fail
            );
        }
        let seeds: Vec<_> = calls(&f)
            .into_iter()
            .filter(|a| a.iter().any(|s| s == "sh"))
            .collect();
        assert_eq!(seeds.len(), 2);
        for seed in seeds {
            assert_eq!(&seed[4..],["--","sh","-lc","if [ -d /opt/codex-home ]; then cp -r --update=none /opt/codex-home/. /root/.codex/; fi"]);
        }
        assert_eq!(workspace_machine(&f)["state"], "stopped");
    }

    #[test]
    fn creation_uses_selected_default_hosts_plus_normalized_additions_and_revision() {
        let f = Fixture::new();
        f.configure("codex");
        fs::write(f.definition(),"# codex-sandbox-harness: codex\nimage='debian'\n[network]\nallow_hosts=['selected.example','duplicate.example','selected.example']\n").unwrap();
        assert_ok(&f.run(&[
            "start",
            "--allow-host",
            "z.example",
            "--allow-host",
            "duplicate.example",
            "--allow-host",
            "z.example",
        ]));
        let creation = calls(&f)
            .into_iter()
            .find(|a| {
                a.get(1).is_some_and(|s| s == "create")
                    && a.iter().any(|s| s == "codex-sandbox.kind=workspace")
            })
            .unwrap();
        let hosts: Vec<_> = creation
            .windows(2)
            .filter(|a| a[0] == "--allow-host")
            .map(|a| a[1].as_str())
            .collect();
        assert_eq!(
            hosts,
            ["duplicate.example", "selected.example", "z.example"]
        );
        assert!(creation.iter().any(|a| a == "codex-sandbox.revision=5"));
        assert_eq!(
            workspace_machine(&f)["labels"]["codex-sandbox.configuration"],
            "custom"
        );
    }
    #[test]
    fn inventory_command_failure_preserves_machines_and_caches() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let before = f.inventory();
        let caches = cache_files(&f);
        reset_calls(&f);
        let output = f
            .command()
            .env("SMOLVM_TEST_INVENTORY_FAIL", "1")
            .arg("start")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_no_mutations(&f);
        assert_eq!(f.inventory(), before);
        assert_caches_unchanged(&caches);
    }

    #[test]
    fn replacement_rejects_failed_creation_and_every_unready_refresh_without_cleanup() {
        for case in [
            "create",
            "refresh-error",
            "malformed",
            "missing",
            "duplicate",
            "contradictory",
            "failed",
            "override-contradictory",
        ] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start"]));
            let desired = workspace_machine(&f);
            let stale = stale_machine(&desired, "4");
            let caches = cache_files(&f);
            f.set_inventory(std::slice::from_ref(&stale));
            reset_calls(&f);
            let mut refreshed = desired.clone();
            if case == "contradictory" || case == "override-contradictory" {
                refreshed["labels"]["codex-sandbox.base-id"] = json!("111111111111");
            }
            if case == "failed" {
                refreshed["state"] = json!("failed");
            }
            let fresh = match case {
                "malformed" => "{}".to_owned(),
                "missing" => json!([stale]).to_string(),
                "duplicate" => json!([stale, desired, desired]).to_string(),
                _ => json!([stale, refreshed]).to_string(),
            };
            let sequence = sequence(&f, &[json!([stale]).to_string(), fresh]);
            let mut command = f.command();
            command.env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", &sequence);
            if case == "create" {
                command.env("SMOLVM_TEST_CREATE_FAIL", "1");
            }
            if case == "refresh-error" {
                command.env("SMOLVM_TEST_INVENTORY_FAIL_AT", "3");
            }
            if case == "override-contradictory" {
                command.env("CODEX_MACHINE", desired["name"].as_str().unwrap());
            }
            let output = command
                .args(["repair", "--replace-stale"])
                .output()
                .unwrap();
            assert!(!output.status.success(), "{case}");
            assert!(
                !f.log().contains("machine delete --name"),
                "{case}: {}",
                f.log()
            );
            assert!(
                !f.log().contains("machine stop --name"),
                "{case}: {}",
                f.log()
            );
            assert!(
                f.inventory().iter().any(|m| m["name"] == stale["name"]),
                "{case}"
            );
            assert_caches_unchanged(&caches);
        }
    }

    #[test]
    fn existing_desired_replacement_is_not_recreated_and_preserves_unrelated_machines() {
        for state in ["created", "stopped", "running", "failed"] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start"]));
            let mut desired = workspace_machine(&f);
            desired["state"] = json!(state);
            let stale = stale_machine(&desired, "4");
            let unrelated = json!({"name":"unrelated","state":"running","labels":{}});
            f.set_inventory(&[desired.clone(), stale.clone(), unrelated.clone()]);
            let state_root = f.home.join(".smolsandbox/state");
            fs::write(state_root.join("base-id"), "stale-base\n").unwrap();
            fs::write(state_root.join("machine-revision"), "99\n").unwrap();
            let caches = cache_files(&f);
            reset_calls(&f);
            let output = f.run(&["repair", "--replace-stale", "--force"]);
            assert_eq!(output.status.success(), state != "failed", "{state}");
            assert!(!f.log().contains("machine create --name"), "{state}");
            assert!(f.inventory().contains(&unrelated));
            assert_eq!(
                f.inventory().iter().any(|m| m["name"] == stale["name"]),
                state == "failed"
            );
            assert!(f.inventory().iter().any(|m| m["name"] == desired["name"]));
            if state == "failed" {
                assert_caches_unchanged(&caches);
            } else {
                assert_eq!(
                    fs::read_to_string(state_root.join("base-id")).unwrap(),
                    format!(
                        "{}\n",
                        desired["labels"]["codex-sandbox.base-id"].as_str().unwrap()
                    )
                );
                assert_eq!(
                    fs::read_to_string(state_root.join("machine-revision")).unwrap(),
                    "5\n"
                );
            }
        }
    }

    #[test]
    fn running_stale_replacement_requires_force_and_stops_before_deleting() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let desired = workspace_machine(&f);
        let mut stale = stale_machine(&desired, "4");
        stale["state"] = json!("running");
        f.set_inventory(std::slice::from_ref(&stale));
        let caches = cache_files(&f);
        reset_calls(&f);
        assert!(!f.run(&["repair", "--replace-stale"]).status.success());
        assert_no_mutations(&f);
        assert_caches_unchanged(&caches);
        assert_ok(&f.run(&["repair", "--replace-stale", "--force"]));
        let log = f.log();
        let create = log.find("machine create --name").unwrap();
        let stop = log
            .find(&format!(
                "machine stop --name {}",
                stale["name"].as_str().unwrap()
            ))
            .unwrap();
        let delete = log
            .find(&format!(
                "machine delete --name {}",
                stale["name"].as_str().unwrap()
            ))
            .unwrap();
        assert!(create < stop && stop < delete);
    }

    #[test]
    fn replacement_partial_cleanup_failure_preserves_desired_later_machines_and_caches() {
        for operation in ["DELETE", "STOP"] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start"]));
            let desired = workspace_machine(&f);
            let first = stale_machine(&desired, "3");
            let mut second = stale_machine(&desired, "4");
            if operation == "STOP" {
                second["state"] = json!("running");
            }
            let later = stale_machine(&desired, "2");
            f.set_inventory(&[first.clone(), second.clone(), later.clone()]);
            let caches = cache_files(&f);
            reset_calls(&f);
            let output = f
                .command()
                .env(
                    format!("SMOLVM_TEST_{operation}_FAIL_NAME"),
                    second["name"].as_str().unwrap(),
                )
                .args(["repair", "--replace-stale", "--force"])
                .output()
                .unwrap();
            assert!(!output.status.success());
            let inventory = f.inventory();
            assert!(!inventory.iter().any(|m| m["name"] == first["name"]));
            for kept in [&desired, &second, &later] {
                assert!(inventory.iter().any(|m| m["name"] == kept["name"]));
            }
            assert!(!f.log().contains(&format!(
                "machine delete --name {}",
                later["name"].as_str().unwrap()
            )));
            if operation == "STOP" {
                assert!(!f.log().contains(&format!(
                    "machine delete --name {}",
                    second["name"].as_str().unwrap()
                )));
            }
            assert_caches_unchanged(&caches);
        }
    }

    #[test]
    fn explicit_machine_override_accepts_legacy_ownership_and_rejects_contradictions() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let mut machine = workspace_machine(&f);
        machine["name"] = json!("explicit-name");
        machine["labels"] = json!({});
        f.set_inventory(std::slice::from_ref(&machine));
        reset_calls(&f);
        assert_ok(
            &f.command()
                .env("CODEX_MACHINE", "explicit-name")
                .arg("start")
                .output()
                .unwrap(),
        );
        assert!(!f.log().contains("machine create --name"));
        machine["labels"] = json!({"codex-sandbox.kind":"login"});
        f.set_inventory(&[machine]);
        reset_calls(&f);
        assert!(!f
            .command()
            .env("CODEX_MACHINE", "explicit-name")
            .args(["repair", "--replace-stale"])
            .output()
            .unwrap()
            .status
            .success());
        assert_no_mutations(&f);
    }

    #[test]
    fn concurrent_creation_and_replacement_register_and_delete_each_machine_once() {
        for replacement in [false, true] {
            let f = Fixture::new();
            f.configure("codex");
            let mut stale = None;
            if replacement {
                assert_ok(&f.run(&["start"]));
                let m = stale_machine(&workspace_machine(&f), "4");
                stale = Some(m["name"].as_str().unwrap().to_owned());
                f.set_inventory(&[m]);
            }
            reset_calls(&f);
            let args = if replacement {
                vec!["repair", "--replace-stale"]
            } else {
                vec!["start"]
            };
            let first = ChildGuard::spawn(f.command().args(&args), None);
            let second = ChildGuard::spawn(f.command().args(&args), None);
            assert_ok(&first.finish());
            assert_ok(&second.finish());
            let calls = calls(&f);
            assert_eq!(
                calls
                    .iter()
                    .filter(|a| a.get(1).is_some_and(|s| s == "create")
                        && a.iter().any(|s| s == "codex-sandbox.kind=workspace"))
                    .count(),
                1
            );
            if let Some(stale) = stale {
                assert_eq!(
                    calls
                        .iter()
                        .filter(|a| a.get(1).is_some_and(|s| s == "delete")
                            && a.iter().any(|s| s == &stale))
                        .count(),
                    1
                );
            }
        }
    }

    #[test]
    fn generation_clean_removes_only_current_machines_artifacts_and_caches() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["build"]));
        let workspace = workspace_machine(&f);
        let base = workspace["labels"]["codex-sandbox.base-id"]
            .as_str()
            .unwrap();
        let artifact_dir = f.home.join(".smolsandbox/artifacts");
        fs::write(
            artifact_dir.join(format!("codex-devtools-{base}")),
            "legacy artifact",
        )
        .unwrap();
        let other_artifact = artifact_dir.join("codex-devtools-fedcba987654.smolmachine");
        fs::write(&other_artifact, "keep").unwrap();
        let harness = f.home.join(".smolsandbox/codex-home/user-config");
        fs::write(&harness, "keep").unwrap();
        let mut inventory = f.inventory();
        inventory.extend([
            json!({"name":format!("codex-builder-{base}"),"state":"stopped","labels":{}}),
            json!({"name":format!("codex-login-{base}-r4"),"state":"stopped","labels":{}}),
            stale_machine(&workspace, "4"),
        ]);
        let other = json!({"name":"codex-builder-fedcba987654","state":"stopped","labels":{}});
        let unrelated = json!({"name":"unrelated-machine","state":"running","labels":{}});
        inventory.extend([other.clone(), unrelated.clone()]);
        f.set_inventory(&inventory);
        reset_calls(&f);
        let output = f.run(&["clean"]);
        assert_ok(&output);
        assert_eq!(f.inventory(), vec![other, unrelated]);
        assert!(String::from_utf8_lossy(&output.stdout)
            .contains("Removed 5 VM(s) and the cached artifact."));
        for suffix in ["", ".smolmachine"] {
            assert!(!artifact_dir
                .join(format!("codex-devtools-{base}{suffix}"))
                .exists());
        }
        for cache in ["base-id", "machine-revision", "generation.json"] {
            assert!(!f.home.join(".smolsandbox/state").join(cache).exists());
        }
        assert!(other_artifact.exists());
        assert!(harness.exists());
        assert!(f.definition().exists());
    }

    #[test]
    fn workspace_clean_spans_generations_preserves_other_paths_and_rejects_contradictions() {
        for contradictory in [false, true] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["build"]));
            let desired = workspace_machine(&f);
            let mut old = stale_machine(&desired, "3");
            old["labels"]["codex-sandbox.base-id"] = json!("fedcba987654");
            old["labels"]["codex-sandbox.configuration"] = json!("custom");
            old["name"] = json!(format!(
                "codex-workspace-fedcba987654-r3-{}",
                &old["labels"]["codex-sandbox.workspace-sha256"]
                    .as_str()
                    .unwrap()[..12]
            ));
            let mut other = stale_machine(&desired, "2");
            other["labels"]["codex-sandbox.workspace-path-sha256"] = json!("f".repeat(64));
            if contradictory {
                old["labels"]["codex-sandbox.workspace-sha256"] = json!("a".repeat(64));
            }
            let login = f
                .inventory()
                .into_iter()
                .find(|m| m["labels"]["codex-sandbox.kind"] == "login")
                .unwrap();
            f.set_inventory(&[desired.clone(), old.clone(), other.clone(), login.clone()]);
            let caches = cache_files(&f);
            let before = fs::read_dir(f.home.join(".smolsandbox/artifacts"))
                .unwrap()
                .count();
            let (_, pending) = smolsandbox::ports::paths(
                &f.home.join(".smolsandbox/state"),
                old["name"].as_str().unwrap(),
            );
            fs::create_dir_all(pending.parent().unwrap()).unwrap();
            fs::write(&pending, "{}").unwrap();
            let (manifest, _) = smolsandbox::ports::paths(
                &f.home.join(".smolsandbox/state"),
                desired["name"].as_str().unwrap(),
            );
            fs::write(&manifest, "{}").unwrap();
            let harness = f.home.join(".smolsandbox/codex-home/user-config");
            fs::write(&harness, "keep").unwrap();
            reset_calls(&f);
            let output = f.run(&["clean", "--workspace"]);
            if contradictory {
                assert!(!output.status.success());
                assert_no_mutations(&f);
                assert!(pending.exists());
                assert!(manifest.exists());
                assert!(String::from_utf8_lossy(&output.stderr)
                    .contains("managed labels are contradictory"));
            } else {
                assert_ok(&output);
                assert_eq!(f.inventory(), vec![other, login]);
                assert!(!pending.exists());
                assert!(!manifest.exists());
                let expected_workspace = f
                    .workspace
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                #[cfg(windows)]
                let expected_workspace = expected_workspace
                    .strip_prefix(r"\\?\")
                    .unwrap_or(&expected_workspace)
                    .to_owned();
                assert!(String::from_utf8_lossy(&output.stdout).contains(&expected_workspace));
                assert!(
                    String::from_utf8_lossy(&output.stdout).contains("Removed 2 workspace VM(s)")
                );
            }
            assert_eq!(fs::read_to_string(&harness).unwrap(), "keep");
            assert_caches_unchanged(&caches);
            assert!(f.definition().exists());
            assert_eq!(
                fs::read_dir(f.home.join(".smolsandbox/artifacts"))
                    .unwrap()
                    .count(),
                before
            );
        }
    }

    #[test]
    fn generation_clean_partial_delete_failure_preserves_artifacts_caches_and_later_machines() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["build"]));
        let workspace = workspace_machine(&f);
        let caches = cache_files(&f);
        let mut before = f.inventory();
        for machine in &mut before {
            machine["state"] = json!("stopped");
        }
        f.set_inventory(&before);
        let output = f
            .command()
            .env(
                "SMOLVM_TEST_DELETE_FAIL_NAME",
                workspace["name"].as_str().unwrap(),
            )
            .arg("clean")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(f.inventory(), before);
        assert_caches_unchanged(&caches);
        assert!(fs::read_dir(f.home.join(".smolsandbox/artifacts"))
            .unwrap()
            .next()
            .is_some());
    }

    #[test]
    fn purge_missing_artifact_allows_only_stopped_builder_recovery() {
        for case in [
            "stopped-builder",
            "missing-builder",
            "running-builder",
            "empty-artifact",
        ] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start"]));
            let workspace = workspace_machine(&f);
            let base = workspace["labels"]["codex-sandbox.base-id"]
                .as_str()
                .unwrap();
            let artifacts = f.home.join(".smolsandbox/artifacts");
            let current = artifacts.join(format!("codex-devtools-{base}.smolmachine"));
            fs::remove_file(&current).unwrap();
            if case == "empty-artifact" {
                fs::write(&current, "").unwrap();
            }
            let old = json!({"name":"codex-builder-111111111111","state":"stopped","labels":{}});
            let mut inventory = vec![old.clone()];
            if case != "missing-builder" {
                inventory.push(json!({"name":format!("codex-builder-{base}"),"state":if case == "running-builder" {"running"} else {"stopped"},"labels":{}}));
            }
            f.set_inventory(&inventory);
            let old_artifact = artifacts.join("codex-devtools-111111111111.smolmachine");
            fs::write(&old_artifact, "old").unwrap();
            reset_calls(&f);
            let output = f.run(&["purge"]);
            assert_eq!(
                output.status.success(),
                case == "stopped-builder",
                "{case}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(f.inventory().iter().any(|m| m["name"] == old["name"]));
            assert!(old_artifact.exists());
            assert!(!f
                .log()
                .contains("machine delete --name codex-builder-111111111111"));
            if case == "stopped-builder" {
                assert!(String::from_utf8_lossy(&output.stdout)
                    .contains("Removed 1 machine(s), 0 generation(s), and 0 artifact file(s)."));
                assert!(!f
                    .inventory()
                    .iter()
                    .any(|m| m["name"] == format!("codex-builder-{base}")));
            } else {
                assert_no_mutations(&f);
            }
        }
    }

    #[test]
    fn purge_selects_exact_artifact_names_and_reports_removals() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let artifacts = f.home.join(".smolsandbox/artifacts");
        for suffix in ["", ".smolmachine", ".backup", ".smolmachine.backup"] {
            fs::write(
                artifacts.join(format!("codex-devtools-111111111111{suffix}")),
                "old",
            )
            .unwrap();
        }
        let unrelated = artifacts.join("unrelated");
        fs::write(&unrelated, "keep").unwrap();
        let directory = artifacts.join("codex-devtools-222222222222.smolmachine");
        fs::create_dir(&directory).unwrap();
        let mut inv = f.inventory();
        inv.push(json!({"name":"codex-builder-111111111111","state":"stopped","labels":{}}));
        f.set_inventory(&inv);
        let output = f.run(&["purge"]);
        assert_ok(&output);
        assert!(String::from_utf8_lossy(&output.stdout)
            .contains("Removed 1 machine(s), 1 generation(s), and 2 artifact file(s)."));
        for suffix in ["", ".smolmachine"] {
            assert!(!artifacts
                .join(format!("codex-devtools-111111111111{suffix}"))
                .exists());
        }
        for suffix in [".backup", ".smolmachine.backup"] {
            assert!(artifacts
                .join(format!("codex-devtools-111111111111{suffix}"))
                .exists());
        }
        assert!(unrelated.exists());
        assert!(directory.is_dir());
    }

    #[test]
    fn purge_fresh_running_change_prevents_all_deletions() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let old = json!({"name":"codex-builder-111111111111","state":"stopped","labels":{}});
        let mut running = old.clone();
        running["state"] = json!("running");
        let artifact = f
            .home
            .join(".smolsandbox/artifacts/codex-devtools-111111111111.smolmachine");
        fs::write(&artifact, "old").unwrap();
        let sequence = sequence(
            &f,
            &[json!([old]).to_string(), json!([running]).to_string()],
        );
        reset_calls(&f);
        let output = f
            .command()
            .env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence)
            .arg("purge")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_no_mutations(&f);
        assert!(artifact.exists());
    }

    #[test]
    fn purge_partial_machine_failure_preserves_all_artifacts_and_later_machines() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let mut inv = f.inventory();
        for base in ["111111111111", "222222222222", "333333333333"] {
            inv.push(json!({"name":format!("codex-builder-{base}"),"state":"stopped","labels":{}}));
            fs::write(
                f.home.join(format!(
                    ".smolsandbox/artifacts/codex-devtools-{base}.smolmachine"
                )),
                "old",
            )
            .unwrap();
        }
        f.set_inventory(&inv);
        reset_calls(&f);
        let output = f
            .command()
            .env("SMOLVM_TEST_DELETE_FAIL_NAME", "codex-builder-222222222222")
            .arg("purge")
            .output()
            .unwrap();
        assert!(!output.status.success());
        let inv = f.inventory();
        assert!(!inv
            .iter()
            .any(|m| m["name"] == "codex-builder-111111111111"));
        for base in ["222222222222", "333333333333"] {
            assert!(inv
                .iter()
                .any(|m| m["name"] == format!("codex-builder-{base}")));
        }
        for base in ["111111111111", "222222222222", "333333333333"] {
            assert!(f
                .home
                .join(format!(
                    ".smolsandbox/artifacts/codex-devtools-{base}.smolmachine"
                ))
                .exists());
        }
        assert!(!f
            .log()
            .contains("machine delete --name codex-builder-333333333333"));
    }

    #[test]
    fn build_preserves_custom_configurations_when_automatic_purge_refuses() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        assert_ok(&f.run(&["start", "--allow-host", "custom.example"]));
        let before = f.inventory();
        let old_artifact = f
            .home
            .join(".smolsandbox/artifacts/codex-devtools-111111111111.smolmachine");
        fs::write(&old_artifact, "old").unwrap();
        reset_calls(&f);
        let output = f.run(&["build"]);
        assert_ok(&output);
        assert!(String::from_utf8_lossy(&output.stderr).contains("parallel configurations exist"));
        assert!(!f.log().contains("machine delete --name"));
        assert!(old_artifact.exists());
        for machine in before {
            assert!(f.inventory().iter().any(|m| m["name"] == machine["name"]));
        }
    }
    #[test]
    fn incompatible_port_capability_refuses_before_inventory_or_mutations() {
        let f = Fixture::new();
        f.configure("codex");
        let output = f
            .command()
            .env("SMOLVM_TEST_INCOMPATIBLE_UPDATE", "1")
            .arg("start")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_no_mutations(&f);
        assert!(!calls(&f).iter().any(|a| a == &["machine", "ls", "--json"]));
    }

    #[test]
    fn malformed_port_metadata_and_unknown_positive_sets_fail_before_mutation() {
        for case in ["timestamp", "count", "unknown-state", "unknown-set"] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start"]));
            let mut machine = workspace_machine(&f);
            match case {
                "timestamp" => machine["created_at"] = json!(true),
                "count" => {
                    machine.as_object_mut().unwrap().remove("_test_ports");
                    machine["ports"] = json!(-1);
                }
                "unknown-state" => machine["state"] = json!("failed"),
                _ => {
                    machine["_test_ports"] = json!([4444]);
                    machine["ports"] = json!(1);
                }
            }
            f.set_inventory(std::slice::from_ref(&machine));
            let (manifest, journal) = smolsandbox::ports::paths(
                &f.home.join(".smolsandbox/state"),
                machine["name"].as_str().unwrap(),
            );
            if case == "unknown-set" {
                fs::remove_file(manifest).unwrap();
                assert!(!journal.exists());
            }
            reset_calls(&f);
            assert!(
                !f.run(&["start", "--port", "4444"]).status.success(),
                "{case}"
            );
            assert_no_mutations(&f);
        }
    }

    #[test]
    fn new_zero_port_incarnation_replaces_stale_manifest_without_changing_identity() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start", "--port", "3000"]));
        let mut machine = workspace_machine(&f);
        machine["_test_ports"] = json!([]);
        machine["ports"] = json!(0);
        machine["created_at"] = json!(2);
        f.set_inventory(std::slice::from_ref(&machine));
        assert_ok(&f.run(&["start", "--port", "4000"]));
        let selected = workspace_machine(&f);
        assert_eq!(selected["name"], machine["name"]);
        assert_eq!(selected["_test_ports"], json!([4000]));
        let (manifest, journal) = smolsandbox::ports::paths(
            &f.home.join(".smolsandbox/state"),
            selected["name"].as_str().unwrap(),
        );
        let value: Value = serde_json::from_slice(&fs::read(manifest).unwrap()).unwrap();
        assert_eq!(value["created_at"], 2);
        assert_eq!(value["ports"], json!([4000]));
        assert!(!journal.exists());
    }

    #[test]
    fn exact_running_port_set_executes_harness_without_update_start_or_owned_stop() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start", "--port", "3000"]));
        let mut machine = workspace_machine(&f);
        machine["state"] = json!("running");
        f.set_inventory(&[machine]);
        reset_calls(&f);
        assert_ok(&f.run(&["start", "--port", "3000"]));
        let calls = calls(&f);
        assert!(calls.iter().any(|a| a.iter().any(|s| s == "--no-daemon")));
        assert!(!calls.iter().any(|a| matches!(
            a.get(1).map(String::as_str),
            Some("start" | "stop" | "update")
        ) && a.get(2).is_some_and(|s| s == "--name")));
        assert_eq!(workspace_machine(&f)["state"], "running");
    }

    #[test]
    fn successful_workspace_deletion_removes_manifest_and_pending_journal() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start", "--port", "3000"]));
        let machine = workspace_machine(&f);
        let (manifest, journal) = smolsandbox::ports::paths(
            &f.home.join(".smolsandbox/state"),
            machine["name"].as_str().unwrap(),
        );
        fs::write(&journal, "{}").unwrap();
        assert_ok(&f.run(&["clean", "--workspace"]));
        assert!(!manifest.exists());
        assert!(!journal.exists());
    }

    #[test]
    fn failed_fresh_builder_never_publishes_an_artifact_or_generation_cache() {
        for failure in ["SMOLVM_TEST_CREATE_FAIL_AFTER", "SMOLVM_TEST_START_FAIL"] {
            let f = Fixture::new();
            f.configure("codex");
            let definition = fs::read(f.definition()).unwrap();
            let output = f.command().env(failure, "1").arg("build").output().unwrap();
            assert!(!output.status.success(), "{failure}");
            assert!(f.log().contains("machine create --name codex-builder-"));
            assert!(!f.log().contains("pack create"), "{failure}");
            let application_home = f.home.join(".smolsandbox");
            assert_eq!(
                fs::read_dir(application_home.join("artifacts"))
                    .unwrap()
                    .count(),
                0
            );
            for cache in ["base-id", "machine-revision", "generation.json"] {
                assert!(
                    !application_home.join("state").join(cache).exists(),
                    "{failure}: {cache}"
                );
            }
            assert_eq!(fs::read(f.definition()).unwrap(), definition);
            assert!(f
                .inventory()
                .iter()
                .all(|machine| machine["labels"]["codex-sandbox.kind"] == "builder"));
        }
    }

    #[test]
    fn builders_are_packed_before_workspace_creation_and_removed_when_artifacts_exist() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let log = f.log();
        let pack = log.find("pack create --from-vm").unwrap();
        let workspace_create = log.find("codex-sandbox.kind=workspace").unwrap();
        assert!(pack < workspace_create);
        assert!(!f
            .inventory()
            .iter()
            .any(|m| m["labels"]["codex-sandbox.kind"] == "builder"));
        let workspace = workspace_machine(&f);
        let base = workspace["labels"]["codex-sandbox.base-id"]
            .as_str()
            .unwrap();
        let artifacts = f.home.join(".smolsandbox/artifacts");
        fs::write(
            artifacts.join("codex-devtools-111111111111.smolmachine"),
            "old",
        )
        .unwrap();
        let mut inventory = f.inventory();
        for base in [base, "111111111111"] {
            inventory.push(
                json!({"name":format!("codex-builder-{base}"),"state":"stopped","labels":{}}),
            );
        }
        f.set_inventory(&inventory);
        reset_calls(&f);
        assert_ok(&f.run(&["start"]));
        for base in [base, "111111111111"] {
            assert!(f.log().contains(&format!(
                "machine delete --name codex-builder-{base} --force"
            )));
        }
        assert!(!f.log().contains("pack create"));
    }
    #[test]
    fn desired_machine_recovery_republishes_missing_and_stale_generation_caches() {
        for missing in [false, true] {
            let f = Fixture::new();
            f.configure("codex");
            assert_ok(&f.run(&["start"]));
            let machine = workspace_machine(&f);
            let root = f.home.join(".smolsandbox/state");
            if missing {
                for name in ["base-id", "machine-revision", "generation.json"] {
                    fs::remove_file(root.join(name)).unwrap();
                }
            } else {
                fs::write(root.join("base-id"), "stale-base\n").unwrap();
                fs::write(root.join("machine-revision"), "99\n").unwrap();
            }
            reset_calls(&f);
            assert_ok(&f.run(&["start"]));
            assert!(!f.log().contains("machine create --name"));
            let base = machine["labels"]["codex-sandbox.base-id"].as_str().unwrap();
            assert_eq!(
                fs::read_to_string(root.join("base-id")).unwrap(),
                format!("{base}\n")
            );
            assert_eq!(
                fs::read_to_string(root.join("machine-revision")).unwrap(),
                "5\n"
            );
            let generation: Value =
                serde_json::from_slice(&fs::read(root.join("generation.json")).unwrap()).unwrap();
            assert_eq!(
                generation,
                json!({"version":1,"base_id":base,"revision":"5"})
            );
        }
    }

    #[test]
    fn stale_only_normal_start_preserves_every_cache_and_machine() {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["start"]));
        let stale = stale_machine(&workspace_machine(&f), "4");
        f.set_inventory(std::slice::from_ref(&stale));
        let caches = cache_files(&f);
        reset_calls(&f);
        let output = f.run(&["start"]);
        assert!(!output.status.success());
        assert_no_mutations(&f);
        assert_eq!(f.inventory(), vec![stale.clone()]);
        assert_caches_unchanged(&caches);
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostics.contains(stale["name"].as_str().unwrap()));
        assert!(diagnostics.contains("--replace-stale"));
    }
}
