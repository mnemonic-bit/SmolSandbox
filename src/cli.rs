use clap::{Args, Parser, Subcommand};
use std::{ffi::OsString, path::PathBuf};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    CreateConfig,
    #[default]
    Build,
    Start,
    List,
    Login,
    Repair,
    Clean,
    Purge,
}

#[derive(Debug, Default)]
pub struct Options {
    pub command: Command,
    pub workspace: Option<PathBuf>,
    pub smolfile: Option<String>,
    pub smolvm: Option<PathBuf>,
    pub allow_hosts: Vec<String>,
    pub ports: Vec<String>,
    pub harness_args: Vec<OsString>,
    pub check: bool,
    pub replace_stale: bool,
    pub force: bool,
    pub force_all: bool,
    pub workspace_only: bool,
    pub harness: Option<String>,
    pub file: Option<PathBuf>,
    pub all: bool,
    pub json: bool,
}

impl Options {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .ports
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            > 64
        {
            return Err("at most 64 distinct --port values may be requested".into());
        }
        Ok(())
    }
}

#[derive(Debug, Parser)]
#[command(
    bin_name = "smolsandbox",
    version,
    about = "Run a coding-agent harness in a persistent SmolVM sandbox"
)]
pub struct Cli {
    #[command(subcommand)]
    command: ParsedCommand,
}

#[derive(Debug, Args)]
struct Selection {
    /// Select an exact definition filename or explicit path.
    #[arg(long, value_name = "NAME_OR_PATH")]
    smolfile: Option<String>,
    /// Select the independently installed SmolVM executable.
    #[arg(long, value_name = "PATH")]
    smolvm: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum ParsedCommand {
    /// Create a configuration without replacing an existing file.
    CreateConfig {
        #[arg(long, value_parser = ["claude", "codex", "opencode", "hermes"])]
        harness: String,
        /// Write to PATH, relative to the current directory. Defaults to ~/.smolsandbox/default.smlbx.
        #[arg(long = "smolfile", alias = "file", value_name = "PATH", value_parser = clap::value_parser!(PathBuf))]
        file: Option<PathBuf>,
    },
    /// Build the selected base and prepare persistent machines.
    Build {
        #[command(flatten)]
        selection: Selection,
        /// Select the workspace. The current directory is used when WORKSPACE is omitted.
        workspace: Option<PathBuf>,
    },
    /// Reconcile ports and start the selected guest harness.
    Start {
        #[command(flatten)]
        selection: Selection,
        /// Add a host to the selected definition's network allowlist. Repeat --allow-host HOST or --allow-host=HOST before or after WORKSPACE. Each host must be nonempty, contain no line breaks, and not start with -.
        #[arg(long = "allow-host", value_parser = parse_host)]
        allow_hosts: Vec<String>,
        /// Request an exact TCP port set; omission clears previous mappings. Repeat --port PORT or --port=PORT to publish the same guest and host port. Ports bind to host loopback by default.
        #[arg(long = "port", value_parser = parse_port)]
        ports: Vec<String>,
        workspace: Option<PathBuf>,
        /// Pass arguments unchanged to the guest harness after --. Use -- --help to request help from the selected harness.
        #[arg(last = true, allow_hyphen_values = true)]
        harness_args: Vec<OsString>,
    },
    /// List workspace machines across profiles and generations without changing application state or taking lifecycle locks.
    #[command(
        long_about = "List managed machines associated with the canonical workspace (the current directory by default), across profiles, generations, policy revisions, and configurations. Use --all to include other workspaces and shared builder/login machines. WORKSPACE still selects the comparison context with --all. Legacy workspace names cannot establish a path association; unassociated candidates are counted and appear only with --all. Assessments compare identity with this invocation, separately from runtime state; they do not authorize deletion or guarantee a successful start. This command takes one inventory snapshot without lifecycle locks, virtualization preflight, or application writes. SmolVM may maintain its own registry while answering queries."
    )]
    List {
        #[command(flatten)]
        selection: Selection,
        /// Include all managed machines, including shared machines and unassociated legacy workspaces.
        #[arg(long)]
        all: bool,
        /// Write one schema-versioned JSON report.
        #[arg(long)]
        json: bool,
        /// Extend the reference launch allowlist. Repeat the same additions used by start.
        #[arg(long = "allow-host", value_parser = parse_host)]
        allow_hosts: Vec<String>,
        /// Select the reference workspace. Defaults to the invocation directory, including with --all.
        workspace: Option<PathBuf>,
    },
    /// Authenticate using the selected harness's supported login workflow. Codex uses device authentication; other harnesses use their own login workflow.
    Login {
        #[command(flatten)]
        selection: Selection,
    },
    /// Inspect reconciliation or replace stale machines after readiness checks.
    #[command(group(clap::ArgGroup::new("mode").required(true).args(["check", "replace_stale"])))]
    Repair {
        #[command(flatten)]
        selection: Selection,
        /// Inspect the selected workspace without replacing stale machines. Choose exactly one of --check or --replace-stale.
        #[arg(long)]
        check: bool,
        /// Replace stale workspace machines after the replacement passes readiness checks.
        #[arg(long)]
        replace_stale: bool,
        /// Permit removal of running stale machines. Use with --replace-stale only.
        #[arg(long, requires = "replace_stale", conflicts_with = "check")]
        force: bool,
        /// Add a host to the workspace network allowlist used by start. Repeat --allow-host HOST or --allow-host=HOST before or after WORKSPACE. Each host must be nonempty, contain no line breaks, and not start with -.
        #[arg(long = "allow-host", value_parser = parse_host)]
        allow_hosts: Vec<String>,
        workspace: Option<PathBuf>,
    },
    /// Remove the current generation or only machines for a workspace. Preserve shared harness state, including credentials and settings.
    Clean {
        #[command(flatten)]
        selection: Selection,
        #[arg(long = "workspace")]
        workspace_only: bool,
        #[arg(requires = "workspace_only")]
        workspace: Option<PathBuf>,
    },
    /// Purge eligible stopped machines and unprotected superseded artifacts. Use the conservative policy when no force option is given. --force and --force-all are mutually exclusive. Preserve running machines and protected artifacts under every policy.
    Purge {
        #[command(flatten)]
        selection: Selection,
        /// Remove eligible stopped custom and legacy machines while preserving the current base's stopped default configurations.
        #[arg(long, conflicts_with = "force_all")]
        force: bool,
        /// Remove every eligible stopped managed machine, including stopped default configurations.
        #[arg(long)]
        force_all: bool,
    },
}

fn parse_host(raw: &str) -> Result<String, String> {
    if raw.is_empty() || raw.starts_with('-') || raw.contains(['\r', '\n']) {
        return Err("a host must be nonempty, contain no line breaks, and not start with -".into());
    }
    Ok(raw.to_owned())
}

fn parse_port(raw: &str) -> Result<String, String> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err("a port must be an ASCII decimal integer from 1 through 65535".into());
    }
    let trimmed = raw.trim_start_matches('0');
    let value = trimmed.parse::<u16>().ok().filter(|value| *value != 0);
    value
        .map(|v| v.to_string())
        .ok_or_else(|| "a port must be from 1 through 65535".into())
}

impl Cli {
    pub fn into_options(self) -> Options {
        let mut result = Options::default();
        let selection = match self.command {
            ParsedCommand::CreateConfig { harness, file } => {
                result.command = Command::CreateConfig;
                result.harness = Some(harness);
                result.file = file;
                None
            }
            ParsedCommand::Build {
                selection,
                workspace,
            } => {
                result.command = Command::Build;
                result.workspace = workspace;
                Some(selection)
            }
            ParsedCommand::Start {
                selection,
                workspace,
                allow_hosts,
                ports,
                harness_args,
            } => {
                result.command = Command::Start;
                result.workspace = workspace;
                result.allow_hosts = allow_hosts;
                result.ports = ports;
                result.harness_args = harness_args;
                Some(selection)
            }
            ParsedCommand::Login { selection } => {
                result.command = Command::Login;
                Some(selection)
            }
            ParsedCommand::List {
                selection,
                workspace,
                allow_hosts,
                all,
                json,
            } => {
                result.command = Command::List;
                result.workspace = workspace;
                result.allow_hosts = allow_hosts;
                result.all = all;
                result.json = json;
                Some(selection)
            }
            ParsedCommand::Repair {
                selection,
                workspace,
                check,
                replace_stale,
                force,
                allow_hosts,
            } => {
                result.command = Command::Repair;
                result.workspace = workspace;
                result.check = check;
                result.replace_stale = replace_stale;
                result.force = force;
                result.allow_hosts = allow_hosts;
                Some(selection)
            }
            ParsedCommand::Clean {
                selection,
                workspace_only,
                workspace,
            } => {
                result.command = Command::Clean;
                result.workspace = workspace;
                result.workspace_only = workspace_only;
                Some(selection)
            }
            ParsedCommand::Purge {
                selection,
                force,
                force_all,
            } => {
                result.command = Command::Purge;
                result.force = force;
                result.force_all = force_all;
                Some(selection)
            }
        };
        if let Some(selection) = selection {
            result.smolfile = selection.smolfile;
            result.smolvm = selection.smolvm;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delimiter_preserves_harness_arguments() {
        let options = Cli::try_parse_from([
            "smolsandbox",
            "start",
            "--port",
            "00080",
            ".",
            "--",
            "--help",
            "a b",
            "",
        ])
        .unwrap()
        .into_options();
        assert_eq!(options.ports, ["80"]);
        assert_eq!(options.harness_args, ["--help", "a b", ""]);
    }

    #[test]
    fn rejects_invalid_usage() {
        for args in [
            vec!["login", "extra"],
            vec!["repair"],
            vec!["repair", "--check", "--force"],
            vec!["repair", "--check", "--replace-stale"],
            vec!["purge", "--force", "--force-all"],
            vec!["start", "--port", "65536"],
            vec!["start", "--port", "0"],
            vec!["start", "--port", "１２"],
            vec!["start", "--allow-host", "bad\nhost"],
            vec!["create-config", "--harness", "other"],
            vec!["create-config"],
            vec!["clean", "somewhere"],
        ] {
            assert!(
                Cli::try_parse_from(std::iter::once("smolsandbox").chain(args.clone())).is_err(),
                "{args:?}"
            );
        }
    }
}
