//! Validate the Rust suite in a source copy without the retained baseline.

use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const BASELINE: &[&str] = &[
    "src-old",
    "build-sandbox.sh",
    "build-login.sh",
    "start-codex.sh",
    "codex-sandbox.sh",
    "start-login.sh",
    "repair-sandbox.sh",
    "clean-sandbox.sh",
    "purge-sandbox.sh",
    "sandbox-common.sh",
    "sandbox-inventory.py",
    "sandbox-port-state.py",
    "devtools-base.smolfile",
    "tests/fixtures/smolvm",
];

fn excluded(relative: &Path) -> bool {
    BASELINE.iter().any(|name| relative == Path::new(name))
        || relative.parent() == Some(Path::new("tests"))
            && relative
                .file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with("-test.sh"))
        || matches!(
            relative
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str()),
            Some(".git" | "target" | "artifacts" | ".smolvm" | ".codebase-memory")
        )
}

fn copy_source(source: &Path, destination: &Path, relative: &Path) -> Result<()> {
    for entry in fs::read_dir(source.join(relative))? {
        let entry = entry?;
        let path = relative.join(entry.file_name());
        if excluded(&path) {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_dir() {
            fs::create_dir(destination.join(&path))?;
            copy_source(source, destination, &path)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), destination.join(&path))?;
        } else {
            bail!("unsupported source entry {}", entry.path().display());
        }
    }
    Ok(())
}

fn cargo(root: &Path, arguments: &[&str]) -> Command {
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .current_dir(root)
        .env_remove("CARGO_TARGET_DIR")
        .args(arguments);
    command
}

fn check(root: &Path, arguments: &[&str]) -> Result<()> {
    println!("Running cargo {}", arguments.join(" "));
    ensure!(
        cargo(root, arguments).status()?.success(),
        "cargo {} failed",
        arguments.join(" ")
    );
    Ok(())
}

fn run_without_host_tools(root: &Path, adapter: bool) -> Result<()> {
    let mut arguments = vec![
        "test",
        "--locked",
        "--all-targets",
        "--no-run",
        "--message-format=json",
    ];
    if adapter {
        arguments.extend(["--features", "test-adapter"]);
    }
    let output = cargo(root, &arguments).output()?;
    ensure!(
        output.status.success(),
        "test compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut executables = BTreeSet::<PathBuf>::new();
    for line in String::from_utf8(output.stdout)?.lines() {
        let artifact: Value = serde_json::from_str(line)?;
        if artifact["reason"] == "compiler-artifact" && artifact["profile"]["test"] == true {
            if let Some(executable) = artifact["executable"].as_str() {
                executables.insert(executable.into());
            }
        }
    }
    ensure!(
        !executables.is_empty(),
        "Cargo reported no test executables"
    );
    for executable in &executables {
        println!(
            "Running {} with empty PATH (test-adapter={adapter})",
            executable.display()
        );
        ensure!(
            Command::new(executable)
                .current_dir(root)
                .env("PATH", "")
                .status()?
                .success(),
            "{} failed with empty PATH",
            executable.display()
        );
    }
    println!(
        "passed: {} compiled test executables with empty PATH (test-adapter={adapter})",
        executables.len()
    );
    Ok(())
}

fn main() -> Result<()> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let copy = tempfile::tempdir().context("cannot create isolated source copy")?;
    copy_source(source, copy.path(), Path::new(""))?;
    for name in BASELINE {
        ensure!(
            !copy.path().join(name).exists(),
            "baseline retained: {name}"
        );
    }
    ensure!(
        fs::read_dir(copy.path().join("tests"))?.all(|entry| {
            entry.is_ok_and(|entry| !entry.file_name().to_string_lossy().ends_with("-test.sh"))
        }),
        "a shell test remains in the isolated copy"
    );
    println!("Isolated source: {}", copy.path().display());
    for arguments in [
        vec!["fmt", "--all", "--", "--check"],
        vec![
            "clippy",
            "--locked",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        vec![
            "clippy",
            "--locked",
            "--features",
            "test-adapter",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        vec!["test", "--locked", "--all-targets"],
        vec![
            "test",
            "--locked",
            "--features",
            "test-adapter",
            "--all-targets",
        ],
        vec!["build", "--locked", "--release", "--bin", "smolsandbox"],
    ] {
        check(copy.path(), &arguments)?;
    }
    run_without_host_tools(copy.path(), false)?;
    run_without_host_tools(copy.path(), true)?;
    println!("passed: baseline removal, isolated Rust checks, and compiled tests with empty PATH");
    if env::var_os("BASELINE_INDEPENDENCE_KEEP_COPY").is_some() {
        println!("Retained isolated source: {}", copy.keep().display());
    }
    Ok(())
}
