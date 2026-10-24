use clap::{Args, Parser, Subcommand};
use std::{ffi::OsString, path::PathBuf};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    CreateConfig,
    #[default]
    Build,
    Start,
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
    /// Create ~/.smolsandbox/default.smlbx without replacing an existing file.
    CreateConfig {
        #[arg(long, value_parser = ["claude", "codex", "opencode", "hermes"])]
        harness: String,
    },
    /// Build the selected base and prepare persistent machines.
    Build {
        #[command(flatten)]
        selection: Selection,
        workspace: Option<PathBuf>,
    },
    /// Reconcile ports and start the selected guest harness.
    Start {
        #[command(flatten)]
        selection: Selection,
        #[arg(long = "allow-host", value_parser = parse_host)]
        allow_hosts: Vec<String>,
        /// Request an exact TCP port set; omission clears previous mappings.
        #[arg(long = "port", value_parser = parse_port)]
        ports: Vec<String>,
        workspace: Option<PathBuf>,
        /// Pass arguments unchanged to the guest harness after --.
        #[arg(last = true, allow_hyphen_values = true)]
        harness_args: Vec<OsString>,
    },
    /// Authenticate using the selected harness's supported login workflow.
    Login {
        #[command(flatten)]
        selection: Selection,
    },
    /// Inspect reconciliation or replace stale machines after readiness checks.
    #[command(group(clap::ArgGroup::new("mode").required(true).args(["check", "replace_stale"])))]
    Repair {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        replace_stale: bool,
        #[arg(long, requires = "replace_stale", conflicts_with = "check")]
        force: bool,
        #[arg(long = "allow-host", value_parser = parse_host)]
        allow_hosts: Vec<String>,
        workspace: Option<PathBuf>,
    },
    /// Remove the current generation or only machines for a workspace.
    Clean {
        #[command(flatten)]
        selection: Selection,
        #[arg(long = "workspace")]
        workspace_only: bool,
        #[arg(requires = "workspace_only")]
        workspace: Option<PathBuf>,
    },
    /// Purge eligible stopped machines and unprotected superseded artifacts.
    Purge {
        #[command(flatten)]
        selection: Selection,
        #[arg(long, conflicts_with = "force_all")]
        force: bool,
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
            ParsedCommand::CreateConfig { harness } => {
                result.command = Command::CreateConfig;
                result.harness = Some(harness);
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
            "codex-sandbox",
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
                Cli::try_parse_from(std::iter::once("codex-sandbox").chain(args.clone())).is_err(),
                "{args:?}"
            );
        }
    }
}
