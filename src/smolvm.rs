use crate::{
    domain::{parse_inventory, Inventory},
    platform,
};
use anyhow::{bail, Context, Result};
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
    thread,
    time::Duration,
};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static CANCELED: AtomicBool = AtomicBool::new(false);
static SIGNAL_HANDLER: OnceLock<std::result::Result<(), String>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Smolvm {
    pub path: PathBuf,
    pub version: String,
    verbose: bool,
}
impl Smolvm {
    pub fn discover(explicit: Option<&Path>, verbose: bool) -> Result<Self> {
        let selected = if let Some(path) = explicit {
            path.to_owned()
        } else if let Some(path) = env::var_os("SMOLVM_BIN").filter(|value| !value.is_empty()) {
            PathBuf::from(path)
        } else {
            find_on_path(OsStr::new("smolvm")).with_context(|| format!("SmolVM discovery failed: SmolVM is not installed or is not on PATH; install SmolVM separately or select --smolvm <path>. {RUNTIME_GUIDANCE}"))?
        };
        let path = fs::canonicalize(&selected).with_context(|| {
            format!(
                "SmolVM discovery failed: cannot resolve selected SmolVM executable {}. {RUNTIME_GUIDANCE}",
                selected.display()
            )
        })?;
        if !path.is_file() {
            bail!(
                "SmolVM discovery failed: selected SmolVM executable is not a file: {}. {RUNTIME_GUIDANCE}",
                path.display()
            );
        }
        let mut runtime = Self {
            path,
            version: String::new(),
            verbose,
        };
        let mut command = Command::new(&runtime.path);
        command.arg("--version").stdin(Stdio::null());
        let version_output = runtime.capture_command(command, "SmolVM version query")?;
        if !version_output.status.success() {
            bail!(
                "selected SmolVM --version failed with exit status {}. {RUNTIME_GUIDANCE}",
                exit_code(version_output.status)
            );
        }
        let version = std::str::from_utf8(&version_output.stdout)
            .with_context(|| format!("SmolVM version output is not UTF-8. {RUNTIME_GUIDANCE}"))?
            .trim_end_matches(['\r', '\n'])
            .to_owned();
        if version.trim().is_empty() || !version.to_ascii_lowercase().contains("smolvm") {
            bail!(
                "selected executable does not identify itself as SmolVM: {}. {RUNTIME_GUIDANCE}",
                runtime.path.display()
            );
        }
        runtime.version = version;
        Ok(runtime)
    }
    pub fn capture(&self, args: &[OsString]) -> Result<Output> {
        let mut command = Command::new(&self.path);
        command.args(args);
        self.capture_command(command, &operation_context(args))
    }

    fn capture_command(&self, mut command: Command, operation: &str) -> Result<Output> {
        install_signal_handler()?;
        let context = || format!("{operation} failed. {RUNTIME_GUIDANCE}");
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(context)?;
        let mut stdout = child.stdout.take().expect("piped SmolVM stdout");
        let stderr = child.stderr.take().expect("piped SmolVM stderr");
        let verbose = self.verbose;
        let stdout = thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        let stderr = thread::spawn(move || capture_stderr(stderr, verbose));
        let status = wait_child(&mut child);
        if status.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        // Join both readers before returning, including on cancellation or a read failure.
        let stdout = stdout.join();
        let stderr = stderr.join();
        let status = status.with_context(context)?;
        let stdout = stdout
            .map_err(|_| anyhow::anyhow!("SmolVM stdout reader failed"))
            .with_context(context)?
            .with_context(context)?;
        let stderr = stderr
            .map_err(|_| anyhow::anyhow!("SmolVM stderr reader failed"))
            .with_context(context)?
            .with_context(context)?;
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }

    pub fn run(&self, args: &[OsString]) -> Result<i32> {
        install_signal_handler()?;
        let operation = operation_context(args);
        let context = || format!("{operation} failed. {RUNTIME_GUIDANCE}");
        let mut command = Command::new(&self.path);
        command.args(args);
        if self.verbose {
            command
                .stdout(duplicate_stderr().with_context(context)?)
                .stderr(Stdio::inherit());
        } else {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let mut child = command.spawn().with_context(context)?;
        let status = wait_child(&mut child);
        if status.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let status = exit_code(status.with_context(context)?);
        if status != 0 {
            bail!("{operation} failed with exit status {status}. {RUNTIME_GUIDANCE}");
        }
        Ok(status)
    }
    pub fn inventory(&self) -> Result<Inventory> {
        let output = self.capture(&strings(&["machine", "ls", "--json"]))?;
        if !output.status.success() {
            bail!(
                "SmolVM inventory failed with exit status {}; no mutation is authorized. {RUNTIME_GUIDANCE}",
                exit_code(output.status)
            );
        }
        parse_inventory(&output.stdout).with_context(|| {
            format!("invalid SmolVM inventory; no mutation is authorized. {RUNTIME_GUIDANCE}")
        })
    }
    pub fn preflight(&self) -> Result<()> {
        for (command, required) in [
            (
                vec!["machine", "create", "--help"],
                vec![
                    "--name",
                    "--from",
                    "--smolfile",
                    "--label",
                    "--volume",
                    "--allow-host",
                    "--mem",
                    "--cpus",
                ],
            ),
            (vec!["machine", "ls", "--help"], vec!["--json"]),
            (
                vec!["machine", "update", "--help"],
                vec!["--port", "--remove-port"],
            ),
            (
                vec!["machine", "exec", "--help"],
                vec!["--name", "--secret-env", "-it"],
            ),
            (vec!["machine", "start", "--help"], vec!["--name"]),
            (vec!["machine", "stop", "--help"], vec!["--name"]),
            (
                vec!["machine", "delete", "--help"],
                vec!["--name", "--force"],
            ),
            (vec!["pack", "create", "--help"], vec!["--from-vm", "-o"]),
        ] {
            let output = self.capture(&strings(&command))?;
            if !output.status.success() {
                bail!(
                    "incompatible SmolVM: {} failed with exit status {}. {RUNTIME_GUIDANCE}",
                    command.join(" "),
                    exit_code(output.status)
                );
            }
            let mut help = String::from_utf8(output.stdout).with_context(|| {
                format!(
                    "SmolVM {} capability help output is not UTF-8. {RUNTIME_GUIDANCE}",
                    command[..command.len() - 1].join(" ")
                )
            })?;
            help.push_str(std::str::from_utf8(&output.stderr).with_context(|| {
                format!(
                    "SmolVM {} capability help output is not UTF-8. {RUNTIME_GUIDANCE}",
                    command[..command.len() - 1].join(" ")
                )
            })?);
            let flags: Vec<&str> = help
                .split_whitespace()
                .map(|token| {
                    token
                        .trim_matches([',', '[', ']'])
                        .split('=')
                        .next()
                        .unwrap_or("")
                })
                .collect();
            for flag in required {
                let present = flags.contains(&flag)
                    || (flag == "-it" && flags.contains(&"-i") && flags.contains(&"-t"));
                if !present {
                    bail!(
                        "incompatible SmolVM: {} must support {flag}. {RUNTIME_GUIDANCE}",
                        command[..command.len() - 1].join(" ")
                    );
                }
            }
        }
        self.inventory()?;
        #[cfg(feature = "test-adapter")]
        if self.version.starts_with("smolvm test ") {
            return Ok(());
        }
        platform::virtualization_preflight()
    }
    pub fn interactive(&self, args: &[OsString], secret_names: &[&str]) -> Result<i32> {
        install_signal_handler()?;
        if CANCELED.load(Ordering::SeqCst) {
            return Ok(130);
        }
        let mut command = Command::new(&self.path);
        let delimiter = args
            .iter()
            .position(|arg| arg == "--")
            .unwrap_or(args.len());
        command.args(&args[..delimiter]);
        for name in secret_names {
            if env::var_os(name).is_some_and(|value| !value.is_empty()) {
                command.arg("--secret-env").arg(format!("{name}={name}"));
            }
        }
        command
            .args(&args[delimiter..])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let mut child = command
            .spawn()
            .with_context(|| format!("cannot start interactive SmolVM {}", self.path.display()))?;
        Ok(exit_code(wait_child(&mut child)?))
    }
}
const RUNTIME_GUIDANCE: &str = "Use --verbose to display runtime output.";

fn operation_context(args: &[OsString]) -> String {
    let mut operation = if args.first().is_some_and(|arg| arg == "--version") {
        "SmolVM version query".to_owned()
    } else {
        let command = args
            .first()
            .map(|arg| arg.to_string_lossy())
            .unwrap_or_default();
        let action = args
            .get(1)
            .map(|arg| arg.to_string_lossy())
            .unwrap_or_default();
        format!("SmolVM {command} {action}")
    };
    if args.iter().any(|arg| arg == "--help") {
        operation.push_str(" capability inspection");
    }
    if let Some(name) = args
        .windows(2)
        .find_map(|pair| (pair[0] == "--name" || pair[0] == "--from-vm").then_some(&pair[1]))
    {
        operation.push_str(" for ");
        operation.push_str(&name.to_string_lossy());
    }
    operation
}

fn capture_stderr(mut source: impl Read, verbose: bool) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    let mut write_error = None;
    loop {
        let count = match source.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        if verbose && write_error.is_none() {
            let mut destination = io::stderr().lock();
            if let Err(error) = destination
                .write_all(&buffer[..count])
                .and_then(|()| destination.flush())
            {
                write_error = Some(error);
            }
        }
    }
    match write_error {
        Some(error) => Err(error),
        None => Ok(bytes),
    }
}

fn duplicate_stderr() -> io::Result<Stdio> {
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        io::stderr().as_fd().try_clone_to_owned().map(Stdio::from)
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsHandle;
        io::stderr()
            .as_handle()
            .try_clone_to_owned()
            .map(Stdio::from)
    }
}

fn install_signal_handler() -> Result<()> {
    let handler = SIGNAL_HANDLER.get_or_init(|| {
        ctrlc::set_handler(|| {
            CANCELED.store(true, Ordering::SeqCst);
            INTERRUPTED.store(true, Ordering::SeqCst)
        })
        .map_err(|error| error.to_string())
    });
    if let Err(error) = handler {
        bail!("cannot install termination handler: {error}");
    }
    Ok(())
}
fn wait_child(child: &mut Child) -> Result<ExitStatus> {
    loop {
        if INTERRUPTED.swap(false, Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                return Ok(ExitStatus::from_raw(130 << 8));
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::ExitStatusExt;
                return Ok(ExitStatus::from_raw(130));
            }
        }
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        thread::sleep(Duration::from_millis(25));
    }
}
fn strings(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}
pub fn exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        128 + status.signal().unwrap_or(1)
    }
    #[cfg(not(unix))]
    {
        1
    }
}
fn find_on_path(name: &OsStr) -> Option<PathBuf> {
    let search = env::var_os("PATH")?;
    for directory in env::split_paths(&search) {
        #[cfg(windows)]
        let candidate = directory.join(name).with_extension("exe");
        #[cfg(not(windows))]
        let candidate = directory.join(name);
        if candidate.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if fs::metadata(&candidate).ok()?.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            return Some(candidate);
        }
    }
    None
}
