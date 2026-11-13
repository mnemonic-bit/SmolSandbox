use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

struct Fixture {
    _temporary: TempDir,
    root: PathBuf,
    home: PathBuf,
    workspace: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = native_path(temporary.path());
        let home = root.join("home");
        let workspace = root.join("project with spaces");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(root.join("inventory.json"), "[]").unwrap();
        Self {
            _temporary: temporary,
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
            .env("SMOLVM_TEST_HOME", &self.home)
            .env("STATE_ROOT", self.state())
            .env("ARTIFACTS_DIR", self.home.join("artifacts"))
            .env("SMOLVM_BIN", env!("CARGO_BIN_EXE_smolvm-test-double"))
            .env("SMOLVM_TEST_LOG", self.root.join("calls.log"))
            .env(
                "SMOLVM_TEST_INVENTORY_FILE",
                self.root.join("inventory.json"),
            );
        for name in [
            "BASE_SMOLFILE",
            "CODEX_MACHINE",
            "LOGIN_MACHINE",
            "BUILDER_MACHINE",
            "SMOLVM_TEST_ALLOW_CREATE",
            "SMOLVM_TEST_ALLOW_LIFECYCLE",
            "SMOLVM_TEST_ALLOW_CLEANUP",
            "SMOLVM_TEST_INVENTORY_SEQUENCE_FILE",
            "SMOLVM_TEST_INVENTORY_FAIL",
            "SMOLVM_TEST_INVENTORY_FAIL_AT",
            "SMOLVM_TEST_STOP_FAIL",
            "SMOLVM_TEST_DELETE_FAIL",
            "SMOLVM_TEST_DELETE_FAIL_NAME",
            "SMOLVM_TEST_HELP_FAIL",
        ] {
            c.env_remove(name);
        }
        c.arg("remove");
        c
    }
    fn state(&self) -> PathBuf {
        self.home.join("custom-state")
    }
    fn set(&self, rows: &[Value]) {
        fs::write(
            self.root.join("inventory.json"),
            serde_json::to_vec(rows).unwrap(),
        )
        .unwrap();
    }
    #[cfg(feature = "test-adapter")]
    fn sequence(&self, rows: &[Vec<Value>]) -> PathBuf {
        let path = self.root.join("sequence.jsonl");
        let lines = rows
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&path, lines).unwrap();
        path
    }
    fn log(&self) -> Vec<String> {
        fs::read_to_string(self.root.join("calls.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
    fn mutations(&self) -> Vec<String> {
        self.log()
            .into_iter()
            .filter(|l| {
                !l.ends_with("--help")
                    && (l.starts_with("machine stop ") || l.starts_with("machine delete "))
            })
            .collect()
    }
    fn workspace(&self, name: &str, revision: &str, state: &str) -> Value {
        json!({"name":name,"state":state,"created_at":17,"ports":0,"labels":{
            "codex-sandbox.kind":"workspace", "codex-sandbox.base-id":"111111111111",
            "codex-sandbox.revision":revision, "codex-sandbox.workspace-sha256":"a".repeat(64),
            "codex-sandbox.workspace-path-sha256":digest(self.workspace.to_str().unwrap()),
            "codex-sandbox.configuration":"custom"
        }})
    }
    #[cfg(feature = "test-adapter")]
    fn port_files(&self, name: &str) -> (PathBuf, PathBuf) {
        let root = self.state().join("ports");
        fs::create_dir_all(&root).unwrap();
        let paths = (
            root.join(format!("{name}.json")),
            root.join(format!("{name}.pending.json")),
        );
        fs::write(&paths.0, "manifest sentinel").unwrap();
        fs::write(&paths.1, "pending sentinel").unwrap();
        paths
    }
}
fn native_path(path: &Path) -> PathBuf {
    let canonical = fs::canonicalize(path).unwrap();
    #[cfg(windows)]
    {
        let text = canonical.to_str().unwrap();
        if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text))
    }
    #[cfg(not(windows))]
    canonical
}
fn digest(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn assert_ok(out: &Output) {
    assert!(
        out.status.success(),
        "status {:?}\nstdout {}\nstderr {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
fn assert_failed(out: &Output) {
    assert!(
        !out.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_ne!(out.status.code(), Some(2));
}

#[test]
fn dry_run_is_definition_independent_lock_free_and_does_not_change_permissions() {
    let f = Fixture::new();
    f.set(&[
        f.workspace("preview-stale", "4", "running"),
        f.workspace("preview-current", "0005", "stopped"),
    ]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&f.home, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let before = fs::read(f.root.join("inventory.json")).unwrap();
    let out = f
        .command()
        .args(["--dry-run", "--force"])
        .env("BASE_SMOLFILE", f.root.join("missing.smlbx"))
        .output()
        .unwrap();
    assert_ok(&out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("preview-stale") && text.contains("stop-and-delete"),
        "{text}"
    );
    assert!(!text.contains("Removed preview-stale"));
    assert_eq!(fs::read(f.root.join("inventory.json")).unwrap(), before);
    assert_eq!(fs::read_dir(&f.home).unwrap().count(), 0);
    assert_eq!(f.log(), ["--version", "machine ls --json"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&f.home).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
}

#[test]
fn stale_comparison_uses_decimal_values_and_preserves_base_or_name_only_differences() {
    let f = Fixture::new();
    let future = "9".repeat(200);
    let old = format!("{}4", "0".repeat(200));
    let mut different_base = f.workspace("different-base-name", "5", "created");
    different_base["labels"]["codex-sandbox.base-id"] = json!("999999999999");
    f.set(&[
        f.workspace("older-decimal", &old, "stopped"),
        f.workspace("equal-decimal", "000005", "stopped"),
        f.workspace("future-decimal", &future, "stopped"),
        different_base,
    ]);
    let out = f.command().arg("--dry-run").output().unwrap();
    assert_ok(&out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("older-decimal"), "{text}");
    for name in ["equal-decimal", "future-decimal", "different-base-name"] {
        assert!(!text.contains(name), "{text}");
    }
    assert!(f.mutations().is_empty());
}

#[test]
fn scoped_preview_counts_unassociated_candidates_without_guessing_paths() {
    let f = Fixture::new();
    let mut missing = f.workspace("missing-path", "4", "stopped");
    missing["labels"]
        .as_object_mut()
        .unwrap()
        .remove("codex-sandbox.workspace-path-sha256");
    let mut malformed = f.workspace("malformed-path", "4", "stopped");
    malformed["labels"]["codex-sandbox.workspace-path-sha256"] = json!("bad");
    let legacy = json!({"name":"codex-workspace-111111111111-r4-aaaaaaaaaaaa","state":"stopped"});
    let mut other = f.workspace("other-project", "4", "stopped");
    other["labels"]["codex-sandbox.workspace-path-sha256"] = json!("b".repeat(64));
    f.set(&[missing, malformed, legacy, other]);
    let out = f
        .command()
        .args(["--dry-run", "--all", "--force"])
        .output()
        .unwrap();
    assert_ok(&out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.to_lowercase().contains("unassociated") && text.contains('3'),
        "{text}"
    );
    assert!(text.to_lowercase().contains("no eligible"), "{text}");
    assert!(f.mutations().is_empty());
}

#[test]
fn ownership_and_inventory_errors_refuse_before_any_application_write() {
    for kind in 0..13 {
        let f = Fixture::new();
        let mut bad = f.workspace("z-invalid", "4", "stopped");
        let mut rows = vec![f.workspace("a-valid", "4", "stopped")];
        match kind {
            0 => bad["labels"]["codex-sandbox.kind"] = json!("login"),
            1 => bad["labels"]["codex-sandbox.base-id"] = json!("BAD"),
            2 => bad["labels"]["codex-sandbox.revision"] = json!("-1"),
            3 => bad["labels"]["codex-sandbox.configuration"] = json!("unsupported"),
            4 => bad["state"] = json!("suspended"),
            5 => bad["created_at"] = Value::Null,
            6 => bad["name"] = json!("../escape"),
            7 => bad["name"] = json!("codex-workspace-222222222222-r4-aaaaaaaaaaaa"),
            8 => bad["name"] = json!("a-valid"),
            9 => bad
                .as_object_mut()
                .unwrap()
                .remove("name")
                .map(|_| ())
                .unwrap(),
            10 => bad["labels"]["codex-sandbox.kind"] = json!(17),
            11 => bad["state"] = json!(true),
            12 => bad["labels"] = json!([]),
            _ => unreachable!(),
        }
        rows.push(bad);
        f.set(&rows);
        let out = f
            .command()
            .args(["--all", "--force", "--dry-run"])
            .output()
            .unwrap();
        assert_failed(&out);
        assert!(f.mutations().is_empty(), "case {kind}");
        assert_eq!(fs::read_dir(&f.home).unwrap().count(), 0, "case {kind}");
    }
}

#[test]
fn global_managed_namespace_requires_complete_labels_without_name_fallback() {
    for labels in [
        json!({"codex-sandbox.note":"candidate"}),
        json!({"codex-sandbox.kind":"workspace"}),
        json!({"codex-sandbox.kind":"builder","codex-sandbox.base-id":"111111111111","codex-sandbox.revision":"4"}),
    ] {
        let f = Fixture::new();
        f.set(&[json!({"name":"codex-builder-111111111111","state":"stopped","labels":labels})]);
        let out = f
            .command()
            .args(["--global", "--all", "--force", "--dry-run"])
            .output()
            .unwrap();
        assert_failed(&out);
        assert!(f.mutations().is_empty());
    }
}

#[test]
fn malformed_inventory_and_runtime_failure_produce_no_partial_plan() {
    for raw in ["not-json", "{}", "[null]", "[{\"state\":\"stopped\"}]"] {
        let f = Fixture::new();
        let sequence = f.root.join("malformed-sequence");
        fs::write(&sequence, raw).unwrap();
        let out = f
            .command()
            .arg("--dry-run")
            .env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence)
            .output()
            .unwrap();
        assert_failed(&out);
        assert!(f.mutations().is_empty());
        assert_eq!(fs::read_dir(&f.home).unwrap().count(), 0);
    }
    let f = Fixture::new();
    let out = f
        .command()
        .arg("--dry-run")
        .env("SMOLVM_TEST_INVENTORY_FAIL", "1")
        .output()
        .unwrap();
    assert_failed(&out);
    assert!(f.mutations().is_empty());
}

#[test]
fn explicit_relative_and_subdirectory_paths_preserve_canonical_workspace_scope() {
    let f = Fixture::new();
    let child = f.workspace.join("subdirectory");
    fs::create_dir_all(&child).unwrap();
    fs::create_dir_all(f.workspace.join(".git")).unwrap();
    let parent_row = f.workspace("parent-machine", "4", "stopped");
    let mut child_row = f.workspace("child-machine", "4", "stopped");
    child_row["labels"]["codex-sandbox.workspace-path-sha256"] =
        json!(digest(native_path(&child).to_str().unwrap()));
    f.set(&[parent_row, child_row]);
    let out = f
        .command()
        .args(["--dry-run", "subdirectory", "--all"])
        .output()
        .unwrap();
    assert_ok(&out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("child-machine"), "{text}");
    assert!(!text.contains("parent-machine"), "{text}");
    for path in [f.root.join("absent"), f.root.join("inventory.json")] {
        assert_failed(&f.command().arg("--dry-run").arg(path).output().unwrap());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&child, f.workspace.join("alias")).unwrap();
        let out = f.command().args(["--dry-run", "alias"]).output().unwrap();
        assert_ok(&out);
        assert!(String::from_utf8_lossy(&out.stdout).contains("child-machine"));
    }
}

#[cfg(feature = "test-adapter")]
#[test]
fn all_eight_scope_revision_and_force_combinations_have_fixed_deletion_sets() {
    for global in [false, true] {
        for all in [false, true] {
            for force in [false, true] {
                let f = Fixture::new();
                let mut rows = vec![];
                for (name, revision, state) in [
                    ("a-old-stopped", "4", "stopped"),
                    ("b-old-created", "4", "created"),
                    ("c-old-running", "4", "running"),
                    ("d-current", "0005", "stopped"),
                    ("e-future", "6", "stopped"),
                    ("f-current-running", "5", "running"),
                ] {
                    let mut row = f.workspace(name, revision, state);
                    if name == "a-old-stopped" {
                        row["labels"]["codex-sandbox.base-id"] = json!("777777777777");
                        row["labels"]["codex-sandbox.configuration"] = json!("default");
                    } else if name == "b-old-created" {
                        row["labels"]["codex-sandbox.base-id"] = json!("888888888888");
                        row["labels"]["codex-sandbox.workspace-sha256"] = json!("d".repeat(64));
                    }
                    rows.push(row);
                }
                let mut other = f.workspace("g-other-workspace", "4", "stopped");
                other["labels"]["codex-sandbox.workspace-path-sha256"] = json!("b".repeat(64));
                rows.extend([other,
            json!({"name":"h-custom-login","state":"running","labels":{"codex-sandbox.kind":"login","codex-sandbox.base-id":"222222222222","codex-sandbox.revision":"4"}}),
            json!({"name":"i-custom-builder","state":"stopped","labels":{"codex-sandbox.kind":"builder","codex-sandbox.base-id":"333333333333"}}),
            json!({"name":"codex-login-444444444444-r4","state":"stopped"}),
            json!({"name":"codex-workspace-555555555555-r4-cccccccccccc","state":"stopped"}),
            json!({"name":"codex-builder-666666666666","state":"running"}),
            json!({"name":"codex-workspace-unrelated","state":"stopped"}),
            json!({"name":"unrelated","state":"running"})]);
                f.set(&rows);
                let mut c = f.command();
                if global {
                    c.arg("--global");
                }
                if all {
                    c.arg("--all");
                }
                if force {
                    c.arg("--force");
                }
                let out = c.output().unwrap();
                assert_ok(&out);
                let mut expected = vec!["a-old-stopped", "b-old-created"];
                if force {
                    expected.push("c-old-running");
                }
                if all {
                    expected.extend(["d-current", "e-future"]);
                    if force {
                        expected.push("f-current-running");
                    }
                }
                if global {
                    expected.extend([
                        "g-other-workspace",
                        "codex-login-444444444444-r4",
                        "codex-workspace-555555555555-r4-cccccccccccc",
                    ]);
                    if force {
                        expected.push("h-custom-login");
                    }
                    if all {
                        expected.push("i-custom-builder");
                        if force {
                            expected.push("codex-builder-666666666666");
                        }
                    }
                }
                expected.sort();
                let deletions: Vec<_> = f
                    .mutations()
                    .into_iter()
                    .filter(|l| l.starts_with("machine delete "))
                    .collect();
                let actual: Vec<_> = deletions
                    .iter()
                    .map(|l| {
                        l.split("--name ")
                            .nth(1)
                            .unwrap()
                            .split_whitespace()
                            .next()
                            .unwrap()
                    })
                    .collect();
                assert_eq!(actual, expected, "global={global} all={all} force={force}");
                let stops = f
                    .mutations()
                    .into_iter()
                    .filter(|l| l.starts_with("machine stop "))
                    .count();
                let expected_stops = usize::from(force)
                    * (1 + usize::from(all) + usize::from(global) + usize::from(global && all));
                assert_eq!(stops, expected_stops);
                let names: Vec<_> = serde_json::from_slice::<Vec<Value>>(
                    &fs::read(f.root.join("inventory.json")).unwrap(),
                )
                .unwrap()
                .into_iter()
                .map(|m| m["name"].as_str().unwrap().to_owned())
                .collect();
                for name in expected {
                    assert!(!names.iter().any(|n| n == name));
                }
            }
        }
    }
}

#[cfg(feature = "test-adapter")]
#[test]
fn deletion_cleans_only_machine_port_files_and_preserves_all_other_resources() {
    let f = Fixture::new();
    f.set(&[
        f.workspace("a-selected", "4", "stopped"),
        f.workspace("b-running", "4", "running"),
    ]);
    let selected = f.port_files("a-selected");
    let running = f.port_files("b-running");
    let preserved = [
        f.home.join("artifacts/unreferenced.smolmachine"),
        f.home.join("default.smlbx"),
        f.home.join("codex-home/auth.json"),
        f.home.join("harness-home/hermes/credentials"),
        f.state().join("generation.json"),
        f.state().join("base-id"),
        f.state().join("machine-revision"),
    ];
    for path in &preserved {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "resource sentinel").unwrap();
    }
    let out = f.command().output().unwrap();
    assert_ok(&out);
    assert!(!selected.0.exists() && !selected.1.exists());
    assert!(running.0.exists() && running.1.exists());
    for path in preserved {
        assert_eq!(fs::read_to_string(path).unwrap(), "resource sentinel");
    }
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("skip-running") && text.contains("--force"),
        "{text}"
    );
}

#[cfg(feature = "test-adapter")]
#[test]
fn failed_stop_and_delete_preserve_port_state_and_later_targets() {
    for stop in [false, true] {
        let f = Fixture::new();
        f.set(&[
            f.workspace("a-target", "4", if stop { "running" } else { "stopped" }),
            f.workspace("b-later", "4", "stopped"),
        ]);
        let paths = f.port_files("a-target");
        let out = f
            .command()
            .arg("--force")
            .env(
                if stop {
                    "SMOLVM_TEST_STOP_FAIL"
                } else {
                    "SMOLVM_TEST_DELETE_FAIL"
                },
                "1",
            )
            .output()
            .unwrap();
        assert_failed(&out);
        assert!(paths.0.exists() && paths.1.exists());
        let calls = f.mutations();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].contains("a-target") && !calls[0].contains("b-later"));
        if stop {
            assert!(calls[0].starts_with("machine stop "));
        }
        let error = String::from_utf8_lossy(&out.stderr);
        assert!(
            error.contains("a-target") && error.contains("b-later"),
            "{error}"
        );
    }
}

#[cfg(feature = "test-adapter")]
#[test]
fn inventory_changes_under_lock_refuse_before_mutation() {
    for change in 0..6 {
        let f = Fixture::new();
        let planned = f.workspace("selected", "4", "stopped");
        let mut fresh = planned.clone();
        match change {
            0 => fresh["state"] = json!("running"),
            1 => fresh["created_at"] = json!(18),
            2 => fresh["ports"] = json!(1),
            3 => fresh["labels"]["codex-sandbox.base-id"] = json!("222222222222"),
            4 => fresh["labels"]["codex-sandbox.workspace-path-sha256"] = json!("b".repeat(64)),
            5 => fresh["name"] = json!("new-target"),
            _ => unreachable!(),
        }
        f.set(std::slice::from_ref(&planned));
        let sequence = f.sequence(&[vec![planned], vec![fresh]]);
        let out = f
            .command()
            .arg("--force")
            .env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence)
            .output()
            .unwrap();
        assert_failed(&out);
        assert!(f.mutations().is_empty(), "change {change}");
    }
}

#[cfg(feature = "test-adapter")]
#[test]
fn deletion_absence_failure_preserves_port_files_after_name_reuse() {
    let f = Fixture::new();
    let planned = f.workspace("selected", "4", "stopped");
    let mut reused = planned.clone();
    reused["created_at"] = json!(99);
    reused["state"] = json!("running");
    f.set(std::slice::from_ref(&planned));
    let paths = f.port_files("selected");
    let sequence = f.sequence(&[
        vec![planned.clone()],
        vec![planned.clone()],
        vec![planned],
        vec![reused],
    ]);
    let out = f
        .command()
        .env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence)
        .output()
        .unwrap();
    assert_failed(&out);
    assert_eq!(f.mutations().len(), 1);
    assert!(f.mutations()[0].starts_with("machine delete "));
    assert!(paths.0.exists() && paths.1.exists());
}

#[cfg(feature = "test-adapter")]
#[test]
fn successful_stop_requires_same_incarnation_stopped_before_delete() {
    for change in 0..5 {
        let f = Fixture::new();
        let planned = f.workspace("a-running", "4", "running");
        let mut after = planned.clone();
        after["state"] = json!("stopped");
        match change {
            0 => after["state"] = json!("running"),
            1 => after["created_at"] = json!(18),
            2 => after["labels"]["codex-sandbox.revision"] = json!("3"),
            3 => after["ports"] = json!(1),
            4 => after["state"] = json!("created"),
            _ => unreachable!(),
        }
        f.set(std::slice::from_ref(&planned));
        let paths = f.port_files("a-running");
        let sequence = f.sequence(&[
            vec![planned.clone()],
            vec![planned.clone()],
            vec![planned],
            vec![after],
        ]);
        let out = f
            .command()
            .arg("--force")
            .env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence)
            .output()
            .unwrap();
        assert_failed(&out);
        assert_eq!(f.mutations().len(), 1, "change {change}");
        assert!(f.mutations()[0].starts_with("machine stop "));
        assert!(paths.0.exists() && paths.1.exists());
    }
}

#[cfg(feature = "test-adapter")]
#[test]
fn forced_stop_delete_and_absence_queries_have_fixed_order() {
    let f = Fixture::new();
    f.set(&[f.workspace("running-target", "4", "running")]);
    let paths = f.port_files("running-target");
    let out = f.command().arg("--force").output().unwrap();
    assert_ok(&out);
    let calls: Vec<_> = f
        .log()
        .into_iter()
        .filter(|l| !l.ends_with("--help") && l != "--version")
        .collect();
    assert_eq!(calls.len(), 8, "{calls:?}");
    assert_eq!(
        &calls[..3],
        [
            "machine ls --json",
            "machine ls --json",
            "machine ls --json"
        ]
    );
    assert!(calls[3].starts_with("machine stop ") && calls[3].contains("running-target"));
    assert_eq!(calls[4], "machine ls --json");
    assert_eq!(calls[5], "machine ls --json");
    assert!(calls[6].starts_with("machine delete ") && calls[6].contains("running-target"));
    assert_eq!(calls[7], "machine ls --json");
    assert!(!paths.0.exists() && !paths.1.exists());
}

#[cfg(feature = "test-adapter")]
#[test]
fn change_before_later_target_stops_partial_execution() {
    let f = Fixture::new();
    let first = f.workspace("a-first", "4", "stopped");
    let second = f.workspace("b-later", "4", "stopped");
    let mut changed = second.clone();
    changed["state"] = json!("running");
    f.set(&[first.clone(), second.clone()]);
    let first_paths = f.port_files("a-first");
    let later_paths = f.port_files("b-later");
    let sequence = f.sequence(&[
        vec![first.clone(), second.clone()],
        vec![first.clone(), second.clone()],
        vec![first, second.clone()],
        vec![second],
        vec![changed],
    ]);
    let out = f
        .command()
        .arg("--force")
        .env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence)
        .output()
        .unwrap();
    assert_failed(&out);
    assert_eq!(f.mutations().len(), 1);
    assert!(f.mutations()[0].contains("a-first"));
    assert!(!first_paths.0.exists() && !first_paths.1.exists());
    assert!(later_paths.0.exists() && later_paths.1.exists());
    let error = String::from_utf8_lossy(&out.stderr);
    assert!(
        error.contains("a-first") && error.contains("b-later"),
        "{error}"
    );
}

#[cfg(feature = "test-adapter")]
#[test]
fn inventory_failure_after_delete_preserves_local_state() {
    let f = Fixture::new();
    let row = f.workspace("selected", "4", "stopped");
    f.set(std::slice::from_ref(&row));
    let paths = f.port_files("selected");
    let sequence = f.sequence(&[vec![row.clone()], vec![row.clone()], vec![row], vec![]]);
    let out = f
        .command()
        .env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence)
        .env("SMOLVM_TEST_INVENTORY_FAIL_AT", "4")
        .output()
        .unwrap();
    assert_failed(&out);
    assert_eq!(f.mutations().len(), 1);
    assert!(paths.0.exists() && paths.1.exists());
}

#[cfg(feature = "test-adapter")]
#[test]
fn local_port_cleanup_failure_reports_deleted_machine_and_preserves_later_target() {
    let f = Fixture::new();
    f.set(&[
        f.workspace("a-first", "4", "stopped"),
        f.workspace("b-later", "4", "stopped"),
    ]);
    fs::create_dir_all(f.state().join("ports/a-first.json")).unwrap();
    fs::write(f.state().join("ports/a-first.json/blocked"), "sentinel").unwrap();
    let later = f.port_files("b-later");
    let out = f.command().output().unwrap();
    assert_failed(&out);
    assert_eq!(f.mutations().len(), 1);
    assert!(f.mutations()[0].contains("a-first"));
    assert!(later.0.exists() && later.1.exists());
    let error = String::from_utf8_lossy(&out.stderr);
    assert!(
        error.contains("a-first") && error.contains("b-later"),
        "{error}"
    );
}

#[cfg(feature = "test-adapter")]
#[test]
fn originally_skipped_running_target_remains_skipped_after_external_stop() {
    let f = Fixture::new();
    let selected = f.workspace("a-selected", "4", "stopped");
    let skipped = f.workspace("b-skipped", "4", "running");
    let mut stopped = skipped.clone();
    stopped["state"] = json!("stopped");
    f.set(&[selected.clone(), skipped.clone()]);
    let sequence = f.sequence(&[
        vec![selected.clone(), skipped],
        vec![selected.clone(), stopped.clone()],
        vec![selected, stopped.clone()],
        vec![stopped],
    ]);
    let out = f
        .command()
        .env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence)
        .output()
        .unwrap();
    assert_ok(&out);
    assert_eq!(f.mutations().len(), 1);
    assert!(f.mutations()[0].contains("a-selected"));
}

#[cfg(feature = "test-adapter")]
#[test]
fn application_mutation_lock_blocks_removal_until_fresh_inventory_can_be_checked() {
    use fs2::FileExt;
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    f.set(&[f.workspace("selected", "4", "stopped")]);
    let locks = f.home.join(".smolsandbox/locks");
    fs::create_dir_all(&locks).unwrap();
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(locks.join("artifacts.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    let marker = f.root.join("contended");
    let mut child = f
        .command()
        .env("SMOLVM_TEST_LOCK_CONTENDED", &marker)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.exists() {
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("removal did not wait for the application mutation lock");
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(child.try_wait().unwrap().is_none());
    assert!(f.mutations().is_empty());
    f.set(&[f.workspace("selected", "4", "running")]);
    FileExt::unlock(&lock).unwrap();
    let out = child.wait_with_output().unwrap();
    assert_failed(&out);
    assert!(f.mutations().is_empty());
}

#[test]
fn invalid_numeric_inventory_field_types_refuse_even_on_unrelated_rows() {
    for field in ["ports", "created_at"] {
        for value in [
            json!("17"),
            json!(-1),
            json!(1.5),
            json!(true),
            json!([]),
            json!({}),
        ] {
            for relevant in [false, true] {
                let f = Fixture::new();
                let mut row = if relevant {
                    f.workspace("selected", "4", "stopped")
                } else {
                    json!({"name":"unrelated", "state":"stopped"})
                };
                row[field] = value.clone();
                f.set(&[row]);
                let out = f.command().arg("--dry-run").output().unwrap();
                assert_failed(&out);
                assert!(f.mutations().is_empty());
                assert_eq!(fs::read_dir(&f.home).unwrap().count(), 0);
            }
        }
    }
}

#[test]
fn empty_and_running_only_selections_succeed_without_capability_queries_or_state_creation() {
    for running in [false, true] {
        let f = Fixture::new();
        if running {
            f.set(&[f.workspace("skip-only", "4", "running")]);
        }
        let out = f
            .command()
            .env("SMOLVM_TEST_HELP_FAIL", "1")
            .output()
            .unwrap();
        assert_ok(&out);
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.to_lowercase().contains("no eligible"), "{text}");
        if running {
            assert!(
                text.contains("skip-running") && text.contains("--force"),
                "{text}"
            );
        }
        assert_eq!(f.log(), ["--version", "machine ls --json"]);
        assert_eq!(fs::read_dir(&f.home).unwrap().count(), 0);
    }
}

#[test]
fn unknown_states_are_ignored_only_when_revision_or_workspace_scope_protects_the_machine() {
    let f = Fixture::new();
    let current = f.workspace("current-unknown-state", "5", "future-state");
    let mut other = f.workspace("other-unknown-state", "4", "future-state");
    other["labels"]["codex-sandbox.workspace-path-sha256"] = json!("b".repeat(64));
    f.set(&[current, other]);
    assert_ok(&f.command().arg("--dry-run").output().unwrap());
    assert_failed(&f.command().args(["--dry-run", "--all"]).output().unwrap());
    assert_failed(
        &f.command()
            .args(["--dry-run", "--global"])
            .output()
            .unwrap(),
    );
    assert!(f.mutations().is_empty());
}

#[cfg(feature = "test-adapter")]
#[test]
fn later_delete_failure_reports_completed_deletions_and_unprocessed_targets() {
    let f = Fixture::new();
    f.set(&[
        f.workspace("a-completed", "4", "stopped"),
        f.workspace("b-failed", "4", "stopped"),
        f.workspace("c-unprocessed", "4", "stopped"),
    ]);
    let completed = f.port_files("a-completed");
    let failed = f.port_files("b-failed");
    let unprocessed = f.port_files("c-unprocessed");
    let out = f
        .command()
        .env("SMOLVM_TEST_DELETE_FAIL_NAME", "b-failed")
        .output()
        .unwrap();
    assert_failed(&out);
    assert!(!completed.0.exists() && !completed.1.exists());
    assert!(failed.0.exists() && failed.1.exists());
    assert!(unprocessed.0.exists() && unprocessed.1.exists());
    let calls = f.mutations();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].contains("a-completed") && calls[1].contains("b-failed"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("Removed: a-completed"), "{stdout}");
    for text in ["removed=1", "a-completed", "b-failed", "c-unprocessed"] {
        assert!(stderr.contains(text), "{stderr}");
    }
}

#[cfg(feature = "test-adapter")]
#[test]
fn newly_discovered_eligible_target_does_not_expand_authorized_plan() {
    let f = Fixture::new();
    let planned = f.workspace("a-planned", "4", "stopped");
    let added = f.workspace("b-added", "4", "stopped");
    f.set(std::slice::from_ref(&planned));
    let sequence = f.sequence(&[vec![planned.clone()], vec![planned, added]]);
    let out = f
        .command()
        .args(["--all", "--force"])
        .env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence)
        .output()
        .unwrap();
    assert_failed(&out);
    assert!(f.mutations().is_empty());
}
