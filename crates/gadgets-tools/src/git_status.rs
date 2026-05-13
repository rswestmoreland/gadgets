//! Local Git status provider.
//!
//! This module runs only fixed local Git observe commands selected by the
//! Gadgets runtime. It does not accept arbitrary Git arguments, run through a
//! shell, create branches, create commits, push remotes, create PRs, call model
//! providers, apply patches, or perform host/server administration.

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
const GIT_STATUS_CAPABILITY: &str = "git.status";
const GIT_STATUS_TOOL: &str = "git.status";
const MAX_CAPTURE_BYTES: usize = 262_144;

#[derive(Debug)]
pub enum GitStatusError {
    Io(std::io::Error),
    Ledger(LedgerError),
    Evidence(EvidenceError),
    Capability(String),
    PolicyDenied(String),
    InvalidProjectRoot(PathBuf),
}

impl fmt::Display for GitStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "git status I/O error: {err}"),
            Self::Ledger(err) => write!(f, "git status ledger error: {err}"),
            Self::Evidence(err) => write!(f, "git status evidence error: {err}"),
            Self::Capability(err) => write!(f, "git status capability error: {err}"),
            Self::PolicyDenied(reason) => write!(f, "git status denied by policy: {reason}"),
            Self::InvalidProjectRoot(path) => write!(f, "invalid project root: {}", path.display()),
        }
    }
}

impl Error for GitStatusError {}

impl From<std::io::Error> for GitStatusError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<LedgerError> for GitStatusError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<EvidenceError> for GitStatusError {
    fn from(value: EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusRequest {
    pub run_id: String,
    pub created_at: String,
    pub zone: String,
    pub runtime_mode: RuntimeMode,
}

impl GitStatusRequest {
    pub fn local_status(run_id: impl Into<String>, created_at: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            created_at: created_at.into(),
            zone: DEFAULT_ZONE.to_string(),
            runtime_mode: RuntimeMode::Safe,
        }
    }

    pub fn with_runtime_mode(mut self, runtime_mode: RuntimeMode) -> Self {
        self.runtime_mode = runtime_mode;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusReport {
    pub run_id: String,
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub branch: Option<String>,
    pub changed_entries: usize,
    pub duration_ms: u128,
    pub evidence_bundle_path: PathBuf,
    pub ledger_path: PathBuf,
    pub ledger_events_appended: usize,
}

#[derive(Debug, Default)]
struct GitStatusState {
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

pub fn run_git_status(
    project_root: &Path,
    gadget: &GadgetManifest,
    request: GitStatusRequest,
) -> Result<GitStatusReport, GitStatusError> {
    if !project_root.exists() || !project_root.is_dir() {
        return Err(GitStatusError::InvalidProjectRoot(
            project_root.to_path_buf(),
        ));
    }

    let project_root = project_root.canonicalize()?;
    let ledger_path = default_ledger_path(&project_root);
    let runs_root = default_runs_root(&project_root);
    let mut state = GitStatusState::default();

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.status.requested",
        "user",
        "cli",
        Some(("git_worktree", ".")),
        "allowed",
        "Local Git status requested through explicit CLI command.",
    )?;

    let action = ActionRequest {
        action_request_id: format!("actreq_{}_1", request.run_id),
        run_id: request.run_id.clone(),
        requested_by_gadget: gadget.metadata.name.clone(),
        capability: CapabilityName::new(GIT_STATUS_CAPABILITY)
            .map_err(|err| GitStatusError::Capability(err.to_string()))?,
        tool: GIT_STATUS_TOOL.to_string(),
        target: ActionTarget {
            zone: Some(request.zone.clone()),
            path: Some(".".to_string()),
            resource: Some("working_tree".to_string()),
        },
        reason: "Read local Git working tree status using a fixed runtime command.".to_string(),
    };
    let context = PolicyContext {
        mode: request.runtime_mode,
        approval_present: false,
        allowlisted_test_command: false,
        allowlisted_git_branch_create: false,
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
        Some(("git_worktree", ".")),
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
        Some(("git_worktree", ".")),
        decision_kind_as_str(&evaluation.decision.decision),
        &evaluation.decision.reason,
    )?;
    if evaluation.decision.decision != DecisionKind::Allowed {
        return Err(GitStatusError::PolicyDenied(evaluation.decision.reason));
    }

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.status.started",
        "gadget",
        &gadget.metadata.name,
        Some(("git_worktree", ".")),
        "allowed",
        "Fixed local git status command started without shell, branch, commit, push, PR, provider, patch, or admin actions.",
    )?;

    let capture = execute_git_status(&project_root)?;
    let branch = extract_branch(&capture.stdout);
    let changed_entries = count_status_entries(&capture.stdout);
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        if capture.passed {
            "git.status.completed"
        } else {
            "git.status.failed"
        },
        "gadget",
        &gadget.metadata.name,
        Some(("git_worktree", ".")),
        if capture.passed { "allowed" } else { "failed" },
        if capture.passed {
            "Fixed local git status command completed with exit status 0."
        } else {
            "Fixed local git status command completed with nonzero exit status."
        },
    )?;

    let mut evidence_request = EvidenceWriteRequest::observe(
        request.run_id.clone(),
        gadget.metadata.name.clone(),
        request.created_at.clone(),
        build_summary(&capture, branch.as_deref(), changed_entries),
    );
    evidence_request.status = if capture.passed {
        "completed".to_string()
    } else {
        "failed".to_string()
    };
    evidence_request.assumptions = build_assumptions();
    evidence_request.extra_artifacts = vec![
        EvidenceTextArtifact::new("git_command", "git_command.txt", git_command_artifact()),
        EvidenceTextArtifact::new("git_status", "git_status.txt", capture.stdout.clone()),
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
            "branch",
            "branch.txt",
            format!(
                "{}\n",
                branch.clone().unwrap_or_else(|| "unknown".to_string())
            ),
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
        "Git status evidence bundle created for fixed local observe command.",
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
        "Git status run completed without branch, commit, push, PR, provider, patch, shell, or admin actions.",
    )?;

    Ok(GitStatusReport {
        run_id: request.run_id,
        passed: capture.passed,
        exit_code: capture.exit_code,
        branch,
        changed_entries,
        duration_ms: capture.duration_ms,
        evidence_bundle_path: evidence_report.bundle_path,
        ledger_path,
        ledger_events_appended: state.ledger_events_appended,
    })
}

fn execute_git_status(project_root: &Path) -> Result<GitCapture, GitStatusError> {
    let start = Instant::now();
    let output = Command::new("git")
        .args(["status", "--short", "--branch", "--untracked-files=normal"])
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
    request: &GitStatusRequest,
    state: &mut GitStatusState,
    event_type: &str,
    actor_kind: &str,
    actor_id: &str,
    target: Option<(&str, &str)>,
    decision: &str,
    summary: &str,
) -> Result<(), GitStatusError> {
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

fn build_summary(capture: &GitCapture, branch: Option<&str>, changed_entries: usize) -> String {
    let status = if capture.passed {
        "completed"
    } else {
        "failed"
    };
    let mut out = String::new();
    out.push_str("Local Git status completed.\n\n");
    out.push_str("Command: git status --short --branch --untracked-files=normal\n");
    out.push_str(&format!("Status: {}\n", status));
    out.push_str(&format!(
        "Exit code: {}\n",
        display_exit_code(capture.exit_code)
    ));
    out.push_str(&format!("Duration ms: {}\n", capture.duration_ms));
    out.push_str(&format!("Branch: {}\n", branch.unwrap_or("unknown")));
    out.push_str(&format!("Changed entries: {}\n\n", changed_entries));
    out.push_str("No branch, commit, push, PR, provider tool, patch, shell, Linux admin, database, cloud, or deployment action was executed by this provider.\n");
    out
}

fn build_assumptions() -> Vec<String> {
    vec![
        "The Git command was selected by the runtime and was not supplied by model output.".to_string(),
        "The Git command was launched directly without sh -c or provider-side tools.".to_string(),
        "Only local working tree status was requested; no branch, commit, push, pull, fetch, merge, PR, or remote operation was attempted.".to_string(),
        "Captured Git output was capped and secret-like lines were redacted before evidence write.".to_string(),
    ]
}

fn git_command_artifact() -> String {
    "program=git\nargs=status --short --branch --untracked-files=normal\nremote=false\nmutates=false\n".to_string()
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

fn extract_branch(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let branch = line.strip_prefix("## ")?;
        Some(
            branch
                .split("...")
                .next()
                .unwrap_or(branch)
                .trim()
                .to_string(),
        )
    })
}

fn count_status_entries(stdout: &str) -> usize {
    stdout
        .lines()
        .filter(|line| !line.starts_with("## "))
        .count()
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
            out.push_str("[redacted secret-like git status line]\n");
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
    out.push_str("\n[output truncated by Gadgets Git status provider]\n");
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
    fn extracts_branch_from_status_output() {
        assert_eq!(
            extract_branch("## main...origin/main\n M README.md\n").as_deref(),
            Some("main")
        );
    }

    #[test]
    fn counts_changed_entries_excluding_branch_line() {
        assert_eq!(
            count_status_entries("## main\n M README.md\n?? docs/new.md\n"),
            2
        );
    }

    #[test]
    fn redacts_secret_like_status_lines() {
        let output = sanitize_output(b"## main\n?? .env\n?? docs/readme.md\n");
        assert!(output.contains("docs/readme.md"));
        assert!(!output.contains(".env"));
    }
}
