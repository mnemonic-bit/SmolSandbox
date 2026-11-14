use super::*;
use std::{
    io::{Read, Write},
    process::{Child, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

fn command(f: &Fixture, verbose: bool) -> Command {
    let mut command = f.command();
    command.env("SMOLVM_TEST_OUTPUT", "1");
    if verbose {
        command.arg("--verbose");
    }
    command
}

fn wait(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("application did not finish within 20 seconds");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn reader<R: Read + Send + 'static>(
    mut input: R,
) -> (mpsc::Receiver<Vec<u8>>, thread::JoinHandle<Vec<u8>>) {
    let (send, receive) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut all = Vec::new();
        let mut buffer = [0; 8192];
        loop {
            let count = input.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            all.extend_from_slice(&buffer[..count]);
            let _ = send.send(buffer[..count].to_vec());
        }
        all
    });
    (receive, handle)
}

fn receive_until(receive: &mpsc::Receiver<Vec<u8>>, needles: &[&[u8]]) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut bytes = Vec::new();
    while !needles
        .iter()
        .all(|needle| bytes.windows(needle.len()).any(|part| part == *needle))
    {
        bytes.extend(
            receive
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|error| {
                    panic!(
                        "missing live markers {needles:?}: {error}; received {}",
                        String::from_utf8_lossy(&bytes)
                    )
                }),
        );
    }
}

#[test]
fn management_output_policy_covers_build_seed_ports_and_cleanup() {
    for verbose in [false, true] {
        let f = Fixture::new();
        f.configure("codex");
        let build = command(&f, verbose).arg("build").output().unwrap();
        assert_ok(&build);
        assert!(String::from_utf8_lossy(&build.stdout).contains("Next: smolsbx login"));
        let start = command(&f, verbose)
            .args(["start", "--port", "3000"])
            .output()
            .unwrap();
        assert_ok(&start);
        let clean = command(&f, verbose)
            .args(["clean", "--workspace"])
            .output()
            .unwrap();
        assert_ok(&clean);
        let mut errors = Vec::new();
        for output in [&build, &start, &clean] {
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(!stdout.contains("runtime-"), "{stdout}");
            assert!(!stdout.contains("query-"), "{stdout}");
            errors.extend_from_slice(&output.stderr);
        }
        let errors = String::from_utf8(errors).unwrap();
        for stage in [
            "create", "pack", "start", "stop", "update", "delete", "seed",
        ] {
            for stream in ["stdout", "stderr"] {
                let marker = format!("runtime-{stage}-{stream}");
                assert_eq!(errors.contains(&marker), verbose, "{marker}: {errors}");
            }
        }
        for query in ["version", "help", "inventory"] {
            assert_eq!(errors.contains(&format!("query-{query}-stderr")), verbose);
        }
        assert!(!errors.contains("SmolVM test adapter:"));
        assert!(!errors.contains("smolvm test 1.19.2"));
        assert!(String::from_utf8_lossy(&start.stdout).contains("guest-stdout"));
        assert!(String::from_utf8_lossy(&start.stderr).contains("guest-stderr"));
    }
}

#[test]
fn verbose_management_streams_both_outputs_before_exit_and_drains_before_guest() {
    let f = Fixture::new();
    f.configure("codex");
    assert_ok(&f.run(&["build"]));
    let release = f.root.path().join("release");
    let mut child = command(&f, true)
        .env("SMOLVM_TEST_OUTPUT_GATE_STAGE", "start")
        .env("SMOLVM_TEST_OUTPUT_GATE_READY", f.root.path().join("ready"))
        .env("SMOLVM_TEST_OUTPUT_GATE_RELEASE", &release)
        .arg("start")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let (_, stdout) = reader(child.stdout.take().unwrap());
    let (receive, stderr) = reader(child.stderr.take().unwrap());
    receive_until(
        &receive,
        &[b"runtime-start-stdout\n", b"runtime-start-stderr\n"],
    );
    assert!(child.try_wait().unwrap().is_none());
    fs::write(release, "release").unwrap();
    assert!(wait(&mut child).success());
    let stdout = String::from_utf8(stdout.join().unwrap()).unwrap();
    let stderr = String::from_utf8(stderr.join().unwrap()).unwrap();
    assert!(!stdout.contains("runtime-"));
    let guest = stderr.find("guest-stderr").unwrap();
    for marker in [
        "runtime-start-stdout",
        "runtime-start-stderr",
        "runtime-seed-stdout",
        "runtime-seed-stderr",
    ] {
        assert!(stderr.find(marker).unwrap() < guest, "{stderr}");
        assert_eq!(stderr.matches(marker).count(), 1, "{stderr}");
    }
    for marker in ["runtime-stop-stdout", "runtime-stop-stderr"] {
        assert!(stderr.find(marker).unwrap() > guest, "{stderr}");
    }
}

#[test]
fn simultaneous_large_streams_finish_without_corrupting_interactive_output() {
    for verbose in [false, true] {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["build"]));
        let mut child = command(&f, verbose)
            .env("SMOLVM_TEST_LARGE_OUTPUT_STAGE", "start")
            .arg("start")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (_, stdout) = reader(child.stdout.take().unwrap());
        let (_, stderr) = reader(child.stderr.take().unwrap());
        assert!(wait(&mut child).success());
        let stdout = String::from_utf8(stdout.join().unwrap()).unwrap();
        let stderr = String::from_utf8(stderr.join().unwrap()).unwrap();
        assert!(!stdout.contains("large-"));
        assert!(stdout.contains("guest-stdout"));
        for byte in *b"OE" {
            assert_eq!(
                stderr.bytes().filter(|value| *value == byte).count(),
                if verbose { 1024 * 1024 } else { 0 }
            );
        }
        for marker in ["large-stdout-finished", "large-stderr-finished"] {
            assert_eq!(stderr.contains(marker), verbose);
            if verbose {
                assert!(stderr.find(marker).unwrap() < stderr.find("guest-stderr").unwrap());
            }
        }
    }
}

#[test]
fn interactive_prompt_stdin_ansi_and_guest_arguments_survive_both_modes() {
    for verbose in [false, true] {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["build"]));
        fs::write(f.root.path().join("argv.jsonl"), "").unwrap();
        let mut child = command(&f, verbose)
            .env("SMOLVM_TEST_STDIN_ROUNDTRIP", "1")
            .env("OPENAI_API_KEY", "synthetic-secret-never-log-0014")
            .args(["start", "--", "--verbose", "--help", "", "two words"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (receive, stdout) = reader(child.stdout.take().unwrap());
        let (_, stderr) = reader(child.stderr.take().unwrap());
        receive_until(&receive, &[b"guest-prompt>\x1b[32mready\x1b[0m\n"]);
        assert!(child.try_wait().unwrap().is_none());
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"typed after prompt\n")
            .unwrap();
        assert!(wait(&mut child).success());
        let stdout = String::from_utf8(stdout.join().unwrap()).unwrap();
        let stderr = String::from_utf8(stderr.join().unwrap()).unwrap();
        assert!(stdout.contains("guest-terminals: stdin=false stdout=false stderr=false"));
        assert!(stdout.contains("guest-received:typed after prompt\n"));
        assert!(stderr.contains("guest-stderr"));
        assert_eq!(stderr.contains("runtime-start-stdout"), verbose);
        assert!(!stdout.contains("synthetic-secret-never-log-0014"));
        assert!(!stderr.contains("synthetic-secret-never-log-0014"));
        let calls: Vec<Vec<String>> = fs::read_to_string(f.root.path().join("argv.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let interactive = calls
            .iter()
            .find(|args| args.iter().any(|arg| arg == "-it"))
            .unwrap();
        let delimiter = interactive.iter().position(|arg| arg == "--").unwrap();
        assert_eq!(
            &interactive[interactive.len() - 4..],
            ["--verbose", "--help", "", "two words"]
        );
        assert!(!interactive[..delimiter]
            .iter()
            .any(|arg| arg == "--verbose"));
        assert!(interactive[..delimiter]
            .windows(2)
            .any(|pair| pair == ["--secret-env", "OPENAI_API_KEY=OPENAI_API_KEY"]));
    }
}

#[test]
fn noninteractive_failures_keep_context_and_warning_policy_without_raw_default_logs() {
    for verbose in [false, true] {
        for (flag, operation, status, prepare, args, warning) in [
            ("VERSION", "version", "69", false, vec!["build"], false),
            ("HELP", "help", "68", false, vec!["build"], false),
            ("INVENTORY", "inventory", "70", false, vec!["build"], false),
            ("CREATE", "create", "71", false, vec!["build"], false),
            ("PACK", "pack", "74", false, vec!["build"], false),
            ("START", "start", "75", true, vec!["start"], false),
            ("STOP", "stop", "72", true, vec!["start"], false),
            (
                "DELETE",
                "delete",
                "73",
                true,
                vec!["clean", "--workspace"],
                false,
            ),
            ("SEED", "exec", "1", true, vec!["start"], true),
        ] {
            let f = Fixture::new();
            f.configure("codex");
            if prepare {
                assert_ok(&f.run(&["build"]));
            }
            let output = command(&f, verbose)
                .env(format!("SMOLVM_TEST_{flag}_FAIL"), "1")
                .args(&args)
                .output()
                .unwrap();
            assert_eq!(
                output.status.success(),
                warning,
                "{flag}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let errors = String::from_utf8_lossy(&output.stderr);
            let lower = errors.to_ascii_lowercase();
            assert!(lower.contains(operation), "{flag}: {errors}");
            assert!(lower.contains(status), "{flag}: {errors}");
            assert!(errors.contains("--verbose"), "{flag}: {errors}");
            assert_eq!(errors.contains("simulated"), verbose, "{flag}: {errors}");
            if !verbose {
                assert!(!errors.contains("runtime-"), "{flag}: {errors}");
                assert!(!errors.contains("query-"), "{flag}: {errors}");
            }
            if warning {
                assert!(
                    errors.contains("WARNING: could not seed persistent harness state; continuing")
                );
            }
            if matches!(flag, "CREATE" | "START" | "STOP" | "DELETE" | "SEED") {
                assert!(errors.contains("codex-"), "{flag}: {errors}");
            }
        }
    }
}

#[test]
fn login_sessions_and_clean_stop_warnings_preserve_existing_policy() {
    for verbose in [false, true] {
        let f = Fixture::new();
        f.configure("codex");
        assert_ok(&f.run(&["build"]));
        let login = command(&f, verbose).arg("login").output().unwrap();
        assert_ok(&login);
        assert!(String::from_utf8_lossy(&login.stdout).contains("guest-stdout"));
        assert!(String::from_utf8_lossy(&login.stderr).contains("guest-stderr"));
        assert!(!String::from_utf8_lossy(&login.stdout).contains("runtime-"));
        assert_eq!(
            String::from_utf8_lossy(&login.stderr).contains("runtime-stop-stdout"),
            verbose
        );
        let clean = command(&f, verbose)
            .env("SMOLVM_TEST_STOP_FAIL", "1")
            .args(["clean", "--workspace"])
            .output()
            .unwrap();
        assert_ok(&clean);
        let errors = String::from_utf8_lossy(&clean.stderr);
        assert!(errors.contains("WARNING: could not stop machine"));
        assert!(errors.contains("before deletion"));
        assert!(errors.contains("72"));
        assert!(errors.contains("--verbose"));
        assert_eq!(errors.contains("simulated machine stop failure"), verbose);
        assert!(!f
            .inventory()
            .iter()
            .any(|machine| machine["labels"]["codex-sandbox.kind"] == "workspace"));
    }
}

#[cfg(unix)]
#[test]
fn cancelling_initial_query_drains_readers_without_starting_lifecycle_work() {
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    for verbose in [false, true] {
        let f = Fixture::new();
        f.configure("codex");
        let ready = f.root.path().join("query-ready");
        let mut child = command(&f, verbose)
            .env("SMOLVM_TEST_VERSION_GATE_READY", &ready)
            .env(
                "SMOLVM_TEST_VERSION_GATE_RELEASE",
                f.root.path().join("never-release"),
            )
            .args(["list", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (_, stdout) = reader(child.stdout.take().unwrap());
        let (receive, stderr) = reader(child.stderr.take().unwrap());
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("initial version query never reached its gate");
            }
            thread::sleep(Duration::from_millis(10));
        }
        if verbose {
            receive_until(&receive, &[b"query-version-stderr\n"]);
        }
        assert_eq!(unsafe { kill(child.id() as i32, 15) }, 0);
        assert!(!wait(&mut child).success());
        assert!(stdout.join().unwrap().is_empty());
        let errors = String::from_utf8(stderr.join().unwrap()).unwrap();
        assert!(errors.contains("130"));
        assert!(errors.contains("--verbose"));
        assert_eq!(errors.contains("query-version-stderr"), verbose);
        assert_eq!(f.log(), "--version");
        assert!(!f.home.join(".smolsandbox/state").exists());
        assert!(!f.home.join(".smolsandbox/locks").exists());
    }
}
