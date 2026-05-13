//! Local pull request body generator.
//!
//! This module writes reviewable pull request text as evidence only. It does
//! not push, fetch, call GitHub or GitLab, create a remote pull request, run
//! shell commands, call model providers, apply patches, run tests, or perform
//! host/server administration.

use crate::redaction::{redact_one_line, sanitize_text, RedactionConfig};
use gadgets_approval::{
    read_approval, read_request, verify_approval, ApprovalError, ApprovalRecord,
    ApprovalRequestRecord,
};
use gadgets_core::{ActionRequest, ActionTarget, CapabilityName, DecisionKind, GadgetManifest};
use gadgets_evidence::{
    bundle_path_for_run, create_observe_bundle, default_runs_root, read_bundle, summarize_bundle,
    EvidenceError, EvidenceTextArtifact, EvidenceWriteRequest,
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

const DEFAULT_ZONE: &str = "local_repo";
const PR_BODY_CAPABILITY: &str = "git.pr.body.generate";
const PR_BODY_TOOL: &str = "git.pr.body.generate";
const MAX_TITLE_BYTES: usize = 160;
const MAX_BODY_BYTES: usize = 65_536;

#[derive(Debug)]
pub enum GitPrBodyError {
    Io(std::io::Error),
    Approval(ApprovalError),
    Ledger(LedgerError),
    Evidence(EvidenceError),
    Capability(String),
    PolicyDenied(String),
    ApprovalNotVerified(Vec<String>),
    ApprovalRecordMissing(String),
    InvalidProjectRoot(PathBuf),
    InvalidRunId(String),
    InvalidTitle(String),
    InvalidPatch(String),
    InvalidPath(String),
}

impl fmt::Display for GitPrBodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "PR body I/O error: {err}"),
            Self::Approval(err) => write!(f, "PR body approval error: {err}"),
            Self::Ledger(err) => write!(f, "PR body ledger error: {err}"),
            Self::Evidence(err) => write!(f, "PR body evidence error: {err}"),
            Self::Capability(err) => write!(f, "PR body capability error: {err}"),
            Self::PolicyDenied(reason) => write!(f, "PR body denied by policy: {reason}"),
            Self::ApprovalNotVerified(errors) => {
                write!(f, "approval verification failed: {}", errors.join("; "))
            }
            Self::ApprovalRecordMissing(id) => write!(f, "approval record missing for {id}"),
            Self::InvalidProjectRoot(path) => write!(f, "invalid project root: {}", path.display()),
            Self::InvalidRunId(value) => write!(f, "invalid run id: {value}"),
            Self::InvalidTitle(reason) => write!(f, "invalid PR title: {reason}"),
            Self::InvalidPatch(reason) => write!(f, "invalid approved patch: {reason}"),
            Self::InvalidPath(path) => {
                write!(f, "approved patch path is not safe for PR body: {path}")
            }
        }
    }
}

impl Error for GitPrBodyError {}

impl From<std::io::Error> for GitPrBodyError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ApprovalError> for GitPrBodyError {
    fn from(value: ApprovalError) -> Self {
        Self::Approval(value)
    }
}

impl From<LedgerError> for GitPrBodyError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<EvidenceError> for GitPrBodyError {
    fn from(value: EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitPrBodyRequest {
    pub run_id: String,
    pub created_at: String,
    pub zone: String,
    pub runtime_mode: RuntimeMode,
    pub approval_request_id: String,
    pub test_run_id: Option<String>,
    pub commit_run_id: Option<String>,
    pub title_override: Option<String>,
}

impl GitPrBodyRequest {
    pub fn local_body(
        run_id: impl Into<String>,
        created_at: impl Into<String>,
        approval_request_id: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            created_at: created_at.into(),
            zone: DEFAULT_ZONE.to_string(),
            runtime_mode: RuntimeMode::Safe,
            approval_request_id: approval_request_id.into(),
            test_run_id: None,
            commit_run_id: None,
            title_override: None,
        }
    }

    pub fn with_runtime_mode(mut self, runtime_mode: RuntimeMode) -> Self {
        self.runtime_mode = runtime_mode;
        self
    }

    pub fn with_test_run_id(mut self, test_run_id: Option<String>) -> Self {
        self.test_run_id = test_run_id;
        self
    }

    pub fn with_commit_run_id(mut self, commit_run_id: Option<String>) -> Self {
        self.commit_run_id = commit_run_id;
        self
    }

    pub fn with_title_override(mut self, title_override: Option<String>) -> Self {
        self.title_override = title_override;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitPrBodyReport {
    pub run_id: String,
    pub approval_request_id: String,
    pub title: String,
    pub body_artifact_path: PathBuf,
    pub evidence_bundle_path: PathBuf,
    pub ledger_path: PathBuf,
    pub ledger_events_appended: usize,
}

#[derive(Debug, Default)]
struct GitPrBodyState {
    ledger_events_appended: usize,
    next_event_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PatchSummary {
    files: Vec<String>,
    additions: usize,
    deletions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferencedEvidence {
    run_id: String,
    gadget: String,
    status: String,
    summary: String,
    bundle_path: PathBuf,
    extra: Vec<String>,
}

pub fn run_git_pr_body(
    project_root: &Path,
    gadget: &GadgetManifest,
    request: GitPrBodyRequest,
) -> Result<GitPrBodyReport, GitPrBodyError> {
    if !project_root.exists() || !project_root.is_dir() {
        return Err(GitPrBodyError::InvalidProjectRoot(
            project_root.to_path_buf(),
        ));
    }

    validate_optional_run_id(request.test_run_id.as_deref())?;
    validate_optional_run_id(request.commit_run_id.as_deref())?;
    validate_optional_title(request.title_override.as_deref())?;

    let project_root = project_root.canonicalize()?;
    let ledger_path = default_ledger_path(&project_root);
    let runs_root = default_runs_root(&project_root);
    let mut state = GitPrBodyState::default();

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.pr.body.requested",
        "user",
        "cli",
        Some(("approval", &request.approval_request_id)),
        "allowed",
        "Local PR body generation requested through explicit CLI command.",
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
            "Approval verification failed before PR body generation.",
        )?;
        return Err(GitPrBodyError::ApprovalNotVerified(verification.errors));
    }

    let approval =
        read_approval(&project_root, &request.approval_request_id)?.ok_or_else(|| {
            GitPrBodyError::ApprovalRecordMissing(request.approval_request_id.clone())
        })?;
    if approval.status != "approved" {
        return Err(GitPrBodyError::ApprovalNotVerified(vec![format!(
            "approval status is {}",
            approval.status
        )]));
    }
    let approval_request = read_request(&project_root, &request.approval_request_id)?;
    validate_request_for_pr_body(&approval_request)?;
    if approval.scope_hash != approval_request.scope_hash {
        return Err(GitPrBodyError::ApprovalNotVerified(vec![
            "approval scope hash does not match request scope hash".to_string(),
        ]));
    }

    let action = ActionRequest {
        action_request_id: format!("actreq_{}_1", request.run_id),
        run_id: request.run_id.clone(),
        requested_by_gadget: gadget.metadata.name.clone(),
        capability: CapabilityName::new(PR_BODY_CAPABILITY)
            .map_err(|err| GitPrBodyError::Capability(err.to_string()))?,
        tool: PR_BODY_TOOL.to_string(),
        target: ActionTarget {
            zone: Some(request.zone.clone()),
            path: Some(".".to_string()),
            resource: Some(format!("approval:{}", request.approval_request_id)),
        },
        reason: "Generate local pull request body text from verified approval and evidence."
            .to_string(),
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
        return Err(GitPrBodyError::PolicyDenied(evaluation.decision.reason));
    }

    let patch_text = fs::read_to_string(&verification.patch_path)?;
    let patch_summary = summarize_patch(&patch_text)?;
    let test_evidence = read_optional_evidence(&runs_root, request.test_run_id.as_deref())?;
    let commit_evidence = read_optional_evidence(&runs_root, request.commit_run_id.as_deref())?;
    let title = request
        .title_override
        .clone()
        .unwrap_or_else(|| default_title(&approval_request, &patch_summary));
    let pr_body = build_pr_body(
        &request,
        &approval_request,
        &approval,
        &patch_summary,
        test_evidence.as_ref(),
        commit_evidence.as_ref(),
    );
    let pr_body = truncate_body(&pr_body);

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.pr.body.generated",
        "gadget",
        &gadget.metadata.name,
        Some(("approval", &request.approval_request_id)),
        "allowed",
        "Local PR body Markdown generated as evidence only; no remote provider was called.",
    )?;

    let mut evidence_request = EvidenceWriteRequest::observe(
        request.run_id.clone(),
        gadget.metadata.name.clone(),
        request.created_at.clone(),
        build_summary(
            &request,
            &title,
            &patch_summary,
            test_evidence.as_ref(),
            commit_evidence.as_ref(),
        ),
    );
    evidence_request.status = "completed".to_string();
    evidence_request.assumptions = build_assumptions();
    evidence_request.extra_artifacts = vec![
        EvidenceTextArtifact::new("pr_title", "pr_title.txt", format!("{}\n", title)),
        EvidenceTextArtifact::new("pr_body", "pr_body.md", pr_body),
        EvidenceTextArtifact::new(
            "approval_verification",
            "approval_verification.txt",
            format_approval_verification(&request, &approval_request, &approval),
        ),
        EvidenceTextArtifact::new(
            "patch_summary",
            "patch_summary.txt",
            format_patch_summary(&patch_summary),
        ),
        EvidenceTextArtifact::new(
            "test_evidence",
            "test_evidence.txt",
            format_optional_evidence(test_evidence.as_ref()),
        ),
        EvidenceTextArtifact::new(
            "commit_evidence",
            "commit_evidence.txt",
            format_optional_evidence(commit_evidence.as_ref()),
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
    let body_artifact_path = evidence_report.evidence_dir.join("pr_body.md");
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
        "PR body evidence bundle created.",
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
        "PR body generation completed without remote PR creation, push, fetch, shell, provider, patch apply, test run, or admin action.",
    )?;

    Ok(GitPrBodyReport {
        run_id: request.run_id,
        approval_request_id: request.approval_request_id,
        title,
        body_artifact_path,
        evidence_bundle_path: evidence_report.bundle_path,
        ledger_path,
        ledger_events_appended: state.ledger_events_appended,
    })
}

fn validate_request_for_pr_body(request: &ApprovalRequestRecord) -> Result<(), GitPrBodyError> {
    if request.action_kind != "repo.patch.apply" {
        return Err(GitPrBodyError::ApprovalNotVerified(vec![format!(
            "approval action kind is {}, expected repo.patch.apply",
            request.action_kind
        )]));
    }
    if request.executor_gadget != "patch.writer" {
        return Err(GitPrBodyError::ApprovalNotVerified(vec![format!(
            "approval executor is {}, expected patch.writer",
            request.executor_gadget
        )]));
    }
    if request.target.zone != DEFAULT_ZONE {
        return Err(GitPrBodyError::ApprovalNotVerified(vec![format!(
            "approval zone is {}, expected {DEFAULT_ZONE}",
            request.target.zone
        )]));
    }
    Ok(())
}

fn validate_optional_run_id(value: Option<&str>) -> Result<(), GitPrBodyError> {
    if let Some(value) = value {
        if value.is_empty()
            || value.trim() != value
            || !value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return Err(GitPrBodyError::InvalidRunId(value.to_string()));
        }
    }
    Ok(())
}

fn validate_optional_title(value: Option<&str>) -> Result<(), GitPrBodyError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.trim() != value || value.is_empty() {
        return Err(GitPrBodyError::InvalidTitle(
            "title must be non-empty and must not contain leading or trailing whitespace"
                .to_string(),
        ));
    }
    if value.len() > MAX_TITLE_BYTES {
        return Err(GitPrBodyError::InvalidTitle(format!(
            "title exceeds {MAX_TITLE_BYTES} bytes"
        )));
    }
    if !value.is_ascii() || value.chars().any(|c| c.is_control()) {
        return Err(GitPrBodyError::InvalidTitle(
            "title must be printable ASCII on one line".to_string(),
        ));
    }
    Ok(())
}

fn summarize_patch(patch_text: &str) -> Result<PatchSummary, GitPrBodyError> {
    let mut files = BTreeSet::new();
    let mut additions = 0usize;
    let mut deletions = 0usize;
    for line in patch_text.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            let raw_path = path.split_whitespace().next().unwrap_or(path);
            if raw_path == "/dev/null" {
                return Err(GitPrBodyError::InvalidPatch(
                    "file deletion patches are not supported for PR body generation".to_string(),
                ));
            }
            let without_prefix = raw_path
                .strip_prefix("b/")
                .or_else(|| raw_path.strip_prefix("a/"))
                .unwrap_or(raw_path);
            let safe = normalize_safe_relative_path(without_prefix)?;
            files.insert(safe);
        } else if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }
    if files.is_empty() {
        return Err(GitPrBodyError::InvalidPatch(
            "patch contains no +++ file paths".to_string(),
        ));
    }
    Ok(PatchSummary {
        files: files.into_iter().collect(),
        additions,
        deletions,
    })
}

fn normalize_safe_relative_path(input: &str) -> Result<String, GitPrBodyError> {
    let path = Path::new(input);
    if path.is_absolute() {
        return Err(GitPrBodyError::InvalidPath(input.to_string()));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(GitPrBodyError::InvalidPath(input.to_string()))
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(GitPrBodyError::InvalidPath(input.to_string()));
    }
    let value = normalized.to_string_lossy().replace('\\', "/");
    let lower = value.to_ascii_lowercase();
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
        return Err(GitPrBodyError::InvalidPath(value));
    }
    Ok(value)
}

fn read_optional_evidence(
    runs_root: &Path,
    run_id: Option<&str>,
) -> Result<Option<ReferencedEvidence>, GitPrBodyError> {
    let Some(run_id) = run_id else {
        return Ok(None);
    };
    let bundle_path = bundle_path_for_run(runs_root, run_id)?;
    let bundle = read_bundle(&bundle_path)?;
    let summary = summarize_bundle(&bundle_path)?;
    let evidence_dir = bundle_path.parent().unwrap_or_else(|| Path::new("."));
    let mut extra = Vec::new();
    for file_name in ["exit_status.txt", "commit_hash.txt", "current_branch.txt"] {
        let path = evidence_dir.join(file_name);
        if path.exists() {
            let text = fs::read_to_string(path)?;
            extra.push(format!("{file_name}: {}", one_line(&text)));
        }
    }
    Ok(Some(ReferencedEvidence {
        run_id: bundle.run_id,
        gadget: bundle.gadget,
        status: summary.status,
        summary: summary.summary,
        bundle_path,
        extra,
    }))
}

fn default_title(request: &ApprovalRequestRecord, patch: &PatchSummary) -> String {
    let file_count = patch.files.len();
    let run_id = request.target.run_id.as_str();
    format!("Apply approved Gadgets patch {run_id} ({file_count} files)")
}

fn build_pr_body(
    request: &GitPrBodyRequest,
    approval_request: &ApprovalRequestRecord,
    approval: &ApprovalRecord,
    patch: &PatchSummary,
    test_evidence: Option<&ReferencedEvidence>,
    commit_evidence: Option<&ReferencedEvidence>,
) -> String {
    let mut out = String::new();
    out.push_str("## Summary\n\n");
    out.push_str("This PR applies a Gadgets-approved local patch. The PR body was generated locally from verified approval and evidence artifacts.\n\n");
    out.push_str("## Scope\n\n");
    out.push_str(&format!(
        "- Approval request: `{}`\n",
        request.approval_request_id
    ));
    out.push_str(&format!(
        "- Source patch run: `{}`\n",
        approval_request.target.run_id
    ));
    out.push_str(&format!("- Approved by: `{}`\n", approval.approved_by));
    out.push_str(&format!("- Scope hash: `{}`\n", approval.scope_hash));
    out.push_str(&format!(
        "- Patch SHA-256: `{}`\n",
        approval_request.target.patch_sha256
    ));
    out.push_str(&format!("- Files changed: {}\n", patch.files.len()));
    out.push_str(&format!("- Approximate additions: {}\n", patch.additions));
    out.push_str(&format!("- Approximate deletions: {}\n\n", patch.deletions));
    out.push_str("### Files\n\n");
    for file in &patch.files {
        out.push_str(&format!("- `{file}`\n"));
    }
    out.push_str("\n## Validation\n\n");
    match test_evidence {
        Some(evidence) => {
            out.push_str(&format!("- Test run: `{}`\n", evidence.run_id));
            out.push_str(&format!("- Test status: `{}`\n", evidence.status));
            out.push_str(&format!("- Test gadget: `{}`\n", evidence.gadget));
            for line in &evidence.extra {
                out.push_str(&format!("- {line}\n"));
            }
        }
        None => out.push_str("- Test run: not provided to PR body generator.\n"),
    }
    out.push_str("\n## Commit Evidence\n\n");
    match commit_evidence {
        Some(evidence) => {
            out.push_str(&format!("- Commit run: `{}`\n", evidence.run_id));
            out.push_str(&format!("- Commit status: `{}`\n", evidence.status));
            out.push_str(&format!("- Commit gadget: `{}`\n", evidence.gadget));
            for line in &evidence.extra {
                out.push_str(&format!("- {line}\n"));
            }
        }
        None => out.push_str("- Commit run: not provided to PR body generator.\n"),
    }
    out.push_str("\n## Risk Notes\n\n");
    out.push_str("- This body does not create or update a remote PR.\n");
    out.push_str("- No remote Git provider, push, fetch, shell, provider SDK tool call, Linux admin, database, cloud, or deployment action was executed.\n");
    out.push_str("- Reviewers should inspect the patch evidence, approval record, test output, and local commit evidence before creating a remote PR.\n\n");
    out.push_str("## Evidence References\n\n");
    out.push_str(&format!(
        "- Approval request evidence: `.gadgets/approvals/{}/request.yaml`\n",
        request.approval_request_id
    ));
    out.push_str(&format!(
        "- Approval record: `.gadgets/approvals/{}/approval.yaml`\n",
        request.approval_request_id
    ));
    out.push_str(&format!(
        "- Patch evidence bundle: `.gadgets/runs/{}/evidence/bundle.yaml`\n",
        approval_request.target.run_id
    ));
    if let Some(evidence) = test_evidence {
        out.push_str(&format!(
            "- Test evidence bundle: `{}`\n",
            evidence.bundle_path.display()
        ));
    }
    if let Some(evidence) = commit_evidence {
        out.push_str(&format!(
            "- Commit evidence bundle: `{}`\n",
            evidence.bundle_path.display()
        ));
    }
    out.push_str("\n---\n\nGenerated by Gadgets Framework local PR body generator.\n");
    out
}

fn build_summary(
    request: &GitPrBodyRequest,
    title: &str,
    patch: &PatchSummary,
    test_evidence: Option<&ReferencedEvidence>,
    commit_evidence: Option<&ReferencedEvidence>,
) -> String {
    let mut out = String::new();
    out.push_str("Local PR body generated.\n\n");
    out.push_str(&format!(
        "Approval request: {}\n",
        request.approval_request_id
    ));
    out.push_str(&format!("Title: {}\n", title));
    out.push_str(&format!("Files summarized: {}\n", patch.files.len()));
    out.push_str(&format!(
        "Test evidence: {}\n",
        evidence_label(test_evidence)
    ));
    out.push_str(&format!(
        "Commit evidence: {}\n",
        evidence_label(commit_evidence)
    ));
    out.push_str("Remote PR creation: false\n");
    out.push_str("Remote provider call: false\n");
    out
}

fn evidence_label(evidence: Option<&ReferencedEvidence>) -> String {
    evidence
        .map(|item| item.run_id.clone())
        .unwrap_or_else(|| "not provided".to_string())
}

fn build_assumptions() -> Vec<String> {
    vec![
        "The PR body generator uses verified local approval and evidence artifacts only.".to_string(),
        "Optional test and commit evidence run IDs are references; the generator does not run tests or Git commands.".to_string(),
        "No remote Git provider API, push, fetch, shell, provider-side tool, Linux admin, database, cloud, or deployment action was attempted.".to_string(),
        "The generated PR body is reviewable Markdown evidence, not a remote pull request.".to_string(),
    ]
}

fn format_approval_verification(
    request: &GitPrBodyRequest,
    approval_request: &ApprovalRequestRecord,
    approval: &ApprovalRecord,
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
    out.push_str("pr_body_scope=local_markdown_only\n");
    out
}

fn format_patch_summary(patch: &PatchSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!("files={}\n", patch.files.len()));
    out.push_str(&format!("additions={}\n", patch.additions));
    out.push_str(&format!("deletions={}\n", patch.deletions));
    out.push_str("paths=\n");
    for file in &patch.files {
        out.push_str(file);
        out.push('\n');
    }
    out
}

fn format_optional_evidence(evidence: Option<&ReferencedEvidence>) -> String {
    let Some(evidence) = evidence else {
        return "provided=false\n".to_string();
    };
    let mut out = String::new();
    out.push_str("provided=true\n");
    out.push_str(&format!("run_id={}\n", evidence.run_id));
    out.push_str(&format!("gadget={}\n", evidence.gadget));
    out.push_str(&format!("status={}\n", evidence.status));
    out.push_str(&format!("bundle={}\n", evidence.bundle_path.display()));
    out.push_str(&format!("summary={}\n", one_line(&evidence.summary)));
    for line in &evidence.extra {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn one_line(input: &str) -> String {
    redact_one_line(input, "[redacted secret-like evidence summary]", 240)
}

fn truncate_body(input: &str) -> String {
    sanitize_text(
        input,
        RedactionConfig {
            max_bytes: MAX_BODY_BYTES,
            redacted_line: "[redacted secret-like PR body line]",
            truncated_notice: "\n\n[PR body truncated by Gadgets local PR body generator]\n",
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn append_audit(
    ledger_path: &Path,
    request: &GitPrBodyRequest,
    state: &mut GitPrBodyState,
    event_type: &str,
    actor_kind: &str,
    actor_id: &str,
    target: Option<(&str, &str)>,
    decision: &str,
    summary: &str,
) -> Result<(), GitPrBodyError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_patch_paths_and_counts() {
        let patch = "diff --git a/docs/a.md b/docs/a.md\n--- a/docs/a.md\n+++ b/docs/a.md\n@@ -1 +1,2 @@\n-old\n+new\n+line\n";
        let summary = summarize_patch(patch).unwrap();
        assert_eq!(summary.files, vec!["docs/a.md".to_string()]);
        assert_eq!(summary.additions, 2);
        assert_eq!(summary.deletions, 1);
    }

    #[test]
    fn rejects_parent_patch_paths() {
        let patch =
            "diff --git a/../bad b/../bad\n--- a/../bad\n+++ b/../bad\n@@ -0,0 +1 @@\n+bad\n";
        assert!(summarize_patch(patch).is_err());
    }

    #[test]
    fn validates_title() {
        assert!(validate_optional_title(Some("Apply approved patch")).is_ok());
        assert!(validate_optional_title(Some("bad\nmessage")).is_err());
        assert!(validate_optional_title(Some("")).is_err());
    }
}
