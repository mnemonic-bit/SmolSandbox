use clap::Parser;
use codex_sandbox::cli::{Cli, Options};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct OfflineInvocation {
    root: tempfile::TempDir,
}

impl OfflineInvocation {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        for name in ["state", "artifacts", "home", "workspace", "no-tools"] {
            fs::create_dir(root.path().join(name)).unwrap();
        }
        for name in ["state", "artifacts", "home"] {
            fs::write(root.path().join(name).join("sentinel"), b"preserve\n").unwrap();
        }
        Self { root }
    }

    fn run(&self, arguments: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codex-sandbox"));
        command
            .env_clear()
            .args(arguments)
            .current_dir(self.root.path().join("workspace"))
            .env("PATH", self.root.path().join("no-tools"))
            .env("HOME", self.root.path().join("home"))
            .env("USERPROFILE", self.root.path().join("home"))
            .env("SMOLVM_TEST_HOME", self.root.path().join("home"))
            .env("STATE_ROOT", self.root.path().join("state"))
            .env("ARTIFACTS_DIR", self.root.path().join("artifacts"))
            .env("BASE_SMOLFILE", self.root.path().join("missing.smlbx"))
            .env("SMOLVM_TEST_LOG", self.root.path().join("unexpected.log"));
        #[cfg(windows)]
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", system_root);
        }
        let before = snapshot(self.root.path());
        let output = command.output().unwrap();
        assert_eq!(snapshot(self.root.path()), before, "{arguments:?}");
        output
    }

    fn usage_error(&self, arguments: &[&str]) {
        let output = self.run(arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}: {output:?}");
        assert!(!output.stderr.is_empty(), "{arguments:?}: {output:?}");
    }
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, directory: &Path, entries: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() {
                entries.insert(relative, None);
                visit(root, &path, entries);
            } else {
                entries.insert(relative, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

fn parse(arguments: &[String]) -> Options {
    Cli::try_parse_from(
        std::iter::once("codex-sandbox").chain(arguments.iter().map(String::as_str)),
    )
    .unwrap()
    .into_options()
}

#[test]
fn help_is_complete_and_does_not_require_tools_configuration_or_mutation() {
    let invocation = OfflineInvocation::new();
    let cases: &[(&[&str], &[&str])] = &[
        (
            &["--help"],
            &[
                "Run a coding-agent harness in a persistent SmolVM sandbox",
                "Usage:",
                "Commands:",
                "create-config",
                "build",
                "start",
                "login",
                "repair",
                "clean",
                "purge",
            ],
        ),
        (
            &["create-config", "--help"],
            &[
                "without replacing an existing file",
                "--harness",
                "claude",
                "codex",
                "opencode",
                "hermes",
            ],
        ),
        (
            &["build", "--help"],
            &[
                "Build the selected base",
                "current directory is used when WORKSPACE is omitted",
                "[WORKSPACE]",
                "--smolfile",
                "--smolvm",
            ],
        ),
        (
            &["start", "--help"],
            &[
                "Reconcile ports",
                "[WORKSPACE]",
                "HARNESS_ARGS",
                "--allow-host",
                "Repeat --allow-host HOST or --allow-host=HOST before or after WORKSPACE",
                "Each host must be nonempty, contain no line breaks, and not start with -",
                "--port",
                "omission clears previous mappings",
                "Repeat --port PORT or --port=PORT",
                "same guest and host port",
                "Ports bind to host loopback by default",
                "Pass arguments unchanged",
                "Use -- --help to request help from the selected harness",
                "--smolfile",
                "--smolvm",
            ],
        ),
        (
            &["login", "--help"],
            &[
                "Authenticate",
                "supported login workflow",
                "Codex uses device authentication",
                "other harnesses use their own login workflow",
                "--smolfile",
                "--smolvm",
            ],
        ),
        (
            &["repair", "--help"],
            &[
                "readiness checks",
                "--check",
                "--replace-stale",
                "--force",
                "--allow-host",
                "workspace network allowlist used by start",
                "Repeat --allow-host HOST or --allow-host=HOST before or after WORKSPACE",
                "Each host must be nonempty, contain no line breaks, and not start with -",
                "Choose exactly one of --check or --replace-stale",
                "Use with --replace-stale only",
                "[WORKSPACE]",
            ],
        ),
        (
            &["clean", "--help"],
            &[
                "current generation",
                "only machines for a workspace",
                "Preserve shared harness state, including credentials and settings",
                "--workspace",
                "[WORKSPACE]",
            ],
        ),
        (
            &["purge", "--help"],
            &[
                "eligible stopped machines",
                "unprotected superseded artifacts",
                "--force",
                "--force-all",
                "conservative policy when no force option is given",
                "--force and --force-all are mutually exclusive",
                "Preserve running machines and protected artifacts under every policy",
                "preserving the current base's stopped default configurations",
                "including stopped default configurations",
            ],
        ),
    ];
    for (arguments, expected) in cases {
        let output = invocation.run(arguments);
        assert!(output.status.success(), "{arguments:?}: {output:?}");
        assert!(output.stderr.is_empty(), "{arguments:?}: {output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(!stdout.is_empty());
        let normalized = stdout.split_whitespace().collect::<Vec<_>>().join(" ");
        for text in expected
            .iter()
            .chain(["Usage:", "Options:", "--help"].iter())
        {
            assert!(
                normalized.contains(text),
                "{arguments:?}: missing {text}: {stdout}"
            );
        }
    }
}

#[test]
fn usage_errors_exit_two_on_stderr_before_any_lifecycle_work() {
    let invocation = OfflineInvocation::new();
    for arguments in [
        vec![],
        vec!["unknown"],
        vec!["build", "one", "two"],
        vec!["start", "--unknown"],
        vec!["login", "extra"],
        vec!["repair"],
        vec!["repair", "--check", "--replace-stale"],
        vec!["repair", "--check", "--force"],
        vec!["clean", "unexpected"],
        vec!["clean", "--workspace", "one", "two"],
        vec!["purge", "--force", "--force-all"],
        vec!["create-config"],
        vec!["create-config", "--harness", "unknown"],
    ] {
        invocation.usage_error(&arguments);
    }
}

#[test]
fn invalid_hosts_fail_before_tools_or_state_access() {
    let invocation = OfflineInvocation::new();
    for command in ["start", "repair"] {
        let mut prefix = vec![command];
        if command == "repair" {
            prefix.push("--check");
        }
        let mut missing = prefix.clone();
        missing.push("--allow-host");
        invocation.usage_error(&missing);
        for host in ["", "-host", "bad\nhost", "bad\rhost"] {
            let option = format!("--allow-host={host}");
            let mut arguments = prefix.clone();
            arguments.push(&option);
            invocation.usage_error(&arguments);
        }
    }
}

#[test]
fn invalid_ports_fail_before_tools_or_state_access() {
    let invocation = OfflineInvocation::new();
    invocation.usage_error(&["start", "--port"]);
    for value in [
        "",
        "+1",
        "-1",
        " 1",
        "1 ",
        "0",
        "00000",
        "65536",
        "1:1",
        "1-2",
        "1/tcp",
        "１２",
        "1\n",
        "1\r",
        "999999999999999999999999999999999999",
    ] {
        invocation.usage_error(&["start", &format!("--port={value}")]);
    }
    let mut too_many = vec!["start".to_owned()];
    too_many.extend((1..=65).map(|port| format!("--port={port}")));
    invocation.usage_error(&too_many.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn port_limit_counts_distinct_normalized_values() {
    let mut arguments = vec!["start".to_owned()];
    arguments.extend((10001..=10064).map(|port| format!("--port={port}")));
    let maximum = parse(&arguments);
    assert!(maximum.validate().is_ok());
    assert_eq!(maximum.ports.len(), 64);
    for _ in 0..100 {
        arguments.extend(["--port=010001".to_owned(), "--port=10064".to_owned()]);
    }
    let duplicates = parse(&arguments);
    assert!(duplicates.validate().is_ok());
    assert_eq!(duplicates.ports.iter().collect::<BTreeSet<_>>().len(), 64);
    arguments.push("--port=65535".to_owned());
    assert!(parse(&arguments).validate().is_err());
    assert_eq!(
        parse(&["start".into(), "--port=00001".into(), "--port=65535".into()]).ports,
        ["1", "65535"]
    );
}

#[test]
fn repeated_options_and_argument_boundaries_preserve_exact_values() {
    let arguments: Vec<_> = [
        "start",
        "--allow-host=z.example",
        "workspace with spaces",
        "--allow-host",
        "a.example",
        "--allow-host=a.example",
        "--port=05173",
        "--port",
        "3000",
        "--",
        "--help",
        "--port",
        "9000",
        "a b",
        "",
        "--allow-host=guest.example",
        "semi;dollar$quote'",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let options = parse(&arguments);
    assert_eq!(
        options.workspace,
        Some(PathBuf::from("workspace with spaces"))
    );
    assert_eq!(options.allow_hosts, ["z.example", "a.example", "a.example"]);
    assert_eq!(options.ports, ["5173", "3000"]);
    let expected: Vec<OsString> = [
        "--help",
        "--port",
        "9000",
        "a b",
        "",
        "--allow-host=guest.example",
        "semi;dollar$quote'",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    assert_eq!(options.harness_args, expected);
    let repair = parse(&[
        "repair".into(),
        "--replace-stale".into(),
        "--allow-host=z.example".into(),
        "--allow-host".into(),
        "a.example".into(),
    ]);
    assert_eq!(repair.allow_hosts, ["z.example", "a.example"]);
}
