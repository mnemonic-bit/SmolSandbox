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
        let mut command = Command::new(env!("CARGO_BIN_EXE_smolsbx"));
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

    fn immutable(&self, additions: &[&str]) -> Vec<String> {
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
        immutable
    }

    fn hash(&self, additions: &[&str]) -> String {
        digest(&serde_json::to_vec(&self.immutable(additions)).unwrap())
    }

    fn selected_name(&self) -> String {
        format!(
            "codex-workspace-{}-r7-{}",
            self.base(),
            &self.hash(&[])[..12]
        )
    }

    fn path_hash(&self) -> String {
        digest(self.workspace.to_str().unwrap().as_bytes())
    }

    fn workspace_row(&self, name: &str) -> Value {
        json!({"name": name, "state": "stopped", "labels": {
            "SmolSandbox.kind": "workspace",
            "SmolSandbox.base-id": self.base(),
            "SmolSandbox.revision": "7",
            "SmolSandbox.workspace-sha256": self.hash(&[]),
            "SmolSandbox.workspace-path-sha256": self.path_hash(),
            "SmolSandbox.configuration": "default"
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
    policy["labels"]["SmolSandbox.revision"] = json!("4");
    policy["labels"]["SmolSandbox.base-id"] = json!("111111111111");
    policy["labels"]["SmolSandbox.workspace-sha256"] = json!("b".repeat(64));
    policy["labels"]["SmolSandbox.configuration"] = json!("custom");
    policy["state"] = json!("paused-by-new-runtime");
    let mut base = f.workspace_row("another-harness-generation");
    base["labels"]["SmolSandbox.base-id"] = json!("222222222222");
    let mut configuration = f.workspace_row("custom-options");
    configuration["labels"]["SmolSandbox.workspace-sha256"] = json!("c".repeat(64));
    configuration["labels"]["SmolSandbox.configuration"] = json!("custom");
    let different_name = f.workspace_row("name-override-from-another-invocation");
    let mut other = f.workspace_row("other-workspace");
    other["labels"]["SmolSandbox.workspace-path-sha256"] = json!("d".repeat(64));
    let mut contradiction = f.workspace_row("codex-builder-333333333333");
    contradiction["labels"]["SmolSandbox.kind"] = json!("login");
    let mut missing = f.workspace_row("missing-path");
    missing["labels"]
        .as_object_mut()
        .unwrap()
        .remove("SmolSandbox.workspace-path-sha256");
    let mut malformed = f.workspace_row("malformed-path");
    malformed["labels"]["SmolSandbox.workspace-path-sha256"] = json!("not-a-hash");
    let builder = json!({"name":"custom-builder", "state":"running", "labels":{
        "SmolSandbox.kind":"builder", "SmolSandbox.base-id":f.base()
    }});
    let login = json!({"name":"custom-login", "state":"created", "labels":{
        "SmolSandbox.kind":"login", "SmolSandbox.base-id":f.base(), "SmolSandbox.revision":"7"
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
        json!({"name":"foreign-namespace-label","labels":{"SmolSandbox.future":"value"}}),
    ]);
    let scoped = document(f.list().arg("--json").output().unwrap());
    assert_eq!(scoped["version"], 1);
    assert_eq!(scoped["scope"], "workspace");
    assert_eq!(scoped["workspace"], f.workspace.to_str().unwrap());
    assert_eq!(scoped["unassociated_workspace_count"], 3);
    assert_eq!(scoped["reference"]["base_id"], f.base());
    assert_eq!(scoped["reference"]["revision"], "7");
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

    assert_eq!(
        formatted_document(
            f.list().args(["--format", "yaml"]).output().unwrap(),
            "yaml"
        ),
        scoped
    );
    let all = document(f.list().args(["--all", "--json"]).output().unwrap());
    assert_eq!(
        formatted_document(
            f.list().args(["--all", "--format=yaml"]).output().unwrap(),
            "yaml"
        ),
        all
    );
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
            "workspace_path",
            "workspace_path_source",
            "harness",
            "base_inputs",
            "machine_inputs",
            "metadata_status",
            "metadata_reasons",
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
    selected["labels"]["SmolSandbox.workspace-sha256"] = json!(hash);
    selected["labels"]["SmolSandbox.configuration"] = json!("custom");
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
        json!([{"name":"bad-label","labels":{"SmolSandbox.kind":false}}]),
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

fn base_metadata(f: &ListingFixture) -> Value {
    json!({
        "version": 1,
        "smolvm_version": VERSION,
        "definition_sha256": digest(DEFINITION.as_bytes()),
        "definition_path": f.root.join("deleted definition = λ.smlbx").to_str().unwrap(),
        "harness": "codex"
    })
}

fn machine_metadata(f: &ListingFixture) -> Value {
    json!({"version":1,"workspace_path":f.workspace.to_str().unwrap(),"options":f.immutable(&[])})
}

fn with_metadata(mut machine: Value, base: Option<Value>, inputs: Option<Value>) -> Value {
    for (key, value) in [
        ("SmolSandbox.base-inputs", base),
        ("SmolSandbox.machine-inputs", inputs),
    ] {
        if let Some(value) = value {
            machine["labels"][key] = json!(serde_json::to_string(&value).unwrap());
        }
    }
    machine
}

#[test]
fn metadata_status_components_and_reference_fallback_are_independent() {
    for format in ["yaml", "json"] {
        let f = ListingFixture::new();
        let base = base_metadata(&f);
        let inputs = machine_metadata(&f);
        let mut rows = Vec::new();
        for (name, b, m) in [
            ("complete", Some(base.clone()), Some(inputs.clone())),
            ("base-only", Some(base.clone()), None),
            ("machine-only", None, Some(inputs.clone())),
            ("unavailable", None, None),
            (
                "unsupported-base",
                Some(json!({"version":99})),
                Some(inputs.clone()),
            ),
            (
                "unsupported-machine",
                Some(base.clone()),
                Some(json!({"version":99})),
            ),
            (
                "both-future",
                Some(json!({"version":99})),
                Some(json!({"version":99})),
            ),
            (
                "invalid-precedes-future",
                Some(json!({"version":1})),
                Some(json!({"version":99})),
            ),
        ] {
            rows.push(with_metadata(f.workspace_row(name), b, m));
        }
        let mut other = f.workspace_row("old-other-workspace");
        other["labels"]["SmolSandbox.workspace-path-sha256"] = json!("c".repeat(64));
        rows.push(other);
        rows.push(json!({"name":"codex-builder-aaaaaaaaaaaa","state":"stopped"}));
        f.inventory(&rows);
        let before = snapshot(&f.home);
        let report = formatted_document(
            f.list()
                .args(["--all", "--format", format])
                .output()
                .unwrap(),
            format,
        );
        for (name, status, b, m, source) in [
            ("complete", "complete", true, true, "label"),
            ("base-only", "partial", true, false, "reference"),
            ("machine-only", "partial", false, true, "label"),
            ("unavailable", "unavailable", false, false, "reference"),
            ("unsupported-base", "unsupported", false, true, "label"),
            (
                "unsupported-machine",
                "unsupported",
                true,
                false,
                "reference",
            ),
            ("both-future", "unsupported", false, false, "reference"),
            (
                "invalid-precedes-future",
                "invalid",
                false,
                false,
                "reference",
            ),
        ] {
            let value = row(&report, name);
            assert_eq!(value["metadata_status"], status, "{name}");
            assert_eq!(!value["base_inputs"].is_null(), b, "{name}");
            assert_eq!(!value["machine_inputs"].is_null(), m, "{name}");
            assert_eq!(
                value["harness"],
                if b { json!("codex") } else { Value::Null },
                "{name}"
            );
            assert_eq!(value["workspace_path_source"], source, "{name}");
            assert_eq!(
                value["workspace_path"],
                f.workspace.to_str().unwrap(),
                "{name}"
            );
            assert_eq!(value["assessment"], "different-configuration");
            assert_eq!(
                value["metadata_reasons"].as_array().unwrap().is_empty(),
                status == "complete"
            );
        }
        assert_eq!(row(&report, "complete")["base_inputs"], base);
        assert_eq!(row(&report, "complete")["machine_inputs"], inputs);
        for name in ["old-other-workspace", "codex-builder-aaaaaaaaaaaa"] {
            assert!(row(&report, name)["workspace_path"].is_null());
            assert_eq!(row(&report, name)["workspace_path_source"], "unknown");
            assert_eq!(row(&report, name)["metadata_status"], "unavailable");
        }
        assert_eq!(snapshot(&f.home), before);
        assert_eq!(
            fs::read_to_string(f.root.join("calls.log")).unwrap(),
            "--version\nmachine ls --json\n"
        );
    }
}

#[test]
fn malformed_or_contradictory_metadata_preserves_scope_and_authoritative_assessment() {
    for format in ["yaml", "json"] {
        let f = ListingFixture::new();
        let base = base_metadata(&f);
        let inputs = machine_metadata(&f);
        let mut cases: Vec<(&str, Value, Value, &str)> = Vec::new();
        for (name, field, value) in [
            (
                "wrong-fingerprint",
                "definition_sha256",
                json!("z".repeat(64)),
            ),
            ("wrong-profile", "harness", json!("future-profile")),
            ("wrong-version-type", "version", json!("1")),
            ("wrong-definition-type", "definition_path", json!(false)),
            ("wrong-runtime-type", "smolvm_version", json!([])),
        ] {
            let mut changed = base.clone();
            changed[field] = value;
            cases.push((name, changed, inputs.clone(), "base-inputs"));
        }
        let mut absent = base.clone();
        absent.as_object_mut().unwrap().remove("harness");
        cases.push(("missing-base-field", absent, inputs.clone(), "base-inputs"));
        for name in [
            "wrong-path",
            "wrong-mount",
            "wrong-from",
            "wrong-resource",
            "unsorted",
            "duplicates",
            "wrong-option-type",
            "wrong-array-type",
            "missing-machine-field",
        ] {
            let mut changed = inputs.clone();
            match name {
                "wrong-path" => {
                    changed["workspace_path"] = json!(f.root.join("other").to_str().unwrap())
                }
                "wrong-array-type" => changed["options"] = json!(false),
                "missing-machine-field" => {
                    changed.as_object_mut().unwrap().remove("workspace_path");
                }
                _ => {
                    let options = changed["options"].as_array_mut().unwrap();
                    match name {
                        "unsorted" => options.reverse(),
                        "duplicates" => options.push(options[0].clone()),
                        "wrong-option-type" => options[0] = json!(true),
                        _ => {
                            let prefix = match name {
                                "wrong-mount" => "--volume=",
                                "wrong-from" => "--from=",
                                _ => "--mem=",
                            };
                            let index = options
                                .iter()
                                .position(|v| v.as_str().unwrap().starts_with(prefix))
                                .unwrap();
                            options[index] = json!(match name {
                                "wrong-mount" =>
                                    "--volume=/different:/workspace/myrepo".to_string(),
                                "wrong-from" => format!(
                                    "--from={}/codex-devtools-aaaaaaaaaaaa.smolmachine",
                                    f.root.display()
                                ),
                                _ => "--mem=4096".to_string(),
                            });
                            options.sort_by(|a, b| a.as_str().unwrap().cmp(b.as_str().unwrap()));
                        }
                    }
                }
            }
            cases.push((name, base.clone(), changed, "machine-inputs"));
        }
        let mut machines = Vec::new();
        for (name, b, m, _) in &cases {
            machines.push(with_metadata(
                f.workspace_row(name),
                Some(b.clone()),
                Some(m.clone()),
            ));
        }
        let mut malformed = with_metadata(f.workspace_row("malformed-json"), None, Some(inputs));
        malformed["labels"]["SmolSandbox.base-inputs"] = json!("{secret-malformed-sentinel");
        machines.push(malformed);
        let mut future_extra = base;
        future_extra["future_field"] = json!({"unknown":"accepted"});
        machines.push(with_metadata(
            f.workspace_row("unknown-extra-field"),
            Some(future_extra),
            None,
        ));
        f.inventory(&machines);
        let described = formatted_document(
            f.list()
                .args(["--all", "--format", format])
                .output()
                .unwrap(),
            format,
        );
        for (name, _, _, component) in &cases {
            let result = row(&described, name);
            assert_eq!(result["metadata_status"], "invalid", "{name}: {result}");
            assert!(
                result[if *component == "base-inputs" {
                    "base_inputs"
                } else {
                    "machine_inputs"
                }]
                .is_null(),
                "{name}"
            );
            assert!(
                !result[if *component == "base-inputs" {
                    "machine_inputs"
                } else {
                    "base_inputs"
                }]
                .is_null(),
                "{name}"
            );
            let reasons = result["metadata_reasons"].to_string();
            assert!(
                reasons.contains(name) && reasons.contains(component),
                "{reasons}"
            );
        }
        assert_eq!(
            row(&described, "malformed-json")["metadata_status"],
            "invalid"
        );
        assert!(!described.to_string().contains("secret-malformed-sentinel"));
        assert_eq!(
            row(&described, "unknown-extra-field")["metadata_status"],
            "partial"
        );
        for machine in &mut machines {
            let labels = machine["labels"].as_object_mut().unwrap();
            labels.remove("SmolSandbox.base-inputs");
            labels.remove("SmolSandbox.machine-inputs");
        }
        f.inventory(&machines);
        let legacy = formatted_document(
            f.list()
                .args(["--all", "--format", format])
                .output()
                .unwrap(),
            format,
        );
        assert_eq!(names(&legacy), names(&described));
        for name in names(&legacy) {
            for field in [
                "source",
                "assessment",
                "reasons",
                "workspace_path_sha256",
                "workspace_sha256",
            ] {
                assert_eq!(
                    row(&legacy, name)[field],
                    row(&described, name)[field],
                    "{name}: {field}"
                );
            }
        }
    }
}

#[test]
fn readable_metadata_round_trips_full_values_and_escapes_human_controls() {
    let f = ListingFixture::new();
    let mut base = base_metadata(&f);
    base["smolvm_version"] = json!("smolvm build=λ\\debug\tline\nnext\u{1b}[31m");
    base["definition_path"] = json!(format!(
        "{}/missing = λ\\definition\t\n.smlbx",
        f.root.display()
    ));
    let name = "machine\twith-control\u{1b}[31m";
    let mut machine = f.workspace_row(name);
    machine["state"] = json!("paused\u{7}state");
    let inputs = machine_metadata(&f);
    f.inventory(&[with_metadata(
        machine,
        Some(base.clone()),
        Some(inputs.clone()),
    )]);
    let report = document(f.list().args(["--all", "--json"]).output().unwrap());
    let result = row(&report, name);
    assert_eq!(result["metadata_status"], "complete");
    assert_eq!(result["base_inputs"], base);
    assert_eq!(result["machine_inputs"], inputs);
    let human = String::from_utf8(success(f.list().arg("--all").output().unwrap()).stdout).unwrap();
    for text in [
        "WORKSPACE",
        "HARNESS",
        "codex",
        "--mem=12288",
        "--cpus=4",
        "--allow-host=api.openai.com",
        "--allow-host=downloads.example.com",
        f.workspace.to_str().unwrap(),
        &digest(DEFINITION.as_bytes()),
    ] {
        assert!(human.contains(text), "missing {text}: {human}");
    }
    for character in ['\u{1b}', '\u{7}', '\r'] {
        assert!(!human.contains(character), "unescaped control: {human:?}");
    }
    assert!(!human.contains("machine\twith\ncontrol"));
    assert!(!human.contains("debug\tline\nnext"));
    assert!(human.contains("\\t") && human.contains("\\n"));
    let matching_paths: Vec<_> = report["machines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["workspace_path"] == f.workspace.to_str().unwrap())
        .collect();
    let matching_hosts: Vec<_> = report["machines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| {
            v["machine_inputs"]["options"]
                .as_array()
                .is_some_and(|options| {
                    options.contains(&json!("--allow-host=downloads.example.com"))
                })
        })
        .collect();
    assert_eq!(matching_paths, matching_hosts);
    assert_eq!(matching_paths.len(), 1);
}

#[test]
fn descriptive_labels_cannot_establish_ownership_or_workspace_association() {
    let f = ListingFixture::new();
    let inputs = machine_metadata(&f);
    let base = base_metadata(&f);
    let legacy_name = format!("codex-workspace-{}-r3-{}", f.base(), &f.hash(&[])[..12]);
    f.inventory(&[
        with_metadata(
            json!({"name":"metadata-only","state":"stopped","labels":{}}),
            Some(base.clone()),
            Some(inputs.clone()),
        ),
        with_metadata(
            json!({"name":legacy_name,"state":"stopped","labels":{}}),
            Some(base),
            Some(inputs),
        ),
    ]);
    let scoped = document(f.list().arg("--json").output().unwrap());
    assert_eq!(scoped["machines"], json!([]));
    let all = document(f.list().args(["--all", "--json"]).output().unwrap());
    assert_eq!(names(&all), [legacy_name.as_str()]);
    let value = row(&all, &legacy_name);
    assert_eq!(value["assessment"], "unknown");
    assert!(value["workspace_path"].is_null());
    assert_eq!(value["workspace_path_source"], "unknown");
}

#[test]
fn published_json_filters_select_recorded_paths_and_exact_host_options() {
    use std::{io::Write, process::Stdio};

    let f = ListingFixture::new();
    let described = with_metadata(
        f.workspace_row("recorded-workspace"),
        Some(base_metadata(&f)),
        Some(machine_metadata(&f)),
    );
    let mut other = f.workspace_row("old-other-workspace");
    other["labels"]["SmolSandbox.workspace-path-sha256"] = json!("b".repeat(64));
    f.inventory(&[described, other]);
    let report = success(f.list().args(["--all", "--json"]).output().unwrap());
    for (source, variable, value, expression) in [
        (
            report.stdout.clone(),
            "path",
            f.workspace.to_str().unwrap(),
            ".machines[] | select(.workspace_path == $path)",
        ),
        (
            report.stdout,
            "option",
            "--allow-host=downloads.example.com",
            ".machines[] | select((.machine_inputs.options // []) | index($option))",
        ),
        (
            fs::read(f.root.join("inventory.json")).unwrap(),
            "path",
            f.workspace.to_str().unwrap(),
            ".[] | (.labels[\"SmolSandbox.machine-inputs\"]? | fromjson?) as $inputs | select($inputs.workspace_path == $path)",
        ),
    ] {
        let child = Command::new("jq")
            .args(["--arg", variable, value, expression])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => panic!("could not start optional jq: {error}"),
        };
        child.stdin.take().unwrap().write_all(&source).unwrap();
        let result = success(child.wait_with_output().unwrap());
        let selected: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(selected["name"], "recorded-workspace");
    }
}

#[test]
fn mixed_label_prefix_inventory_reads_legacy_metadata_and_exposes_conflicts() {
    let f = ListingFixture::new();
    let described = with_metadata(
        f.workspace_row("new-prefix"),
        Some(base_metadata(&f)),
        Some(machine_metadata(&f)),
    );
    let mut rows = vec![described.clone()];
    for (name, keep_current) in [("old-prefix", false), ("dual-prefix", true)] {
        let mut machine = described.clone();
        machine["name"] = json!(name);
        let labels = machine["labels"].as_object_mut().unwrap();
        for (key, value) in labels.clone() {
            let suffix = key.strip_prefix("SmolSandbox.").unwrap();
            labels.insert(format!("codex-sandbox.{suffix}"), value);
            if !keep_current {
                labels.remove(&key);
            }
        }
        rows.push(machine);
    }
    let mut conflict = rows[2].clone();
    conflict["name"] = json!("ownership-conflict");
    conflict["labels"]["codex-sandbox.configuration"] = json!("custom");
    rows.push(conflict);
    let mut descriptive_conflict = rows[2].clone();
    descriptive_conflict["name"] = json!("descriptive-conflict");
    descriptive_conflict["labels"]["codex-sandbox.machine-inputs"] = json!("{}");
    rows.push(descriptive_conflict);
    rows.push(
        json!({"name":"descriptive-only","state":"stopped","labels":{
            "SmolSandbox.base-inputs":"{}", "codex-sandbox.machine-inputs":"{}"
        }}),
    );
    f.inventory(&rows);
    let report = document(f.list().args(["--all", "--json"]).output().unwrap());
    assert_eq!(names(&report).len(), 5);
    for name in ["new-prefix", "old-prefix", "dual-prefix"] {
        assert_eq!(row(&report, name)["source"], "labels", "{name}");
        assert_eq!(
            row(&report, name)["workspace_path_sha256"],
            f.path_hash(),
            "{name}"
        );
        assert_eq!(row(&report, name)["metadata_status"], "complete", "{name}");
        assert_eq!(
            row(&report, name)["machine_inputs"]["workspace_path"],
            f.workspace.to_str().unwrap(),
            "{name}"
        );
    }
    assert_eq!(row(&report, "ownership-conflict")["assessment"], "unknown");
    assert_eq!(
        row(&report, "descriptive-conflict")["metadata_status"],
        "invalid"
    );
    assert_eq!(
        row(&report, "descriptive-conflict")["assessment"],
        "different-configuration"
    );
}

fn formatted_document(output: Output, format: &str) -> Value {
    let output = success(output);
    assert!(output.stderr.is_empty(), "{output:?}");
    if format == "yaml" {
        serde_saphyr::from_slice(&output.stdout).unwrap()
    } else {
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

#[test]
fn format_syntax_positions_alias_and_empty_report_preserve_the_contract() {
    let f = ListingFixture::new();
    let expected = json!({
        "version": 1, "scope": "all", "workspace": f.workspace,
        "reference": {"definition": f.definition, "base_id": f.base(), "revision": "7",
            "workspace_sha256": f.hash(&["extra.example"]),
            "desired_machine": format!("codex-workspace-{}-r7-{}", f.base(), &f.hash(&["extra.example"])[..12])},
        "unassociated_workspace_count": 0, "warnings": [], "machines": []
    });
    for format in ["yaml", "json"] {
        for equals in [false, true] {
            for workspace_first in [false, true] {
                let mut command = f.command();
                command
                    .args(["list", "--all", "--verbose", "--smolfile"])
                    .arg(&f.definition)
                    .arg("--smolvm")
                    .arg(env!("CARGO_BIN_EXE_smolvm-test-double"))
                    .args([
                        "--allow-host",
                        "extra.example",
                        "--allow-host=extra.example",
                    ]);
                if workspace_first {
                    command.arg(&f.workspace);
                }
                if equals {
                    command.arg(format!("--format={format}"));
                } else {
                    command.args(["--format", format]);
                }
                if !workspace_first {
                    command.arg(&f.workspace);
                }
                assert_eq!(
                    formatted_document(command.output().unwrap(), format),
                    expected
                );
            }
        }
    }
    let alias = success(f.list().arg("--json").output().unwrap());
    let primary = success(f.list().args(["--format", "json"]).output().unwrap());
    assert_eq!(alias.stdout, primary.stdout);
    assert_eq!(primary.stdout.last(), Some(&b'\n'));
    assert_eq!(
        primary.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    let yaml = success(f.list().arg("--format=yaml").output().unwrap());
    let text = String::from_utf8(yaml.stdout.clone()).unwrap();
    assert!(text.starts_with("version: 1\n"), "{text}");
    assert!(text.contains("\nreference:\n  definition:"), "{text}");
    assert!(text.ends_with("machines: []\n"), "{text}");
    assert!(!text.contains("---") && !text.contains("..."));
    assert!(!text.ends_with("\n\n"));
    assert_eq!(
        yaml.stdout,
        success(f.list().arg("--format=yaml").output().unwrap()).stdout
    );
}

#[test]
fn structured_formats_keep_native_strings_nested_inputs_and_sorted_rows() {
    let f = ListingFixture::new();
    let values = [
        "null",
        "true",
        "00123",
        "with spaces λ",
        r"C:\Users\name\a=b",
        "colon: value # hash",
        "quote\"single'",
        "line\nnext\t\u{1b}\u{7}\u{2028}",
    ];
    let mut rows = Vec::new();
    for (index, value) in values.into_iter().enumerate().rev() {
        let name = format!("{index}-{}", value.replace('\n', "\\n"));
        let mut machine = f.workspace_row(&name);
        machine["state"] = json!(value.replace('\n', "\\n"));
        let mut base = base_metadata(&f);
        base["smolvm_version"] = json!(value);
        base["definition_path"] = json!(format!("/recorded/{value}/definition.smlbx"));
        rows.push(with_metadata(
            machine,
            Some(base),
            Some(machine_metadata(&f)),
        ));
    }
    f.inventory(&rows);
    let json_output = success(f.list().args(["--all", "--format=json"]).output().unwrap());
    let json_report: Value = serde_json::from_slice(&json_output.stdout).unwrap();
    for format in ["yaml", "json"] {
        let output = success(
            f.list()
                .args(["--all", "--format", format])
                .output()
                .unwrap(),
        );
        let report = formatted_document(output.clone(), format);
        assert_eq!(report, json_report);
        assert_eq!(report["version"], 1);
        assert_eq!(report["workspace"], f.workspace.to_str().unwrap());
        assert_eq!(
            report["reference"]["definition"],
            f.definition.to_str().unwrap()
        );
        assert_eq!(report["reference"]["revision"], "7");
        let mut expected_names = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            let name = format!("{index}-{}", value.replace('\n', "\\n"));
            expected_names.push(name.clone());
            let machine = row(&report, &name);
            assert_eq!(machine["state"], value.replace('\n', "\\n"));
            assert_eq!(machine["metadata_status"], "complete", "{machine}");
            assert_eq!(machine["base_inputs"]["version"], 1);
            assert_eq!(machine["base_inputs"]["smolvm_version"], value);
            assert_eq!(
                machine["base_inputs"]["definition_path"],
                format!("/recorded/{value}/definition.smlbx")
            );
            assert_eq!(machine["machine_inputs"], machine_metadata(&f));
            assert_eq!(machine["metadata_reasons"], json!([]));
        }
        assert_eq!(names(&report), expected_names);
        assert_eq!(
            output.stdout,
            success(
                f.list()
                    .args(["--all", "--format", format])
                    .output()
                    .unwrap()
            )
            .stdout
        );
        if format == "yaml" {
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(text.contains("\nmachines:\n  - name:"), "{text}");
            assert!(
                text.contains("\n    base_inputs:\n      version: 1\n"),
                "{text}"
            );
            assert!(text.contains("\n      options:\n        - "), "{text}");
            assert!(!text.contains('\t') && !text.contains('\u{1b}') && !text.contains('\u{7}'));
            assert!(!text.lines().any(|line| line == "---" || line == "..."));
            assert!(text
                .lines()
                .all(|line| line.bytes().take_while(|byte| *byte == b' ').count() % 2 == 0));
        }
    }
}

fn human_reference(f: &ListingFixture) -> String {
    format!("Reference workspace: {}\nSelected definition: {}\nDesired workspace machine: {}\nCurrent policy revision: 7\n", f.workspace.display(), f.definition.display(), f.selected_name())
}

const HUMAN_SUMMARY: &str = "Unassociated workspace candidates: 0 (missing or malformed path labels; association with the reference workspace is unknown).\nAssessments compare the current invocation only; no assessment authorizes deletion. An identity match does not establish artifact availability, port reconciliation, guest boot, or authentication.\n";

#[test]
fn human_output_has_exact_separators_for_zero_one_and_multiple_entries() {
    let f = ListingFixture::new();
    let empty = success(f.list().output().unwrap());
    assert_eq!(
        String::from_utf8(empty.stdout).unwrap(),
        format!(
            "{}\n{HUMAN_SUMMARY}\nNo matching machines.\n",
            human_reference(&f)
        )
    );
    let selected = f.selected_name();
    let mut base = base_metadata(&f);
    base["smolvm_version"] = json!("line\nnext\t\u{1b}");
    let complete = with_metadata(
        f.workspace_row(&selected),
        Some(base.clone()),
        Some(machine_metadata(&f)),
    );
    let detail = |name: &str, assessment: &str, reason: &str| {
        let mut entry = format!("{name}\tstopped\tworkspace\t{}\t7\t{assessment}\t{}\tcodex\n{reason}  Workspace SHA-256: {}\n  Metadata: complete\n  Recorded SmolVM version: line\\nnext\\t\\u{{1b}}\n  Recorded definition: {}\n  Definition SHA-256: {}\n", f.base(), f.workspace.display(), f.hash(&[]), base["definition_path"].as_str().unwrap(), digest(DEFINITION.as_bytes()));
        for option in f.immutable(&[]) {
            entry.push_str(&format!("  Creation option: {option}\n"));
        }
        entry
    };
    let header = "NAME\tSTATE\tKIND\tBASE\tREVISION\tASSESSMENT\tWORKSPACE\tHARNESS\n";
    f.inventory(std::slice::from_ref(&complete));
    assert_eq!(
        String::from_utf8(success(f.list().output().unwrap()).stdout).unwrap(),
        format!(
            "{}\n{HUMAN_SUMMARY}\n{header}{}",
            human_reference(&f),
            detail(&selected, "selected", "")
        )
    );
    let mut other = complete.clone();
    other["name"] = json!("zz-other");
    f.inventory(&[other, complete]);
    let reason = format!(
        "  Machine name differs from the desired machine {selected} for the current invocation.\n"
    );
    assert_eq!(
        String::from_utf8(success(f.list().output().unwrap()).stdout).unwrap(),
        format!(
            "{}\n{HUMAN_SUMMARY}\n{header}{}\n{}",
            human_reference(&f),
            detail(&selected, "selected", ""),
            detail("zz-other", "different-configuration", &reason)
        )
    );
}

#[test]
fn every_format_preserves_absent_state_and_query_only_calls() {
    for selector in [
        vec![],
        vec!["--json"],
        vec!["--format=json"],
        vec!["--format=yaml"],
    ] {
        let f = ListingFixture::new();
        let before = snapshot(&f.home);
        success(f.list().args(&selector).output().unwrap());
        assert_eq!(snapshot(&f.home), before);
        assert!(!f.home.join(".smolsandbox").exists());
        for directory in ["state/ports", "artifacts", "locks", "codex-home"] {
            fs::create_dir_all(f.home.join(".smolsandbox").join(directory)).unwrap();
        }
        fs::write(
            f.home.join(".smolsandbox/locks/artifacts.lock"),
            "preserve-lock",
        )
        .unwrap();
        fs::write(
            f.home.join(".smolsandbox/state/generation.json"),
            "invalid state",
        )
        .unwrap();
        let before = snapshot(&f.home);
        success(f.list().args(&selector).arg("--all").output().unwrap());
        assert_eq!(snapshot(&f.home), before);
        assert_eq!(
            fs::read_to_string(f.root.join("calls.log")).unwrap(),
            "--version\nmachine ls --json\n--version\nmachine ls --json\n"
        );
    }
}

#[test]
fn both_structured_formats_preserve_failures_and_verbose_streams() {
    for format in ["yaml", "json"] {
        for verbose in [false, true] {
            let f = ListingFixture::new();
            let before = snapshot(&f.home);
            let mut command = f.list();
            command
                .args(["--format", format])
                .env("SMOLVM_TEST_OUTPUT", "1");
            if verbose {
                command.arg("--verbose");
            }
            let output = success(command.output().unwrap());
            let report: Value = if format == "yaml" {
                serde_saphyr::from_slice(&output.stdout).unwrap()
            } else {
                serde_json::from_slice(&output.stdout).unwrap()
            };
            assert_eq!(report["machines"], json!([]));
            let errors = String::from_utf8_lossy(&output.stderr);
            assert_eq!(errors.contains("query-version-stderr"), verbose);
            assert_eq!(errors.contains("query-inventory-stderr"), verbose);
            for inventory in [
                json!({"wrong":"shape"}),
                json!([{"state":"stopped"}]),
                json!([{"name":"same"},{"name":"same"}]),
                json!([{"name":"bad", "labels":[]}]),
            ] {
                fs::write(
                    f.root.join("inventory.json"),
                    serde_json::to_vec(&inventory).unwrap(),
                )
                .unwrap();
                let mut command = f.list();
                command.args(["--format", format]);
                if verbose {
                    command.arg("--verbose");
                }
                let output = command.output().unwrap();
                assert!(!output.status.success());
                assert!(output.stdout.is_empty());
                assert!(!output.stderr.is_empty());
            }
            for failure in ["inventory", "runtime", "workspace", "definition"] {
                let mut command = f.list();
                command.args(["--format", format]);
                if verbose {
                    command.arg("--verbose");
                }
                match failure {
                    "inventory" => {
                        command.env("SMOLVM_TEST_INVENTORY_FAIL", "1");
                    }
                    "runtime" => {
                        command.arg("--smolvm").arg(f.root.join("missing-runtime"));
                    }
                    "workspace" => {
                        command.arg(f.root.join("missing-workspace"));
                    }
                    "definition" => {
                        fs::write(&f.definition, "invalid TOML").unwrap();
                    }
                    _ => unreachable!(),
                }
                let output = command.output().unwrap();
                assert!(!output.status.success(), "{failure}");
                assert!(output.stdout.is_empty(), "{failure}");
                assert!(!output.stderr.is_empty(), "{failure}");
            }
            assert_eq!(snapshot(&f.home), before);
        }
    }
}

#[cfg(feature = "test-adapter")]
#[test]
fn missing_default_preserves_nulls_and_exact_human_warning_blocks() {
    let f = ListingFixture::new();
    let reference = format!("Reference workspace: {}\nSelected definition: unavailable\nDesired workspace machine: unavailable\nCurrent policy revision: 7\n", f.workspace.display());
    let warning = "WARNING: No automatic default definition is available. Select a definition with --smolfile to compare launch identity.\n";
    let prefix = format!("{reference}\n{warning}\n{HUMAN_SUMMARY}\n");
    for count in 0..=2 {
        let rows: Vec<_> = (0..count)
            .map(|index| {
                with_metadata(
                    f.workspace_row(&format!("machine-{index}")),
                    Some(base_metadata(&f)),
                    Some(machine_metadata(&f)),
                )
            })
            .collect();
        f.inventory(&rows);
        for format in ["yaml", "json"] {
            let report = formatted_document(
                f.command()
                    .args(["list", "--format", format])
                    .output()
                    .unwrap(),
                format,
            );
            assert_eq!(
                report["reference"],
                json!({"definition":null, "base_id":null, "revision":"7", "workspace_sha256":null, "desired_machine":null})
            );
            assert_eq!(report["warnings"], json!(["No automatic default definition is available. Select a definition with --smolfile to compare launch identity."]));
            assert_eq!(report["machines"].as_array().unwrap().len(), count);
            for machine in report["machines"].as_array().unwrap() {
                assert_eq!(machine["assessment"], "unassessed");
                assert_eq!(machine["metadata_status"], "complete");
            }
        }
        let mut expected = prefix.clone();
        if count == 0 {
            expected.push_str("No matching machines.\n");
        } else {
            expected
                .push_str("NAME\tSTATE\tKIND\tBASE\tREVISION\tASSESSMENT\tWORKSPACE\tHARNESS\n");
            for index in 0..count {
                if index > 0 {
                    expected.push('\n');
                }
                expected.push_str(&format!("machine-{index}\tstopped\tworkspace\t{}\t7\tunassessed\t{}\tcodex\n  Reference definition is unavailable; launch identity cannot be assessed.\n  Workspace SHA-256: {}\n  Metadata: complete\n  Recorded SmolVM version: {VERSION}\n  Recorded definition: {}\n  Definition SHA-256: {}\n", f.base(), f.workspace.display(), f.hash(&[]), base_metadata(&f)["definition_path"].as_str().unwrap(), digest(DEFINITION.as_bytes())));
                for option in f.immutable(&[]) {
                    expected.push_str(&format!("  Creation option: {option}\n"));
                }
            }
        }
        let human = success(f.command().arg("list").output().unwrap());
        assert_eq!(String::from_utf8(human.stdout).unwrap(), expected);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn structured_stdout_write_failures_return_nonzero() {
    for format in ["yaml", "json"] {
        let f = ListingFixture::new();
        let output = f
            .list()
            .args(["--format", format])
            .stdout(
                fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .unwrap(),
            )
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn invalid_format_selectors_do_not_query_an_available_runtime() {
    let f = ListingFixture::new();
    for arguments in [
        vec!["--format"],
        vec!["--format="],
        vec!["--format", ""],
        vec!["--format=text"],
        vec!["--format=JSON"],
        vec!["--format=xml"],
        vec!["--format=yaml", "--format=yaml"],
        vec!["--format=json", "--format=yaml"],
        vec!["--json", "--format=json"],
        vec!["--format=json", "--json"],
        vec!["--json", "--format=yaml"],
        vec!["--format=yaml", "--json"],
    ] {
        let output = f.list().args(&arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        assert!(!output.stderr.is_empty(), "{arguments:?}");
    }
    assert!(!f.root.join("calls.log").exists());
    assert!(!f.home.join(".smolsandbox").exists());
}
