use crate::{
    domain::{parse_inventory, Inventory},
    platform,
};
use anyhow::{bail, Context, Result};
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::Read,
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
}
impl Smolvm {
    pub fn discover(explicit: Option<&Path>) -> Result<Self> {
        let selected = if let Some(path) = explicit {
            path.to_owned()
        } else if let Some(path) = env::var_os("SMOLVM_BIN").filter(|value| !value.is_empty()) {
            PathBuf::from(path)
        } else {
            find_on_path(OsStr::new("smolvm")).context("SmolVM is not installed or is not on PATH; install SmolVM separately or select --smolvm <path>")?
        };
        let path = fs::canonicalize(&selected).with_context(|| {
            format!(
                "cannot resolve selected SmolVM executable {}",
                selected.display()
            )
        })?;
        if !path.is_file() {
            bail!(
                "selected SmolVM executable is not a file: {}",
                path.display()
            );
        }
        let version_output = Command::new(&path)
            .arg("--version")
            .output()
            .with_context(|| format!("cannot execute selected SmolVM {}", path.display()))?;
        if !version_output.status.success() {
            bail!(
                "selected SmolVM --version failed with status {}",
                exit_code(version_output.status)
            );
        }
        let version = std::str::from_utf8(&version_output.stdout)
            .context("SmolVM version output is not UTF-8")?
            .trim_end_matches(['\r', '\n'])
            .to_owned();
        if version.trim().is_empty() || !version.to_ascii_lowercase().contains("smolvm") {
            bail!(
                "selected executable does not identify itself as SmolVM: {}",
                path.display()
            );
        }
        Ok(Self { path, version })
    }
    pub fn capture(&self, args: &[OsString]) -> Result<Output> {
        install_signal_handler()?;
        let mut child = Command::new(&self.path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("cannot run SmolVM {}", self.path.display()))?;
        let mut stdout = child
            .stdout
            .take()
            .context("cannot capture SmolVM stdout")?;
        let mut stderr = child
            .stderr
            .take()
            .context("cannot capture SmolVM stderr")?;
        let stdout = thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        let stderr = thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        });
        let status = wait_child(&mut child)?;
        Ok(Output {
            status,
            stdout: stdout
                .join()
                .map_err(|_| anyhow::anyhow!("SmolVM stdout reader failed"))??,
            stderr: stderr
                .join()
                .map_err(|_| anyhow::anyhow!("SmolVM stderr reader failed"))??,
        })
    }
    pub fn run(&self, args: &[OsString]) -> Result<i32> {
        install_signal_handler()?;
        let mut child = Command::new(&self.path)
            .args(args)
            .spawn()
            .with_context(|| format!("cannot run SmolVM {}", self.path.display()))?;
        let status = wait_child(&mut child)?;
        Ok(exit_code(status))
    }
    pub fn inventory(&self) -> Result<Inventory> {
        let output = self.capture(&strings(&["machine", "ls", "--json"]))?;
        if !output.status.success() {
            bail!(
                "SmolVM inventory failed with status {}; no mutation is authorized",
                exit_code(output.status)
            );
        }
        parse_inventory(&output.stdout)
            .context("invalid SmolVM inventory; no mutation is authorized")
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
                bail!("incompatible SmolVM: {} failed", command.join(" "));
            }
            let mut help =
                String::from_utf8(output.stdout).context("SmolVM help output is not UTF-8")?;
            help.push_str(
                std::str::from_utf8(&output.stderr).context("SmolVM help output is not UTF-8")?,
            );
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
                        "incompatible SmolVM: {} must support {flag}",
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
