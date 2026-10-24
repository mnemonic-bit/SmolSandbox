use crate::{cli::Options, platform};
use anyhow::{bail, Context, Result};
use std::{
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Codex,
    Claude,
    OpenCode,
    Hermes,
}
impl Profile {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude" => Ok(Self::Claude),
            "opencode" => Ok(Self::OpenCode),
            "hermes" => Ok(Self::Hermes),
            _ => bail!("unsupported harness {value:?}; select claude, codex, opencode, or hermes"),
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::OpenCode => "opencode",
            Self::Hermes => "hermes",
        }
    }
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude",
            Self::OpenCode => "OpenCode",
            Self::Hermes => "Hermes Agent",
        }
    }
    pub fn guest_home(self) -> &'static str {
        match self {
            Self::Codex => "/root/.codex",
            Self::Claude => "/root/.claude",
            Self::OpenCode => "/root/.local/share/opencode",
            Self::Hermes => "/root/.hermes",
        }
    }
    pub fn seed_path(self) -> &'static str {
        match self {
            Self::Codex => "/opt/codex-home",
            Self::Claude => "/opt/claude-home",
            Self::OpenCode => "/opt/opencode-home",
            Self::Hermes => "/opt/hermes-home",
        }
    }
    pub fn launch(self) -> Vec<OsString> {
        let args: &[&str] = match self {
            Self::Codex => &["codex", "--no-daemon"],
            Self::Claude => &["env", "CLAUDE_CONFIG_DIR=/root/.claude", "claude"],
            Self::OpenCode => &["opencode"],
            Self::Hermes => &["hermes"],
        };
        args.iter().map(OsString::from).collect()
    }
    pub fn login(self) -> Option<Vec<OsString>> {
        let args: &[&str] = match self {
            Self::Codex => &["codex", "login", "--device-auth"],
            Self::Claude => &[
                "env",
                "CLAUDE_CONFIG_DIR=/root/.claude",
                "claude",
                "auth",
                "login",
            ],
            Self::OpenCode => &["opencode", "auth", "login"],
            Self::Hermes => &["hermes", "setup"],
        };
        Some(args.iter().map(OsString::from).collect())
    }
    pub fn authentication_hint(self) -> &'static str {
        match self {
            Self::Claude => {
                "Run codex-sandbox login for Claude authentication, or use /login inside Claude."
            }
            Self::Codex => "Run codex-sandbox login for Codex device authentication.",
            Self::OpenCode => "Run codex-sandbox login to select and authenticate a provider.",
            Self::Hermes => "Run codex-sandbox login to configure Hermes Agent with hermes setup.",
        }
    }
    pub fn login_hosts(self) -> Vec<&'static str> {
        match self {
            Self::Codex => vec!["auth.openai.com", "chatgpt.com", "api.openai.com"],
            Self::Claude => vec![
                "api.anthropic.com",
                "claude.ai",
                "console.anthropic.com",
                "platform.claude.com",
            ],
            Self::OpenCode => vec![
                "opencode.ai",
                "models.dev",
                "api.anthropic.com",
                "api.openai.com",
                "auth.openai.com",
                "chatgpt.com",
                "github.com",
                "api.github.com",
            ],
            Self::Hermes => vec![
                "hermes-agent.nousresearch.com",
                "portal.nousresearch.com",
                "openrouter.ai",
                "api.openai.com",
                "api.anthropic.com",
            ],
        }
    }
    pub fn template(self) -> &'static str {
        match self {
            Self::Codex => include_str!("../templates/codex.smlbx"),
            Self::Claude => include_str!("../templates/claude.smlbx"),
            Self::OpenCode => include_str!("../templates/opencode.smlbx"),
            Self::Hermes => include_str!("../templates/hermes.smlbx"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub profile: Profile,
    pub allow_hosts: Vec<String>,
}
impl Definition {
    pub fn read(path: PathBuf) -> Result<Self> {
        let bytes = fs::read(&path)
            .with_context(|| format!("cannot read selected Smolfile {}", path.display()))?;
        Self::parse(path, bytes)
    }
    pub fn parse(path: PathBuf, bytes: Vec<u8>) -> Result<Self> {
        let text = std::str::from_utf8(&bytes)
            .with_context(|| format!("selected Smolfile {} is not UTF-8", path.display()))?;
        let document: toml::Value = toml::from_str(text).map_err(|_| {
            anyhow::anyhow!(
                "malformed selected Smolfile {}; correct its TOML syntax",
                path.display()
            )
        })?;
        validate_smolfile(&document)
            .with_context(|| format!("invalid selected Smolfile {}", path.display()))?;
        let profiles: Vec<&str> = text
            .lines()
            .filter_map(|line| line.strip_prefix("# codex-sandbox-harness:"))
            .map(str::trim)
            .collect();
        if profiles.len() != 1 {
            bail!("selected Smolfile {} requires exactly one '# codex-sandbox-harness: <profile>' metadata line; generate an example with codex-sandbox create-config --harness codex",path.display());
        }
        let profile = Profile::parse(profiles[0])
            .with_context(|| format!("invalid selected Smolfile {}", path.display()))?;
        let allow_hosts = document
            .get("network")
            .and_then(|v| v.get("allow_hosts"))
            .and_then(toml::Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let allow_hosts = allow_hosts
            .iter()
            .map(|v| {
                let value = v.as_str().context("allow_hosts entries must be strings")?;
                if value.is_empty() || value.starts_with('-') || value.contains(['\r', '\n']) {
                    bail!("invalid allow_hosts entry in {}", path.display());
                }
                Ok(value.to_owned())
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            path,
            bytes,
            profile,
            allow_hosts,
        })
    }
}

fn validate_smolfile(value: &toml::Value) -> Result<()> {
    validate_table(value, "")
}
fn validate_table(value: &toml::Value, section: &str) -> Result<()> {
    let table = value
        .as_table()
        .with_context(|| format!("{section} must be a table"))?;
    if section.is_empty() && table.contains_key("fork") && table.contains_key("branch") {
        bail!("use either [branch] or [fork], not both");
    }
    for (key, value) in table {
        let location = if section.is_empty() {
            key.clone()
        } else {
            format!("{section}.{key}")
        };
        let kind=match (section,key.as_str()) {
            ("","image"|"workdir"|"user"|"net_backend"|"block_io")|("dev","workdir"|"user")|("artifact"|"pack","oci_platform")|("health","interval"|"timeout"|"startup_grace")|("restart","policy"|"max_backoff")|("service","protocol")|("secret","from_env"|"from_file")=>"string",
            ("","entrypoint"|"cmd"|"env"|"ports"|"volumes"|"init")|("dev","volumes"|"env"|"init"|"ports")|("artifact"|"pack","entrypoint"|"cmd")|("network","allow_hosts"|"allow_cidrs")|("health","exec")=>"strings",
            ("","net"|"gpu"|"rosetta"|"cuda"|"auto_graph"|"docker_socket")|("auth","ssh_agent")|("branch"|"fork","enabled")=>"bool",
            (""|"artifact"|"pack","cpus")=>"u8",
            ("","memory"|"gpu_vram")|("artifact"|"pack","memory")|("branch"|"fork","pool_size")|("health","retries")|("restart","max_retries")=>"u32",
            ("","storage"|"overlay")|("branch"|"fork","cuda_vram_limit_mib")=>"u64",
            ("service","port"|"listen")=>"u16",
            ("","dev"|"artifact"|"pack"|"network"|"branch"|"fork"|"health"|"restart"|"auth"|"service")=>{validate_table(value,key)?;continue;},
            ("","secrets")=>{for (_,secret) in value.as_table().context("secrets must be a table")? {validate_table(secret,"secret")?;}continue;},
            _=>bail!("unknown Smolfile field {location}; this application supports the SmolVM 1.16 Smolfile schema"),
        };
        let valid = match kind {
            "string" => value.is_str(),
            "strings" => value
                .as_array()
                .is_some_and(|values| values.iter().all(toml::Value::is_str)),
            "bool" => value.is_bool(),
            "u8" => value.as_integer().is_some_and(|v| (0..=255).contains(&v)),
            "u16" => value.as_integer().is_some_and(|v| (0..=65535).contains(&v)),
            "u32" => value
                .as_integer()
                .is_some_and(|v| (0..=u32::MAX as i64).contains(&v)),
            "u64" => value.as_integer().is_some_and(|v| v >= 0),
            _ => false,
        };
        if !valid {
            bail!("Smolfile field {location} must have type {kind}");
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Config {
    pub home: PathBuf,
    pub artifacts_dir: PathBuf,
    pub state_root: PathBuf,
    pub locks_dir: PathBuf,
    pub harness_home: PathBuf,
    pub definition: Definition,
    pub workspace: PathBuf,
}
impl Config {
    pub fn resolve(options: &Options) -> Result<Self> {
        let cwd = env::current_dir().context("cannot read invocation directory")?;
        let workspace =
            platform::canonical_workspace(options.workspace.as_deref().unwrap_or(&cwd))?;
        let home = application_home()?;
        let definition_path = select_definition(
            &cwd,
            &workspace,
            &home,
            options.smolfile.as_deref(),
            env::var_os("BASE_SMOLFILE").as_deref(),
        )?;
        let definition = Definition::read(definition_path)?;
        let artifacts_dir = component_path("ARTIFACTS_DIR", home.join("artifacts"), &cwd);
        let state_root = component_path("STATE_ROOT", home.join("state"), &cwd);
        let locks_dir = home.join("locks");
        let harness_home = if definition.profile == Profile::Codex {
            component_path("CODEX_HOME_DIR", home.join("codex-home"), &cwd)
        } else {
            home.join("harness-home").join(definition.profile.id())
        };
        Ok(Self {
            home,
            artifacts_dir,
            state_root,
            locks_dir,
            harness_home,
            definition,
            workspace,
        })
    }
}
fn component_path(name: &str, default: PathBuf, cwd: &Path) -> PathBuf {
    match env::var_os(name).filter(|v| !v.is_empty()) {
        Some(path) => absolute(cwd, Path::new(&path)),
        None => default,
    }
}
pub fn application_home() -> Result<PathBuf> {
    #[cfg(feature = "test-adapter")]
    if let Some(home) = env::var_os("SMOLVM_TEST_HOME") {
        let home = PathBuf::from(home);
        if !home.is_absolute() {
            bail!("SMOLVM_TEST_HOME must be an absolute path");
        }
        return Ok(home.join(".smolsandbox"));
    }
    Ok(dirs::home_dir()
        .context("cannot resolve the current user's native home directory")?
        .join(".smolsandbox"))
}
fn absolute(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        cwd.join(path)
    }
}
pub fn repository_root(workspace: &Path) -> Option<PathBuf> {
    workspace
        .ancestors()
        .find(|path| path.join(".git").is_dir() || path.join(".git").is_file())
        .map(Path::to_owned)
}
pub fn select_definition(
    cwd: &Path,
    workspace: &Path,
    home: &Path,
    flag: Option<&str>,
    base: Option<&std::ffi::OsStr>,
) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(value) = flag {
        if value.is_empty() {
            bail!("--smolfile requires a nonempty filename or path");
        }
        let path = Path::new(value);
        #[cfg(windows)]
        let native_prefix = matches!(
            path.components().next(),
            Some(std::path::Component::Prefix(_))
        );
        #[cfg(not(windows))]
        let native_prefix = false;
        if path.is_absolute()
            || native_prefix
            || value.contains(std::path::MAIN_SEPARATOR)
            || value.contains('/')
        {
            candidates.push(absolute(cwd, path));
        } else {
            if let Some(repository) = repository_root(workspace) {
                candidates.push(repository.join(".smolsandbox").join(value));
            }
            candidates.push(home.join(value));
        }
    } else if let Some(value) = base.filter(|v| !v.is_empty()) {
        candidates.push(absolute(cwd, Path::new(value)));
    } else {
        if let Some(repository) = repository_root(workspace) {
            candidates.push(repository.join(".smolsandbox/default.smlbx"));
        }
        candidates.push(home.join("default.smlbx"));
    }
    for candidate in &candidates {
        match fs::symlink_metadata(candidate) {
            Ok(_) => return Ok(candidate.clone()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("cannot inspect selected Smolfile {}", candidate.display())
                })
            }
        }
    }
    bail!("Smolfile not found; searched: {}. Run codex-sandbox create-config --harness codex, or select an existing file with --smolfile",candidates.iter().map(|p|p.display().to_string()).collect::<Vec<_>>().join(", "))
}
pub fn create_config(harness: &str) -> Result<PathBuf> {
    let profile = Profile::parse(harness)?;
    let destination = application_home()?.join("default.smlbx");
    Definition::parse(destination.clone(), profile.template().as_bytes().to_vec())?;
    platform::publish_new(&destination, profile.template().as_bytes())?;
    println!(
        "Created {} for {}.\nNext: codex-sandbox build\nThen: codex-sandbox start",
        destination.display(),
        profile.display_name()
    );
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn four_templates_are_complete_and_isolated() {
        for profile in [
            Profile::Codex,
            Profile::Claude,
            Profile::OpenCode,
            Profile::Hermes,
        ] {
            let definition = Definition::parse(
                PathBuf::from("test.smlbx"),
                profile.template().as_bytes().to_vec(),
            )
            .unwrap();
            assert_eq!(definition.profile, profile);
            if profile != Profile::Codex {
                assert!(!profile.template().contains("@openai/codex"));
                assert!(!profile.launch().contains(&OsString::from("--no-daemon")));
            }
        }
    }
    #[test]
    fn nearest_file_is_selected_even_when_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let home = temp.path().join("home");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::create_dir_all(repo.join(".smolsandbox")).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(repo.join(".smolsandbox/default.smlbx"), "invalid").unwrap();
        fs::write(home.join("default.smlbx"), Profile::Codex.template()).unwrap();
        let selected = select_definition(temp.path(), &repo, &home, None, None).unwrap();
        assert_eq!(selected, repo.join(".smolsandbox/default.smlbx"));
        assert!(Definition::read(selected).is_err());
    }
    #[test]
    fn named_lookup_is_exact_and_paths_bypass_lookup() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let home = temp.path().join("home");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::create_dir_all(repo.join(".smolsandbox")).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join("custom"), "home").unwrap();
        fs::write(temp.path().join("custom"), "explicit").unwrap();
        assert_eq!(
            select_definition(
                temp.path(),
                &repo,
                &home,
                Some("custom"),
                Some(std::ffi::OsStr::new("missing"))
            )
            .unwrap(),
            home.join("custom")
        );
        assert_eq!(
            select_definition(temp.path(), &repo, &home, Some("./custom"), None).unwrap(),
            temp.path().join("./custom")
        );
        assert_eq!(
            select_definition(
                temp.path(),
                &repo,
                &home,
                None,
                Some(std::ffi::OsStr::new("custom"))
            )
            .unwrap(),
            temp.path().join("custom")
        );
        assert!(select_definition(temp.path(), &repo, &home, Some("custom.smlbx"), None).is_err());
    }
    #[test]
    fn metadata_is_required_and_unique() {
        for text in [
            "image = 'debian'".to_owned(),
            format!(
                "# codex-sandbox-harness: codex\n{}",
                Profile::Codex.template()
            ),
            Profile::Codex
                .template()
                .replacen("harness: codex", "harness: unknown", 1),
        ] {
            assert!(Definition::parse(PathBuf::from("bad"), text.into_bytes()).is_err());
        }
    }
    #[test]
    fn schema_rejects_invalid_known_fields_and_unknown_fields() {
        for text in [
            "workdir = 3",
            "cpus = 256",
            "memory = -1",
            "env = [3]",
            "unknown = true",
            "[dev]\ninit = [false]",
            "[network]\nallow_hosts = [1]",
            "[network]\nallow_unknown = []",
            "[auth]\nssh_agent = 'yes'",
            "[secrets]\nKEY = { from_env = 3 }",
            "[branch]\nenabled = true\n[fork]\nenabled = false",
        ] {
            let text = format!("# codex-sandbox-harness: codex\n{text}");
            let error =
                Definition::parse(PathBuf::from("invalid.smlbx"), text.into_bytes()).unwrap_err();
            assert!(format!("{error:#}").contains("invalid.smlbx"));
        }
    }
    #[test]
    fn valid_optional_smolfile_fields_do_not_require_template_defaults() {
        let text="# codex-sandbox-harness: claude\nworkdir = '/workspace/myrepo'\n[dev]\ninit = ['claude --version']\n";
        let definition =
            Definition::parse(PathBuf::from("minimal.smlbx"), text.as_bytes().to_vec()).unwrap();
        assert_eq!(definition.profile, Profile::Claude);
        assert!(definition.allow_hosts.is_empty());
    }
    #[test]
    fn codex_template_preserves_baseline_provisioning_pins_and_verification() {
        let baseline = include_str!("../devtools-base.smolfile");
        assert!(Profile::Codex.template().ends_with(baseline));
        assert!(Profile::Codex.template().contains("@openai/codex@0.164.0"));
        assert!(Profile::Codex.template().contains("CBM_DOWNLOAD_URL=https://github.com/DeusData/codebase-memory-mcp/releases/download/v0.11.0"));
        assert!(Profile::Codex
            .template()
            .contains("/root/.local/bin/codebase-memory-mcp --version"));
    }
    #[cfg(windows)]
    #[test]
    fn windows_explicit_paths_bypass_filename_lookup() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = temp.path();
        let home = cwd.join("home");
        fs::create_dir(&home).unwrap();
        let file = cwd.join("custom.smlbx");
        fs::write(&file, Profile::Codex.template()).unwrap();
        assert_eq!(
            select_definition(cwd, cwd, &home, Some(file.to_str().unwrap()), None).unwrap(),
            file
        );
        assert_eq!(
            select_definition(cwd, cwd, &home, Some(r".\custom.smlbx"), None).unwrap(),
            cwd.join(r".\custom.smlbx")
        );
    }
}
