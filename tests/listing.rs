use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

const DEFINITION: &str = "# codex-sandbox-harness: codex\nimage = 'debian:bookworm-slim'\n[network]\nallow_hosts = ['api.openai.com', 'downloads.example.com']\n";
const VERSION: &str = "smolvm 1.19.2";

struct ListingFixture {
    _temporary: TempDir,
    root: PathBuf,
    workspace: PathBuf,
    home: PathBuf,
    definition: PathBuf,
}

impl ListingFixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = native_path(temporary.path());
        let workspace = root.join("project with spaces");
        let home = root.join("home");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&home).unwrap();
        let definition = root.join("reference.smlbx");
        fs::write(&definition, DEFINITION).unwrap();
        fs::write(root.join("inventory.json"), "[]").unwrap();
        Self {
            _temporary: temporary,
            root,
            workspace,
            home,
            definition,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_smolsandbox"));
        command
            .current_dir(&self.workspace)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("SMOLVM_TEST_HOME", &self.home)
            .env("ARTIFACTS_DIR", self.home.join(".smolsandbox/artifacts"))
            .env("STATE_ROOT", self.home.join(".smolsandbox/state"))
            .env("CODEX_HOME_DIR", self.home.join(".smolsandbox/codex-home"))
            .env("SMOLVM_BIN", env!("CARGO_BIN_EXE_smolvm-test-double"))
            .env("SMOLVM_TEST_VERSION", VERSION)
            .env("SMOLVM_TEST_LOG", self.root.join("calls.log"))
            .env(
                "SMOLVM_TEST_INVENTORY_FILE",
                self.root.join("inventory.json"),
            );
        for variable in [
            "BASE_SMOLFILE",
            "CODEX_MACHINE",
            "BUILDER_MACHINE",
            "LOGIN_MACHINE",
            "SMOLVM_TEST_INVENTORY_SEQUENCE_FILE",
            "SMOLVM_TEST_INVENTORY_FAIL",
            "SMOLVM_TEST_ALLOW_CREATE",
            "SMOLVM_TEST_ALLOW_LIFECYCLE",
            "SMOLVM_TEST_ALLOW_CLEANUP",
        ] {
            command.env_remove(variable);
        }
        command
    }

    fn list(&self) -> Command {
        let mut command = self.command();
        command.args(["list", "--smolfile"]).arg(&self.definition);
        command
    }

    fn inventory(&self, rows: &[Value]) {
        fs::write(
            self.root.join("inventory.json"),
            serde_json::to_vec(rows).unwrap(),
        )
        .unwrap();
    }

    fn base(&self) -> String {
        digest(format!("{VERSION}\n{DEFINITION}").as_bytes())[..12].into()
    }

    fn hash(&self, additions: &[&str]) -> String {
        let artifact = self
            .home
            .join(".smolsandbox/artifacts")
            .join(format!("codex-devtools-{}.smolmachine", self.base()));
        let mut immutable = vec![
            format!("--from={}", artifact.display()),
            "--mem=12288".into(),
            "--cpus=4".into(),
            format!(
                "--volume={}:/root/.codex",
                self.home.join(".smolsandbox/codex-home").display()
            ),
            format!("--volume={}:/workspace/myrepo", self.workspace.display()),
            "--allow-host=api.openai.com".into(),
            "--allow-host=downloads.example.com".into(),
        ];
        immutable.extend(additions.iter().map(|host| format!("--allow-host={host}")));
        immutable.sort();
        immutable.dedup();
        digest(&serde_json::to_vec(&immutable).unwrap())
    }

    fn selected_name(&self) -> String {
        format!(
            "codex-workspace-{}-r5-{}",
            self.base(),
            &self.hash(&[])[..12]
        )
    }

    fn path_hash(&self) -> String {
        digest(self.workspace.to_str().unwrap().as_bytes())
    }

    fn workspace_row(&self, name: &str) -> Value {
        json!({"name": name, "state": "stopped", "labels": {
            "codex-sandbox.kind": "workspace",
            "codex-sandbox.base-id": self.base(),
            "codex-sandbox.revision": "5",
            "codex-sandbox.workspace-sha256": self.hash(&[]),
            "codex-sandbox.workspace-path-sha256": self.path_hash(),
            "codex-sandbox.configuration": "default"
        }})
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

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "status {:?}, stdout {}, stderr {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn document(output: Output) -> Value {
    let output = success(output);
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn names(document: &Value) -> Vec<&str> {
    document["machines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect()
}

fn row<'a>(document: &'a Value, name: &str) -> &'a Value {
    document["machines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap()
}

#[test]
fn mixed_inventory_preserves_association_assessment_and_complete_identities() {
    let f = ListingFixture::new();
    let selected = f.workspace_row(&f.selected_name());
    let mut policy = f.workspace_row("policy-and-base-and-options");
    policy["labels"]["codex-sandbox.revision"] = json!("4");
    policy["labels"]["codex-sandbox.base-id"] = json!("111111111111");
    policy["labels"]["codex-sandbox.workspace-sha256"] = json!("b".repeat(64));
    policy["labels"]["codex-sandbox.configuration"] = json!("custom");
    policy["state"] = json!("paused-by-new-runtime");
    let mut base = f.workspace_row("another-harness-generation");
    base["labels"]["codex-sandbox.base-id"] = json!("222222222222");
    let mut configuration = f.workspace_row("custom-options");
    configuration["labels"]["codex-sandbox.workspace-sha256"] = json!("c".repeat(64));
    configuration["labels"]["codex-sandbox.configuration"] = json!("custom");
    let different_name = f.workspace_row("name-override-from-another-invocation");
    let mut other = f.workspace_row("other-workspace");
    other["labels"]["codex-sandbox.workspace-path-sha256"] = json!("d".repeat(64));
    let mut contradiction = f.workspace_row("codex-builder-333333333333");
    contradiction["labels"]["codex-sandbox.kind"] = json!("login");
    let mut missing = f.workspace_row("missing-path");
    missing["labels"]
        .as_object_mut()
        .unwrap()
        .remove("codex-sandbox.workspace-path-sha256");
    let mut malformed = f.workspace_row("malformed-path");
    malformed["labels"]["codex-sandbox.workspace-path-sha256"] = json!("not-a-hash");
    let builder = json!({"name":"custom-builder", "state":"running", "labels":{
        "codex-sandbox.kind":"builder", "codex-sandbox.base-id":f.base()
    }});
    let login = json!({"name":"custom-login", "state":"created", "labels":{
        "codex-sandbox.kind":"login", "codex-sandbox.base-id":f.base(), "codex-sandbox.revision":"5"
    }});
    let legacy_workspace = format!("codex-workspace-{}-r3-{}", f.base(), &f.hash(&[])[..12]);
    f.inventory(&[
        selected,
        policy,
        base,
        configuration,
        different_name,
        other,
        contradiction,
        missing,
        malformed,
        builder,
        login,
        json!({"name":legacy_workspace,"state":"stopped"}),
        json!({"name":"codex-builder-aaaaaaaaaaaa","state":"stopped"}),
        json!({"name":"codex-login-aaaaaaaaaaaa-r3","state":"stopped"}),
        json!({"name":"prefix-codex-builder-aaaaaaaaaaaa","state":"stopped"}),
        json!({"name":"codex-workspace-invalid","state":"stopped"}),
        json!({"name":"foreign-namespace-label","labels":{"codex-sandbox.future":"value"}}),
    ]);
    let scoped = document(f.list().arg("--json").output().unwrap());
    assert_eq!(scoped["version"], 1);
    assert_eq!(scoped["scope"], "workspace");
    assert_eq!(scoped["workspace"], f.workspace.to_str().unwrap());
    assert_eq!(scoped["unassociated_workspace_count"], 3);
    assert_eq!(scoped["reference"]["base_id"], f.base());
    assert_eq!(scoped["reference"]["revision"], "5");
    assert_eq!(scoped["reference"]["workspace_sha256"], f.hash(&[]));
    assert_eq!(scoped["reference"]["desired_machine"], f.selected_name());
    assert_eq!(
        scoped["reference"]["definition"],
        f.definition.to_str().unwrap()
    );
    let mut expected = vec![
        f.selected_name(),
        "policy-and-base-and-options".into(),
        "another-harness-generation".into(),
        "custom-options".into(),
        "name-override-from-another-invocation".into(),
        "codex-builder-333333333333".into(),
    ];
    expected.sort();
    assert_eq!(names(&scoped), expected);
    for (name, assessment) in [
        (f.selected_name().as_str(), "selected"),
        ("policy-and-base-and-options", "different-policy"),
        ("another-harness-generation", "different-base"),
        ("custom-options", "different-configuration"),
        (
            "name-override-from-another-invocation",
            "different-configuration",
        ),
        ("codex-builder-333333333333", "unknown"),
    ] {
        assert_eq!(row(&scoped, name)["assessment"], assessment, "{name}");
    }
    let differences = row(&scoped, "policy-and-base-and-options")["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    for required in ["base", "configuration", "name", "unknown"] {
        assert!(
            differences.contains(required),
            "missing {required}: {differences}"
        );
    }
    assert!(differences.contains("revision") || differences.contains("policy"));
    assert_eq!(
        row(&scoped, "policy-and-base-and-options")["state"],
        "paused-by-new-runtime"
    );

    let all = document(f.list().args(["--all", "--json"]).output().unwrap());
    assert_eq!(all["scope"], "all");
    assert_eq!(all["unassociated_workspace_count"], 3);
    assert_eq!(all["machines"].as_array().unwrap().len(), 15);
    for name in [
        "custom-builder",
        "custom-login",
        "codex-builder-aaaaaaaaaaaa",
        "codex-login-aaaaaaaaaaaa-r3",
    ] {
        assert_eq!(row(&all, name)["assessment"], "shared");
        assert!(row(&all, name)["workspace_path_sha256"].is_null());
    }
    assert_eq!(
        row(&all, "other-workspace")["assessment"],
        "other-workspace"
    );
    assert_eq!(row(&all, &legacy_workspace)["assessment"], "unknown");
    assert_eq!(row(&all, &legacy_workspace)["source"], "legacy-name");
    assert!(row(&all, &legacy_workspace)["workspace_path_sha256"].is_null());
    assert_eq!(row(&all, "custom-builder")["source"], "labels");
    let sorted = names(&all);
    let mut ordered = sorted.clone();
    ordered.sort();
    assert_eq!(sorted, ordered);
    for value in all["machines"].as_array().unwrap() {
        for field in [
            "name",
            "state",
            "kind",
            "base_id",
            "revision",
            "workspace_path_sha256",
            "workspace_sha256",
            "configuration",
            "source",
            "assessment",
            "reasons",
        ] {
            assert!(value.get(field).is_some(), "missing {field}: {value}");
        }
    }
    let human = String::from_utf8(success(f.list().arg("--all").output().unwrap()).stdout).unwrap();
    for value in [
        f.selected_name(),
        f.path_hash(),
        f.workspace.display().to_string(),
        f.definition.display().to_string(),
        "ASSESSMENT".into(),
        "shared".into(),
        "paused-by-new-runtime".into(),
    ] {
        assert!(human.contains(&value), "missing {value}: {human}");
    }
}

#[test]
fn reference_identity_uses_normalized_hosts_and_name_override() {
    let f = ListingFixture::new();
    let hash = f.hash(&["packages.example.com", "more.example.com"]);
    let mut selected = f.workspace_row("my-explicit-workspace");
    selected["labels"]["codex-sandbox.workspace-sha256"] = json!(hash);
    selected["labels"]["codex-sandbox.configuration"] = json!("custom");
    f.inventory(&[selected]);
    let first = document(
        f.list()
            .env("CODEX_MACHINE", "my-explicit-workspace")
            .args(["--json", "--allow-host", "packages.example.com"])
            .arg(&f.workspace)
            .args([
                "--allow-host=more.example.com",
                "--allow-host",
                "packages.example.com",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(first["reference"]["workspace_sha256"], hash);
    assert_eq!(
        row(&first, "my-explicit-workspace")["assessment"],
        "selected"
    );
    let second = document(
        f.list()
            .env("CODEX_MACHINE", "my-explicit-workspace")
            .args([
                "--allow-host",
                "more.example.com",
                "--json",
                "--allow-host",
                "packages.example.com",
                "--allow-host",
                "api.openai.com",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(first["reference"], second["reference"]);
}

fn snapshot(path: &Path) -> BTreeMap<PathBuf, (Vec<u8>, bool, u32)> {
    let mut result = BTreeMap::new();
    if !path.exists() {
        return result;
    }
    let metadata = fs::metadata(path).unwrap();
    #[cfg(unix)]
    let mode = {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    };
    #[cfg(not(unix))]
    let mode = 0;
    result.insert(
        path.to_owned(),
        (
            if metadata.is_file() {
                fs::read(path).unwrap()
            } else {
                Vec::new()
            },
            metadata.permissions().readonly(),
            mode,
        ),
    );
    if metadata.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            result.extend(snapshot(&entry.unwrap().path()));
        }
    }
    result
}

#[test]
fn listing_skips_preflight_and_preserves_absent_and_invalid_application_state() {
    let f = ListingFixture::new();
    assert!(!f.home.join(".smolsandbox").exists());
    let empty = document(f.list().arg("--json").output().unwrap());
    assert_eq!(empty["machines"], json!([]));
    assert!(!f.home.join(".smolsandbox").exists());
    let human = String::from_utf8(success(f.list().output().unwrap()).stdout).unwrap();
    assert!(human.to_ascii_lowercase().contains("no matching machines"));
    for directory in ["state/ports", "artifacts", "locks", "codex-home"] {
        fs::create_dir_all(f.home.join(".smolsandbox").join(directory)).unwrap();
    }
    for (name, bytes) in [
        ("state/generation.json", "{invalid generation state"),
        ("state/base-id", "previous-generation"),
        (
            "state/ports/my-machine.pending.json",
            "{invalid port journal",
        ),
        ("locks/artifacts.lock", "existing-lock"),
        ("codex-home/auth.json", "existing-credentials"),
    ] {
        fs::write(f.home.join(".smolsandbox").join(name), bytes).unwrap();
    }
    let before = snapshot(&f.home);
    document(f.list().args(["--all", "--json"]).output().unwrap());
    assert_eq!(snapshot(&f.home), before);
    let calls = fs::read_to_string(f.root.join("calls.log")).unwrap();
    assert_eq!(
        calls.lines().collect::<Vec<_>>(),
        [
            "--version",
            "machine ls --json",
            "--version",
            "machine ls --json",
            "--version",
            "machine ls --json"
        ]
    );
}

#[test]
fn workspace_arguments_preserve_relative_and_repository_subdirectory_semantics() {
    let f = ListingFixture::new();
    let subdirectory = f.workspace.join("nested");
    fs::create_dir_all(&subdirectory).unwrap();
    fs::create_dir_all(f.workspace.join(".git")).unwrap();
    f.inventory(&[f.workspace_row(&f.selected_name())]);
    let relative = document(f.list().args(["--json", "."]).output().unwrap());
    let explicit = document(f.list().arg(&f.workspace).arg("--json").output().unwrap());
    assert_eq!(relative, explicit);
    let nested = document(f.list().args(["nested", "--json"]).output().unwrap());
    assert_eq!(nested["workspace"], subdirectory.to_str().unwrap());
    assert_eq!(nested["machines"], json!([]));
    assert_ne!(
        nested["reference"]["workspace_sha256"],
        relative["reference"]["workspace_sha256"]
    );
    let all_nested = document(
        f.list()
            .arg("--all")
            .arg(&subdirectory)
            .arg("--json")
            .output()
            .unwrap(),
    );
    assert_eq!(all_nested["workspace"], nested["workspace"]);
    assert_eq!(all_nested["reference"], nested["reference"]);
    assert_eq!(
        row(&all_nested, &f.selected_name())["assessment"],
        "other-workspace"
    );
    #[cfg(unix)]
    {
        let link = f.root.join("project-link");
        std::os::unix::fs::symlink(&f.workspace, &link).unwrap();
        let linked = document(f.list().arg(link).arg("--json").output().unwrap());
        assert_eq!(linked, explicit);
    }
    let missing = f
        .list()
        .arg(f.root.join("missing-workspace"))
        .arg("--json")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    let file = f.list().arg(&f.definition).arg("--json").output().unwrap();
    assert!(!file.status.success());
    assert!(file.stdout.is_empty());
    let definition_directory = f
        .command()
        .args(["list", "--smolfile"])
        .arg(&f.workspace)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(definition_directory.status.code(), Some(1));
    assert!(definition_directory.stdout.is_empty());
    assert!(String::from_utf8_lossy(&definition_directory.stderr).contains("selected Smolfile"));
}

#[cfg(feature = "test-adapter")]
#[test]
fn absent_default_is_unassessed_but_explicit_or_invalid_definition_is_an_error() {
    let f = ListingFixture::new();
    f.inventory(&[f.workspace_row("my-workspace")]);
    let report = document(f.command().args(["list", "--json"]).output().unwrap());
    assert_eq!(row(&report, "my-workspace")["assessment"], "unassessed");
    assert!(report["reference"]["definition"].is_null());
    assert!(report["reference"]["base_id"].is_null());
    assert!(report["reference"]["workspace_sha256"].is_null());
    assert!(report["reference"]["desired_machine"].is_null());
    assert!(!report["warnings"].as_array().unwrap().is_empty());
    for output in [
        f.command()
            .args(["list", "--json", "--allow-host", "packages.example.com"])
            .output()
            .unwrap(),
        f.command()
            .args(["list", "--json", "--smolfile", "missing.smlbx"])
            .output()
            .unwrap(),
        f.command()
            .env("BASE_SMOLFILE", "missing.smlbx")
            .args(["list", "--json"])
            .output()
            .unwrap(),
    ] {
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    fs::create_dir_all(f.home.join(".smolsandbox")).unwrap();
    fs::write(f.home.join(".smolsandbox/default.smlbx"), DEFINITION).unwrap();
    fs::create_dir_all(f.workspace.join(".git")).unwrap();
    fs::create_dir_all(f.workspace.join(".smolsandbox")).unwrap();
    fs::write(
        f.workspace.join(".smolsandbox/default.smlbx"),
        "malformed TOML",
    )
    .unwrap();
    let invalid_nearer = f.command().args(["list", "--json"]).output().unwrap();
    assert!(!invalid_nearer.status.success());
    assert!(invalid_nearer.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid_nearer.stderr).contains("selected Smolfile"));
    fs::write(&f.definition, "malformed TOML").unwrap();
    let invalid_explicit = f.list().arg("--json").output().unwrap();
    assert!(!invalid_explicit.status.success());
    assert!(invalid_explicit.stdout.is_empty());
}

#[test]
fn inventory_failures_emit_no_partial_json() {
    let f = ListingFixture::new();
    for inventory in [
        json!({"not":"an array"}),
        json!([{"state":"stopped"}]),
        json!([{"name":"same"},{"name":"same"}]),
        json!([{"name":"bad-label","labels":{"codex-sandbox.kind":false}}]),
        json!([{"name":"bad-state","state":42}]),
        json!([{"name":"bad-label-map","labels":[]}]),
    ] {
        fs::write(
            f.root.join("inventory.json"),
            serde_json::to_vec(&inventory).unwrap(),
        )
        .unwrap();
        let output = f.list().arg("--json").output().unwrap();
        assert!(!output.status.success(), "inventory {inventory}");
        assert!(output.stdout.is_empty(), "inventory {inventory}");
        assert!(!output.stderr.is_empty());
    }
    fs::write(f.root.join("invalid-sequence.jsonl"), "{malformed JSON\n").unwrap();
    let invalid_json = f
        .list()
        .env(
            "SMOLVM_TEST_INVENTORY_SEQUENCE_FILE",
            f.root.join("invalid-sequence.jsonl"),
        )
        .arg("--json")
        .output()
        .unwrap();
    assert!(!invalid_json.status.success());
    assert!(invalid_json.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid_json.stderr).contains("inventory"));
    let failed = f
        .list()
        .env("SMOLVM_TEST_INVENTORY_FAIL", "1")
        .arg("--json")
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    let runtime_missing = f
        .list()
        .arg("--smolvm")
        .arg(f.root.join("missing-runtime"))
        .arg("--json")
        .output()
        .unwrap();
    assert!(!runtime_missing.status.success());
    assert!(runtime_missing.stdout.is_empty());
}

#[test]
fn listing_help_and_usage_are_offline_and_reject_mutation_options() {
    let f = ListingFixture::new();
    let help = success(
        f.command()
            .env("SMOLVM_BIN", f.root.join("missing-runtime"))
            .args(["list", "--help"])
            .output()
            .unwrap(),
    );
    let text = String::from_utf8(help.stdout).unwrap().to_ascii_lowercase();
    for word in [
        "--all",
        "--json",
        "--allow-host",
        "--smolfile",
        "--smolvm",
        "application writes",
        "legacy",
        "workspace",
    ] {
        assert!(text.contains(word), "missing {word}: {text}");
    }
    for arguments in [
        vec!["--port", "3000"],
        vec!["--force"],
        vec!["--force-all"],
        vec!["--replace-stale"],
        vec!["--check"],
        vec!["--workspace"],
        vec![".", "another"],
        vec![".", "--", "--help"],
        vec!["--allow-host", "bad\nhost"],
    ] {
        let output = f.command().arg("list").args(&arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
    }
    assert!(!f.root.join("calls.log").exists());
    assert!(!f.home.join(".smolsandbox").exists());
}

#[test]
fn verbose_queries_preserve_json_identity_calls_and_read_only_state() {
    let f = ListingFixture::new();
    f.inventory(&[f.workspace_row(&f.selected_name())]);
    let before = snapshot(&f.home);
    let mut reports = Vec::new();
    for verbose in [false, true] {
        let mut command = f.list();
        command.env("SMOLVM_TEST_OUTPUT", "1").arg("--json");
        if verbose {
            command.arg("--verbose");
        }
        let output = success(command.output().unwrap());
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["reference"]["base_id"], f.base());
        assert_eq!(report["reference"]["workspace_sha256"], f.hash(&[]));
        let errors = String::from_utf8_lossy(&output.stderr);
        for query in ["version", "inventory"] {
            assert_eq!(errors.contains(&format!("query-{query}-stderr")), verbose);
        }
        assert!(!errors.contains(VERSION));
        assert!(!errors.contains(&f.selected_name()));
        assert!(!errors.contains("runtime-"));
        assert_eq!(snapshot(&f.home), before);
        reports.push(report);
    }
    assert_eq!(reports[0], reports[1]);
    assert_eq!(
        fs::read_to_string(f.root.join("calls.log")).unwrap(),
        "--version\nmachine ls --json\n--version\nmachine ls --json\n"
    );
}

#[test]
fn verbose_query_failures_keep_json_empty_and_default_logs_suppressed() {
    for verbose in [false, true] {
        for malformed in [false, true] {
            let f = ListingFixture::new();
            let sequence = f.root.join("sequence.jsonl");
            fs::write(&sequence, "{secret-raw-invalid-inventory\n").unwrap();
            let mut command = f.list();
            command.env("SMOLVM_TEST_OUTPUT", "1").arg("--json");
            if verbose {
                command.arg("--verbose");
            }
            if malformed {
                command.env("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE", sequence);
            } else {
                command.env("SMOLVM_TEST_INVENTORY_FAIL", "1");
            }
            let output = command.output().unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            let errors = String::from_utf8_lossy(&output.stderr);
            assert!(errors.contains("inventory"));
            assert!(errors.contains("--verbose"));
            assert_eq!(errors.contains("query-inventory-stderr"), verbose);
            assert!(!errors.contains("secret-raw-invalid-inventory"));
            assert_eq!(
                errors.contains("simulated inventory failure"),
                verbose && !malformed
            );
            if !malformed {
                assert!(errors.contains("70"));
            }
        }
    }
}

#[test]
fn selected_runtime_spawn_failures_report_context_without_partial_json() {
    for verbose in [false, true] {
        let f = ListingFixture::new();
        let executable = f.root.join("invalid-runtime.exe");
        fs::write(&executable, "invalid executable fixture\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o644)).unwrap();
        }
        let before = snapshot(&f.home);
        let mut command = f.list();
        command.arg("--json").arg("--smolvm").arg(&executable);
        if verbose {
            command.arg("--verbose");
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let errors = String::from_utf8_lossy(&output.stderr);
        assert!(errors.contains("SmolVM version query failed"), "{errors}");
        assert!(errors.contains("--verbose"), "{errors}");
        assert!(errors.contains("os error"), "{errors}");
        assert!(!errors.contains("invalid executable fixture"));
        assert!(!f.root.join("calls.log").exists());
        assert_eq!(snapshot(&f.home), before);
    }
}
