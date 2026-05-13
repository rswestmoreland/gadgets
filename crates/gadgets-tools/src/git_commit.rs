//! Approved local Git commit provider.
//!
//! This module creates a local commit only from files named by a verified
//! approved patch request. It does not checkout or switch branches, push, pull,
//! fetch, merge, rebase, create PRs, run shell commands, call model providers,
//! apply patches, run tests, or perform host/server administration.

use crate::redaction::{sanitize_bytes, RedactionConfig, DEFAULT_CAPTURE_BYTES};
use gadgets_approval::{
    read_approval, read_request, verify_approval, ApprovalError, ApprovalRequestRecord,
};
use gadgets_core::{ActionRequest, ActionTarget, CapabilityName, DecisionKind, GadgetManifest};
use gadgets_evidence::{
    create_observe_bundle, default_runs_root, EvidenceError, EvidenceTextArtifact,
    EvidenceWriteRequest,
};
use gadgets_ledger::{
    append_event, default_ledger_path, new_audit_event, with_target, LedgerError,
};
use gadgets_policy::{evaluate_action, PolicyContext, RuntimeMode};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

const DEFAULT_ZONE: &str = "local_repo";
const GIT_COMMIT_CAPABILITY: &str = "git.commit.create";
const GIT_COMMIT_TOOL: &str = "git.commit.create";
const MAX_COMMIT_MESSAGE_BYTES: usize = 512;

#[derive(Debug)]
pub enum GitCommitError {
    Io(std::io::Error),
    Approval(ApprovalError),
    Ledger(LedgerError),
    Evidence(EvidenceError),
    Capability(String),
    PolicyDenied(String),
    ApprovalNotVerified(Vec<String>),
    ApprovalRecordMissing(String),
    InvalidProjectRoot(PathBuf),
    InvalidPatch(String),
    InvalidPath(String),
    InvalidCommitMessage(String),
    ProtectedBranch(String),
    DetachedHead,
    GitCommandFailed { command: String, stderr: String },
    PreexistingStagedChanges(Vec<String>),
    UnexpectedStagedFiles(Vec<String>),
    NoApprovedFiles,
    NoStagedApprovedFiles,
}

impl fmt::Display for GitCommitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "git commit I/O error: {err}"),
            Self::Approval(err) => write!(f, "git commit approval error: {err}"),
            Self::Ledger(err) => write!(f, "git commit ledger error: {err}"),
            Self::Evidence(err) => write!(f, "git commit evidence error: {err}"),
            Self::Capability(err) => write!(f, "git commit capability error: {err}"),
            Self::PolicyDenied(reason) => write!(f, "git commit denied by policy: {reason}"),
            Self::ApprovalNotVerified(errors) => {
                write!(f, "approval verification failed: {}", errors.join("; "))
            }
            Self::ApprovalRecordMissing(id) => write!(f, "approval record missing for {id}"),
            Self::InvalidProjectRoot(path) => write!(f, "invalid project root: {}", path.display()),
            Self::InvalidPatch(reason) => write!(f, "invalid approved patch: {reason}"),
            Self::InvalidPath(path) => {
                write!(f, "approved patch path is not safe for commit: {path}")
            }
            Self::InvalidCommitMessage(reason) => write!(f, "invalid commit message: {reason}"),
            Self::ProtectedBranch(branch) => {
                write!(f, "current branch is protected by config: {branch}")
            }
            Self::DetachedHead => {
                write!(f, "cannot create a Gadgets commit while HEAD is detached")
            }
            Self::GitCommandFailed { command, stderr } => {
                write!(f, "git command failed ({command}): {stderr}")
            }
            Self::PreexistingStagedChanges(paths) => write!(
                f,
                "preexisting staged changes must be cleared before commit: {}",
                paths.join(", ")
            ),
            Self::UnexpectedStagedFiles(paths) => write!(
                f,
                "unexpected staged files detected after staging approved paths: {}",
                paths.join(", ")
            ),
            Self::NoApprovedFiles => {
                write!(f, "approval patch did not contain commit-safe file paths")
            }
            Self::NoStagedApprovedFiles => {
                write!(f, "no approved files had staged changes to commit")
            }
        }
    }
}

impl Error for GitCommitError {}

impl From<std::io::Error> for GitCommitError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ApprovalError> for GitCommitError {
    fn from(value: ApprovalError) -> Self {
        Self::Approval(value)
    }
}

impl From<LedgerError> for GitCommitError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<EvidenceError> for GitCommitError {
    fn from(value: EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitRequest {
    pub run_id: String,
    pub created_at: String,
    pub zone: String,
    pub runtime_mode: RuntimeMode,
    pub approval_request_id: String,
    pub commit_message: String,
    pub protected_branches: Vec<String>,
}

impl GitCommitRequest {
    pub fn approved_patch_commit(
        run_id: impl Into<String>,
        created_at: impl Into<String>,
        approval_request_id: impl Into<String>,
        commit_message: impl Into<String>,
        protected_branches: Vec<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            created_at: created_at.into(),
            zone: DEFAULT_ZONE.to_string(),
            runtime_mode: RuntimeMode::Safe,
            approval_request_id: approval_request_id.into(),
            commit_message: commit_message.into(),
            protected_branches,
        }
    }

    pub fn with_runtime_mode(mut self, runtime_mode: RuntimeMode) -> Self {
        self.runtime_mode = runtime_mode;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitReport {
    pub run_id: String,
    pub approval_request_id: String,
    pub branch_name: String,
    pub approved_files: Vec<String>,
    pub staged_files: Vec<String>,
    pub commit_hash: Option<String>,
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
    pub evidence_bundle_path: PathBuf,
    pub ledger_path: PathBuf,
    pub ledger_events_appended: usize,
}

#[derive(Debug, Default)]
struct GitCommitState {
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

pub fn run_git_commit_approved_patch(
    project_root: &Path,
    gadget: &GadgetManifest,
    request: GitCommitRequest,
) -> Result<GitCommitReport, GitCommitError> {
    if !project_root.exists() || !project_root.is_dir() {
        return Err(GitCommitError::InvalidProjectRoot(
            project_root.to_path_buf(),
        ));
    }

    validate_commit_message(&request.commit_message)?;

    let project_root = project_root.canonicalize()?;
    let ledger_path = default_ledger_path(&project_root);
    let runs_root = default_runs_root(&project_root);
    let mut state = GitCommitState::default();

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.commit.requested",
        "user",
        "cli",
        Some(("approval", &request.approval_request_id)),
        "allowed",
        "Local Git commit requested through explicit CLI command for an approved patch scope.",
    )?;

    let verification = verify_approval(&project_root, &request.approval_request_id)?;
    if !verification.valid {
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "approval.verification_failed",
            "gadget",
            &gadget.metadata.name,
            Some(("approval", &request.approval_request_id)),
            "denied",
            "Approval verification failed before Git commit.",
        )?;
        return Err(GitCommitError::ApprovalNotVerified(verification.errors));
    }

    let approval =
        read_approval(&project_root, &request.approval_request_id)?.ok_or_else(|| {
            GitCommitError::ApprovalRecordMissing(request.approval_request_id.clone())
        })?;
    if approval.status != "approved" {
        return Err(GitCommitError::ApprovalNotVerified(vec![format!(
            "approval status is {}",
            approval.status
        )]));
    }

    let approval_request = read_request(&project_root, &request.approval_request_id)?;
    validate_request_for_commit(&approval_request)?;
    if approval.scope_hash != approval_request.scope_hash {
        return Err(GitCommitError::ApprovalNotVerified(vec![
            "approval scope hash does not match request scope hash".to_string(),
        ]));
    }

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "approval.used",
        "gadget",
        &gadget.metadata.name,
        Some(("approval", &request.approval_request_id)),
        "allowed",
        "Verified approval record will be used only to stage and commit approved patch files.",
    )?;

    let patch_text = fs::read_to_string(&verification.patch_path)?;
    let approved_files = extract_approved_files(&patch_text)?;
    if approved_files.is_empty() {
        return Err(GitCommitError::NoApprovedFiles);
    }

    let current_branch = current_branch(&project_root)?;
    if current_branch == "HEAD" {
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "action.denied",
            "gadget",
            &gadget.metadata.name,
            Some(("git_branch", "HEAD")),
            "denied",
            "Git commit denied because HEAD is detached.",
        )?;
        return Err(GitCommitError::DetachedHead);
    }
    if branch_matches_protected(&current_branch, &request.protected_branches) {
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "action.denied",
            "gadget",
            &gadget.metadata.name,
            Some(("git_branch", &current_branch)),
            "denied",
            "Git commit denied because the current branch matches protected branch config.",
        )?;
        return Err(GitCommitError::ProtectedBranch(current_branch));
    }

    let action = ActionRequest {
        action_request_id: format!("actreq_{}_1", request.run_id),
        run_id: request.run_id.clone(),
        requested_by_gadget: gadget.metadata.name.clone(),
        capability: CapabilityName::new(GIT_COMMIT_CAPABILITY)
            .map_err(|err| GitCommitError::Capability(err.to_string()))?,
        tool: GIT_COMMIT_TOOL.to_string(),
        target: ActionTarget {
            zone: Some(request.zone.clone()),
            path: Some(".".to_string()),
            resource: Some(format!("approval:{}", request.approval_request_id)),
        },
        reason: "Create one local Git commit from files named by a verified approved patch."
            .to_string(),
    };
    let context = PolicyContext {
        mode: request.runtime_mode,
        approval_present: true,
        allowlisted_test_command: false,
        allowlisted_git_branch_create: false,
        approved_git_commit: true,
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
        Some(("approval", &request.approval_request_id)),
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
        Some(("approval", &request.approval_request_id)),
        decision_kind_as_str(&evaluation.decision.decision),
        &evaluation.decision.reason,
    )?;
    if evaluation.decision.decision != DecisionKind::Allowed {
        return Err(GitCommitError::PolicyDenied(evaluation.decision.reason));
    }

    let preexisting_staged = staged_files(&project_root)?;
    if !preexisting_staged.is_empty() {
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "action.denied",
            "gadget",
            &gadget.metadata.name,
            Some(("git_index", "staged")),
            "denied",
            "Git commit denied because preexisting staged changes were present before Gadgets staging.",
        )?;
        return Err(GitCommitError::PreexistingStagedChanges(preexisting_staged));
    }

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.commit.stage_started",
        "gadget",
        &gadget.metadata.name,
        Some(("approval", &request.approval_request_id)),
        "allowed",
        "Staging approved patch files only using a fixed git add command.",
    )?;
    let add_capture = execute_git_add(&project_root, &approved_files)?;
    if !add_capture.passed {
        return Err(GitCommitError::GitCommandFailed {
            command: "git add".to_string(),
            stderr: add_capture.stderr,
        });
    }

    let staged_files = staged_files(&project_root)?;
    if staged_files.is_empty() {
        return Err(GitCommitError::NoStagedApprovedFiles);
    }
    let approved_set = approved_files.iter().cloned().collect::<BTreeSet<_>>();
    let unexpected = staged_files
        .iter()
        .filter(|path| !approved_set.contains(*path))
        .cloned()
        .collect::<Vec<_>>();
    if !unexpected.is_empty() {
        return Err(GitCommitError::UnexpectedStagedFiles(unexpected));
    }

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.commit.started",
        "gadget",
        &gadget.metadata.name,
        Some(("git_branch", &current_branch)),
        "allowed",
        "Fixed local git commit command started for staged approved patch files only.",
    )?;
    let commit_capture = execute_git_commit(&project_root, &request.commit_message, &staged_files)?;
    if !commit_capture.passed {
        let _ = execute_git_reset(&project_root, &staged_files);
    }
    let commit_hash = if commit_capture.passed {
        Some(current_commit_hash(&project_root)?)
    } else {
        None
    };

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        if commit_capture.passed {
            "git.commit.completed"
        } else {
            "git.commit.failed"
        },
        "gadget",
        &gadget.metadata.name,
        Some(("git_branch", &current_branch)),
        if commit_capture.passed {
            "allowed"
        } else {
            "failed"
        },
        if commit_capture.passed {
            "Fixed local git commit command completed with exit status 0."
        } else {
            "Fixed local git commit command completed with nonzero exit status; approved files were reset from the index on best effort."
        },
    )?;

    let duration_ms = add_capture.duration_ms + commit_capture.duration_ms;
    let mut evidence_request = EvidenceWriteRequest::observe(
        request.run_id.clone(),
        gadget.metadata.name.clone(),
        request.created_at.clone(),
        build_summary(
            &request,
            &current_branch,
            &approved_files,
            &staged_files,
            commit_hash.as_deref(),
            &commit_capture,
        ),
    );
    evidence_request.status = if commit_capture.passed {
        "completed".to_string()
    } else {
        "failed".to_string()
    };
    evidence_request.assumptions = build_assumptions();
    evidence_request.extra_artifacts = vec![
        EvidenceTextArtifact::new(
            "git_command",
            "git_command.txt",
            git_command_artifact(&staged_files),
        ),
        EvidenceTextArtifact::new(
            "approval_verification",
            "approval_verification.txt",
            format_approval_verification(&request, &approval_request, &approval),
        ),
        EvidenceTextArtifact::new(
            "approved_files",
            "approved_files.txt",
            approved_files.join("\n"),
        ),
        EvidenceTextArtifact::new("staged_files", "staged_files.txt", staged_files.join("\n")),
        EvidenceTextArtifact::new(
            "current_branch",
            "current_branch.txt",
            format!("{}\n", current_branch),
        ),
        EvidenceTextArtifact::new(
            "commit_message",
            "commit_message.txt",
            format!("{}\n", request.commit_message),
        ),
        EvidenceTextArtifact::new(
            "commit_hash",
            "commit_hash.txt",
            format!(
                "{}\n",
                commit_hash.clone().unwrap_or_else(|| "none".to_string())
            ),
        ),
        EvidenceTextArtifact::new(
            "git_add_stdout",
            "git_add_stdout.txt",
            add_capture.stdout.clone(),
        ),
        EvidenceTextArtifact::new(
            "git_add_stderr",
            "git_add_stderr.txt",
            add_capture.stderr.clone(),
        ),
        EvidenceTextArtifact::new(
            "git_commit_stdout",
            "git_commit_stdout.txt",
            commit_capture.stdout.clone(),
        ),
        EvidenceTextArtifact::new(
            "git_commit_stderr",
            "git_commit_stderr.txt",
            commit_capture.stderr.clone(),
        ),
        EvidenceTextArtifact::new(
            "exit_status",
            "exit_status.txt",
            format_exit_status(&commit_capture),
        ),
        EvidenceTextArtifact::new(
            "duration",
            "duration.txt",
            format!("duration_ms={}\n", duration_ms),
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
        "Git commit evidence bundle created for approved local commit command.",
    )?;
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "run.completed",
        "gadget",
        "coordinator",
        None,
        if commit_capture.passed { "allowed" } else { "failed" },
        "Git commit run completed without checkout, push, PR, provider, patch apply, shell, tests, or admin actions.",
    )?;

    Ok(GitCommitReport {
        run_id: request.run_id,
        approval_request_id: request.approval_request_id,
        branch_name: current_branch,
        approved_files,
        staged_files,
        commit_hash,
        passed: commit_capture.passed,
        exit_code: commit_capture.exit_code,
        duration_ms,
        evidence_bundle_path: evidence_report.bundle_path,
        ledger_path,
        ledger_events_appended: state.ledger_events_appended,
    })
}

fn validate_request_for_commit(request: &ApprovalRequestRecord) -> Result<(), GitCommitError> {
    if request.action_kind != "repo.patch.apply" {
        return Err(GitCommitError::ApprovalNotVerified(vec![format!(
            "approval action kind is {}, expected repo.patch.apply",
            request.action_kind
        )]));
    }
    if request.executor_gadget != "patch.writer" {
        return Err(GitCommitError::ApprovalNotVerified(vec![format!(
            "approval executor is {}, expected patch.writer",
            request.executor_gadget
        )]));
    }
    if request.target.zone != DEFAULT_ZONE {
        return Err(GitCommitError::ApprovalNotVerified(vec![format!(
            "approval zone is {}, expected {DEFAULT_ZONE}",
            request.target.zone
        )]));
    }
    Ok(())
}

fn validate_commit_message(message: &str) -> Result<(), GitCommitError> {
    if message.trim() != message || message.is_empty() {
        return Err(GitCommitError::InvalidCommitMessage(
            "message must be non-empty and must not contain leading or trailing whitespace"
                .to_string(),
        ));
    }
    if message.len() > MAX_COMMIT_MESSAGE_BYTES {
        return Err(GitCommitError::InvalidCommitMessage(format!(
            "message exceeds {MAX_COMMIT_MESSAGE_BYTES} bytes"
        )));
    }
    if !message.is_ascii() || message.chars().any(|c| c.is_control()) {
        return Err(GitCommitError::InvalidCommitMessage(
            "message must be printable ASCII on one line".to_string(),
        ));
    }
    Ok(())
}

fn extract_approved_files(patch_text: &str) -> Result<Vec<String>, GitCommitError> {
    let mut files = BTreeSet::new();
    for line in patch_text.lines() {
        let Some(path) = line.strip_prefix("+++ ") else {
            continue;
        };
        let raw_path = path.split_whitespace().next().unwrap_or(path);
        if raw_path == "/dev/null" {
            return Err(GitCommitError::InvalidPatch(
                "file deletion patches are not supported for approved commit scaffolding"
                    .to_string(),
            ));
        }
        let without_prefix = raw_path
            .strip_prefix("b/")
            .or_else(|| raw_path.strip_prefix("a/"))
            .unwrap_or(raw_path);
        let safe = normalize_safe_relative_path(without_prefix)?;
        validate_commit_path(&safe)?;
        files.insert(safe);
    }
    if files.is_empty() {
        return Err(GitCommitError::InvalidPatch(
            "patch contains no +++ file paths".to_string(),
        ));
    }
    Ok(files.into_iter().collect())
}

fn normalize_safe_relative_path(input: &str) -> Result<String, GitCommitError> {
    let path = Path::new(input);
    if path.is_absolute() {
        return Err(GitCommitError::InvalidPath(input.to_string()));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(GitCommitError::InvalidPath(input.to_string()))
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(GitCommitError::InvalidPath(input.to_string()));
    }
    Ok(normalized.to_string_lossy().replace('\\', "/"))
}

fn validate_commit_path(path: &str) -> Result<(), GitCommitError> {
    let lower = path.to_ascii_lowercase();
    if lower == ".git"
        || lower.starts_with(".git/")
        || lower == ".gadgets"
        || lower.starts_with(".gadgets/")
        || lower == ".env"
        || lower.ends_with("/.env")
        || lower.starts_with("secrets/")
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.contains("secret")
        || lower.contains("credential")
    {
        return Err(GitCommitError::InvalidPath(path.to_string()));
    }
    Ok(())
}

fn current_branch(project_root: &Path) -> Result<String, GitCommitError> {
    let capture = run_git_capture(project_root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if !capture.passed {
        return Err(GitCommitError::GitCommandFailed {
            command: "git rev-parse --abbrev-ref HEAD".to_string(),
            stderr: capture.stderr,
        });
    }
    Ok(capture.stdout.trim().to_string())
}

fn current_commit_hash(project_root: &Path) -> Result<String, GitCommitError> {
    let capture = run_git_capture(project_root, &["rev-parse", "HEAD"])?;
    if !capture.passed {
        return Err(GitCommitError::GitCommandFailed {
            command: "git rev-parse HEAD".to_string(),
            stderr: capture.stderr,
        });
    }
    Ok(capture.stdout.trim().to_string())
}

fn staged_files(project_root: &Path) -> Result<Vec<String>, GitCommitError> {
    let capture = run_git_capture(project_root, &["diff", "--cached", "--name-only", "--"])?;
    if !capture.passed {
        return Err(GitCommitError::GitCommandFailed {
            command: "git diff --cached --name-only --".to_string(),
            stderr: capture.stderr,
        });
    }
    let mut files = Vec::new();
    for item in capture.stdout.lines() {
        if item.is_empty() {
            continue;
        }
        let safe = normalize_safe_relative_path(item)?;
        files.push(safe);
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn execute_git_add(project_root: &Path, files: &[String]) -> Result<GitCapture, GitCommitError> {
    let mut args = vec!["add", "--"];
    for file in files {
        args.push(file.as_str());
    }
    run_git_capture(project_root, &args)
}

fn execute_git_commit(
    project_root: &Path,
    message: &str,
    files: &[String],
) -> Result<GitCapture, GitCommitError> {
    let mut args = vec!["commit", "-m", message, "--"];
    for file in files {
        args.push(file.as_str());
    }
    run_git_capture(project_root, &args)
}

fn execute_git_reset(project_root: &Path, files: &[String]) -> Result<GitCapture, GitCommitError> {
    let mut args = vec!["reset", "--"];
    for file in files {
        args.push(file.as_str());
    }
    run_git_capture(project_root, &args)
}

fn run_git_capture(project_root: &Path, args: &[&str]) -> Result<GitCapture, GitCommitError> {
    let start = Instant::now();
    let output = Command::new("git")
        .args(args)
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

fn branch_matches_protected(branch_name: &str, protected_branches: &[String]) -> bool {
    protected_branches.iter().any(|item| {
        if let Some(prefix) = item.strip_suffix('/') {
            branch_name.starts_with(&format!("{prefix}/"))
        } else {
            branch_name == item
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn append_audit(
    ledger_path: &Path,
    request: &GitCommitRequest,
    state: &mut GitCommitState,
    event_type: &str,
    actor_kind: &str,
    actor_id: &str,
    target: Option<(&str, &str)>,
    decision: &str,
    summary: &str,
) -> Result<(), GitCommitError> {
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

fn build_summary(
    request: &GitCommitRequest,
    branch: &str,
    approved_files: &[String],
    staged_files: &[String],
    commit_hash: Option<&str>,
    capture: &GitCapture,
) -> String {
    let status = if capture.passed {
        "completed"
    } else {
        "failed"
    };
    let mut out = String::new();
    out.push_str("Approved local Git commit completed.\n\n");
    out.push_str(&format!(
        "Approval request: {}\n",
        request.approval_request_id
    ));
    out.push_str(&format!("Branch: {}\n", branch));
    out.push_str(&format!("Status: {}\n", status));
    out.push_str(&format!("Commit hash: {}\n", commit_hash.unwrap_or("none")));
    out.push_str(&format!(
        "Exit code: {}\n\n",
        display_exit_code(capture.exit_code)
    ));
    out.push_str("Approved files from patch:\n");
    for path in approved_files {
        out.push_str("- ");
        out.push_str(path);
        out.push('\n');
    }
    out.push_str("\nStaged files committed:\n");
    for path in staged_files {
        out.push_str("- ");
        out.push_str(path);
        out.push('\n');
    }
    out.push_str("\nNo checkout, switch, push, pull, fetch, merge, rebase, PR, provider tool, patch apply, shell, test, Linux admin, database, cloud, or deployment action was executed by this provider.\n");
    out
}

fn build_assumptions() -> Vec<String> {
    vec![
        "The approval request and approval record were verified before any Git staging or commit command.".to_string(),
        "Only files named by the approved patch artifact were staged by this provider.".to_string(),
        "The current branch was checked and protected branches were rejected before any staging.".to_string(),
        "No checkout, switch, push, pull, fetch, merge, rebase, PR, remote operation, shell, provider-side tool, test command, or admin action was attempted.".to_string(),
        "Captured Git output was capped and secret-like lines were redacted before evidence write.".to_string(),
    ]
}

fn git_command_artifact(staged_files: &[String]) -> String {
    let mut out = String::new();
    out.push_str("program=git\n");
    out.push_str("stage_args=add -- <approved-files>\n");
    out.push_str("commit_args=commit -m <validated-message> -- <staged-approved-files>\n");
    out.push_str("checkout=false\nremote=false\npush=false\npr=false\n");
    out.push_str("staged_files=\n");
    for path in staged_files {
        out.push_str(path);
        out.push('\n');
    }
    out
}

fn format_approval_verification(
    request: &GitCommitRequest,
    approval_request: &ApprovalRequestRecord,
    approval: &gadgets_approval::ApprovalRecord,
) -> String {
    let mut out = String::new();
    out.push_str("approval_verified=true\n");
    out.push_str(&format!(
        "approval_request_id={}\n",
        request.approval_request_id
    ));
    out.push_str(&format!("approval_id={}\n", approval.approval_id));
    out.push_str(&format!("approved_by={}\n", approval.approved_by));
    out.push_str(&format!("scope_hash={}\n", approval.scope_hash));
    out.push_str(&format!(
        "patch_sha256={}\n",
        approval_request.target.patch_sha256
    ));
    out.push_str("commit_scope=approved_patch_files_only\n");
    out
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
    sanitize_bytes(
        bytes,
        RedactionConfig {
            max_bytes: DEFAULT_CAPTURE_BYTES,
            redacted_line: "[redacted secret-like git commit line]",
            truncated_notice: "\n[output truncated by Gadgets Git Commit]\n",
        },
    )
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
    fn extracts_safe_patch_paths() {
        let patch = "diff --git a/docs/a.md b/docs/a.md\n--- a/docs/a.md\n+++ b/docs/a.md\n@@ -0,0 +1 @@\n+hello\n";
        let files = extract_approved_files(patch).unwrap();
        assert_eq!(files, vec!["docs/a.md".to_string()]);
    }

    #[test]
    fn rejects_parent_paths() {
        let patch =
            "diff --git a/../bad b/../bad\n--- a/../bad\n+++ b/../bad\n@@ -0,0 +1 @@\n+bad\n";
        assert!(extract_approved_files(patch).is_err());
    }

    #[test]
    fn rejects_protected_branch_match() {
        let protected = vec!["main".to_string(), "release/".to_string()];
        assert!(branch_matches_protected("main", &protected));
        assert!(branch_matches_protected("release/1.0", &protected));
        assert!(!branch_matches_protected("feature/demo", &protected));
    }

    #[test]
    fn validates_commit_message() {
        assert!(validate_commit_message("Apply approved patch").is_ok());
        assert!(validate_commit_message("").is_err());
        assert!(validate_commit_message("bad\nmessage").is_err());
    }
}
