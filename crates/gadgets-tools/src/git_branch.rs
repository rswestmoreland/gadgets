//! Local Git branch provider.
//!
//! This module creates a local branch through one fixed runtime-selected Git
//! command. It does not switch branches, stage files, commit, push, pull, fetch,
//! merge, rebase, create PRs, run shell commands, call model providers, apply
//! patches, or perform host/server administration.

use gadgets_core::{ActionRequest, ActionTarget, CapabilityName, DecisionKind, GadgetManifest};
use gadgets_evidence::{
    create_observe_bundle, default_runs_root, EvidenceError, EvidenceTextArtifact,
    EvidenceWriteRequest,
};
use gadgets_ledger::{
    append_event, default_ledger_path, new_audit_event, with_target, LedgerError,
};
use gadgets_policy::{evaluate_action, PolicyContext, RuntimeMode};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

const DEFAULT_ZONE: &str = "local_repo";
const GIT_BRANCH_CREATE_CAPABILITY: &str = "git.branch.create";
const GIT_BRANCH_CREATE_TOOL: &str = "git.branch.create";
const MAX_CAPTURE_BYTES: usize = 262_144;
const MAX_BRANCH_NAME_BYTES: usize = 128;

#[derive(Debug)]
pub enum GitBranchError {
    Io(std::io::Error),
    Ledger(LedgerError),
    Evidence(EvidenceError),
    Capability(String),
    PolicyDenied(String),
    InvalidProjectRoot(PathBuf),
    InvalidBranchName(String),
    ProtectedBranch(String),
}

impl fmt::Display for GitBranchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "git branch I/O error: {err}"),
            Self::Ledger(err) => write!(f, "git branch ledger error: {err}"),
            Self::Evidence(err) => write!(f, "git branch evidence error: {err}"),
            Self::Capability(err) => write!(f, "git branch capability error: {err}"),
            Self::PolicyDenied(reason) => write!(f, "git branch create denied by policy: {reason}"),
            Self::InvalidProjectRoot(path) => write!(f, "invalid project root: {}", path.display()),
            Self::InvalidBranchName(reason) => write!(f, "invalid branch name: {reason}"),
            Self::ProtectedBranch(branch) => {
                write!(f, "branch name is protected by config: {branch}")
            }
        }
    }
}

impl Error for GitBranchError {}

impl From<std::io::Error> for GitBranchError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<LedgerError> for GitBranchError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<EvidenceError> for GitBranchError {
    fn from(value: EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBranchCreateRequest {
    pub run_id: String,
    pub created_at: String,
    pub zone: String,
    pub runtime_mode: RuntimeMode,
    pub branch_name: String,
    pub protected_branches: Vec<String>,
}

impl GitBranchCreateRequest {
    pub fn create_branch(
        run_id: impl Into<String>,
        created_at: impl Into<String>,
        branch_name: impl Into<String>,
        protected_branches: Vec<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            created_at: created_at.into(),
            zone: DEFAULT_ZONE.to_string(),
            runtime_mode: RuntimeMode::Safe,
            branch_name: branch_name.into(),
            protected_branches,
        }
    }

    pub fn with_runtime_mode(mut self, runtime_mode: RuntimeMode) -> Self {
        self.runtime_mode = runtime_mode;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBranchCreateReport {
    pub run_id: String,
    pub branch_name: String,
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
    pub evidence_bundle_path: PathBuf,
    pub ledger_path: PathBuf,
    pub ledger_events_appended: usize,
}

#[derive(Debug, Default)]
struct GitBranchState {
    ledger_events_appended: usize,
    next_event_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitCapture {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    passed: bool,
    duration_ms: u128,
}

pub fn run_git_branch_create(
    project_root: &Path,
    gadget: &GadgetManifest,
    request: GitBranchCreateRequest,
) -> Result<GitBranchCreateReport, GitBranchError> {
    if !project_root.exists() || !project_root.is_dir() {
        return Err(GitBranchError::InvalidProjectRoot(
            project_root.to_path_buf(),
        ));
    }

    let project_root = project_root.canonicalize()?;
    let ledger_path = default_ledger_path(&project_root);
    let runs_root = default_runs_root(&project_root);
    let mut state = GitBranchState::default();

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.branch.create.requested",
        "user",
        "cli",
        Some(("git_branch", &request.branch_name)),
        "allowed",
        "Local Git branch creation requested through explicit CLI command.",
    )?;

    if let Err(err) = validate_branch_name(&request.branch_name) {
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "action.denied",
            "gadget",
            &gadget.metadata.name,
            Some(("git_branch", &request.branch_name)),
            "denied",
            "Branch creation denied because the requested branch name failed runtime validation.",
        )?;
        return Err(err);
    }

    if branch_matches_protected(&request.branch_name, &request.protected_branches) {
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "action.denied",
            "gadget",
            &gadget.metadata.name,
            Some(("git_branch", &request.branch_name)),
            "denied",
            "Branch creation denied because the requested branch name matches protected branch config.",
        )?;
        return Err(GitBranchError::ProtectedBranch(request.branch_name.clone()));
    }

    let action = ActionRequest {
        action_request_id: format!("actreq_{}_1", request.run_id),
        run_id: request.run_id.clone(),
        requested_by_gadget: gadget.metadata.name.clone(),
        capability: CapabilityName::new(GIT_BRANCH_CREATE_CAPABILITY)
            .map_err(|err| GitBranchError::Capability(err.to_string()))?,
        tool: GIT_BRANCH_CREATE_TOOL.to_string(),
        target: ActionTarget {
            zone: Some(request.zone.clone()),
            path: Some(".".to_string()),
            resource: Some(format!("branch:{}", request.branch_name)),
        },
        reason: "Create one validated local Git branch using a fixed runtime command.".to_string(),
    };
    let context = PolicyContext {
        mode: request.runtime_mode,
        approval_present: false,
        allowlisted_test_command: false,
        allowlisted_git_branch_create: true,
        approved_git_commit: false,
        approved_git_pr_create: false,
        decision_id: format!("dec_{}_1", request.run_id),
    };
    let evaluation = evaluate_action(gadget, &action, &context);
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "policy.checked",
        "gadget",
        &gadget.metadata.name,
        Some(("git_branch", &request.branch_name)),
        decision_kind_as_str(&evaluation.decision.decision),
        &evaluation.decision.reason,
    )?;
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        match evaluation.decision.decision {
            DecisionKind::Allowed => "action.allowed",
            DecisionKind::Denied => "action.denied",
            DecisionKind::RequiresApproval => "action.requires_approval",
        },
        "gadget",
        &gadget.metadata.name,
        Some(("git_branch", &request.branch_name)),
        decision_kind_as_str(&evaluation.decision.decision),
        &evaluation.decision.reason,
    )?;
    if evaluation.decision.decision != DecisionKind::Allowed {
        return Err(GitBranchError::PolicyDenied(evaluation.decision.reason));
    }

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.branch.create.started",
        "gadget",
        &gadget.metadata.name,
        Some(("git_branch", &request.branch_name)),
        "allowed",
        "Fixed local git branch create command started without checkout, commit, push, PR, provider, patch, shell, or admin actions.",
    )?;

    let capture = execute_git_branch_create(&project_root, &request.branch_name)?;
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        if capture.passed {
            "git.branch.create.completed"
        } else {
            "git.branch.create.failed"
        },
        "gadget",
        &gadget.metadata.name,
        Some(("git_branch", &request.branch_name)),
        if capture.passed { "allowed" } else { "failed" },
        if capture.passed {
            "Fixed local git branch create command completed with exit status 0."
        } else {
            "Fixed local git branch create command completed with nonzero exit status."
        },
    )?;

    let mut evidence_request = EvidenceWriteRequest::observe(
        request.run_id.clone(),
        gadget.metadata.name.clone(),
        request.created_at.clone(),
        build_summary(&request, &capture),
    );
    evidence_request.status = if capture.passed {
        "completed".to_string()
    } else {
        "failed".to_string()
    };
    evidence_request.assumptions = build_assumptions();
    evidence_request.extra_artifacts = vec![
        EvidenceTextArtifact::new(
            "git_command",
            "git_command.txt",
            git_command_artifact(&request.branch_name),
        ),
        EvidenceTextArtifact::new(
            "branch_name",
            "branch_name.txt",
            format!("{}\n", request.branch_name),
        ),
        EvidenceTextArtifact::new(
            "protected_branches",
            "protected_branches.txt",
            request.protected_branches.join("\n"),
        ),
        EvidenceTextArtifact::new("stdout", "stdout.txt", capture.stdout.clone()),
        EvidenceTextArtifact::new("stderr", "stderr.txt", capture.stderr.clone()),
        EvidenceTextArtifact::new(
            "exit_status",
            "exit_status.txt",
            format_exit_status(&capture),
        ),
        EvidenceTextArtifact::new(
            "duration",
            "duration.txt",
            format!("duration_ms={}\n", capture.duration_ms),
        ),
        EvidenceTextArtifact::new(
            "policy_decision",
            "policy_decision.txt",
            format!(
                "decision={:?}\nreason={}\nmatched_rules={}\n",
                evaluation.decision.decision,
                evaluation.decision.reason,
                evaluation.decision.matched_rules.join(",")
            ),
        ),
    ];

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
        "Git branch creation evidence bundle created for fixed local command.",
    )?;
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "run.completed",
        "gadget",
        "coordinator",
        None,
        if capture.passed { "allowed" } else { "failed" },
        "Git branch creation run completed without checkout, commit, push, PR, provider, patch, shell, or admin actions.",
    )?;

    Ok(GitBranchCreateReport {
        run_id: request.run_id,
        branch_name: request.branch_name,
        passed: capture.passed,
        exit_code: capture.exit_code,
        duration_ms: capture.duration_ms,
        evidence_bundle_path: evidence_report.bundle_path,
        ledger_path,
        ledger_events_appended: state.ledger_events_appended,
    })
}

fn execute_git_branch_create(
    project_root: &Path,
    branch_name: &str,
) -> Result<GitCapture, GitBranchError> {
    let start = Instant::now();
    let output = Command::new("git")
        .args(["branch", branch_name])
        .current_dir(project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    let duration_ms = start.elapsed().as_millis();
    Ok(GitCapture {
        stdout: sanitize_output(&output.stdout),
        stderr: sanitize_output(&output.stderr),
        exit_code: output.status.code(),
        passed: output.status.success(),
        duration_ms,
    })
}

#[allow(clippy::too_many_arguments)]
fn append_audit(
    ledger_path: &Path,
    request: &GitBranchCreateRequest,
    state: &mut GitBranchState,
    event_type: &str,
    actor_kind: &str,
    actor_id: &str,
    target: Option<(&str, &str)>,
    decision: &str,
    summary: &str,
) -> Result<(), GitBranchError> {
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

fn validate_branch_name(input: &str) -> Result<(), GitBranchError> {
    if input.trim() != input || input.is_empty() {
        return Err(GitBranchError::InvalidBranchName(
            "branch name must be non-empty and must not contain leading or trailing whitespace"
                .to_string(),
        ));
    }
    if input.len() > MAX_BRANCH_NAME_BYTES {
        return Err(GitBranchError::InvalidBranchName(format!(
            "branch name exceeds {MAX_BRANCH_NAME_BYTES} bytes"
        )));
    }
    if input.starts_with('-')
        || input.starts_with('/')
        || input.ends_with('/')
        || input.ends_with('.')
        || input.ends_with(".lock")
        || input.contains("..")
        || input.contains("//")
        || input.contains("@{")
        || input.eq_ignore_ascii_case("head")
    {
        return Err(GitBranchError::InvalidBranchName(
            "branch name uses a denied ref pattern".to_string(),
        ));
    }

    for part in input.split('/') {
        if part.is_empty() || part.starts_with('.') {
            return Err(GitBranchError::InvalidBranchName(
                "branch path segments must be non-empty and must not start with dot".to_string(),
            ));
        }
        if !part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        {
            return Err(GitBranchError::InvalidBranchName(
                "branch names may contain only ASCII letters, digits, slash, underscore, dash, and dot".to_string(),
            ));
        }
    }

    Ok(())
}

fn branch_matches_protected(branch_name: &str, protected_branches: &[String]) -> bool {
    protected_branches.iter().any(|item| {
        if let Some(prefix) = item.strip_suffix('/') {
            branch_name.starts_with(&format!("{prefix}/"))
        } else {
            branch_name == item
        }
    })
}

fn build_summary(request: &GitBranchCreateRequest, capture: &GitCapture) -> String {
    let status = if capture.passed {
        "completed"
    } else {
        "failed"
    };
    let mut out = String::new();
    out.push_str("Local Git branch creation completed.\n\n");
    out.push_str("Command: git branch <validated-branch-name>\n");
    out.push_str(&format!("Branch: {}\n", request.branch_name));
    out.push_str(&format!("Status: {}\n", status));
    out.push_str(&format!(
        "Exit code: {}\n",
        display_exit_code(capture.exit_code)
    ));
    out.push_str(&format!("Duration ms: {}\n\n", capture.duration_ms));
    out.push_str("No checkout, switch, stage, commit, push, pull, fetch, merge, rebase, PR, provider tool, patch, shell, Linux admin, database, cloud, or deployment action was executed by this provider.\n");
    out
}

fn build_assumptions() -> Vec<String> {
    vec![
        "The Git command was selected by the runtime and was not supplied by model output.".to_string(),
        "The Git command was launched directly without sh -c or provider-side tools.".to_string(),
        "Only a local branch ref creation was requested; no checkout, commit, push, pull, fetch, merge, PR, or remote operation was attempted.".to_string(),
        "The requested branch name was validated before command execution and protected branches were rejected before policy allow.".to_string(),
        "Captured Git output was capped and secret-like lines were redacted before evidence write.".to_string(),
    ]
}

fn git_command_artifact(branch_name: &str) -> String {
    format!(
        "program=git\nargs=branch <validated-branch-name>\nbranch_name={}\nremote=false\ncheckout=false\ncommit=false\nmutates=true\n",
        branch_name
    )
}

fn format_exit_status(capture: &GitCapture) -> String {
    let mut out = String::new();
    out.push_str(&format!("passed={}\n", capture.passed));
    out.push_str(&format!(
        "exit_code={}\n",
        display_exit_code(capture.exit_code)
    ));
    out
}

fn display_exit_code(value: Option<i32>) -> String {
    value
        .map(|code| code.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn sanitize_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let redacted = redact_secret_like_lines(&text);
    truncate_output(&redacted)
}

fn redact_secret_like_lines(input: &str) -> String {
    let mut out = String::new();
    for line in input.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains(".env")
            || lower.contains("password")
            || lower.contains("secret")
            || lower.contains("token")
            || lower.contains("api_key")
            || lower.contains("apikey")
            || lower.contains("credential")
            || lower.contains(".pem")
            || lower.contains(".key")
        {
            out.push_str("[redacted secret-like git branch line]\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn truncate_output(input: &str) -> String {
    if input.len() <= MAX_CAPTURE_BYTES {
        return input.to_string();
    }
    let mut end = MAX_CAPTURE_BYTES;
    while !input.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = input[..end].to_string();
    out.push_str("\n[output truncated by Gadgets Git branch provider]\n");
    out
}

fn decision_kind_as_str(decision: &DecisionKind) -> &'static str {
    match decision {
        DecisionKind::Allowed => "allowed",
        DecisionKind::Denied => "denied",
        DecisionKind::RequiresApproval => "requires_approval",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_safe_branch_names() {
        assert!(validate_branch_name("feature/test-runner").is_ok());
        assert!(validate_branch_name("fix_123").is_ok());
    }

    #[test]
    fn rejects_option_like_branch_name() {
        assert!(validate_branch_name("--force").is_err());
    }

    #[test]
    fn rejects_parent_or_lock_patterns() {
        assert!(validate_branch_name("feature/../main").is_err());
        assert!(validate_branch_name("feature/main.lock").is_err());
    }

    #[test]
    fn matches_protected_branch_exact_or_prefix() {
        let protected = vec!["main".to_string(), "release/".to_string()];
        assert!(branch_matches_protected("main", &protected));
        assert!(branch_matches_protected("release/1.0", &protected));
        assert!(!branch_matches_protected("feature/main", &protected));
    }
}
