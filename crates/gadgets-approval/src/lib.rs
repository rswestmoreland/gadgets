//! Approval record scaffolding.
//!
//! This crate creates and verifies local approval request and approval record
//! files. It does not apply patches, write project files, execute commands,
//! call model providers, or authorize actions by itself. The runtime must still
//! evaluate policy before any future apply step.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const APPROVAL_REQUEST_SCHEMA_VERSION: &str = "gadgets.framework/approval-request/v0.1";
pub const APPROVAL_RECORD_SCHEMA_VERSION: &str = "gadgets.framework/approval-record/v0.1";
pub const DEFAULT_APPROVALS_RELATIVE_PATH: &str = ".gadgets/approvals";
pub const REQUEST_FILE_NAME: &str = "request.yaml";
pub const APPROVAL_FILE_NAME: &str = "approval.yaml";
pub const PATCH_ARTIFACT_RELATIVE_PATH: &str = "evidence/proposed.patch";
pub const APPROVAL_EXPIRES_AT_FORMAT: &str = "strict RFC3339 UTC without fractional seconds, for example 2999-01-01T00:00:00Z";

#[derive(Debug)]
pub enum ApprovalError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    InvalidId(String),
    InvalidApprover(String),
    MissingPatch(PathBuf),
    MissingRequest(PathBuf),
    MissingApproval(PathBuf),
    AlreadyExists(PathBuf),
    HashMismatch { expected: String, actual: String },
    ScopeMismatch { expected: String, actual: String },
    InvalidExpiresAt(String),
    ExpiredApproval(String),
}

impl fmt::Display for ApprovalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "approval I/O error: {err}"),
            Self::Yaml(err) => write!(f, "approval YAML error: {err}"),
            Self::InvalidId(value) => write!(f, "invalid approval identifier: {value}"),
            Self::InvalidApprover(value) => write!(f, "invalid approver identifier: {value}"),
            Self::MissingPatch(path) => write!(f, "proposed patch artifact not found at {}", path.display()),
            Self::MissingRequest(path) => write!(f, "approval request not found at {}", path.display()),
            Self::MissingApproval(path) => write!(f, "approval record not found at {}", path.display()),
            Self::AlreadyExists(path) => write!(f, "approval artifact already exists at {}", path.display()),
            Self::HashMismatch { expected, actual } => write!(f, "patch hash mismatch: expected {expected}, actual {actual}"),
            Self::ScopeMismatch { expected, actual } => write!(f, "approval scope mismatch: expected {expected}, actual {actual}"),
            Self::InvalidExpiresAt(value) => write!(f, "invalid approval expiration {value:?}; expected {APPROVAL_EXPIRES_AT_FORMAT}"),
            Self::ExpiredApproval(value) => write!(f, "approval expired at {value}"),
        }
    }
}

impl Error for ApprovalError {}

impl From<std::io::Error> for ApprovalError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_yaml::Error> for ApprovalError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Yaml(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchApprovalRequestInput {
    pub run_id: String,
    pub created_at: String,
    pub requested_by_gadget: String,
    pub executor_gadget: String,
    pub zone: String,
    pub action_kind: String,
    pub risk_level: String,
    pub expires_at: Option<String>,
}

impl PatchApprovalRequestInput {
    pub fn local_patch_apply(
        run_id: impl Into<String>,
        created_at: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            created_at: created_at.into(),
            requested_by_gadget: "patch.writer".to_string(),
            executor_gadget: "patch.writer".to_string(),
            zone: "local_repo".to_string(),
            action_kind: "repo.patch.apply".to_string(),
            risk_level: "medium".to_string(),
            expires_at: None,
        }
    }

    pub fn with_expires_at(mut self, expires_at: impl Into<String>) -> Self {
        self.expires_at = Some(expires_at.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalTarget {
    pub zone: String,
    pub run_id: String,
    pub evidence_bundle_id: String,
    pub patch_artifact_path: String,
    pub patch_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestRecord {
    pub approval_request_id: String,
    pub schema_version: String,
    pub created_at: String,
    pub requested_by_gadget: String,
    pub executor_gadget: String,
    pub risk_level: String,
    pub action_kind: String,
    pub target: ApprovalTarget,
    pub scope_hash: String,
    pub status: String,
    pub expires_at: Option<String>,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub approval_id: String,
    pub schema_version: String,
    pub approval_request_id: String,
    pub approved_by: String,
    pub status: String,
    pub scope_hash: String,
    pub approved_at: String,
    pub expires_at: Option<String>,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequestWriteReport {
    pub approval_request_id: String,
    pub request_path: PathBuf,
    pub patch_path: PathBuf,
    pub patch_sha256: String,
    pub scope_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalWriteReport {
    pub approval_id: String,
    pub approval_request_id: String,
    pub approval_path: PathBuf,
    pub scope_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalVerification {
    pub approval_request_id: String,
    pub request_path: PathBuf,
    pub approval_path: Option<PathBuf>,
    pub patch_path: PathBuf,
    pub valid: bool,
    pub errors: Vec<String>,
}

impl ApprovalVerification {
    fn ok(
        approval_request_id: String,
        request_path: PathBuf,
        approval_path: Option<PathBuf>,
        patch_path: PathBuf,
    ) -> Self {
        Self {
            approval_request_id,
            request_path,
            approval_path,
            patch_path,
            valid: true,
            errors: Vec::new(),
        }
    }

    fn invalid(
        approval_request_id: String,
        request_path: PathBuf,
        approval_path: Option<PathBuf>,
        patch_path: PathBuf,
        errors: Vec<String>,
    ) -> Self {
        Self {
            approval_request_id,
            request_path,
            approval_path,
            patch_path,
            valid: false,
            errors,
        }
    }
}

pub fn default_approvals_root(project_root: &Path) -> PathBuf {
    project_root.join(DEFAULT_APPROVALS_RELATIVE_PATH)
}

pub fn create_patch_approval_request(
    project_root: &Path,
    input: PatchApprovalRequestInput,
) -> Result<ApprovalRequestWriteReport, ApprovalError> {
    validate_id(&input.run_id)?;
    validate_id(&input.requested_by_gadget)?;
    validate_id(&input.executor_gadget)?;
    validate_optional_expires_at(input.expires_at.as_deref())?;

    let approval_request_id = patch_approval_request_id(&input.run_id);
    let request_dir = request_dir(project_root, &approval_request_id)?;
    let request_path = request_dir.join(REQUEST_FILE_NAME);
    if request_path.exists() {
        return Err(ApprovalError::AlreadyExists(request_path));
    }

    let patch_path = patch_artifact_path(project_root, &input.run_id)?;
    if !patch_path.exists() {
        return Err(ApprovalError::MissingPatch(patch_path));
    }
    let patch_sha256 = hash_file(&patch_path)?;
    let evidence_bundle_id = format!("evb_{}", input.run_id);
    let target = ApprovalTarget {
        zone: input.zone,
        run_id: input.run_id.clone(),
        evidence_bundle_id,
        patch_artifact_path: PATCH_ARTIFACT_RELATIVE_PATH.to_string(),
        patch_sha256: patch_sha256.clone(),
    };
    let scope_hash = compute_scope_hash(
        &input.action_kind,
        &input.executor_gadget,
        &target.zone,
        &target.run_id,
        &target.patch_artifact_path,
        &target.patch_sha256,
    );
    let record = ApprovalRequestRecord {
        approval_request_id: approval_request_id.clone(),
        schema_version: APPROVAL_REQUEST_SCHEMA_VERSION.to_string(),
        created_at: input.created_at,
        requested_by_gadget: input.requested_by_gadget,
        executor_gadget: input.executor_gadget,
        risk_level: input.risk_level,
        action_kind: input.action_kind,
        target,
        scope_hash: scope_hash.clone(),
        status: "requested".to_string(),
        expires_at: input.expires_at,
        conditions: default_patch_conditions(),
    };

    fs::create_dir_all(&request_dir)?;
    fs::write(&request_path, serde_yaml::to_string(&record)?)?;

    Ok(ApprovalRequestWriteReport {
        approval_request_id,
        request_path,
        patch_path,
        patch_sha256,
        scope_hash,
    })
}

pub fn approve_request(
    project_root: &Path,
    approval_request_id: &str,
    approver: &str,
    approved_at: impl Into<String>,
) -> Result<ApprovalWriteReport, ApprovalError> {
    validate_id(approval_request_id)?;
    validate_approver(approver)?;

    let request_path = request_path(project_root, approval_request_id)?;
    if !request_path.exists() {
        return Err(ApprovalError::MissingRequest(request_path));
    }
    let approval_path = approval_path(project_root, approval_request_id)?;
    if approval_path.exists() {
        return Err(ApprovalError::AlreadyExists(approval_path));
    }

    let request = read_request_record(&request_path)?;
    validate_request_not_expired(&request)?;
    verify_request_patch_hash(project_root, &request)?;
    let recomputed_scope = recompute_request_scope_hash(&request);
    if recomputed_scope != request.scope_hash {
        return Err(ApprovalError::ScopeMismatch {
            expected: request.scope_hash,
            actual: recomputed_scope,
        });
    }

    let approval_id = format!("app_{}", approval_request_id);
    let record = ApprovalRecord {
        approval_id: approval_id.clone(),
        schema_version: APPROVAL_RECORD_SCHEMA_VERSION.to_string(),
        approval_request_id: approval_request_id.to_string(),
        approved_by: approver.to_string(),
        status: "approved".to_string(),
        scope_hash: request.scope_hash.clone(),
        approved_at: approved_at.into(),
        expires_at: request.expires_at.clone(),
        conditions: request.conditions.clone(),
    };

    fs::write(&approval_path, serde_yaml::to_string(&record)?)?;

    Ok(ApprovalWriteReport {
        approval_id,
        approval_request_id: approval_request_id.to_string(),
        approval_path,
        scope_hash: request.scope_hash,
    })
}

pub fn verify_approval(project_root: &Path, approval_request_id: &str) -> Result<ApprovalVerification, ApprovalError> {
    validate_id(approval_request_id)?;
    let request_path = request_path(project_root, approval_request_id)?;
    if !request_path.exists() {
        return Err(ApprovalError::MissingRequest(request_path));
    }
    let request = read_request_record(&request_path)?;
    let approval_path = approval_path(project_root, approval_request_id)?;
    let approval_path_opt = approval_path.exists().then_some(approval_path.clone());
    let patch_path = patch_artifact_path(project_root, &request.target.run_id)?;
    let mut errors = Vec::new();

    if !patch_path.exists() {
        errors.push(format!("missing patch artifact at {}", patch_path.display()));
    } else {
        match hash_file(&patch_path) {
            Ok(actual) if actual == request.target.patch_sha256 => {}
            Ok(actual) => errors.push(format!(
                "patch hash mismatch: expected {}, actual {}",
                request.target.patch_sha256, actual
            )),
            Err(err) => errors.push(format!("failed to hash patch artifact: {err}")),
        }
    }

    if let Some(expires_at) = request.expires_at.as_deref() {
        match validate_expires_at_not_expired(expires_at) {
            Ok(()) => {}
            Err(ApprovalError::ExpiredApproval(value)) => errors.push(format!("approval expired at {value}")),
            Err(ApprovalError::InvalidExpiresAt(value)) => errors.push(format!(
                "invalid approval expiration {value:?}; expected {APPROVAL_EXPIRES_AT_FORMAT}"
            )),
            Err(err) => errors.push(format!("failed to verify approval expiration: {err}")),
        }
    }

    let recomputed_scope = recompute_request_scope_hash(&request);
    if recomputed_scope != request.scope_hash {
        errors.push(format!(
            "request scope hash mismatch: expected {}, actual {}",
            request.scope_hash, recomputed_scope
        ));
    }

    if approval_path.exists() {
        let approval = read_approval_record(&approval_path)?;
        if approval.approval_request_id != request.approval_request_id {
            errors.push("approval request id does not match request file".to_string());
        }
        if approval.scope_hash != request.scope_hash {
            errors.push(format!(
                "approval scope hash mismatch: expected {}, actual {}",
                request.scope_hash, approval.scope_hash
            ));
        }
        if approval.status != "approved" {
            errors.push(format!("approval status is {}", approval.status));
        }
        if approval.expires_at != request.expires_at {
            errors.push("approval expiration does not match request expiration".to_string());
        }
    } else {
        errors.push(format!("approval record missing at {}", approval_path.display()));
    }

    if errors.is_empty() {
        Ok(ApprovalVerification::ok(
            approval_request_id.to_string(),
            request_path,
            approval_path_opt,
            patch_path,
        ))
    } else {
        Ok(ApprovalVerification::invalid(
            approval_request_id.to_string(),
            request_path,
            approval_path_opt,
            patch_path,
            errors,
        ))
    }
}

pub fn read_request(project_root: &Path, approval_request_id: &str) -> Result<ApprovalRequestRecord, ApprovalError> {
    validate_id(approval_request_id)?;
    let path = request_path(project_root, approval_request_id)?;
    if !path.exists() {
        return Err(ApprovalError::MissingRequest(path));
    }
    read_request_record(&path)
}

pub fn read_approval(project_root: &Path, approval_request_id: &str) -> Result<Option<ApprovalRecord>, ApprovalError> {
    validate_id(approval_request_id)?;
    let path = approval_path(project_root, approval_request_id)?;
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(read_approval_record(&path)?))
}

pub fn patch_approval_request_id(run_id: &str) -> String {
    format!("apr_{}_patch", run_id)
}

pub fn patch_artifact_path(project_root: &Path, run_id: &str) -> Result<PathBuf, ApprovalError> {
    validate_id(run_id)?;
    Ok(project_root
        .join(".gadgets")
        .join("runs")
        .join(run_id)
        .join(PATCH_ARTIFACT_RELATIVE_PATH))
}

pub fn request_path(project_root: &Path, approval_request_id: &str) -> Result<PathBuf, ApprovalError> {
    Ok(request_dir(project_root, approval_request_id)?.join(REQUEST_FILE_NAME))
}

pub fn approval_path(project_root: &Path, approval_request_id: &str) -> Result<PathBuf, ApprovalError> {
    Ok(request_dir(project_root, approval_request_id)?.join(APPROVAL_FILE_NAME))
}

fn request_dir(project_root: &Path, approval_request_id: &str) -> Result<PathBuf, ApprovalError> {
    validate_id(approval_request_id)?;
    Ok(default_approvals_root(project_root).join(approval_request_id))
}

fn read_request_record(path: &Path) -> Result<ApprovalRequestRecord, ApprovalError> {
    Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
}

fn read_approval_record(path: &Path) -> Result<ApprovalRecord, ApprovalError> {
    Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
}

fn verify_request_patch_hash(project_root: &Path, request: &ApprovalRequestRecord) -> Result<(), ApprovalError> {
    let patch_path = patch_artifact_path(project_root, &request.target.run_id)?;
    if !patch_path.exists() {
        return Err(ApprovalError::MissingPatch(patch_path));
    }
    let actual = hash_file(&patch_path)?;
    if actual != request.target.patch_sha256 {
        return Err(ApprovalError::HashMismatch {
            expected: request.target.patch_sha256.clone(),
            actual,
        });
    }
    Ok(())
}

fn recompute_request_scope_hash(request: &ApprovalRequestRecord) -> String {
    compute_scope_hash(
        &request.action_kind,
        &request.executor_gadget,
        &request.target.zone,
        &request.target.run_id,
        &request.target.patch_artifact_path,
        &request.target.patch_sha256,
    )
}

fn compute_scope_hash(
    action_kind: &str,
    executor_gadget: &str,
    zone: &str,
    run_id: &str,
    patch_artifact_path: &str,
    patch_sha256: &str,
) -> String {
    let canonical = format!(
        "action_kind={action_kind}\nexecutor_gadget={executor_gadget}\nzone={zone}\nrun_id={run_id}\npatch_artifact_path={patch_artifact_path}\npatch_sha256={patch_sha256}\n"
    );
    hash_bytes(canonical.as_bytes())
}

fn default_patch_conditions() -> Vec<String> {
    vec![
        "Approval is limited to the exact patch artifact hash in this request.".to_string(),
        "Any changed patch contents invalidate this approval.".to_string(),
        "Approval expiration, when present, is strict RFC3339 UTC and is enforced before use.".to_string(),
        "A future apply step must still pass policy boundary checks.".to_string(),
        "This approval record does not apply the patch by itself.".to_string(),
    ]
}


fn validate_optional_expires_at(value: Option<&str>) -> Result<(), ApprovalError> {
    if let Some(expires_at) = value {
        parse_strict_rfc3339_utc(expires_at)?;
    }
    Ok(())
}

fn validate_request_not_expired(request: &ApprovalRequestRecord) -> Result<(), ApprovalError> {
    if let Some(expires_at) = request.expires_at.as_deref() {
        validate_expires_at_not_expired(expires_at)?;
    }
    Ok(())
}

fn validate_expires_at_not_expired(expires_at: &str) -> Result<(), ApprovalError> {
    let expires_at_seconds = parse_strict_rfc3339_utc(expires_at)?;
    if current_unix_seconds() >= expires_at_seconds {
        return Err(ApprovalError::ExpiredApproval(expires_at.to_string()));
    }
    Ok(())
}

fn parse_strict_rfc3339_utc(value: &str) -> Result<i64, ApprovalError> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err(ApprovalError::InvalidExpiresAt(value.to_string()));
    }

    let year = parse_digits(value, 0, 4)?;
    let month = parse_digits(value, 5, 7)?;
    let day = parse_digits(value, 8, 10)?;
    let hour = parse_digits(value, 11, 13)?;
    let minute = parse_digits(value, 14, 16)?;
    let second = parse_digits(value, 17, 19)?;

    if month < 1 || month > 12 {
        return Err(ApprovalError::InvalidExpiresAt(value.to_string()));
    }
    let max_day = days_in_month(year, month);
    if day < 1 || day > max_day {
        return Err(ApprovalError::InvalidExpiresAt(value.to_string()));
    }
    if hour > 23 || minute > 59 || second > 59 {
        return Err(ApprovalError::InvalidExpiresAt(value.to_string()));
    }

    let days = days_from_civil(year, month, day);
    Ok(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn parse_digits(value: &str, start: usize, end: usize) -> Result<i64, ApprovalError> {
    let slice = &value[start..end];
    if !slice.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApprovalError::InvalidExpiresAt(value.to_string()));
    }
    slice
        .parse::<i64>()
        .map_err(|_| ApprovalError::InvalidExpiresAt(value.to_string()))
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let yoe = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month_prime + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn current_unix_seconds() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_secs() as i64,
        Err(_) => 0,
    }
}

fn hash_file(path: &Path) -> Result<String, ApprovalError> {
    Ok(hash_bytes(&fs::read(path)?))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{}", to_hex(&digest))
}

fn validate_id(value: &str) -> Result<(), ApprovalError> {
    if value.is_empty()
        || value.contains("..")
        || value.contains('/')
        || value.contains('\\')
        || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(ApprovalError::InvalidId(value.to_string()));
    }
    Ok(())
}

fn validate_approver(value: &str) -> Result<(), ApprovalError> {
    if value.is_empty()
        || value.contains("..")
        || value.contains('/')
        || value.contains('\\')
        || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '@')
    {
        return Err(ApprovalError::InvalidApprover(value.to_string()));
    }
    Ok(())
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "gadgets-approval-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn request_id_is_bound_to_run() {
        assert_eq!(patch_approval_request_id("run_1"), "apr_run_1_patch");
    }

    #[test]
    fn creates_and_verifies_patch_approval() {
        let project = temp_project("valid");
        let patch_path = patch_artifact_path(&project, "run_1").unwrap();
        fs::create_dir_all(patch_path.parent().unwrap()).unwrap();
        fs::write(&patch_path, "diff --git a/a b/a\n").unwrap();

        let request = create_patch_approval_request(
            &project,
            PatchApprovalRequestInput::local_patch_apply("run_1", "created"),
        )
        .unwrap();
        assert_eq!(request.approval_request_id, "apr_run_1_patch");
        assert!(request.scope_hash.starts_with("sha256:"));

        let approval = approve_request(&project, &request.approval_request_id, "user@example.com", "approved")
            .unwrap();
        assert_eq!(approval.scope_hash, request.scope_hash);

        let verification = verify_approval(&project, &request.approval_request_id).unwrap();
        assert!(verification.valid);
    }

    #[test]
    fn detects_patch_tamper() {
        let project = temp_project("tamper");
        let patch_path = patch_artifact_path(&project, "run_1").unwrap();
        fs::create_dir_all(patch_path.parent().unwrap()).unwrap();
        fs::write(&patch_path, "original\n").unwrap();
        let request = create_patch_approval_request(
            &project,
            PatchApprovalRequestInput::local_patch_apply("run_1", "created"),
        )
        .unwrap();
        approve_request(&project, &request.approval_request_id, "user@example.com", "approved")
            .unwrap();
        fs::write(&patch_path, "changed\n").unwrap();
        let verification = verify_approval(&project, &request.approval_request_id).unwrap();
        assert!(!verification.valid);
        assert!(verification.errors.iter().any(|err| err.contains("patch hash mismatch")));
    }

    #[test]
    fn verify_requires_approval_record() {
        let project = temp_project("missing-approval");
        let patch_path = patch_artifact_path(&project, "run_1").unwrap();
        fs::create_dir_all(patch_path.parent().unwrap()).unwrap();
        fs::write(&patch_path, "diff --git a/a b/a\n").unwrap();
        let request = create_patch_approval_request(
            &project,
            PatchApprovalRequestInput::local_patch_apply("run_1", "created"),
        )
        .unwrap();
        let verification = verify_approval(&project, &request.approval_request_id).unwrap();
        assert!(!verification.valid);
        assert!(verification.errors.iter().any(|err| err.contains("approval record missing")));
    }


    #[test]
    fn parses_strict_utc_expiration() {
        assert_eq!(parse_strict_rfc3339_utc("1970-01-01T00:00:00Z").unwrap(), 0);
        assert!(parse_strict_rfc3339_utc("2026-02-29T00:00:00Z").is_err());
        assert!(parse_strict_rfc3339_utc("2026-05-13T18:30:00+00:00").is_err());
    }

    #[test]
    fn rejects_invalid_expiration_on_request_create() {
        let project = temp_project("invalid-expiry");
        let patch_path = patch_artifact_path(&project, "run_1").unwrap();
        fs::create_dir_all(patch_path.parent().unwrap()).unwrap();
        fs::write(&patch_path, "diff --git a/a b/a\n").unwrap();

        let err = create_patch_approval_request(
            &project,
            PatchApprovalRequestInput::local_patch_apply("run_1", "created")
                .with_expires_at("not-a-timestamp"),
        )
        .unwrap_err();

        assert!(matches!(err, ApprovalError::InvalidExpiresAt(_)));
    }

    #[test]
    fn verifies_future_expiration() {
        let project = temp_project("future-expiry");
        let patch_path = patch_artifact_path(&project, "run_1").unwrap();
        fs::create_dir_all(patch_path.parent().unwrap()).unwrap();
        fs::write(&patch_path, "diff --git a/a b/a\n").unwrap();

        let request = create_patch_approval_request(
            &project,
            PatchApprovalRequestInput::local_patch_apply("run_1", "created")
                .with_expires_at("2999-01-01T00:00:00Z"),
        )
        .unwrap();
        approve_request(&project, &request.approval_request_id, "user@example.com", "approved")
            .unwrap();

        let verification = verify_approval(&project, &request.approval_request_id).unwrap();
        assert!(verification.valid);
    }

    #[test]
    fn rejects_expired_approval_on_approve() {
        let project = temp_project("expired-approve");
        let patch_path = patch_artifact_path(&project, "run_1").unwrap();
        fs::create_dir_all(patch_path.parent().unwrap()).unwrap();
        fs::write(&patch_path, "diff --git a/a b/a\n").unwrap();

        let request = create_patch_approval_request(
            &project,
            PatchApprovalRequestInput::local_patch_apply("run_1", "created")
                .with_expires_at("1970-01-01T00:00:01Z"),
        )
        .unwrap();
        let err = approve_request(&project, &request.approval_request_id, "user@example.com", "approved")
            .unwrap_err();

        assert!(matches!(err, ApprovalError::ExpiredApproval(_)));
    }

    #[test]
    fn verify_rejects_expired_approval_record() {
        let project = temp_project("expired-verify");
        let patch_path = patch_artifact_path(&project, "run_1").unwrap();
        fs::create_dir_all(patch_path.parent().unwrap()).unwrap();
        fs::write(&patch_path, "diff --git a/a b/a\n").unwrap();

        let request = create_patch_approval_request(
            &project,
            PatchApprovalRequestInput::local_patch_apply("run_1", "created")
                .with_expires_at("2999-01-01T00:00:00Z"),
        )
        .unwrap();
        approve_request(&project, &request.approval_request_id, "user@example.com", "approved")
            .unwrap();

        let request_path = request_path(&project, &request.approval_request_id).unwrap();
        let mut request_record = read_request(&project, &request.approval_request_id).unwrap();
        request_record.expires_at = Some("1970-01-01T00:00:01Z".to_string());
        fs::write(&request_path, serde_yaml::to_string(&request_record).unwrap()).unwrap();

        let approval_path = approval_path(&project, &request.approval_request_id).unwrap();
        let mut approval_record = read_approval(&project, &request.approval_request_id)
            .unwrap()
            .unwrap();
        approval_record.expires_at = Some("1970-01-01T00:00:01Z".to_string());
        fs::write(&approval_path, serde_yaml::to_string(&approval_record).unwrap()).unwrap();

        let verification = verify_approval(&project, &request.approval_request_id).unwrap();
        assert!(!verification.valid);
        assert!(verification.errors.iter().any(|err| err.contains("approval expired")));
    }

    #[test]
    fn rejects_path_traversal_ids() {
        assert!(request_path(Path::new("."), "../bad").is_err());
        assert!(patch_artifact_path(Path::new("."), "../bad").is_err());
    }
}
