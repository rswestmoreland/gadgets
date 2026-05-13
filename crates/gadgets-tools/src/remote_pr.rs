//! Guarded remote pull request creation provider.
//!
//! This module creates a remote pull request only when remote PR creation is
//! explicitly enabled in local config and the request is tied to a verified
//! approval plus a local PR body evidence bundle. It does not push, pull,
//! fetch, merge, rebase, checkout, switch, run shell commands, call model
//! providers, run tests, apply patches, or perform host/server administration.

use crate::redaction::{sanitize_text, RedactionConfig};
use gadgets_approval::{
    read_approval, read_request, verify_approval, ApprovalError, ApprovalRequestRecord,
};
use gadgets_core::{ActionRequest, ActionTarget, CapabilityName, DecisionKind, GadgetManifest};
use gadgets_evidence::{
    bundle_path_for_run, create_observe_bundle, default_runs_root, read_bundle, EvidenceError,
    EvidenceTextArtifact, EvidenceWriteRequest,
};
use gadgets_ledger::{
    append_event, default_ledger_path, new_audit_event, with_target, LedgerError,
};
use gadgets_policy::{evaluate_action, PolicyContext, RuntimeMode};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_ZONE: &str = "local_repo";
const REMOTE_PR_CAPABILITY: &str = "git.pr.create";
const REMOTE_PR_TOOL: &str = "git.pr.create";
const MAX_TITLE_BYTES: usize = 160;
const MAX_BRANCH_BYTES: usize = 160;
const MAX_BODY_BYTES: usize = 65_536;
const DEFAULT_GITHUB_API_BASE: &str = "https://api.github.com";

#[derive(Debug)]
pub enum GitRemotePrError {
    Io(std::io::Error),
    Approval(ApprovalError),
    Ledger(LedgerError),
    Evidence(EvidenceError),
    Capability(String),
    PolicyDenied(String),
    ApprovalNotVerified(Vec<String>),
    ApprovalRecordMissing(String),
    InvalidProjectRoot(PathBuf),
    InvalidConfig(String),
    InvalidRunId(String),
    InvalidTitle(String),
    InvalidBranch(String),
    DuplicatePullRequest(String),
    MissingPrBody(String),
    MissingTokenEnv(String),
    Http(String),
    InvalidResponse(String),
}

impl fmt::Display for GitRemotePrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "remote PR I/O error: {err}"),
            Self::Approval(err) => write!(f, "remote PR approval error: {err}"),
            Self::Ledger(err) => write!(f, "remote PR ledger error: {err}"),
            Self::Evidence(err) => write!(f, "remote PR evidence error: {err}"),
            Self::Capability(err) => write!(f, "remote PR capability error: {err}"),
            Self::PolicyDenied(reason) => write!(f, "remote PR denied by policy: {reason}"),
            Self::ApprovalNotVerified(errors) => {
                write!(f, "approval verification failed: {}", errors.join("; "))
            }
            Self::ApprovalRecordMissing(id) => write!(f, "approval record missing for {id}"),
            Self::InvalidProjectRoot(path) => write!(f, "invalid project root: {}", path.display()),
            Self::InvalidConfig(reason) => write!(f, "invalid remote PR config: {reason}"),
            Self::InvalidRunId(value) => write!(f, "invalid PR body run id: {value}"),
            Self::InvalidTitle(reason) => write!(f, "invalid remote PR title: {reason}"),
            Self::InvalidBranch(reason) => write!(f, "invalid remote PR branch: {reason}"),
            Self::DuplicatePullRequest(reason) => write!(f, "duplicate remote PR rejected: {reason}"),
            Self::MissingPrBody(run_id) => write!(
                f,
                "PR body evidence is missing required artifacts for run {run_id}"
            ),
            Self::MissingTokenEnv(name) => {
                write!(f, "remote PR token environment variable is not set: {name}")
            }
            Self::Http(reason) => write!(f, "remote PR HTTP request failed: {reason}"),
            Self::InvalidResponse(reason) => write!(f, "remote PR response was invalid: {reason}"),
        }
    }
}

impl Error for GitRemotePrError {}

impl From<std::io::Error> for GitRemotePrError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ApprovalError> for GitRemotePrError {
    fn from(value: ApprovalError) -> Self {
        Self::Approval(value)
    }
}

impl From<LedgerError> for GitRemotePrError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<EvidenceError> for GitRemotePrError {
    fn from(value: EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePrProviderConfig {
    pub enabled: bool,
    pub dry_run: bool,
    pub provider: String,
    pub owner: String,
    pub repo: String,
    pub api_base: String,
    pub token_env: String,
    pub default_base_branch: String,
    pub allowed_base_branches: Vec<String>,
    pub allowed_head_prefixes: Vec<String>,
    pub duplicate_strategy: String,
}

impl RemotePrProviderConfig {
    pub fn github_disabled() -> Self {
        Self {
            enabled: false,
            dry_run: true,
            provider: "github".to_string(),
            owner: String::new(),
            repo: String::new(),
            api_base: DEFAULT_GITHUB_API_BASE.to_string(),
            token_env: "GITHUB_TOKEN".to_string(),
            default_base_branch: "main".to_string(),
            allowed_base_branches: vec!["main".to_string()],
            allowed_head_prefixes: vec![
                "feature/".to_string(),
                "fix/".to_string(),
                "docs/".to_string(),
            ],
            duplicate_strategy: "fail".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRemotePrRequest {
    pub run_id: String,
    pub created_at: String,
    pub zone: String,
    pub runtime_mode: RuntimeMode,
    pub approval_request_id: String,
    pub pr_body_run_id: String,
    pub head_branch: String,
    pub base_branch: String,
    pub title_override: Option<String>,
    pub config: RemotePrProviderConfig,
}

impl GitRemotePrRequest {
    pub fn create_remote_pr(
        run_id: impl Into<String>,
        created_at: impl Into<String>,
        approval_request_id: impl Into<String>,
        pr_body_run_id: impl Into<String>,
        head_branch: impl Into<String>,
        base_branch: impl Into<String>,
        config: RemotePrProviderConfig,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            created_at: created_at.into(),
            zone: DEFAULT_ZONE.to_string(),
            runtime_mode: RuntimeMode::Safe,
            approval_request_id: approval_request_id.into(),
            pr_body_run_id: pr_body_run_id.into(),
            head_branch: head_branch.into(),
            base_branch: base_branch.into(),
            title_override: None,
            config,
        }
    }

    pub fn with_runtime_mode(mut self, runtime_mode: RuntimeMode) -> Self {
        self.runtime_mode = runtime_mode;
        self
    }

    pub fn with_title_override(mut self, title_override: Option<String>) -> Self {
        self.title_override = title_override;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRemotePrReport {
    pub run_id: String,
    pub approval_request_id: String,
    pub pr_body_run_id: String,
    pub provider: String,
    pub repository: String,
    pub head_branch: String,
    pub base_branch: String,
    pub title: String,
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>,
    pub passed: bool,
    pub dry_run: bool,
    pub duplicate_found: bool,
    pub http_status: Option<u16>,
    pub duration_ms: u128,
    pub evidence_bundle_path: PathBuf,
    pub ledger_path: PathBuf,
    pub ledger_events_appended: usize,
}

#[derive(Debug, Default)]
struct GitRemotePrState {
    ledger_events_appended: usize,
    next_event_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrBodyEvidence {
    title: String,
    body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemotePrCapture {
    passed: bool,
    dry_run: bool,
    duplicate_found: bool,
    http_status: Option<u16>,
    response_text: String,
    pr_number: Option<u64>,
    pr_url: Option<String>,
    duration_ms: u128,
}

pub fn run_git_remote_pr_create(
    project_root: &Path,
    gadget: &GadgetManifest,
    request: GitRemotePrRequest,
) -> Result<GitRemotePrReport, GitRemotePrError> {
    if !project_root.exists() || !project_root.is_dir() {
        return Err(GitRemotePrError::InvalidProjectRoot(
            project_root.to_path_buf(),
        ));
    }

    validate_config(&request.config)?;
    validate_run_id(&request.pr_body_run_id)?;
    validate_branch(&request.head_branch, "head")?;
    validate_branch(&request.base_branch, "base")?;
    validate_branch_rules(&request)?;
    validate_optional_title(request.title_override.as_deref())?;
    if request.head_branch == request.base_branch {
        return Err(GitRemotePrError::InvalidBranch(
            "head branch and base branch must differ".to_string(),
        ));
    }

    let project_root = project_root.canonicalize()?;
    let ledger_path = default_ledger_path(&project_root);
    let runs_root = default_runs_root(&project_root);
    let mut state = GitRemotePrState::default();

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.pr.create.requested",
        "user",
        "cli",
        Some(("approval", &request.approval_request_id)),
        "allowed",
        "Remote PR creation requested through explicit CLI command and enabled config.",
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
            "Approval verification failed before remote PR creation.",
        )?;
        return Err(GitRemotePrError::ApprovalNotVerified(verification.errors));
    }

    let approval =
        read_approval(&project_root, &request.approval_request_id)?.ok_or_else(|| {
            GitRemotePrError::ApprovalRecordMissing(request.approval_request_id.clone())
        })?;
    if approval.status != "approved" {
        return Err(GitRemotePrError::ApprovalNotVerified(vec![format!(
            "approval status is {}",
            approval.status
        )]));
    }
    let approval_request = read_request(&project_root, &request.approval_request_id)?;
    validate_request_for_remote_pr(&approval_request)?;
    if approval.scope_hash != approval_request.scope_hash {
        return Err(GitRemotePrError::ApprovalNotVerified(vec![
            "approval scope hash does not match request scope hash".to_string(),
        ]));
    }

    let pr_body = read_pr_body_evidence(&runs_root, &request.pr_body_run_id)?;
    let title = request
        .title_override
        .clone()
        .unwrap_or_else(|| pr_body.title.clone());
    validate_optional_title(Some(&title))?;

    let action = ActionRequest {
        action_request_id: format!("actreq_{}_1", request.run_id),
        run_id: request.run_id.clone(),
        requested_by_gadget: gadget.metadata.name.clone(),
        capability: CapabilityName::new(REMOTE_PR_CAPABILITY)
            .map_err(|err| GitRemotePrError::Capability(err.to_string()))?,
        tool: REMOTE_PR_TOOL.to_string(),
        target: ActionTarget {
            zone: Some(request.zone.clone()),
            path: Some(".".to_string()),
            resource: Some(format!(
                "repo:{}/{} head:{} base:{}",
                request.config.owner, request.config.repo, request.head_branch, request.base_branch
            )),
        },
        reason: "Create one remote pull request from verified approval and local PR body evidence."
            .to_string(),
    };
    let context = PolicyContext {
        mode: request.runtime_mode,
        approval_present: true,
        allowlisted_test_command: false,
        allowlisted_git_branch_create: false,
        approved_git_commit: false,
        approved_git_pr_create: true,
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
        return Err(GitRemotePrError::PolicyDenied(evaluation.decision.reason));
    }

    let repo_target = format!("{}/{}", request.config.owner, request.config.repo);
    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "git.pr.create.started",
        "gadget",
        &gadget.metadata.name,
        Some(("repo", repo_target.as_str())),
        "allowed",
        if request.config.dry_run {
            "Remote PR dry run started. No GitHub API mutation will run."
        } else {
            "Fixed GitHub API pull request creation started after duplicate-open-PR check. No Git push, fetch, pull, merge, rebase, shell, provider, or admin command will run."
        },
    )?;

    let capture = if request.config.dry_run {
        create_dry_run_capture(&request, &title)
    } else {
        let token = read_token(&request.config.token_env)?;
        if let Some(existing) = find_existing_github_pr(&request, &token)? {
            duplicate_pr_capture(&request, existing)
        } else {
            create_github_pr(&request, &title, &pr_body.body, &token)?
        }
    };

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        if capture.passed {
            if capture.dry_run {
                "git.pr.create.dry_run_completed"
            } else if capture.duplicate_found {
                "git.pr.create.duplicate_reused"
            } else {
                "git.pr.create.completed"
            }
        } else {
            "git.pr.create.failed"
        },
        "gadget",
        &gadget.metadata.name,
        Some(("repo", repo_target.as_str())),
        if capture.passed { "allowed" } else { "failed" },
        if capture.dry_run {
            "Remote pull request dry run completed without GitHub API mutation."
        } else if capture.duplicate_found && capture.passed {
            "Existing open remote pull request was reused according to duplicate strategy."
        } else if capture.duplicate_found {
            "Existing open remote pull request was found and rejected according to duplicate strategy."
        } else if capture.passed {
            "Remote pull request creation completed through configured GitHub API endpoint."
        } else {
            "Remote pull request creation returned an unsuccessful HTTP response."
        },
    )?;

    let mut evidence_request = EvidenceWriteRequest::observe(
        request.run_id.clone(),
        gadget.metadata.name.clone(),
        request.created_at.clone(),
        build_summary(&request, &title, &capture),
    );
    evidence_request.status = if capture.passed {
        "completed".to_string()
    } else {
        "failed".to_string()
    };
    evidence_request.assumptions = build_assumptions();
    evidence_request.extra_artifacts = vec![
        EvidenceTextArtifact::new(
            "remote_pr_request",
            "remote_pr_request.txt",
            format_remote_request(&request, &title),
        ),
        EvidenceTextArtifact::new(
            "approval_verification",
            "approval_verification.txt",
            format_approval_verification(&request, &approval_request, &approval),
        ),
        EvidenceTextArtifact::new(
            "pr_body_reference",
            "pr_body_reference.txt",
            format_pr_body_reference(&request, &pr_body),
        ),
        EvidenceTextArtifact::new(
            "http_status",
            "http_status.txt",
            format_http_status(&capture),
        ),
        EvidenceTextArtifact::new(
            "remote_pr_response",
            "remote_pr_response.txt",
            sanitize_response(&capture.response_text),
        ),
        EvidenceTextArtifact::new(
            "remote_pr_url",
            "remote_pr_url.txt",
            format!(
                "{}\n",
                capture.pr_url.clone().unwrap_or_else(|| "none".to_string())
            ),
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
        "Remote PR creation evidence bundle created.",
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
        "Remote PR creation run completed without Git push, pull, fetch, merge, rebase, shell, provider tool, patch apply, tests, or admin actions.",
    )?;

    Ok(GitRemotePrReport {
        run_id: request.run_id,
        approval_request_id: request.approval_request_id,
        pr_body_run_id: request.pr_body_run_id,
        provider: request.config.provider,
        repository: format!("{}/{}", request.config.owner, request.config.repo),
        head_branch: request.head_branch,
        base_branch: request.base_branch,
        title,
        pr_number: capture.pr_number,
        pr_url: capture.pr_url,
        passed: capture.passed,
        dry_run: capture.dry_run,
        duplicate_found: capture.duplicate_found,
        http_status: capture.http_status,
        duration_ms: capture.duration_ms,
        evidence_bundle_path: evidence_report.bundle_path,
        ledger_path,
        ledger_events_appended: state.ledger_events_appended,
    })
}

fn validate_config(config: &RemotePrProviderConfig) -> Result<(), GitRemotePrError> {
    if !config.enabled {
        return Err(GitRemotePrError::InvalidConfig(
            "remote_pr.enabled must be true before remote PR creation is allowed".to_string(),
        ));
    }
    if config.provider != "github" {
        return Err(GitRemotePrError::InvalidConfig(
            "only provider: github is supported in this checkpoint".to_string(),
        ));
    }
    validate_repo_component(&config.owner, "owner")?;
    validate_repo_component(&config.repo, "repo")?;
    if !config.api_base.starts_with("https://") || config.api_base.ends_with('/') {
        return Err(GitRemotePrError::InvalidConfig(
            "api_base must be an https URL without a trailing slash".to_string(),
        ));
    }
    if !valid_env_name(&config.token_env) {
        return Err(GitRemotePrError::InvalidConfig(
            "token_env must be a valid environment variable name".to_string(),
        ));
    }
    validate_branch(&config.default_base_branch, "default_base_branch")?;
    if !config
        .allowed_base_branches
        .iter()
        .any(|branch| branch == &config.default_base_branch)
    {
        return Err(GitRemotePrError::InvalidConfig(
            "default_base_branch must be present in allowed_base_branches".to_string(),
        ));
    }
    if config.allowed_base_branches.is_empty() {
        return Err(GitRemotePrError::InvalidConfig(
            "allowed_base_branches must not be empty".to_string(),
        ));
    }
    if config.allowed_head_prefixes.is_empty() {
        return Err(GitRemotePrError::InvalidConfig(
            "allowed_head_prefixes must not be empty".to_string(),
        ));
    }
    for branch in &config.allowed_base_branches {
        validate_branch(branch, "allowed_base_branch")?;
    }
    for prefix in &config.allowed_head_prefixes {
        validate_head_prefix(prefix)?;
    }
    if config.duplicate_strategy != "fail" && config.duplicate_strategy != "reuse" {
        return Err(GitRemotePrError::InvalidConfig(
            "duplicate_strategy must be fail or reuse".to_string(),
        ));
    }
    Ok(())
}

fn validate_branch_rules(request: &GitRemotePrRequest) -> Result<(), GitRemotePrError> {
    if !request
        .config
        .allowed_base_branches
        .iter()
        .any(|branch| branch == &request.base_branch)
    {
        return Err(GitRemotePrError::InvalidBranch(format!(
            "base branch {} is not in remote_pr.allowed_base_branches",
            request.base_branch
        )));
    }
    if !request
        .config
        .allowed_head_prefixes
        .iter()
        .any(|prefix| request.head_branch.starts_with(prefix))
    {
        return Err(GitRemotePrError::InvalidBranch(format!(
            "head branch {} does not match remote_pr.allowed_head_prefixes",
            request.head_branch
        )));
    }
    Ok(())
}

fn validate_head_prefix(value: &str) -> Result<(), GitRemotePrError> {
    if !value.ends_with('/')
        || value.len() <= 1
        || value.starts_with('/')
        || value.contains("//")
        || value
            .trim_end_matches('/')
            .split('/')
            .any(|part| validate_branch(part, "head_prefix").is_err())
    {
        return Err(GitRemotePrError::InvalidConfig(format!(
            "invalid allowed head prefix: {value}"
        )));
    }
    Ok(())
}

fn validate_repo_component(value: &str, name: &str) -> Result<(), GitRemotePrError> {
    if value.is_empty()
        || value.trim() != value
        || value.starts_with('.')
        || value.ends_with('.')
        || value.contains("..")
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(GitRemotePrError::InvalidConfig(format!(
            "{name} must be a non-empty safe GitHub repository component"
        )));
    }
    Ok(())
}

fn valid_env_name(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .chars()
            .enumerate()
            .all(|(idx, c)| c == '_' || c.is_ascii_uppercase() || (idx > 0 && c.is_ascii_digit()))
}

fn validate_run_id(value: &str) -> Result<(), GitRemotePrError> {
    if value.is_empty()
        || value.trim() != value
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(GitRemotePrError::InvalidRunId(value.to_string()));
    }
    Ok(())
}

fn validate_optional_title(value: Option<&str>) -> Result<(), GitRemotePrError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.trim() != value || value.is_empty() {
        return Err(GitRemotePrError::InvalidTitle(
            "title must be non-empty and must not contain leading or trailing whitespace"
                .to_string(),
        ));
    }
    if value.len() > MAX_TITLE_BYTES {
        return Err(GitRemotePrError::InvalidTitle(format!(
            "title exceeds {MAX_TITLE_BYTES} bytes"
        )));
    }
    if !value.is_ascii() || value.chars().any(|c| c.is_control()) {
        return Err(GitRemotePrError::InvalidTitle(
            "title must be printable ASCII on one line".to_string(),
        ));
    }
    Ok(())
}

fn validate_branch(value: &str, label: &str) -> Result<(), GitRemotePrError> {
    if value.is_empty()
        || value.len() > MAX_BRANCH_BYTES
        || value.trim() != value
        || value.starts_with('-')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.ends_with('.')
        || value.ends_with(".lock")
        || value.contains("..")
        || value.contains("//")
        || value.contains("@{")
        || value.contains(':')
        || value.eq_ignore_ascii_case("head")
    {
        return Err(GitRemotePrError::InvalidBranch(format!(
            "{label} branch has invalid shape"
        )));
    }
    if !value.split('/').all(|part| {
        !part.is_empty()
            && !part.starts_with('.')
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    }) {
        return Err(GitRemotePrError::InvalidBranch(format!(
            "{label} branch contains unsupported characters"
        )));
    }
    Ok(())
}

fn validate_request_for_remote_pr(request: &ApprovalRequestRecord) -> Result<(), GitRemotePrError> {
    if request.action_kind != "repo.patch.apply" {
        return Err(GitRemotePrError::ApprovalNotVerified(vec![format!(
            "approval action kind is {}, expected repo.patch.apply",
            request.action_kind
        )]));
    }
    if request.executor_gadget != "patch.writer" {
        return Err(GitRemotePrError::ApprovalNotVerified(vec![format!(
            "approval executor is {}, expected patch.writer",
            request.executor_gadget
        )]));
    }
    if request.target.zone != DEFAULT_ZONE {
        return Err(GitRemotePrError::ApprovalNotVerified(vec![format!(
            "approval zone is {}, expected {DEFAULT_ZONE}",
            request.target.zone
        )]));
    }
    Ok(())
}

fn read_pr_body_evidence(
    runs_root: &Path,
    run_id: &str,
) -> Result<PrBodyEvidence, GitRemotePrError> {
    let bundle_path = bundle_path_for_run(runs_root, run_id)?;
    let bundle = read_bundle(&bundle_path)?;
    if bundle.gadget != "git.pr" || bundle.status != "completed" {
        return Err(GitRemotePrError::MissingPrBody(run_id.to_string()));
    }
    let evidence_dir = bundle_path.parent().unwrap_or_else(|| Path::new("."));
    let title = fs::read_to_string(evidence_dir.join("pr_title.txt"))
        .map_err(|_| GitRemotePrError::MissingPrBody(run_id.to_string()))?;
    let body = fs::read_to_string(evidence_dir.join("pr_body.md"))
        .map_err(|_| GitRemotePrError::MissingPrBody(run_id.to_string()))?;
    let title = title.trim_end_matches('\n').to_string();
    validate_optional_title(Some(&title))?;
    if body.is_empty() || body.len() > MAX_BODY_BYTES {
        return Err(GitRemotePrError::MissingPrBody(run_id.to_string()));
    }
    Ok(PrBodyEvidence { title, body })
}

fn read_token(env_name: &str) -> Result<String, GitRemotePrError> {
    let value =
        env::var(env_name).map_err(|_| GitRemotePrError::MissingTokenEnv(env_name.to_string()))?;
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return Err(GitRemotePrError::MissingTokenEnv(env_name.to_string()));
    }
    Ok(value)
}


#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingPullRequest {
    number: Option<u64>,
    url: Option<String>,
}

fn create_dry_run_capture(request: &GitRemotePrRequest, title: &str) -> RemotePrCapture {
    RemotePrCapture {
        passed: true,
        dry_run: true,
        duplicate_found: false,
        http_status: None,
        response_text: format!(
            "dry_run=true\nprovider=github\nrepository={}/{}\nhead_branch={}\nbase_branch={}\ntitle={}\nmutation=false\n",
            request.config.owner, request.config.repo, request.head_branch, request.base_branch, title
        ),
        pr_number: None,
        pr_url: None,
        duration_ms: 0,
    }
}

fn duplicate_pr_capture(
    request: &GitRemotePrRequest,
    existing: ExistingPullRequest,
) -> Result<RemotePrCapture, GitRemotePrError> {
    let response_text = format!(
        "duplicate_found=true\nduplicate_strategy={}\nexisting_pr_number={}\nexisting_pr_url={}\n",
        request.config.duplicate_strategy,
        existing
            .number
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string()),
        existing.url.as_deref().unwrap_or("none")
    );
    if request.config.duplicate_strategy == "reuse" {
        Ok(RemotePrCapture {
            passed: true,
            dry_run: false,
            duplicate_found: true,
            http_status: Some(200),
            response_text,
            pr_number: existing.number,
            pr_url: existing.url,
            duration_ms: 0,
        })
    } else {
        Ok(RemotePrCapture {
            passed: false,
            dry_run: false,
            duplicate_found: true,
            http_status: Some(200),
            response_text,
            pr_number: existing.number,
            pr_url: existing.url,
            duration_ms: 0,
        })
    }
}

fn find_existing_github_pr(
    request: &GitRemotePrRequest,
    token: &str,
) -> Result<Option<ExistingPullRequest>, GitRemotePrError> {
    let url = format!(
        "{}/repos/{}/{}/pulls?state=open&head={}:{}&base={}",
        request.config.api_base,
        request.config.owner,
        request.config.repo,
        request.config.owner,
        request.head_branch,
        request.base_branch
    );
    let auth_header = format!("Bearer {token}");
    let response = ureq::get(&url)
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", &auth_header)
        .set("User-Agent", "gadgets-framework/0.1")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call();

    match response {
        Ok(resp) => {
            let text = resp
                .into_string()
                .map_err(|err| GitRemotePrError::Http(err.to_string()))?;
            let parsed: Value = serde_json::from_str(&text)
                .map_err(|err| GitRemotePrError::InvalidResponse(err.to_string()))?;
            let Some(items) = parsed.as_array() else {
                return Err(GitRemotePrError::InvalidResponse(
                    "duplicate PR lookup response was not an array".to_string(),
                ));
            };
            let Some(first) = items.first() else {
                return Ok(None);
            };
            Ok(Some(ExistingPullRequest {
                number: first.get("number").and_then(|item| item.as_u64()),
                url: first
                    .get("html_url")
                    .and_then(|item| item.as_str())
                    .map(|item| item.to_string()),
            }))
        }
        Err(ureq::Error::Status(status, resp)) => {
            let text = resp
                .into_string()
                .unwrap_or_else(|_| "<failed to read response body>".to_string());
            Err(GitRemotePrError::Http(format!(
                "duplicate PR lookup failed with HTTP {status}: {}",
                sanitize_response(&text)
            )))
        }
        Err(err) => Err(GitRemotePrError::Http(format!(
            "duplicate PR lookup failed: {err}"
        ))),
    }
}

fn create_github_pr(
    request: &GitRemotePrRequest,
    title: &str,
    body: &str,
    token: &str,
) -> Result<RemotePrCapture, GitRemotePrError> {
    let url = format!(
        "{}/repos/{}/{}/pulls",
        request.config.api_base, request.config.owner, request.config.repo
    );
    let payload = json!({
        "title": title,
        "head": request.head_branch.as_str(),
        "base": request.base_branch.as_str(),
        "body": body,
        "maintainer_can_modify": false,
        "draft": false,
    });
    let start = Instant::now();
    let auth_header = format!("Bearer {token}");
    let response = ureq::post(&url)
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", &auth_header)
        .set("User-Agent", "gadgets-framework/0.1")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .send_json(payload);
    let duration_ms = start.elapsed().as_millis();

    match response {
        Ok(resp) => {
            let status = resp.status();
            let text = resp
                .into_string()
                .map_err(|err| GitRemotePrError::Http(err.to_string()))?;
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            let pr_number = parsed
                .as_ref()
                .ok()
                .and_then(|value| value.get("number"))
                .and_then(|item| item.as_u64());
            let pr_url = parsed
                .as_ref()
                .ok()
                .and_then(|value| value.get("html_url"))
                .and_then(|item| item.as_str())
                .map(|item| item.to_string());
            Ok(RemotePrCapture {
                passed: (200..300).contains(&status),
                dry_run: false,
                duplicate_found: false,
                http_status: Some(status),
                response_text: text,
                pr_number,
                pr_url,
                duration_ms,
            })
        }
        Err(ureq::Error::Status(status, resp)) => {
            let text = resp
                .into_string()
                .unwrap_or_else(|_| "<failed to read response body>".to_string());
            Ok(RemotePrCapture {
                passed: false,
                dry_run: false,
                duplicate_found: false,
                http_status: Some(status),
                response_text: text,
                pr_number: None,
                pr_url: None,
                duration_ms,
            })
        }
        Err(err) => Err(GitRemotePrError::Http(err.to_string())),
    }
}

#[allow(clippy::too_many_arguments)]
fn append_audit(
    ledger_path: &Path,
    request: &GitRemotePrRequest,
    state: &mut GitRemotePrState,
    event_type: &str,
    actor_kind: &str,
    actor_id: &str,
    target: Option<(&str, &str)>,
    decision: &str,
    summary: &str,
) -> Result<(), GitRemotePrError> {
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

fn build_summary(request: &GitRemotePrRequest, title: &str, capture: &RemotePrCapture) -> String {
    let status = if capture.passed {
        "completed"
    } else {
        "failed"
    };
    let mut out = String::new();
    out.push_str("Remote pull request creation completed.\n\n");
    out.push_str(&format!("Status: {status}\n"));
    out.push_str(&format!(
        "Repository: {}/{}\n",
        request.config.owner, request.config.repo
    ));
    out.push_str(&format!("Title: {title}\n"));
    out.push_str(&format!("Head branch: {}\n", request.head_branch));
    out.push_str(&format!("Base branch: {}\n", request.base_branch));
    out.push_str(&format!(
        "HTTP status: {}\n",
        display_http_status(capture.http_status)
    ));
    out.push_str(&format!(
        "PR number: {}\n",
        capture
            .pr_number
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string())
    ));
    out.push_str(&format!(
        "PR URL: {}\n",
        capture.pr_url.as_deref().unwrap_or("none")
    ));
    out.push_str("\nNo Git push, fetch, pull, merge, rebase, checkout, shell, provider tool, patch apply, test, Linux admin, database, cloud, or deployment action was executed by this provider.\n");
    out
}

fn build_assumptions() -> Vec<String> {
    vec![
        "Remote PR creation must be explicitly enabled in .gadgets/config.yaml.".to_string(),
        "Dry-run mode performs all local verification but does not call the GitHub mutation endpoint.".to_string(),
        "The approval request and approval record were verified before any remote API mutation.".to_string(),
        "The PR body came from a completed local PR body evidence bundle.".to_string(),
        "The base branch and head branch were checked against configured allowlists.".to_string(),
        "Duplicate-open-PR handling follows remote_pr.duplicate_strategy.".to_string(),
        "The head branch is assumed to already exist on the remote repository; this provider does not push branches.".to_string(),
        "No provider SDK tool call, shell command, Git push, fetch, pull, merge, rebase, checkout, test command, or admin action was attempted.".to_string(),
        "The remote provider response was redacted before evidence write.".to_string(),
    ]
}

fn format_remote_request(request: &GitRemotePrRequest, title: &str) -> String {
    let mut out = String::new();
    out.push_str("provider=github\n");
    out.push_str(&format!(
        "repository={}/{}\n",
        request.config.owner, request.config.repo
    ));
    out.push_str(&format!("api_base={}\n", request.config.api_base));
    out.push_str(&format!("token_env={}\n", request.config.token_env));
    out.push_str(&format!(
        "approval_request_id={}\n",
        request.approval_request_id
    ));
    out.push_str(&format!("pr_body_run_id={}\n", request.pr_body_run_id));
    out.push_str(&format!("head_branch={}\n", request.head_branch));
    out.push_str(&format!("base_branch={}\n", request.base_branch));
    out.push_str(&format!("title={}\n", title));
    out.push_str("push=false\nfetch=false\npull=false\nmerge=false\nrebase=false\nshell=false\n");
    out
}

fn format_approval_verification(
    request: &GitRemotePrRequest,
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
    out.push_str("remote_pr_scope=verified_approval_and_pr_body_only\n");
    out
}

fn format_pr_body_reference(request: &GitRemotePrRequest, pr_body: &PrBodyEvidence) -> String {
    let mut out = String::new();
    out.push_str(&format!("pr_body_run_id={}\n", request.pr_body_run_id));
    out.push_str(&format!("title={}\n", pr_body.title));
    out.push_str(&format!("body_bytes={}\n", pr_body.body.len()));
    out
}

fn format_http_status(capture: &RemotePrCapture) -> String {
    let mut out = String::new();
    out.push_str(&format!("passed={}\n", capture.passed));
    out.push_str(&format!(
        "http_status={}\n",
        display_http_status(capture.http_status)
    ));
    out.push_str(&format!(
        "pr_number={}\n",
        capture
            .pr_number
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string())
    ));
    out.push_str(&format!(
        "pr_url={}\n",
        capture.pr_url.as_deref().unwrap_or("none")
    ));
    out
}

fn display_http_status(status: Option<u16>) -> String {
    status
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn sanitize_response(input: &str) -> String {
    let limited = input.lines().take(400).collect::<Vec<_>>().join("\n");
    sanitize_text(
        &limited,
        RedactionConfig {
            max_bytes: MAX_BODY_BYTES,
            redacted_line: "<redacted secret-like response line>",
            truncated_notice: "\n<truncated>\n",
        },
    )
}

fn decision_kind_as_str(kind: &DecisionKind) -> &'static str {
    match kind {
        DecisionKind::Allowed => "allowed",
        DecisionKind::Denied => "denied",
        DecisionKind::RequiresApproval => "requires_approval",
    }
}
