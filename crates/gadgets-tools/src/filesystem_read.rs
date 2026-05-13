//! Observe-only filesystem read provider.
//!
//! This module implements the first real Gadget slice: scoped repository
//! inspection. Every candidate directory and file is evaluated through the
//! policy engine before being traversed or read. Results are written to an
//! evidence bundle and every policy decision is recorded in the audit ledger.

use gadgets_core::{ActionRequest, ActionTarget, CapabilityName, DecisionKind, GadgetManifest};
use gadgets_evidence::{
    create_observe_bundle, default_runs_root, EvidenceTextArtifact, EvidenceWriteRequest,
};
use gadgets_ledger::{
    append_event, default_ledger_path, new_audit_event, with_target, LedgerError,
};
use gadgets_policy::{evaluate_action, PolicyContext, RuntimeMode};
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const DEFAULT_ZONE: &str = "local_repo";
const FILE_READ_CAPABILITY: &str = "file.read";
const FILE_SEARCH_CAPABILITY: &str = "file.search";
const FILE_READ_TOOL: &str = "file.read";
const FILE_SEARCH_TOOL: &str = "file.search";

#[derive(Debug)]
pub enum FilesystemReadError {
    Io(std::io::Error),
    Ledger(LedgerError),
    Evidence(gadgets_evidence::EvidenceError),
    Capability(String),
    InvalidProjectRoot(PathBuf),
}

impl fmt::Display for FilesystemReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "filesystem read I/O error: {err}"),
            Self::Ledger(err) => write!(f, "filesystem read ledger error: {err}"),
            Self::Evidence(err) => write!(f, "filesystem read evidence error: {err}"),
            Self::Capability(err) => write!(f, "filesystem read capability error: {err}"),
            Self::InvalidProjectRoot(path) => write!(f, "invalid project root: {}", path.display()),
        }
    }
}

impl Error for FilesystemReadError {}

impl From<std::io::Error> for FilesystemReadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<LedgerError> for FilesystemReadError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<gadgets_evidence::EvidenceError> for FilesystemReadError {
    fn from(value: gadgets_evidence::EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemReadRequest {
    pub run_id: String,
    pub created_at: String,
    pub user_prompt: String,
    pub zone: String,
    pub max_files: usize,
    pub max_file_bytes: usize,
    pub coordinator_summary: Option<String>,
    pub handoff_id: Option<String>,
    pub handoff_reason: Option<String>,
    pub provider_name: Option<String>,
    pub runtime_mode: RuntimeMode,
}

impl FilesystemReadRequest {
    pub fn observe_repo(
        run_id: impl Into<String>,
        created_at: impl Into<String>,
        user_prompt: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            created_at: created_at.into(),
            user_prompt: user_prompt.into(),
            zone: DEFAULT_ZONE.to_string(),
            max_files: 200,
            max_file_bytes: 16 * 1024,
            coordinator_summary: None,
            handoff_id: None,
            handoff_reason: None,
            provider_name: None,
            runtime_mode: RuntimeMode::Safe,
        }
    }

    pub fn with_runtime_mode(mut self, runtime_mode: RuntimeMode) -> Self {
        self.runtime_mode = runtime_mode;
        self
    }

    pub fn with_coordinator_handoff(
        mut self,
        coordinator_summary: impl Into<String>,
        handoff_id: impl Into<String>,
        handoff_reason: impl Into<String>,
        provider_name: impl Into<String>,
    ) -> Self {
        self.coordinator_summary = Some(coordinator_summary.into());
        self.handoff_id = Some(handoff_id.into());
        self.handoff_reason = Some(handoff_reason.into());
        self.provider_name = Some(provider_name.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemReadReport {
    pub run_id: String,
    pub files_considered: usize,
    pub files_read: usize,
    pub directories_considered: usize,
    pub denied_paths: Vec<String>,
    pub skipped_paths: Vec<String>,
    pub evidence_bundle_path: PathBuf,
    pub ledger_path: PathBuf,
    pub ledger_events_appended: usize,
}

#[derive(Debug, Default)]
struct InspectionState {
    files_considered: usize,
    files_read: Vec<String>,
    directories_considered: usize,
    denied_paths: Vec<String>,
    skipped_paths: Vec<String>,
    excerpts: Vec<(String, String)>,
    ledger_events_appended: usize,
    next_event_number: usize,
    next_action_number: usize,
}

pub fn run_filesystem_read(
    project_root: &Path,
    gadget: &GadgetManifest,
    request: FilesystemReadRequest,
) -> Result<FilesystemReadReport, FilesystemReadError> {
    if !project_root.exists() || !project_root.is_dir() {
        return Err(FilesystemReadError::InvalidProjectRoot(project_root.to_path_buf()));
    }

    let project_root = project_root.canonicalize()?;
    let ledger_path = default_ledger_path(&project_root);
    let runs_root = default_runs_root(&project_root);
    let mut state = InspectionState::default();

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "run.started",
        "gadget",
        "coordinator",
        None,
        "allowed",
        "Observe-only repository inspection run started.",
    )?;

    if let Some(summary) = request.coordinator_summary.as_deref() {
        let provider_name = request.provider_name.as_deref().unwrap_or("mock");
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "provider.response",
            "provider",
            provider_name,
            Some(("gadget", "coordinator")),
            "allowed",
            summary,
        )?;
    }

    if let Some(handoff_id) = request.handoff_id.as_deref() {
        let reason = request
            .handoff_reason
            .as_deref()
            .unwrap_or("Coordinator requested observe-only repository inspection.");
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "handoff.requested",
            "gadget",
            "coordinator",
            Some(("handoff", handoff_id)),
            "allowed",
            reason,
        )?;
    }

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "handoff.allowed",
        "gadget",
        "coordinator",
        Some(("gadget", &gadget.metadata.name)),
        "allowed",
        "Coordinator handed off repository inspection to Filesystem Read Gadget.",
    )?;

    inspect_directory(&project_root, &project_root, gadget, &request, &ledger_path, &mut state)?;

    let summary = build_summary(&request, &state);
    let mut evidence_request = EvidenceWriteRequest::observe(
        request.run_id.clone(),
        gadget.metadata.name.clone(),
        request.created_at.clone(),
        summary,
    );
    evidence_request.denied_actions = state.denied_paths.clone();
    evidence_request.assumptions = build_assumptions(&request, &state);
    evidence_request.extra_artifacts = build_artifacts(&request, &state);

    let evidence_report = create_observe_bundle(&runs_root, evidence_request)?;

    let evidence_target = evidence_report.bundle_path.to_string_lossy().to_string();
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "evidence.created",
        "gadget",
        &gadget.metadata.name,
        Some(("evidence", evidence_target.as_str())),
        "allowed",
        "Filesystem Read Gadget created observe-only evidence bundle.",
    )?;
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "run.completed",
        "gadget",
        "coordinator",
        None,
        "allowed",
        "Observe-only repository inspection run completed.",
    )?;

    Ok(FilesystemReadReport {
        run_id: request.run_id,
        files_considered: state.files_considered,
        files_read: state.files_read.len(),
        directories_considered: state.directories_considered,
        denied_paths: state.denied_paths,
        skipped_paths: state.skipped_paths,
        evidence_bundle_path: evidence_report.bundle_path,
        ledger_path,
        ledger_events_appended: state.ledger_events_appended,
    })
}

fn inspect_directory(
    project_root: &Path,
    directory: &Path,
    gadget: &GadgetManifest,
    request: &FilesystemReadRequest,
    ledger_path: &Path,
    state: &mut InspectionState,
) -> Result<(), FilesystemReadError> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let rel = relative_path(project_root, &path);
        let rel_text = rel.to_string_lossy().replace('\\', "/");
        let metadata = match entry.metadata() {
            Ok(value) => value,
            Err(err) => {
                state.skipped_paths.push(format!("{rel_text}: metadata error: {err}"));
                continue;
            }
        };

        if metadata.is_dir() {
            state.directories_considered += 1;
            let decision = evaluate_path_action(
                gadget,
                request,
                state,
                FILE_SEARCH_CAPABILITY,
                FILE_SEARCH_TOOL,
                &rel_text,
            )?;
            append_policy_event(ledger_path, request, state, gadget, &rel_text, &decision)?;
            if decision.decision.decision == DecisionKind::Allowed {
                inspect_directory(project_root, &path, gadget, request, ledger_path, state)?;
            } else {
                state.denied_paths.push(format!("{rel_text}: {}", decision.decision.reason));
            }
            continue;
        }

        if !metadata.is_file() {
            state.skipped_paths.push(format!("{rel_text}: not a regular file"));
            continue;
        }

        if state.files_considered >= request.max_files {
            state.skipped_paths.push(format!("{rel_text}: max file limit reached"));
            continue;
        }

        state.files_considered += 1;
        let decision = evaluate_path_action(
            gadget,
            request,
            state,
            FILE_READ_CAPABILITY,
            FILE_READ_TOOL,
            &rel_text,
        )?;
        append_policy_event(ledger_path, request, state, gadget, &rel_text, &decision)?;

        if decision.decision.decision != DecisionKind::Allowed {
            state.denied_paths.push(format!("{rel_text}: {}", decision.decision.reason));
            continue;
        }

        match read_file_excerpt(&path, request.max_file_bytes) {
            Ok(Some(excerpt)) => {
                state.files_read.push(rel_text.clone());
                state.excerpts.push((rel_text, excerpt));
            }
            Ok(None) => {
                state.files_read.push(rel_text.clone());
                state.skipped_paths.push(format!("{rel_text}: binary or empty excerpt skipped"));
            }
            Err(err) => {
                state.skipped_paths.push(format!("{rel_text}: read error: {err}"));
            }
        }
    }

    Ok(())
}

fn evaluate_path_action(
    gadget: &GadgetManifest,
    request: &FilesystemReadRequest,
    state: &mut InspectionState,
    capability: &str,
    tool: &str,
    rel_path: &str,
) -> Result<gadgets_policy::PolicyEvaluation, FilesystemReadError> {
    state.next_action_number += 1;
    let action = ActionRequest {
        action_request_id: format!("actreq_{}_{}", request.run_id, state.next_action_number),
        run_id: request.run_id.clone(),
        requested_by_gadget: gadget.metadata.name.clone(),
        capability: CapabilityName::new(capability)
            .map_err(|err| FilesystemReadError::Capability(err.to_string()))?,
        tool: tool.to_string(),
        target: ActionTarget {
            zone: Some(request.zone.clone()),
            path: Some(rel_path.to_string()),
            resource: None,
        },
        reason: "Observe-only repository inspection.".to_string(),
    };
    let context = PolicyContext {
        mode: request.runtime_mode,
        approval_present: false,
        allowlisted_test_command: false,
        allowlisted_git_branch_create: false,
        approved_git_commit: false,
        approved_git_pr_create: false,
        decision_id: format!("dec_{}_{}", request.run_id, state.next_action_number),
    };
    Ok(evaluate_action(gadget, &action, &context))
}

fn append_policy_event(
    ledger_path: &Path,
    request: &FilesystemReadRequest,
    state: &mut InspectionState,
    gadget: &GadgetManifest,
    rel_path: &str,
    decision: &gadgets_policy::PolicyEvaluation,
) -> Result<(), FilesystemReadError> {
    let event_type = match decision.decision.decision {
        DecisionKind::Allowed => "action.allowed",
        DecisionKind::Denied => "action.denied",
        DecisionKind::RequiresApproval => "action.requires_approval",
    };
    append_audit(
        ledger_path,
        request,
        state,
        event_type,
        "gadget",
        &gadget.metadata.name,
        Some(("path", rel_path)),
        decision_kind_as_str(&decision.decision.decision),
        &decision.decision.reason,
    )
}

fn append_audit(
    ledger_path: &Path,
    request: &FilesystemReadRequest,
    state: &mut InspectionState,
    event_type: &str,
    actor_kind: &str,
    actor_id: &str,
    target: Option<(&str, &str)>,
    decision: &str,
    summary: &str,
) -> Result<(), FilesystemReadError> {
    state.next_event_number += 1;
    let event_id = format!("aud_{}_{}", request.run_id, state.next_event_number);
    let event = new_audit_event(
        event_id,
        request.created_at.clone(),
        event_type,
        actor_kind,
        actor_id,
        request.run_id.clone(),
        decision,
        summary,
    );
    let event = match target {
        Some((kind, id)) => with_target(event, kind, id),
        None => event,
    };
    append_event(ledger_path, event)?;
    state.ledger_events_appended += 1;
    Ok(())
}

fn decision_kind_as_str(decision: &DecisionKind) -> &'static str {
    match decision {
        DecisionKind::Allowed => "allowed",
        DecisionKind::Denied => "denied",
        DecisionKind::RequiresApproval => "requires_approval",
    }
}

fn read_file_excerpt(path: &Path, max_file_bytes: usize) -> Result<Option<String>, FilesystemReadError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(max_file_bytes as u64)
        .read_to_end(&mut bytes)?;

    if bytes.is_empty() || bytes.contains(&0) {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&bytes).to_string();
    if text.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

fn relative_path(project_root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(project_root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn build_summary(request: &FilesystemReadRequest, state: &InspectionState) -> String {
    let mut out = String::new();
    out.push_str("Observe-only repository inspection completed.\n\n");
    out.push_str(&format!("User request: {}\n\n", request.user_prompt));
    if let Some(summary) = request.coordinator_summary.as_deref() {
        out.push_str("Coordinator summary: ");
        out.push_str(summary);
        out.push_str("\n\n");
    }
    out.push_str(&format!("Runtime mode: {}\n", request.runtime_mode.as_str()));
    out.push_str(&format!("Directories considered: {}\n", state.directories_considered));
    out.push_str(&format!("Files considered: {}\n", state.files_considered));
    out.push_str(&format!("Files read: {}\n", state.files_read.len()));
    out.push_str(&format!("Denied paths/actions: {}\n", state.denied_paths.len()));
    out.push_str(&format!("Skipped paths: {}\n", state.skipped_paths.len()));
    out.push_str("\nNo files were modified. No commands were executed. Any provider output was limited to Coordinator handoff planning and still policy-checked by the runtime.\n");
    out
}

fn build_assumptions(request: &FilesystemReadRequest, state: &InspectionState) -> Vec<String> {
    let mut assumptions = vec![
        "This was an observe-only filesystem inspection.".to_string(),
        "The Filesystem Read Gadget did not write files or execute commands.".to_string(),
        format!("Policy decisions were made using {} mode.", request.runtime_mode.as_str()),
    ];
    if state.files_considered >= request.max_files {
        assumptions.push(format!(
            "The file scan stopped at the configured limit of {} files.",
            request.max_files
        ));
    }
    assumptions
}

fn build_artifacts(request: &FilesystemReadRequest, state: &InspectionState) -> Vec<EvidenceTextArtifact> {
    let mut artifacts = vec![
        EvidenceTextArtifact::new("files_read", "files_read.txt", state.files_read.join("\n")),
        EvidenceTextArtifact::new("skipped_paths", "skipped_paths.txt", state.skipped_paths.join("\n")),
        EvidenceTextArtifact::new("file_excerpts", "file_excerpts.md", format_excerpts(&state.excerpts)),
    ];

    if request.coordinator_summary.is_some() || request.handoff_id.is_some() {
        artifacts.push(EvidenceTextArtifact::new(
            "coordinator_plan",
            "coordinator_plan.md",
            format_coordinator_plan(request),
        ));
    }

    artifacts
}

fn format_coordinator_plan(request: &FilesystemReadRequest) -> String {
    let mut out = String::new();
    out.push_str("# Coordinator Plan\n\n");
    if let Some(summary) = request.coordinator_summary.as_deref() {
        out.push_str(summary);
        out.push_str("\n\n");
    }
    if let Some(handoff_id) = request.handoff_id.as_deref() {
        out.push_str("Handoff: ");
        out.push_str(handoff_id);
        out.push_str("\n");
    }
    if let Some(reason) = request.handoff_reason.as_deref() {
        out.push_str("Reason: ");
        out.push_str(reason);
        out.push_str("\n");
    }
    if let Some(provider) = request.provider_name.as_deref() {
        out.push_str("Provider: ");
        out.push_str(provider);
        out.push_str("\n");
    }
    out.push_str("\nAuthority note: the provider and Coordinator requested a handoff only; runtime policy authorized each filesystem action.\n");
    out
}

fn format_excerpts(excerpts: &[(String, String)]) -> String {
    let mut out = String::new();
    out.push_str("# File Excerpts\n\n");
    for (path, excerpt) in excerpts {
        out.push_str("## ");
        out.push_str(path);
        out.push_str("\n\n```text\n");
        out.push_str(excerpt);
        if !excerpt.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```\n\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use gadgets_core::GadgetManifest;
    use std::time::{SystemTime, UNIX_EPOCH};

    const READ_GADGET: &str = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: filesystem.read
  version: 0.1.0
  display_name: Filesystem Read Gadget
  description: Reads scoped project files.
runtime:
  model_profile: mock_default
  execution_mode: observe
permission_level: observe
capabilities:
  - file.read
  - file.search
boundaries:
  zones:
    - local_repo
  filesystem:
    roots:
      - "."
    readable_paths:
      - "."
    writable: false
    denied_paths:
      - ".git/"
      - ".gadgets/"
      - ".env"
      - "secrets/"
      - "**/*.pem"
      - "**/*.key"
      - "**/*secret*"
      - "**/*credential*"
tools:
  allowed:
    - file.read
    - file.search
evidence:
  required:
    - summary
approval:
  required_for: []
"#;

    fn temp_project(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "gadgets-fs-read-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn reads_allowed_files_and_records_denied_paths() {
        let root = temp_project("basic");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("README.md"), "hello\n").unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn demo() {}\n").unwrap();
        fs::write(root.join(".env"), "TOKEN=secret\n").unwrap();
        fs::write(root.join(".git/config"), "private\n").unwrap();

        let gadget = GadgetManifest::from_yaml_str(READ_GADGET).unwrap();
        let request = FilesystemReadRequest::observe_repo("run_test", "unix:1", "Review this repo");
        let report = run_filesystem_read(&root, &gadget, request).unwrap();

        assert!(report.files_read >= 2);
        assert!(report.denied_paths.iter().any(|path| path.contains(".env")));
        assert!(report.evidence_bundle_path.exists());
        assert!(report.ledger_path.exists());

        fs::remove_dir_all(root).unwrap();
    }
}
