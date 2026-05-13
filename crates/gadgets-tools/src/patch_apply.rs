//! Approved Patch Writer apply provider.
//!
//! This module applies a previously proposed patch only after the matching
//! approval request and approval record verify successfully. It does not call
//! model providers, run shell commands, run tests, stage Git changes, commit,
//! open PRs, or perform host/server administration.

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
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

const DEFAULT_ZONE: &str = "local_repo";
const FILE_WRITE_CAPABILITY: &str = "file.write";
const FILE_PATCH_TOOL: &str = "file.patch";

#[derive(Debug)]
pub enum PatchApplyError {
    Io(std::io::Error),
    Approval(ApprovalError),
    Ledger(LedgerError),
    Evidence(EvidenceError),
    Capability(String),
    PolicyDenied(String),
    ApprovalNotVerified(Vec<String>),
    ApprovalRecordMissing(String),
    InvalidPatch(String),
    UnsupportedPatch(String),
    InvalidProjectRoot(PathBuf),
    PathNotSafe(String),
    PatchMismatch(String),
}

impl fmt::Display for PatchApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "patch apply I/O error: {err}"),
            Self::Approval(err) => write!(f, "patch apply approval error: {err}"),
            Self::Ledger(err) => write!(f, "patch apply ledger error: {err}"),
            Self::Evidence(err) => write!(f, "patch apply evidence error: {err}"),
            Self::Capability(err) => write!(f, "patch apply capability error: {err}"),
            Self::PolicyDenied(reason) => write!(f, "patch apply denied by policy: {reason}"),
            Self::ApprovalNotVerified(errors) => {
                write!(f, "approval verification failed: {}", errors.join("; "))
            }
            Self::ApprovalRecordMissing(id) => write!(f, "approval record missing for {id}"),
            Self::InvalidPatch(reason) => write!(f, "invalid patch: {reason}"),
            Self::UnsupportedPatch(reason) => write!(f, "unsupported patch: {reason}"),
            Self::InvalidProjectRoot(path) => write!(f, "invalid project root: {}", path.display()),
            Self::PathNotSafe(path) => write!(f, "patch path is not safe: {path}"),
            Self::PatchMismatch(reason) => write!(f, "patch did not match working tree: {reason}"),
        }
    }
}

impl Error for PatchApplyError {}

impl From<std::io::Error> for PatchApplyError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ApprovalError> for PatchApplyError {
    fn from(value: ApprovalError) -> Self {
        Self::Approval(value)
    }
}

impl From<LedgerError> for PatchApplyError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<EvidenceError> for PatchApplyError {
    fn from(value: EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchApplyRequest {
    pub approval_request_id: String,
    pub apply_run_id: String,
    pub created_at: String,
    pub zone: String,
    pub runtime_mode: RuntimeMode,
}

impl PatchApplyRequest {
    pub fn local_apply(
        approval_request_id: impl Into<String>,
        apply_run_id: impl Into<String>,
        created_at: impl Into<String>,
    ) -> Self {
        Self {
            approval_request_id: approval_request_id.into(),
            apply_run_id: apply_run_id.into(),
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
pub struct PatchApplyReport {
    pub apply_run_id: String,
    pub approval_request_id: String,
    pub source_plan_run_id: String,
    pub files_changed: Vec<String>,
    pub evidence_bundle_path: PathBuf,
    pub ledger_path: PathBuf,
    pub ledger_events_appended: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileChangeHash {
    path: String,
    before_sha256: String,
    after_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedFileChange {
    path: String,
    target_path: PathBuf,
    after_contents: String,
    before_sha256: String,
    after_sha256: String,
}

#[derive(Debug, Default)]
struct ApplyState {
    ledger_events_appended: usize,
    next_event_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PatchFile {
    path: String,
    hunks: Vec<PatchHunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PatchHunk {
    old_start: usize,
    lines: Vec<HunkLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HunkLine {
    Context(String),
    Remove(String),
    Add(String),
}

pub fn run_patch_apply(
    project_root: &Path,
    gadget: &GadgetManifest,
    request: PatchApplyRequest,
) -> Result<PatchApplyReport, PatchApplyError> {
    if !project_root.exists() || !project_root.is_dir() {
        return Err(PatchApplyError::InvalidProjectRoot(
            project_root.to_path_buf(),
        ));
    }

    let project_root = project_root.canonicalize()?;
    let ledger_path = default_ledger_path(&project_root);
    let runs_root = default_runs_root(&project_root);
    let mut state = ApplyState::default();

    append_audit(
        &ledger_path,
        &request,
        &mut state,
        "run.started",
        "gadget",
        "coordinator",
        None,
        "allowed",
        "Approved Patch Writer apply run started.",
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
            "Approval verification failed before patch apply.",
        )?;
        return Err(PatchApplyError::ApprovalNotVerified(verification.errors));
    }

    let approval =
        read_approval(&project_root, &request.approval_request_id)?.ok_or_else(|| {
            PatchApplyError::ApprovalRecordMissing(request.approval_request_id.clone())
        })?;
    if approval.status != "approved" {
        return Err(PatchApplyError::ApprovalNotVerified(vec![format!(
            "approval status is {}",
            approval.status
        )]));
    }

    let approval_request = read_request(&project_root, &request.approval_request_id)?;
    validate_request_for_apply(&approval_request)?;
    if approval.scope_hash != approval_request.scope_hash {
        return Err(PatchApplyError::ApprovalNotVerified(vec![
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
        "Verified approval record will be used for exact patch apply scope.",
    )?;

    let patch_text = fs::read_to_string(&verification.patch_path)?;
    let patch_files = parse_unified_diff(&patch_text)?;
    if patch_files.is_empty() {
        return Err(PatchApplyError::InvalidPatch(
            "patch contains no file changes".to_string(),
        ));
    }

    let mut policy_lines = Vec::new();
    for file in &patch_files {
        let action = ActionRequest {
            action_request_id: format!(
                "actreq_{}_{}",
                request.apply_run_id,
                file.path.replace('/', "_")
            ),
            run_id: request.apply_run_id.clone(),
            requested_by_gadget: gadget.metadata.name.clone(),
            capability: CapabilityName::new(FILE_WRITE_CAPABILITY)
                .map_err(|err| PatchApplyError::Capability(err.to_string()))?,
            tool: FILE_PATCH_TOOL.to_string(),
            target: ActionTarget {
                zone: Some(request.zone.clone()),
                path: Some(file.path.clone()),
                resource: Some("approved_patch".to_string()),
            },
            reason: format!("Apply approved patch to {}", file.path),
        };
        let context = PolicyContext {
            mode: request.runtime_mode,
            approval_present: true,
            allowlisted_test_command: false,
            allowlisted_git_branch_create: false,
            approved_git_commit: false,
            approved_git_pr_create: false,
            decision_id: format!(
                "dec_{}_{}",
                request.apply_run_id,
                file.path.replace('/', "_")
            ),
        };
        let evaluation = evaluate_action(gadget, &action, &context);
        policy_lines.push(format!(
            "{}: {:?} - {}",
            file.path, evaluation.decision.decision, evaluation.decision.reason
        ));
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
            Some(("file", &file.path)),
            decision_kind_as_str(&evaluation.decision.decision),
            &evaluation.decision.reason,
        )?;
        if evaluation.decision.decision != DecisionKind::Allowed {
            return Err(PatchApplyError::PolicyDenied(evaluation.decision.reason));
        }
    }

    let mut prepared_changes = Vec::new();
    for file in &patch_files {
        let relative = normalize_safe_relative_path(&file.path)?;
        let target_path = project_root.join(&relative);
        let before_contents = if target_path.exists() {
            fs::read_to_string(&target_path)?
        } else {
            String::new()
        };
        let after_contents = apply_file_patch(&before_contents, file)?;
        prepared_changes.push(PreparedFileChange {
            path: file.path.clone(),
            target_path,
            before_sha256: hash_bytes(before_contents.as_bytes()),
            after_sha256: hash_bytes(after_contents.as_bytes()),
            after_contents,
        });
    }

    let file_hashes = prepared_changes
        .iter()
        .map(|change| FileChangeHash {
            path: change.path.clone(),
            before_sha256: change.before_sha256.clone(),
            after_sha256: change.after_sha256.clone(),
        })
        .collect::<Vec<_>>();

    for change in &prepared_changes {
        if let Some(parent) = change.target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&change.target_path, &change.after_contents)?;
        append_audit(
            &ledger_path,
            &request,
            &mut state,
            "action.completed",
            "gadget",
            &gadget.metadata.name,
            Some(("file", &change.path)),
            "allowed",
            "Approved patch was applied to this file.",
        )?;
    }

    let files_changed = prepared_changes
        .iter()
        .map(|change| change.path.clone())
        .collect::<Vec<_>>();
    let mut evidence_request = EvidenceWriteRequest::observe(
        request.apply_run_id.clone(),
        gadget.metadata.name.clone(),
        request.created_at.clone(),
        build_summary(&request, &approval_request, &files_changed),
    );
    evidence_request.assumptions = build_assumptions();
    evidence_request.extra_artifacts = vec![
        EvidenceTextArtifact::new("applied_patch", "applied.patch", patch_text),
        EvidenceTextArtifact::new(
            "files_changed",
            "files_changed.txt",
            files_changed.join("\n"),
        ),
        EvidenceTextArtifact::new(
            "before_after_hashes",
            "before_after_hashes.txt",
            format_file_hashes(&file_hashes),
        ),
        EvidenceTextArtifact::new(
            "approval_verification",
            "approval_verification.txt",
            format_approval_verification(&request, &approval_request, &approval),
        ),
        EvidenceTextArtifact::new(
            "policy_decisions",
            "policy_decisions.txt",
            policy_lines.join("\n"),
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
        "Patch Writer created evidence bundle for approved local patch application.",
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
        "Approved Patch Writer apply run completed without shell, tests, Git, PR, or admin actions.",
    )?;

    Ok(PatchApplyReport {
        apply_run_id: request.apply_run_id,
        approval_request_id: request.approval_request_id,
        source_plan_run_id: approval_request.target.run_id,
        files_changed,
        evidence_bundle_path: evidence_report.bundle_path,
        ledger_path,
        ledger_events_appended: state.ledger_events_appended,
    })
}

fn validate_request_for_apply(request: &ApprovalRequestRecord) -> Result<(), PatchApplyError> {
    if request.action_kind != "repo.patch.apply" {
        return Err(PatchApplyError::ApprovalNotVerified(vec![format!(
            "approval action kind is {}, expected repo.patch.apply",
            request.action_kind
        )]));
    }
    if request.executor_gadget != "patch.writer" {
        return Err(PatchApplyError::ApprovalNotVerified(vec![format!(
            "approval executor is {}, expected patch.writer",
            request.executor_gadget
        )]));
    }
    if request.target.zone != DEFAULT_ZONE {
        return Err(PatchApplyError::ApprovalNotVerified(vec![format!(
            "approval zone is {}, expected {DEFAULT_ZONE}",
            request.target.zone
        )]));
    }
    Ok(())
}

fn parse_unified_diff(patch: &str) -> Result<Vec<PatchFile>, PatchApplyError> {
    let mut files = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_hunks: Vec<PatchHunk> = Vec::new();
    let mut current_hunk: Option<PatchHunk> = None;

    for line in patch.lines() {
        if line.starts_with("diff --git ") {
            finish_hunk(&mut current_hunks, &mut current_hunk);
            finish_file(&mut files, &mut current_path, &mut current_hunks)?;
            continue;
        }

        if let Some(path) = line.strip_prefix("+++ ") {
            let normalized = strip_diff_path(path)?;
            if normalized == "/dev/null" {
                return Err(PatchApplyError::UnsupportedPatch(
                    "file deletion patches are not supported in Step 16".to_string(),
                ));
            }
            current_path = Some(normalized);
            continue;
        }

        if line.starts_with("--- ") {
            continue;
        }

        if line.starts_with("@@ ") {
            finish_hunk(&mut current_hunks, &mut current_hunk);
            let old_start = parse_hunk_old_start(line)?;
            current_hunk = Some(PatchHunk {
                old_start,
                lines: Vec::new(),
            });
            continue;
        }

        if let Some(hunk) = current_hunk.as_mut() {
            if let Some(text) = line.strip_prefix(' ') {
                hunk.lines.push(HunkLine::Context(text.to_string()));
            } else if let Some(text) = line.strip_prefix('-') {
                hunk.lines.push(HunkLine::Remove(text.to_string()));
            } else if let Some(text) = line.strip_prefix('+') {
                hunk.lines.push(HunkLine::Add(text.to_string()));
            } else if line.starts_with("\\ No newline at end of file")
                || line.starts_with('#')
                || line.trim().is_empty()
            {
                continue;
            } else {
                return Err(PatchApplyError::InvalidPatch(format!(
                    "unrecognized hunk line: {line}"
                )));
            }
        }
    }

    finish_hunk(&mut current_hunks, &mut current_hunk);
    finish_file(&mut files, &mut current_path, &mut current_hunks)?;
    Ok(files)
}

fn finish_hunk(hunks: &mut Vec<PatchHunk>, current: &mut Option<PatchHunk>) {
    if let Some(hunk) = current.take() {
        hunks.push(hunk);
    }
}

fn finish_file(
    files: &mut Vec<PatchFile>,
    current_path: &mut Option<String>,
    current_hunks: &mut Vec<PatchHunk>,
) -> Result<(), PatchApplyError> {
    if let Some(path) = current_path.take() {
        if current_hunks.is_empty() {
            return Err(PatchApplyError::InvalidPatch(format!(
                "file {path} has no hunks"
            )));
        }
        files.push(PatchFile {
            path,
            hunks: std::mem::take(current_hunks),
        });
    }
    Ok(())
}

fn strip_diff_path(value: &str) -> Result<String, PatchApplyError> {
    let value = value.split_whitespace().next().unwrap_or(value);
    if value == "/dev/null" {
        return Ok(value.to_string());
    }
    let path = value
        .strip_prefix("a/")
        .or_else(|| value.strip_prefix("b/"))
        .unwrap_or(value);
    let normalized = normalize_safe_relative_path(path)?;
    Ok(normalized.to_string_lossy().replace('\\', "/"))
}

fn parse_hunk_old_start(line: &str) -> Result<usize, PatchApplyError> {
    let mut parts = line.split_whitespace();
    let _marker = parts.next();
    let Some(old_range) = parts.next() else {
        return Err(PatchApplyError::InvalidPatch(format!(
            "hunk header missing old range: {line}"
        )));
    };
    let old_range = old_range.trim_start_matches('-');
    let start = old_range.split(',').next().unwrap_or(old_range);
    start.parse::<usize>().map_err(|_| {
        PatchApplyError::InvalidPatch(format!("invalid hunk old start in header: {line}"))
    })
}

fn apply_file_patch(before: &str, patch_file: &PatchFile) -> Result<String, PatchApplyError> {
    let source = split_lines_lossy(before);
    let mut output = Vec::new();
    let mut cursor = 0usize;

    for hunk in &patch_file.hunks {
        let target_index = hunk.old_start.saturating_sub(1);
        if target_index < cursor || target_index > source.len() {
            return Err(PatchApplyError::PatchMismatch(format!(
                "hunk for {} starts at {}, but current cursor is {} and source has {} lines",
                patch_file.path,
                hunk.old_start,
                cursor,
                source.len()
            )));
        }
        output.extend_from_slice(&source[cursor..target_index]);
        cursor = target_index;

        for line in &hunk.lines {
            match line {
                HunkLine::Context(text) => {
                    require_source_line(&source, cursor, text, &patch_file.path)?;
                    output.push(source[cursor].clone());
                    cursor += 1;
                }
                HunkLine::Remove(text) => {
                    require_source_line(&source, cursor, text, &patch_file.path)?;
                    cursor += 1;
                }
                HunkLine::Add(text) => output.push(text.clone()),
            }
        }
    }

    output.extend_from_slice(&source[cursor..]);
    if output.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", output.join("\n")))
    }
}

fn split_lines_lossy(value: &str) -> Vec<String> {
    value.lines().map(|line| line.to_string()).collect()
}

fn require_source_line(
    source: &[String],
    cursor: usize,
    expected: &str,
    path: &str,
) -> Result<(), PatchApplyError> {
    let Some(actual) = source.get(cursor) else {
        return Err(PatchApplyError::PatchMismatch(format!(
            "patch expected line {cursor} in {path}, but file ended"
        )));
    };
    if actual != expected {
        return Err(PatchApplyError::PatchMismatch(format!(
            "patch context mismatch in {path}: expected {expected:?}, actual {actual:?}"
        )));
    }
    Ok(())
}

fn normalize_safe_relative_path(input: &str) -> Result<PathBuf, PatchApplyError> {
    let path = Path::new(input);
    if path.is_absolute() {
        return Err(PatchApplyError::PathNotSafe(input.to_string()));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(PatchApplyError::PathNotSafe(input.to_string()))
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(PatchApplyError::PathNotSafe(input.to_string()));
    }
    Ok(normalized)
}

#[allow(clippy::too_many_arguments)]
fn append_audit(
    ledger_path: &Path,
    request: &PatchApplyRequest,
    state: &mut ApplyState,
    event_type: &str,
    actor_kind: &str,
    actor_id: &str,
    target: Option<(&str, &str)>,
    decision: &str,
    summary: &str,
) -> Result<(), PatchApplyError> {
    state.next_event_number += 1;
    let event_id = format!("aud_{}_{}", request.apply_run_id, state.next_event_number);
    let event = new_audit_event(
        event_id,
        request.created_at.clone(),
        event_type,
        actor_kind,
        actor_id,
        request.apply_run_id.clone(),
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
    request: &PatchApplyRequest,
    approval_request: &ApprovalRequestRecord,
    files_changed: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("Approved local patch application completed.\n\n");
    out.push_str(&format!(
        "Approval request: {}\n",
        request.approval_request_id
    ));
    out.push_str(&format!(
        "Source plan run: {}\n",
        approval_request.target.run_id
    ));
    out.push_str(&format!(
        "Runtime mode: {}\n\n",
        request.runtime_mode.as_str()
    ));
    out.push_str("Files changed:\n");
    for path in files_changed {
        out.push_str("- ");
        out.push_str(path);
        out.push('\n');
    }
    out.push_str("\nNo tests, shell commands, Git actions, PR actions, provider tools, or admin actions were executed.\n");
    out
}

fn build_assumptions() -> Vec<String> {
    vec![
        "The patch was applied from a previously generated proposed.patch artifact.".to_string(),
        "The approval record was verified before any file write.".to_string(),
        "Each target path was checked by the deterministic policy engine with approval present.".to_string(),
        "No shell commands, tests, Git commands, provider-side tools, or admin actions were executed.".to_string(),
    ]
}

fn format_file_hashes(hashes: &[FileChangeHash]) -> String {
    let mut out = String::new();
    for item in hashes {
        out.push_str(&format!("path={}\n", item.path));
        out.push_str(&format!("before_sha256={}\n", item.before_sha256));
        out.push_str(&format!("after_sha256={}\n\n", item.after_sha256));
    }
    out
}

fn format_approval_verification(
    request: &PatchApplyRequest,
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
    out
}

fn decision_kind_as_str(decision: &DecisionKind) -> &'static str {
    match decision {
        DecisionKind::Allowed => "allowed",
        DecisionKind::Denied => "denied",
        DecisionKind::RequiresApproval => "requires_approval",
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{}", to_hex(&digest))
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_added_file_patch() {
        let patch = "diff --git a/docs/example.md b/docs/example.md\n--- a/docs/example.md\n+++ b/docs/example.md\n@@ -0,0 +1,2 @@\n+# Title\n+Body\n";
        let files = parse_unified_diff(patch).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "docs/example.md");
        let after = apply_file_patch("", &files[0]).unwrap();
        assert_eq!(after, "# Title\nBody\n");
    }

    #[test]
    fn rejects_parent_traversal_path() {
        let patch =
            "diff --git a/../bad b/../bad\n--- a/../bad\n+++ b/../bad\n@@ -0,0 +1,1 @@\n+bad\n";
        assert!(parse_unified_diff(patch).is_err());
    }

    #[test]
    fn detects_context_mismatch() {
        let patch = "diff --git a/docs/example.md b/docs/example.md\n--- a/docs/example.md\n+++ b/docs/example.md\n@@ -1,1 +1,1 @@\n-old\n+new\n";
        let files = parse_unified_diff(patch).unwrap();
        assert!(apply_file_patch("different\n", &files[0]).is_err());
    }
}
