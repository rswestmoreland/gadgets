//! Evidence bundle writer.
//!
//! This crate persists observe-only evidence bundles for Gadgets Framework. It
//! does not inspect files, execute commands, call providers, or authorize
//! actions. It only writes and reads evidence metadata and artifacts supplied by
//! callers that have already passed policy checks.

use gadgets_core::{EvidenceArtifact, EvidenceBundle};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const EVIDENCE_SCHEMA_VERSION: &str = "gadgets.framework/evidence/v0.1";
pub const DEFAULT_RUNS_RELATIVE_PATH: &str = ".gadgets/runs";
pub const BUNDLE_FILE_NAME: &str = "bundle.yaml";
pub const SUMMARY_FILE_NAME: &str = "summary.md";
pub const DENIED_ACTIONS_FILE_NAME: &str = "denied_actions.txt";
pub const ASSUMPTIONS_FILE_NAME: &str = "assumptions.txt";

#[derive(Debug)]
pub enum EvidenceError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    InvalidId(String),
    InvalidArtifactName(String),
    AlreadyExists(PathBuf),
    MissingBundle(PathBuf),
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "evidence I/O error: {err}"),
            Self::Yaml(err) => write!(f, "evidence YAML error: {err}"),
            Self::InvalidId(value) => write!(f, "invalid evidence identifier: {value}"),
            Self::InvalidArtifactName(value) => {
                write!(f, "invalid evidence artifact name: {value}")
            }
            Self::AlreadyExists(path) => {
                write!(f, "evidence bundle already exists at {}", path.display())
            }
            Self::MissingBundle(path) => {
                write!(f, "evidence bundle not found at {}", path.display())
            }
        }
    }
}

impl Error for EvidenceError {}

impl From<std::io::Error> for EvidenceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_yaml::Error> for EvidenceError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Yaml(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceWriteRequest {
    pub run_id: String,
    pub gadget: String,
    pub created_at: String,
    pub summary: String,
    pub status: String,
    pub denied_actions: Vec<String>,
    pub assumptions: Vec<String>,
    pub extra_artifacts: Vec<EvidenceTextArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceTextArtifact {
    pub artifact_type: String,
    pub file_name: String,
    pub contents: String,
}

impl EvidenceTextArtifact {
    pub fn new(
        artifact_type: impl Into<String>,
        file_name: impl Into<String>,
        contents: impl Into<String>,
    ) -> Self {
        Self {
            artifact_type: artifact_type.into(),
            file_name: file_name.into(),
            contents: contents.into(),
        }
    }
}

impl EvidenceWriteRequest {
    pub fn observe(
        run_id: impl Into<String>,
        gadget: impl Into<String>,
        created_at: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            gadget: gadget.into(),
            created_at: created_at.into(),
            summary: summary.into(),
            status: "completed".to_string(),
            denied_actions: Vec::new(),
            assumptions: Vec::new(),
            extra_artifacts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceWriteReport {
    pub run_id: String,
    pub evidence_dir: PathBuf,
    pub bundle_path: PathBuf,
    pub summary_path: PathBuf,
    pub bundle_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSummary {
    pub evidence_bundle_id: String,
    pub run_id: String,
    pub gadget: String,
    pub status: String,
    pub summary: String,
    pub artifact_count: usize,
    pub denied_action_count: usize,
    pub assumption_count: usize,
    pub bundle_hash: Option<String>,
}

pub fn default_runs_root(project_root: &Path) -> PathBuf {
    project_root.join(DEFAULT_RUNS_RELATIVE_PATH)
}

pub fn evidence_dir_for_run(runs_root: &Path, run_id: &str) -> Result<PathBuf, EvidenceError> {
    validate_id(run_id)?;
    Ok(runs_root.join(run_id).join("evidence"))
}

pub fn bundle_path_for_run(runs_root: &Path, run_id: &str) -> Result<PathBuf, EvidenceError> {
    Ok(evidence_dir_for_run(runs_root, run_id)?.join(BUNDLE_FILE_NAME))
}

pub fn create_observe_bundle(
    runs_root: &Path,
    request: EvidenceWriteRequest,
) -> Result<EvidenceWriteReport, EvidenceError> {
    validate_id(&request.run_id)?;
    validate_id(&request.gadget)?;

    let evidence_dir = evidence_dir_for_run(runs_root, &request.run_id)?;
    let bundle_path = evidence_dir.join(BUNDLE_FILE_NAME);
    if bundle_path.exists() {
        return Err(EvidenceError::AlreadyExists(bundle_path));
    }

    fs::create_dir_all(&evidence_dir)?;

    let summary_path = evidence_dir.join(SUMMARY_FILE_NAME);
    fs::write(&summary_path, summary_markdown(&request))?;

    let mut artifacts = vec![EvidenceArtifact {
        artifact_type: "summary".to_string(),
        path: SUMMARY_FILE_NAME.to_string(),
        sha256: Some(hash_file(&summary_path)?),
    }];

    if !request.denied_actions.is_empty() {
        let path = evidence_dir.join(DENIED_ACTIONS_FILE_NAME);
        fs::write(&path, request.denied_actions.join("\n"))?;
        artifacts.push(EvidenceArtifact {
            artifact_type: "denied_actions".to_string(),
            path: DENIED_ACTIONS_FILE_NAME.to_string(),
            sha256: Some(hash_file(&path)?),
        });
    }

    if !request.assumptions.is_empty() {
        let path = evidence_dir.join(ASSUMPTIONS_FILE_NAME);
        fs::write(&path, request.assumptions.join("\n"))?;
        artifacts.push(EvidenceArtifact {
            artifact_type: "assumptions".to_string(),
            path: ASSUMPTIONS_FILE_NAME.to_string(),
            sha256: Some(hash_file(&path)?),
        });
    }

    for artifact in &request.extra_artifacts {
        validate_artifact_name(&artifact.file_name)?;
        let path = evidence_dir.join(&artifact.file_name);
        fs::write(&path, &artifact.contents)?;
        artifacts.push(EvidenceArtifact {
            artifact_type: artifact.artifact_type.clone(),
            path: artifact.file_name.clone(),
            sha256: Some(hash_file(&path)?),
        });
    }

    let mut bundle = EvidenceBundle {
        evidence_bundle_id: format!("evb_{}", request.run_id),
        schema_version: EVIDENCE_SCHEMA_VERSION.to_string(),
        created_at: request.created_at,
        gadget: request.gadget,
        run_id: request.run_id.clone(),
        summary: request.summary,
        status: request.status,
        artifacts,
        denied_actions: request.denied_actions,
        assumptions: request.assumptions,
        bundle_hash: None,
    };

    let bundle_hash = compute_bundle_hash(&bundle)?;
    bundle.bundle_hash = Some(bundle_hash.clone());
    fs::write(&bundle_path, serde_yaml::to_string(&bundle)?)?;

    Ok(EvidenceWriteReport {
        run_id: request.run_id,
        evidence_dir,
        bundle_path,
        summary_path,
        bundle_hash,
    })
}

pub fn read_bundle(path: &Path) -> Result<EvidenceBundle, EvidenceError> {
    if !path.exists() {
        return Err(EvidenceError::MissingBundle(path.to_path_buf()));
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&text)?)
}

pub fn summarize_bundle(path: &Path) -> Result<EvidenceSummary, EvidenceError> {
    let bundle = read_bundle(path)?;
    Ok(EvidenceSummary {
        evidence_bundle_id: bundle.evidence_bundle_id,
        run_id: bundle.run_id,
        gadget: bundle.gadget,
        status: bundle.status,
        summary: bundle.summary,
        artifact_count: bundle.artifacts.len(),
        denied_action_count: bundle.denied_actions.len(),
        assumption_count: bundle.assumptions.len(),
        bundle_hash: bundle.bundle_hash,
    })
}

pub fn verify_bundle_hash(path: &Path) -> Result<bool, EvidenceError> {
    let bundle = read_bundle(path)?;
    let evidence_dir = path.parent().unwrap_or_else(|| Path::new("."));

    for artifact in &bundle.artifacts {
        validate_artifact_name(&artifact.path)?;
        let artifact_path = evidence_dir.join(&artifact.path);
        let Some(expected_hash) = artifact.sha256.as_deref() else {
            return Ok(false);
        };
        if !artifact_path.exists() || hash_file(&artifact_path)? != expected_hash {
            return Ok(false);
        }
    }

    let expected = bundle.bundle_hash.clone();
    let actual = compute_bundle_hash(&EvidenceBundle {
        bundle_hash: None,
        ..bundle
    })?;
    Ok(expected.as_deref() == Some(actual.as_str()))
}

pub fn compute_bundle_hash(bundle: &EvidenceBundle) -> Result<String, EvidenceError> {
    let mut canonical = bundle.clone();
    canonical.bundle_hash = None;
    let bytes = serde_yaml::to_string(&canonical)?.into_bytes();
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{}", to_hex(&digest)))
}

fn summary_markdown(request: &EvidenceWriteRequest) -> String {
    let mut out = String::new();
    out.push_str("# Evidence Summary\n\n");
    out.push_str(&format!("Run: {}\n\n", request.run_id));
    out.push_str(&format!("Gadget: {}\n\n", request.gadget));
    out.push_str(&format!("Status: {}\n\n", request.status));
    out.push_str("## Summary\n\n");
    out.push_str(&request.summary);
    out.push('\n');

    if !request.denied_actions.is_empty() {
        out.push_str("\n## Denied Actions\n\n");
        for denied in &request.denied_actions {
            out.push_str("- ");
            out.push_str(denied);
            out.push('\n');
        }
    }

    if !request.assumptions.is_empty() {
        out.push_str("\n## Assumptions\n\n");
        for assumption in &request.assumptions {
            out.push_str("- ");
            out.push_str(assumption);
            out.push('\n');
        }
    }

    out
}

fn hash_file(path: &Path) -> Result<String, EvidenceError> {
    let bytes = fs::read(path)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{}", to_hex(&digest)))
}

fn validate_artifact_name(value: &str) -> Result<(), EvidenceError> {
    if value.is_empty() || value.contains('/') || value.contains('\\') || value.contains("..") {
        return Err(EvidenceError::InvalidArtifactName(value.to_string()));
    }

    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(EvidenceError::InvalidArtifactName(value.to_string()));
    }

    Ok(())
}

fn validate_id(value: &str) -> Result<(), EvidenceError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(EvidenceError::InvalidId(value.to_string()));
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

    fn temp_runs_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "gadgets-evidence-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn writes_and_reads_observe_bundle() {
        let root = temp_runs_root("write-read");
        let mut request = EvidenceWriteRequest::observe(
            "run_1",
            "filesystem.read",
            "2026-05-12T00:00:00Z",
            "Repository inspected successfully.",
        );
        request.denied_actions.push("Denied .env read".to_string());
        request
            .assumptions
            .push("No writes were attempted.".to_string());
        request.extra_artifacts.push(EvidenceTextArtifact::new(
            "files_read",
            "files_read.txt",
            "README.md",
        ));

        let report = create_observe_bundle(&root, request).unwrap();
        assert!(report.bundle_path.exists());
        assert!(report.summary_path.exists());
        assert!(report.bundle_hash.starts_with("sha256:"));

        let summary = summarize_bundle(&report.bundle_path).unwrap();
        assert_eq!(summary.run_id, "run_1");
        assert_eq!(summary.gadget, "filesystem.read");
        assert_eq!(summary.artifact_count, 4);
        assert_eq!(summary.denied_action_count, 1);
        assert_eq!(summary.assumption_count, 1);
        assert!(verify_bundle_hash(&report.bundle_path).unwrap());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_to_overwrite_existing_bundle() {
        let root = temp_runs_root("overwrite");
        let request = EvidenceWriteRequest::observe(
            "run_1",
            "filesystem.read",
            "2026-05-12T00:00:00Z",
            "First bundle.",
        );
        create_observe_bundle(&root, request.clone()).unwrap();
        let err = create_observe_bundle(&root, request).unwrap_err();
        assert!(err.to_string().contains("already exists"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_path_traversal_run_ids() {
        let root = temp_runs_root("bad-id");
        let request = EvidenceWriteRequest::observe(
            "../bad",
            "filesystem.read",
            "2026-05-12T00:00:00Z",
            "Bad bundle.",
        );
        let err = create_observe_bundle(&root, request).unwrap_err();
        assert!(err.to_string().contains("invalid evidence identifier"));
    }

    #[test]
    fn detects_tampered_bundle_metadata() {
        let root = temp_runs_root("tamper");
        let request = EvidenceWriteRequest::observe(
            "run_1",
            "filesystem.read",
            "2026-05-12T00:00:00Z",
            "Original summary.",
        );
        let report = create_observe_bundle(&root, request).unwrap();
        let mut text = fs::read_to_string(&report.bundle_path).unwrap();
        text = text.replace("Original summary.", "Changed summary.");
        fs::write(&report.bundle_path, text).unwrap();
        assert!(!verify_bundle_hash(&report.bundle_path).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detects_tampered_artifact_contents() {
        let root = temp_runs_root("artifact-tamper");
        let mut request = EvidenceWriteRequest::observe(
            "run_1",
            "filesystem.read",
            "2026-05-12T00:00:00Z",
            "Original summary.",
        );
        request.extra_artifacts.push(EvidenceTextArtifact::new(
            "files_read",
            "files_read.txt",
            "README.md",
        ));
        let report = create_observe_bundle(&root, request).unwrap();
        fs::write(report.evidence_dir.join("files_read.txt"), "changed").unwrap();
        assert!(!verify_bundle_hash(&report.bundle_path).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
}
