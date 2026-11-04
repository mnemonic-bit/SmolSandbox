use crate::domain::{hash_bytes, Inventory, Machine, MANAGED_KEYS};
use anyhow::Result;
use regex::Regex;
use serde::Serialize;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
pub struct Reference {
    pub definition: Option<PathBuf>,
    pub base_id: Option<String>,
    pub revision: String,
    pub workspace_sha256: Option<String>,
    pub desired_machine: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Row {
    pub name: String,
    pub state: String,
    pub kind: Option<String>,
    pub base_id: Option<String>,
    pub revision: Option<String>,
    pub workspace_path_sha256: Option<String>,
    pub workspace_sha256: Option<String>,
    pub configuration: Option<String>,
    pub source: &'static str,
    pub assessment: &'static str,
    pub reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub version: u8,
    pub scope: &'static str,
    pub workspace: PathBuf,
    pub reference: Reference,
    pub unassociated_workspace_count: usize,
    pub warnings: Vec<String>,
    pub machines: Vec<Row>,
}

struct Patterns {
    builder: Regex,
    login: Regex,
    workspace: Regex,
}

struct NameIdentity {
    kind: &'static str,
    base_id: String,
    revision: Option<String>,
    workspace_id: Option<String>,
}

impl Patterns {
    fn new() -> Self {
        Self {
            builder: Regex::new(r"^codex-builder-([0-9a-f]{12})$").unwrap(),
            login: Regex::new(r"^codex-login-([0-9a-f]{12})-r([0-9]+)$").unwrap(),
            workspace: Regex::new(r"^codex-workspace-([0-9a-f]{12})-r([0-9]+)-([0-9a-f]{12})$")
                .unwrap(),
        }
    }

    fn identity(&self, name: &str) -> Option<NameIdentity> {
        if let Some(captures) = self.builder.captures(name) {
            Some(NameIdentity {
                kind: "builder",
                base_id: captures[1].into(),
                revision: None,
                workspace_id: None,
            })
        } else if let Some(captures) = self.login.captures(name) {
            Some(NameIdentity {
                kind: "login",
                base_id: captures[1].into(),
                revision: Some(captures[2].into()),
                workspace_id: None,
            })
        } else {
            self.workspace.captures(name).map(|captures| NameIdentity {
                kind: "workspace",
                base_id: captures[1].into(),
                revision: Some(captures[2].into()),
                workspace_id: Some(captures[3].into()),
            })
        }
    }
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn revision(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn scalar(value: &str, valid: bool) -> Option<String> {
    valid.then(|| value.to_owned())
}

fn require(valid: bool, field: &str, value: &str, reasons: &mut Vec<String>) {
    if !valid {
        reasons.push(if value.is_empty() {
            format!("Managed {field} metadata is missing.")
        } else {
            format!("Managed {field} metadata is malformed: {value:?}.")
        });
    }
}

fn inspect(machine: &Machine, name: Option<&NameIdentity>, labels: bool) -> Row {
    let mut row = Row {
        name: machine.name.clone(),
        state: machine.state.clone(),
        kind: None,
        base_id: None,
        revision: None,
        workspace_path_sha256: None,
        workspace_sha256: None,
        configuration: None,
        source: if labels { "labels" } else { "legacy-name" },
        assessment: "unknown",
        reasons: Vec::new(),
    };
    if labels {
        row.kind = scalar(
            machine.label(0),
            ["builder", "login", "workspace"].contains(&machine.label(0)),
        );
        row.base_id = scalar(machine.label(1), hex(machine.label(1), 12));
        row.revision = scalar(machine.label(2), revision(machine.label(2)));
        row.workspace_sha256 = scalar(machine.label(3), hex(machine.label(3), 64));
        row.workspace_path_sha256 = scalar(machine.label(4), hex(machine.label(4), 64));
        row.configuration = scalar(
            machine.label(5),
            ["default", "custom"].contains(&machine.label(5)),
        );
        require(
            row.kind.is_some(),
            "kind",
            machine.label(0),
            &mut row.reasons,
        );
        require(
            row.base_id.is_some(),
            "base-id",
            machine.label(1),
            &mut row.reasons,
        );
        match row.kind.as_deref() {
            Some("workspace") => {
                require(
                    row.revision.is_some(),
                    "revision",
                    machine.label(2),
                    &mut row.reasons,
                );
                require(
                    row.workspace_sha256.is_some(),
                    "workspace-sha256",
                    machine.label(3),
                    &mut row.reasons,
                );
                require(
                    row.workspace_path_sha256.is_some(),
                    "workspace-path-sha256",
                    machine.label(4),
                    &mut row.reasons,
                );
                require(
                    row.configuration.is_some(),
                    "configuration",
                    machine.label(5),
                    &mut row.reasons,
                );
            }
            Some("builder" | "login") => {
                let first_forbidden = if row.kind.as_deref() == Some("builder") {
                    2
                } else {
                    require(
                        row.revision.is_some(),
                        "revision",
                        machine.label(2),
                        &mut row.reasons,
                    );
                    3
                };
                for key in &MANAGED_KEYS[first_forbidden..] {
                    if machine.labels.contains_key(*key) {
                        row.reasons.push(format!(
                            "Managed {} metadata contradicts the shared {} kind.",
                            key,
                            row.kind.as_deref().unwrap()
                        ));
                    }
                }
            }
            _ => {}
        }
        if let Some(name) = name {
            if row.kind.as_deref() != Some(name.kind) {
                row.reasons.push(format!(
                    "Machine name claims kind {:?}, contradicting the managed kind metadata.",
                    name.kind
                ));
            }
            if row.base_id.as_deref() != Some(name.base_id.as_str()) {
                row.reasons.push(format!(
                    "Machine name claims base {}, contradicting the managed base metadata.",
                    name.base_id
                ));
            }
            if name.revision.is_some() && row.revision != name.revision {
                row.reasons.push(format!(
                    "Machine name claims revision {}, contradicting the managed revision metadata.",
                    name.revision.as_deref().unwrap()
                ));
            }
            if let Some(suffix) = &name.workspace_id {
                if !row
                    .workspace_sha256
                    .as_deref()
                    .is_some_and(|hash| hash.starts_with(suffix))
                {
                    row.reasons.push(format!(
                        "Machine name workspace suffix {suffix} contradicts the managed workspace-sha256 metadata."
                    ));
                }
            }
        }
    } else if let Some(name) = name {
        row.kind = Some(name.kind.into());
        row.base_id = Some(name.base_id.clone());
        row.revision = name.revision.clone();
        if name.kind == "workspace" {
            row.reasons.push("Legacy workspace names do not establish a canonical workspace association; workspace-path-sha256 metadata is missing.".into());
        }
    }
    row
}

fn compare(row: &mut Row, reference: &Reference, path_hash: &str) {
    let metadata_unknown = !row.reasons.is_empty();
    let workspace = row.kind.as_deref() == Some("workspace");
    let shared = matches!(row.kind.as_deref(), Some("builder" | "login"));
    let other_workspace = row
        .workspace_path_sha256
        .as_deref()
        .is_some_and(|hash| hash != path_hash);
    let different_policy = row
        .revision
        .as_deref()
        .is_some_and(|revision| revision != reference.revision);
    let different_base = row
        .base_id
        .as_ref()
        .zip(reference.base_id.as_ref())
        .is_some_and(|(actual, desired)| actual != desired);
    let different_hash = row
        .workspace_sha256
        .as_ref()
        .zip(reference.workspace_sha256.as_ref())
        .is_some_and(|(actual, desired)| actual != desired);
    let different_name = workspace
        && reference
            .desired_machine
            .as_deref()
            .is_some_and(|desired| row.name != desired);
    if other_workspace {
        row.reasons.push("Workspace path hash differs from the canonical reference workspace; no filesystem path is inferred from the hash.".into());
    }
    if different_policy {
        row.reasons.push(format!(
            "Policy revision {} differs from current revision {}.",
            row.revision.as_deref().unwrap(),
            reference.revision
        ));
    }
    if different_base {
        row.reasons.push(format!(
            "Base {} differs from reference base {}; another definition or runtime may select this base.",
            row.base_id.as_deref().unwrap(),
            reference.base_id.as_deref().unwrap()
        ));
    }
    if different_hash {
        row.reasons.push("Immutable workspace configuration differs from the current invocation; different --allow-host options or creation settings may select this configuration.".into());
    }
    if different_name {
        row.reasons.push(format!(
            "Machine name differs from the desired machine {} for the current invocation.",
            reference.desired_machine.as_deref().unwrap()
        ));
    }
    row.assessment = if metadata_unknown {
        "unknown"
    } else if shared {
        "shared"
    } else if other_workspace {
        "other-workspace"
    } else if workspace && reference.definition.is_none() {
        row.reasons.push(
            "Reference definition is unavailable; launch identity cannot be assessed.".into(),
        );
        "unassessed"
    } else if different_policy {
        "different-policy"
    } else if different_base {
        "different-base"
    } else if different_hash || different_name {
        "different-configuration"
    } else {
        "selected"
    };
    if !["running", "stopped", "created"].contains(&row.state.to_ascii_lowercase().as_str()) {
        row.reasons.push(format!(
            "Runtime state {:?} is unfamiliar; lifecycle compatibility is unknown.",
            row.state
        ));
    }
}

pub fn report(inventory: &Inventory, workspace: &Path, reference: Reference, all: bool) -> Report {
    let patterns = Patterns::new();
    let path_hash = hash_bytes(
        workspace
            .to_str()
            .expect("canonical reference workspace must be UTF-8")
            .as_bytes(),
    );
    let mut result = Report {
        version: 1,
        scope: if all { "all" } else { "workspace" },
        workspace: workspace.to_owned(),
        reference,
        unassociated_workspace_count: 0,
        warnings: Vec::new(),
        machines: Vec::new(),
    };
    if result.reference.definition.is_none() {
        result.warnings.push("No automatic default definition is available. Select a definition with --smolfile to compare launch identity.".into());
    }
    for machine in inventory {
        let name = patterns.identity(&machine.name);
        let labels = machine
            .labels
            .keys()
            .any(|key| key.starts_with("codex-sandbox."));
        if name.is_none() && !labels {
            continue;
        }
        let workspace_candidate = machine.label(0) == "workspace"
            || name.as_ref().is_some_and(|name| name.kind == "workspace");
        if workspace_candidate && !hex(machine.label(4), 64) {
            result.unassociated_workspace_count += 1;
        }
        if !all && machine.label(4) != path_hash {
            continue;
        }
        let mut row = inspect(machine, name.as_ref(), labels);
        compare(&mut row, &result.reference, &path_hash);
        if row.assessment == "unknown" {
            result.warnings.push(format!(
                "{} has incomplete or contradictory managed metadata; ownership is unverified.",
                row.name
            ));
        }
        result.machines.push(row);
    }
    result.machines.sort_by(|a, b| a.name.cmp(&b.name));
    result.warnings.sort();
    result
}

impl Report {
    pub fn print(&self, json: bool) -> Result<()> {
        let output = if json {
            format!("{}\n", serde_json::to_string(self)?)
        } else {
            self.human()
        };
        io::stdout().lock().write_all(output.as_bytes())?;
        Ok(())
    }

    fn human(&self) -> String {
        use std::fmt::Write;
        let mut output = format!("Reference workspace: {}\n", self.workspace.display());
        let definition = self
            .reference
            .definition
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".into());
        writeln!(output, "Selected definition: {definition}").unwrap();
        writeln!(
            output,
            "Desired workspace machine: {}\nCurrent policy revision: {}",
            self.reference
                .desired_machine
                .as_deref()
                .unwrap_or("unavailable"),
            self.reference.revision
        )
        .unwrap();
        for warning in &self.warnings {
            writeln!(output, "WARNING: {warning}").unwrap();
        }
        writeln!(output, "Unassociated workspace candidates: {} (missing or malformed path labels; association with the reference workspace is unknown).", self.unassociated_workspace_count).unwrap();
        output.push_str("Assessments compare the current invocation only; no assessment authorizes deletion. An identity match does not establish artifact availability, port reconciliation, guest boot, or authentication.\n");
        if self.machines.is_empty() {
            output.push_str("No matching machines.\n");
            return output;
        }
        output.push_str("NAME\tSTATE\tKIND\tBASE\tREVISION\tASSESSMENT");
        if self.scope == "all" {
            output.push_str("\tASSOCIATION");
        }
        output.push('\n');
        for row in &self.machines {
            write!(
                output,
                "{}\t{}\t{}\t{}\t{}\t{}",
                row.name,
                row.state,
                row.kind.as_deref().unwrap_or("unknown"),
                row.base_id.as_deref().unwrap_or("unknown"),
                row.revision.as_deref().unwrap_or("unknown"),
                row.assessment
            )
            .unwrap();
            if self.scope == "all" {
                let association = if row.assessment == "shared" {
                    "shared"
                } else {
                    row.workspace_path_sha256.as_deref().unwrap_or("unknown")
                };
                write!(output, "\t{association}").unwrap();
            }
            output.push('\n');
            for reason in &row.reasons {
                writeln!(output, "  {reason}").unwrap();
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::parse_inventory;
    use serde_json::{json, Value};

    const BASE: &str = "123456789abc";
    const OTHER_BASE: &str = "abcdef123456";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const OTHER_HASH: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    const NAME: &str = "codex-workspace-123456789abc-r7-0123456789ab";

    fn reference(available: bool) -> Reference {
        Reference {
            definition: available.then(|| PathBuf::from("selected.smlbx")),
            base_id: available.then(|| BASE.into()),
            revision: "7".into(),
            workspace_sha256: available.then(|| HASH.into()),
            desired_machine: available.then(|| NAME.into()),
        }
    }

    fn workspace_row(name: &str, path: &Path) -> Value {
        json!({"name":name,"state":"stopped","labels":{
            "codex-sandbox.kind":"workspace",
            "codex-sandbox.base-id":BASE,
            "codex-sandbox.revision":"7",
            "codex-sandbox.workspace-sha256":HASH,
            "codex-sandbox.workspace-path-sha256":hash_bytes(path.to_str().unwrap().as_bytes()),
            "codex-sandbox.configuration":"default"
        }})
    }

    fn inventory(rows: Vec<Value>) -> Inventory {
        parse_inventory(&serde_json::to_vec(&rows).unwrap()).unwrap()
    }

    #[test]
    fn scoped_listing_retains_generations_custom_names_and_matching_path_contradictions() {
        let path = Path::new("reference");
        let selected = workspace_row(NAME, path);
        let mut old = workspace_row("old-custom-name", path);
        old["labels"]["codex-sandbox.base-id"] = json!(OTHER_BASE);
        old["labels"]["codex-sandbox.revision"] = json!("6");
        old["labels"]["codex-sandbox.workspace-sha256"] = json!(OTHER_HASH);
        old["labels"]["codex-sandbox.configuration"] = json!("custom");
        old["state"] = json!("paused-by-future-runtime");
        let contradictory_name =
            workspace_row("codex-workspace-abcdef123456-r7-0123456789ab", path);
        let mut contradictory_kind = workspace_row("kind-contradiction", path);
        contradictory_kind["labels"]["codex-sandbox.kind"] = json!("login");
        let rows = inventory(vec![
            selected,
            old,
            contradictory_name,
            contradictory_kind,
            workspace_row("other-workspace", Path::new("elsewhere")),
            json!({"name":"codex-builder-123456789abc","state":"running"}),
            json!({"name":"unrelated","labels":{}}),
        ]);
        let report = report(&rows, path, reference(true), false);
        assert_eq!(report.machines.len(), 4);
        assert_eq!(report.machines[0].assessment, "selected");
        assert_eq!(report.machines[1].assessment, "unknown");
        assert_eq!(report.machines[2].assessment, "unknown");
        let old = &report.machines[3];
        assert_eq!(old.name, "old-custom-name");
        assert_eq!(old.assessment, "different-policy");
        assert_eq!(old.state, "paused-by-future-runtime");
        let reasons = old.reasons.join(" ");
        for part in [
            "revision 6",
            OTHER_BASE,
            "configuration differs",
            "name differs",
            "compatibility is unknown",
        ] {
            assert!(reasons.contains(part), "{reasons}");
        }
        assert_eq!(report.unassociated_workspace_count, 0);
    }

    #[test]
    fn all_scope_accepts_labelled_shared_overrides_and_complete_legacy_names() {
        let path = Path::new("reference");
        let mut other = workspace_row("another-workspace", Path::new("elsewhere"));
        other["labels"]["codex-sandbox.revision"] = json!("6");
        let rows = inventory(vec![
            other,
            json!({"name":"custom-builder","state":"stopped","labels":{
                "codex-sandbox.kind":"builder","codex-sandbox.base-id":BASE}}),
            json!({"name":"custom-login","state":"stopped","labels":{
                "codex-sandbox.kind":"login","codex-sandbox.base-id":OTHER_BASE,
                "codex-sandbox.revision":"6"}}),
            json!({"name":"codex-builder-abcdef123456","state":"created"}),
            json!({"name":"codex-login-abcdef123456-r6","state":"stopped"}),
            json!({"name":"codex-workspace-abcdef123456-r6-abcdef012345","state":"stopped"}),
            json!({"name":"namespace-only","labels":{"codex-sandbox.extra":"yes"}}),
            json!({"name":"codex-builder-abcdef123456-extra"}),
            json!({"name":"codex-workspace-abcdef123456-r6-abcdef012345-extra"}),
        ]);
        let report = report(&rows, path, reference(true), true);
        assert_eq!(report.machines.len(), 7);
        assert_eq!(report.unassociated_workspace_count, 1);
        assert_eq!(report.machines[0].assessment, "other-workspace");
        for row in &report.machines[1..3] {
            assert_eq!(row.assessment, "shared");
            assert_eq!(row.source, "legacy-name");
        }
        assert_eq!(report.machines[3].assessment, "unknown");
        assert!(report.machines[3].workspace_path_sha256.is_none());
        assert_eq!(report.machines[4].assessment, "shared");
        assert_eq!(report.machines[5].assessment, "shared");
        assert_eq!(report.machines[6].assessment, "unknown");
    }

    #[test]
    fn missing_and_malformed_paths_are_counted_without_guessing_association() {
        let path = Path::new("reference");
        let mut missing = workspace_row(NAME, path);
        missing["labels"]
            .as_object_mut()
            .unwrap()
            .remove("codex-sandbox.workspace-path-sha256");
        let mut malformed = workspace_row("custom-name", path);
        malformed["labels"]["codex-sandbox.workspace-path-sha256"] = json!("not-a-hash");
        let rows = inventory(vec![missing, malformed]);
        let scoped = report(&rows, path, reference(true), false);
        assert!(scoped.machines.is_empty());
        assert_eq!(scoped.unassociated_workspace_count, 2);
        let all = report(&rows, path, reference(true), true);
        assert_eq!(all.machines.len(), 2);
        assert!(all.machines.iter().all(|row| row.assessment == "unknown"));
        assert!(all
            .machines
            .iter()
            .all(|row| row.workspace_path_sha256.is_none()));
        assert!(scoped.human().contains("No matching machines."));
        assert!(scoped
            .human()
            .contains("Unassociated workspace candidates: 2"));
    }

    #[test]
    fn unavailable_reference_does_not_hide_valid_ownership_or_override_unknown() {
        let path = Path::new("reference");
        let mut bad = workspace_row("bad", path);
        bad["labels"]["codex-sandbox.configuration"] = json!("invalid");
        let rows = inventory(vec![
            workspace_row(NAME, path),
            workspace_row("elsewhere", Path::new("elsewhere")),
            bad,
            json!({"name":"codex-builder-123456789abc","state":"stopped"}),
        ]);
        let report = report(&rows, path, reference(false), true);
        let assessments: Vec<_> = report.machines.iter().map(|row| row.assessment).collect();
        assert_eq!(
            assessments,
            ["unknown", "shared", "unassessed", "other-workspace"]
        );
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["version"], 1);
        assert!(value["reference"]["definition"].is_null());
        assert!(value["reference"]["desired_machine"].is_null());
        assert!(value["machines"][0]["configuration"].is_null());
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn policy_base_configuration_and_name_have_separate_assessments() {
        let path = Path::new("reference");
        for (field, value, expected) in [
            ("codex-sandbox.revision", "6", "different-policy"),
            ("codex-sandbox.base-id", OTHER_BASE, "different-base"),
            (
                "codex-sandbox.workspace-sha256",
                OTHER_HASH,
                "different-configuration",
            ),
        ] {
            let mut row = workspace_row("custom", path);
            row["labels"][field] = json!(value);
            let result = report(&inventory(vec![row]), path, reference(true), false);
            assert_eq!(result.machines[0].assessment, expected);
        }
        let result = report(
            &inventory(vec![workspace_row("custom", path)]),
            path,
            reference(true),
            false,
        );
        assert_eq!(result.machines[0].assessment, "different-configuration");
        let mut raw_state = workspace_row(NAME, path);
        raw_state["state"] = json!("Suspended");
        let result = report(&inventory(vec![raw_state]), path, reference(true), false);
        assert_eq!(result.machines[0].assessment, "selected");
        assert_eq!(result.machines[0].state, "Suspended");
        assert!(result.human().contains(NAME));
        assert!(result
            .human()
            .contains("lifecycle compatibility is unknown"));
    }
}
