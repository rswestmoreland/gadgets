//! Non-enforcing pack trust inspection for the Gadgets CLI.
//!
//! These diagnostics may be wrapped by the CLI with evidence and audit output.
//! This module provides non-enforcing trust diagnostics. The signature
//! diagnostic performs cryptographic verification when signed pack metadata
//! and trust-root public keys are available, but it does not mutate trust
//! roots, install packs, download packs, or enforce pack loading.

use crate::manifest_loader::{load_pack_manifest, ManifestLoadError, ManifestSource};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use gadgets_policy::RuntimeMode;
use serde_yaml::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const TRUST_ROOT_RELATIVE_PATH: &str = ".gadgets/trust/trusted_publishers.yaml";
pub const PACK_CONTENTS_FILE_NAME: &str = "pack.contents.yaml";
pub const PACK_SIGNATURE_FILE_NAME: &str = "pack.signature.yaml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackTrustReport {
    pub pack_name: String,
    pub pack_version: String,
    pub source: String,
    pub source_kind: PackSourceKind,
    pub decision: String,
    pub enforcement_enabled: bool,
    pub manifest_sha256: String,
    pub trust_roots_present: bool,
    pub contents_manifest: Option<PackContentsSummary>,
    pub signature: Option<PackSignatureSummary>,
    pub findings: Vec<PackTrustFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackTrustPolicyPreviewReport {
    pub pack: PackTrustReport,
    pub runtime_mode: RuntimeMode,
    pub would_allow_load: bool,
    pub would_require_verified_signature: bool,
    pub would_require_trust_root: bool,
    pub preview_decision: String,
    pub enforcement_active: bool,
    pub signature_metadata_decision: String,
    pub signature_present: bool,
    pub cryptographic_verification_performed: bool,
    pub cryptographic_verification_valid: bool,
    pub content_manifest_valid: bool,
    pub signature_expired: bool,
    pub trust_root_expired: bool,
    pub findings: Vec<PackTrustFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSignatureMetadataReport {
    pub pack: PackTrustReport,
    pub trust_roots: TrustRootsReport,
    pub signature_present: bool,
    pub metadata_valid: bool,
    pub publisher_reference_found: bool,
    pub pack_allowed_by_trust_root: bool,
    pub metadata_decision: String,
    pub cryptographic_verification_performed: bool,
    pub cryptographic_verification_valid: bool,
    pub signature_payload_v1: Option<String>,
    pub content_manifest_valid: bool,
    pub signature_expired: bool,
    pub trust_root_expired: bool,
    pub findings: Vec<PackTrustFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustRootsReport {
    pub path: PathBuf,
    pub exists: bool,
    pub parsed: bool,
    pub version: Option<String>,
    pub publisher_count: usize,
    pub publishers: Vec<TrustedPublisherSummary>,
    pub findings: Vec<PackTrustFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedPublisherSummary {
    pub publisher: Option<String>,
    pub key_id: Option<String>,
    pub algorithm: Option<String>,
    pub public_key_present: bool,
    pub public_key: Option<String>,
    pub allowed_pack_ids: Vec<String>,
    pub allowed_pack_count: usize,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackSourceKind {
    Builtin,
    ProjectLocal,
}

impl PackSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::ProjectLocal => "project_local",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackContentsSummary {
    pub path: PathBuf,
    pub sha256: String,
    pub file_count: usize,
    pub entries: Vec<PackContentEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackContentEntry {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSignatureSummary {
    pub path: PathBuf,
    pub sha256: String,
    pub version: Option<String>,
    pub algorithm: Option<String>,
    pub publisher: Option<String>,
    pub key_id: Option<String>,
    pub pack_id: Option<String>,
    pub pack_version: Option<String>,
    pub manifest_sha256: Option<String>,
    pub contents_sha256: Option<String>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub signature_present: bool,
    pub signature_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackTrustFinding {
    pub severity: PackTrustSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackTrustSeverity {
    Info,
    Warning,
    Error,
}

impl PackTrustSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug)]
pub enum PackTrustError {
    Manifest(ManifestLoadError),
    Io { path: PathBuf, source: std::io::Error },
    Yaml { path: PathBuf, source: serde_yaml::Error },
}

impl fmt::Display for PackTrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(err) => write!(f, "failed to load pack manifest: {err}"),
            Self::Io { path, source } => {
                write!(f, "failed to read pack trust file {}: {source}", path.display())
            }
            Self::Yaml { path, source } => {
                write!(f, "failed to parse pack trust YAML {}: {source}", path.display())
            }
        }
    }
}

impl Error for PackTrustError {}

impl From<ManifestLoadError> for PackTrustError {
    fn from(value: ManifestLoadError) -> Self {
        Self::Manifest(value)
    }
}

pub fn check_pack_trust(
    project_root: &Path,
    pack_name: &str,
) -> Result<PackTrustReport, PackTrustError> {
    let loaded = load_pack_manifest(project_root, pack_name)?;
    let trust_roots_present = project_root.join(TRUST_ROOT_RELATIVE_PATH).exists();
    let mut findings = Vec::new();

    match &loaded.source {
        ManifestSource::Builtin(label) => {
            let manifest_yaml = serde_yaml::to_string(&loaded.manifest).unwrap_or_default();
            findings.push(PackTrustFinding {
                severity: PackTrustSeverity::Info,
                message: "built-in pack is trusted as part of the runtime distribution".to_string(),
            });
            findings.push(PackTrustFinding {
                severity: PackTrustSeverity::Info,
                message: "pack trust enforcement is not enabled for diagnostic commands".to_string(),
            });
            Ok(PackTrustReport {
                pack_name: loaded.manifest.metadata.name,
                pack_version: loaded.manifest.metadata.version,
                source: format!("built-in:{label}"),
                source_kind: PackSourceKind::Builtin,
                decision: "trusted_builtin".to_string(),
                enforcement_enabled: false,
                manifest_sha256: sha256_hex(manifest_yaml.as_bytes()),
                trust_roots_present,
                contents_manifest: None,
                signature: None,
                findings,
            })
        }
        ManifestSource::Project(pack_yaml_path) => {
            let manifest_sha256 = hash_file(pack_yaml_path)?;
            let pack_dir = pack_yaml_path.parent().unwrap_or_else(|| Path::new("."));
            let contents_path = pack_dir.join(PACK_CONTENTS_FILE_NAME);
            let signature_path = pack_dir.join(PACK_SIGNATURE_FILE_NAME);
            let contents_manifest = if contents_path.exists() {
                Some(read_contents_summary(&contents_path, &mut findings)?)
            } else {
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Warning,
                    message: format!("{} is not present", PACK_CONTENTS_FILE_NAME),
                });
                None
            };
            let signature = if signature_path.exists() {
                Some(read_signature_summary(&signature_path)?)
            } else {
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Warning,
                    message: format!("{} is not present", PACK_SIGNATURE_FILE_NAME),
                });
                None
            };

            if let Some(signature) = &signature {
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Warning,
                    message: "signature metadata is present; use pack trust signature or preview for cryptographic diagnostics".to_string(),
                });
                if let Some(expected) = signature.manifest_sha256.as_deref() {
                    if expected != manifest_sha256 {
                        findings.push(PackTrustFinding {
                            severity: PackTrustSeverity::Error,
                            message: "signature manifest_sha256 does not match current pack.yaml"
                                .to_string(),
                        });
                    } else {
                        findings.push(PackTrustFinding {
                            severity: PackTrustSeverity::Info,
                            message: "signature manifest_sha256 matches current pack.yaml".to_string(),
                        });
                    }
                }
                if let (Some(contents), Some(expected)) = (
                    contents_manifest.as_ref(),
                    signature.contents_sha256.as_deref(),
                ) {
                    if expected != contents.sha256 {
                        findings.push(PackTrustFinding {
                            severity: PackTrustSeverity::Error,
                            message: "signature contents_sha256 does not match current pack.contents.yaml"
                                .to_string(),
                        });
                    } else {
                        findings.push(PackTrustFinding {
                            severity: PackTrustSeverity::Info,
                            message: "signature contents_sha256 matches current pack.contents.yaml".to_string(),
                        });
                    }
                }
            }

            if trust_roots_present {
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Info,
                    message: format!(
                        "trust roots file is present at {}",
                        TRUST_ROOT_RELATIVE_PATH
                    ),
                });
            } else {
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Warning,
                    message: format!(
                        "trust roots file is not present at {}",
                        TRUST_ROOT_RELATIVE_PATH
                    ),
                });
            }

            let decision = if signature.is_some() {
                "signed_metadata_unverified"
            } else {
                "allowed_unsigned_local"
            };

            Ok(PackTrustReport {
                pack_name: loaded.manifest.metadata.name,
                pack_version: loaded.manifest.metadata.version,
                source: loaded.source.label(),
                source_kind: PackSourceKind::ProjectLocal,
                decision: decision.to_string(),
                enforcement_enabled: false,
                manifest_sha256,
                trust_roots_present,
                contents_manifest,
                signature,
                findings,
            })
        }
    }
}


pub fn preview_pack_trust_policy(
    project_root: &Path,
    pack_name: &str,
    runtime_mode: RuntimeMode,
) -> Result<PackTrustPolicyPreviewReport, PackTrustError> {
    let signature_report = verify_pack_signature_metadata(project_root, pack_name)?;
    let pack = signature_report.pack.clone();
    let mut findings = signature_report.findings.clone();
    let signature_verified = signature_report.cryptographic_verification_valid
        && signature_report.content_manifest_valid
        && !signature_report.signature_expired
        && !signature_report.trust_root_expired;

    let (would_allow_load, would_require_verified_signature, would_require_trust_root, preview_decision) =
        match (&pack.source_kind, runtime_mode) {
            (PackSourceKind::Builtin, _) => {
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Info,
                    message: "built-in packs would remain trusted as part of the runtime distribution"
                        .to_string(),
                });
                (true, false, false, "trusted_builtin".to_string())
            }
            (PackSourceKind::ProjectLocal, RuntimeMode::Safe) => {
                if signature_verified {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Info,
                        message: "Safe Mode preview found a valid signature, but Safe Mode would still allow local unsigned development packs with warnings".to_string(),
                    });
                    (true, false, false, "safe_allow_verified_local".to_string())
                } else {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Warning,
                        message: "Safe Mode preview would allow this project-local pack for developer workflows even though the signature is not verified".to_string(),
                    });
                    (true, false, false, "safe_allow_unsigned_or_unverified_local".to_string())
                }
            }
            (PackSourceKind::ProjectLocal, RuntimeMode::Team) => {
                if signature_verified {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Info,
                        message: "Team Mode preview would allow this project-local pack because its signature verified against trusted publisher metadata".to_string(),
                    });
                    (true, true, true, "team_allow_verified_signature".to_string())
                } else {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: "Team Mode preview would deny this project-local pack because no valid trusted signature is available".to_string(),
                    });
                    (false, true, true, "team_deny_unverified_signature".to_string())
                }
            }
            (PackSourceKind::ProjectLocal, RuntimeMode::Production) => {
                if signature_verified {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Info,
                        message: "Production Mode preview would allow this project-local pack because its signature verified against trusted publisher metadata".to_string(),
                    });
                    (true, true, true, "production_allow_verified_signature".to_string())
                } else {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: "Production Mode preview would deny this project-local pack unless a valid trusted signature is available".to_string(),
                    });
                    (false, true, true, "production_deny_unverified_signature".to_string())
                }
            }
        };

    findings.push(PackTrustFinding {
        severity: PackTrustSeverity::Info,
        message: "policy preview consumes signature diagnostic results but remains non-enforcing and does not change pack loading behavior"
            .to_string(),
    });

    Ok(PackTrustPolicyPreviewReport {
        pack,
        runtime_mode,
        would_allow_load,
        would_require_verified_signature,
        would_require_trust_root,
        preview_decision,
        enforcement_active: false,
        signature_metadata_decision: signature_report.metadata_decision,
        signature_present: signature_report.signature_present,
        cryptographic_verification_performed: signature_report.cryptographic_verification_performed,
        cryptographic_verification_valid: signature_report.cryptographic_verification_valid,
        content_manifest_valid: signature_report.content_manifest_valid,
        signature_expired: signature_report.signature_expired,
        trust_root_expired: signature_report.trust_root_expired,
        findings,
    })
}

pub fn verify_pack_signature_metadata(
    project_root: &Path,
    pack_name: &str,
) -> Result<PackSignatureMetadataReport, PackTrustError> {
    let pack = check_pack_trust(project_root, pack_name)?;
    let trust_roots = inspect_trust_roots(project_root)?;
    let mut findings = pack.findings.clone();
    let mut metadata_valid = true;
    let mut publisher_reference_found = false;
    let mut pack_allowed_by_trust_root = false;
    let mut cryptographic_verification_valid = false;
    let mut signature_payload_v1 = None;
    let mut signature_expired = false;
    let mut trust_root_expired = false;

    if pack.source_kind == PackSourceKind::Builtin {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Info,
            message: "built-in pack signature metadata is trusted as part of the runtime distribution in this diagnostic scaffold".to_string(),
        });
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Info,
            message: "project-local cryptographic signature verification is not required for built-in packs".to_string(),
        });
        return Ok(PackSignatureMetadataReport {
            pack,
            trust_roots,
            signature_present: false,
            metadata_valid: true,
            publisher_reference_found: false,
            pack_allowed_by_trust_root: false,
            metadata_decision: "builtin_signature_metadata_skipped".to_string(),
            cryptographic_verification_performed: false,
            cryptographic_verification_valid: false,
            signature_payload_v1: None,
            content_manifest_valid: true,
            signature_expired: false,
            trust_root_expired: false,
            findings,
        });
    }

    if trust_roots
        .findings
        .iter()
        .any(|finding| finding.severity == PackTrustSeverity::Error)
    {
        metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "trust-root metadata has errors that prevent valid signature metadata linkage".to_string(),
        });
    }

    let Some(signature) = pack.signature.as_ref() else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: format!("{} is missing", PACK_SIGNATURE_FILE_NAME),
        });
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Info,
            message: "signature metadata verification is diagnostic only and does not block pack loading".to_string(),
        });
        return Ok(PackSignatureMetadataReport {
            pack,
            trust_roots,
            signature_present: false,
            metadata_valid: false,
            publisher_reference_found: false,
            pack_allowed_by_trust_root: false,
            metadata_decision: "signature_metadata_missing".to_string(),
            cryptographic_verification_performed: false,
            cryptographic_verification_valid: false,
            signature_payload_v1: None,
            content_manifest_valid: false,
            signature_expired: false,
            trust_root_expired: false,
            findings,
        });
    };

    require_signature_field("version", signature.version.as_deref(), &mut metadata_valid, &mut findings);
    require_signature_field("algorithm", signature.algorithm.as_deref(), &mut metadata_valid, &mut findings);
    require_signature_field("publisher", signature.publisher.as_deref(), &mut metadata_valid, &mut findings);
    require_signature_field("key_id", signature.key_id.as_deref(), &mut metadata_valid, &mut findings);
    require_signature_field("pack_id", signature.pack_id.as_deref(), &mut metadata_valid, &mut findings);
    require_signature_field("pack_version", signature.pack_version.as_deref(), &mut metadata_valid, &mut findings);
    require_signature_field("manifest_sha256", signature.manifest_sha256.as_deref(), &mut metadata_valid, &mut findings);
    require_signature_field("contents_sha256", signature.contents_sha256.as_deref(), &mut metadata_valid, &mut findings);
    require_signature_field("created_at", signature.created_at.as_deref(), &mut metadata_valid, &mut findings);
    require_signature_field("expires_at", signature.expires_at.as_deref(), &mut metadata_valid, &mut findings);

    if !signature.signature_present {
        metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "signature metadata is missing signature".to_string(),
        });
    }

    if signature.version.as_deref() != Some("1") {
        metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "signature metadata version must be 1".to_string(),
        });
    }

    if signature.algorithm.as_deref() != Some("ed25519") {
        metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "signature metadata algorithm must be ed25519".to_string(),
        });
    }

    if signature.pack_id.as_deref() != Some(pack.pack_name.as_str()) {
        metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "signature pack_id does not match loaded pack name".to_string(),
        });
    }

    if signature.pack_version.as_deref() != Some(pack.pack_version.as_str()) {
        metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "signature pack_version does not match loaded pack version".to_string(),
        });
    }

    if signature.manifest_sha256.as_deref() != Some(pack.manifest_sha256.as_str()) {
        metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "signature manifest_sha256 does not match current pack.yaml".to_string(),
        });
    }

    match (pack.contents_manifest.as_ref(), signature.contents_sha256.as_deref()) {
        (Some(contents), Some(expected)) if expected == contents.sha256 => {}
        (Some(_), Some(_)) => {
            metadata_valid = false;
            findings.push(PackTrustFinding {
                severity: PackTrustSeverity::Error,
                message: "signature contents_sha256 does not match current pack.contents.yaml".to_string(),
            });
        }
        (None, Some(_)) => {
            metadata_valid = false;
            findings.push(PackTrustFinding {
                severity: PackTrustSeverity::Error,
                message: "signature references contents_sha256 but pack.contents.yaml is missing".to_string(),
            });
        }
        (_, None) => {}
    }

    validate_signature_timestamp_field(
        "created_at",
        signature.created_at.as_deref(),
        &mut metadata_valid,
        &mut findings,
    );
    validate_signature_timestamp_field(
        "expires_at",
        signature.expires_at.as_deref(),
        &mut metadata_valid,
        &mut findings,
    );

    if let Some(expires_at) = signature.expires_at.as_deref() {
        match timestamp_expired(expires_at) {
            Some(true) => {
                metadata_valid = false;
                signature_expired = true;
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Error,
                    message: "signature has expired".to_string(),
                });
            }
            Some(false) => {}
            None => {}
        }
    }

    let content_manifest_valid = verify_pack_contents_manifest(&pack, &mut findings);
    if !content_manifest_valid {
        metadata_valid = false;
    }

    let mut matched_publisher: Option<&TrustedPublisherSummary> = None;
    if trust_roots.exists && trust_roots.parsed {
        for publisher in &trust_roots.publishers {
            let publisher_matches = publisher.publisher.as_deref() == signature.publisher.as_deref();
            let key_matches = publisher.key_id.as_deref() == signature.key_id.as_deref();
            let algorithm_matches = publisher.algorithm.as_deref() == signature.algorithm.as_deref();
            if publisher_matches && key_matches && algorithm_matches && publisher.public_key_present {
                publisher_reference_found = true;
                matched_publisher = Some(publisher);
                if publisher
                    .allowed_pack_ids
                    .iter()
                    .any(|allowed| allowed == "*" || allowed == &pack.pack_name)
                {
                    pack_allowed_by_trust_root = true;
                }
                break;
            }
        }
    }

    if publisher_reference_found {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Info,
            message: "signature publisher/key/algorithm has a matching trust-root metadata entry".to_string(),
        });
    } else {
        metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "signature publisher/key/algorithm has no matching trust-root metadata entry".to_string(),
        });
    }

    if pack_allowed_by_trust_root {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Info,
            message: "trust-root metadata allows this pack id".to_string(),
        });
    } else if publisher_reference_found {
        metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "matching trust-root metadata does not allow this pack id".to_string(),
        });
    }

    if let Some(publisher) = matched_publisher {
        if let Some(expires_at) = publisher.expires_at.as_deref() {
            match timestamp_expired(expires_at) {
                Some(true) => {
                    metadata_valid = false;
                    trust_root_expired = true;
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: "matching trust-root publisher key has expired".to_string(),
                    });
                }
                Some(false) => {}
                None => {}
            }
        }
    }

    let mut cryptographic_verification_performed = false;
    if metadata_valid && publisher_reference_found && pack_allowed_by_trust_root {
        if let Some(payload) = build_signature_payload_v1(signature) {
            cryptographic_verification_performed = true;
            cryptographic_verification_valid = verify_ed25519_signature(
                matched_publisher.and_then(|publisher| publisher.public_key.as_deref()),
                signature.signature_value.as_deref(),
                payload.as_bytes(),
                &mut findings,
            );
            signature_payload_v1 = Some(payload);
        } else {
            metadata_valid = false;
            findings.push(PackTrustFinding {
                severity: PackTrustSeverity::Error,
                message: "signature payload v1 could not be built because required fields are missing".to_string(),
            });
        }
    } else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Info,
            message: "cryptographic verification skipped because metadata, trust-root, or content checks did not pass".to_string(),
        });
    }

    findings.push(PackTrustFinding {
        severity: PackTrustSeverity::Info,
        message: "signature verification is diagnostic only and does not enforce pack loading".to_string(),
    });

    let metadata_decision = if cryptographic_verification_valid {
        "trusted_signed"
    } else if signature_expired {
        "denied_expired_signature"
    } else if trust_root_expired {
        "denied_expired_trust_root"
    } else if metadata_valid {
        "denied_signature_mismatch"
    } else {
        "signature_metadata_invalid"
    };

    Ok(PackSignatureMetadataReport {
        pack,
        trust_roots,
        signature_present: true,
        metadata_valid,
        publisher_reference_found,
        pack_allowed_by_trust_root,
        metadata_decision: metadata_decision.to_string(),
        cryptographic_verification_performed,
        cryptographic_verification_valid,
        signature_payload_v1,
        content_manifest_valid,
        signature_expired,
        trust_root_expired,
        findings,
    })
}

pub fn inspect_trust_roots(project_root: &Path) -> Result<TrustRootsReport, PackTrustError> {
    let path = project_root.join(TRUST_ROOT_RELATIVE_PATH);
    let mut findings = Vec::new();

    if !path.exists() {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Warning,
            message: format!("trust roots file is not present at {}", TRUST_ROOT_RELATIVE_PATH),
        });
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Info,
            message: "trust root inspection is diagnostic only and does not enforce signatures"
                .to_string(),
        });
        return Ok(TrustRootsReport {
            path,
            exists: false,
            parsed: false,
            version: None,
            publisher_count: 0,
            publishers: Vec::new(),
            findings,
        });
    }

    let value = read_yaml_value(&path)?;
    let version = yaml_scalar_string(&value, "version");
    if version.as_deref() != Some("1") {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Warning,
            message: "trust roots file version is missing or not 1".to_string(),
        });
    }

    let mut publishers = Vec::new();
    match yaml_get(&value, "trusted_publishers").and_then(Value::as_sequence) {
        Some(entries) => {
            for entry in entries {
                let publisher = yaml_string(entry, "publisher");
                let key_id = yaml_string(entry, "key_id");
                let algorithm = yaml_string(entry, "algorithm");
                let public_key = yaml_string(entry, "public_key");
                let public_key_present = public_key
                    .as_deref()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false);
                let allowed_pack_ids = yaml_get(entry, "allowed_pack_ids")
                    .and_then(Value::as_sequence)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let allowed_pack_count = allowed_pack_ids.len();
                let expires_at = yaml_string(entry, "expires_at");

                if publisher.as_deref().unwrap_or("").trim().is_empty() {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: "trusted publisher entry is missing publisher".to_string(),
                    });
                }
                if key_id.as_deref().unwrap_or("").trim().is_empty() {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: "trusted publisher entry is missing key_id".to_string(),
                    });
                }
                if algorithm.as_deref().unwrap_or("").trim().is_empty() {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: "trusted publisher entry is missing algorithm".to_string(),
                    });
                }
                if !public_key_present {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: "trusted publisher entry is missing public_key".to_string(),
                    });
                }
                if allowed_pack_count == 0 {
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Warning,
                        message: "trusted publisher entry has no allowed_pack_ids".to_string(),
                    });
                }
                if let Some(expires_at_value) = expires_at.as_deref() {
                    if !valid_strict_utc_timestamp(expires_at_value) {
                        findings.push(PackTrustFinding {
                            severity: PackTrustSeverity::Error,
                            message: "trusted publisher entry expires_at is not strict UTC RFC3339 without fractional seconds".to_string(),
                        });
                    }
                }

                publishers.push(TrustedPublisherSummary {
                    publisher,
                    key_id,
                    algorithm,
                    public_key_present,
                    public_key,
                    allowed_pack_ids,
                    allowed_pack_count,
                    expires_at,
                });
            }
        }
        None => findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Warning,
            message: "trusted_publishers list is missing or not a sequence".to_string(),
        }),
    }

    if publishers.is_empty() {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Warning,
            message: "no trusted publishers are configured".to_string(),
        });
    }

    findings.push(PackTrustFinding {
        severity: PackTrustSeverity::Info,
        message: "trust root inspection is diagnostic only and does not verify signatures or mutate trust roots".to_string(),
    });

    Ok(TrustRootsReport {
        path,
        exists: true,
        parsed: true,
        version,
        publisher_count: publishers.len(),
        publishers,
        findings,
    })
}


fn require_signature_field(
    field: &str,
    value: Option<&str>,
    metadata_valid: &mut bool,
    findings: &mut Vec<PackTrustFinding>,
) {
    if value.unwrap_or("").trim().is_empty() {
        *metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: format!("signature metadata is missing {field}"),
        });
    }
}

fn validate_signature_timestamp_field(
    field: &str,
    value: Option<&str>,
    metadata_valid: &mut bool,
    findings: &mut Vec<PackTrustFinding>,
) {
    let Some(value) = value else {
        return;
    };
    if !valid_strict_utc_timestamp(value) {
        *metadata_valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: format!("signature {field} is not strict UTC RFC3339 without fractional seconds"),
        });
    }
}

fn valid_strict_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    let Some(year) = parse_i64_digits(value, 0, 4) else {
        return false;
    };
    let Some(month) = parse_i64_digits(value, 5, 7) else {
        return false;
    };
    let Some(day) = parse_i64_digits(value, 8, 10) else {
        return false;
    };
    let Some(hour) = parse_i64_digits(value, 11, 13) else {
        return false;
    };
    let Some(minute) = parse_i64_digits(value, 14, 16) else {
        return false;
    };
    let Some(second) = parse_i64_digits(value, 17, 19) else {
        return false;
    };
    if !(1..=12).contains(&month) {
        return false;
    }
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    };
    day >= 1 && day <= max_day && hour <= 23 && minute <= 59 && second <= 59
}

fn parse_i64_digits(value: &str, start: usize, end: usize) -> Option<i64> {
    let slice = &value[start..end];
    if !slice.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    slice.parse::<i64>().ok()
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn timestamp_expired(value: &str) -> Option<bool> {
    let expires_at = parse_strict_utc_seconds(value)?;
    Some(current_unix_seconds() >= expires_at)
}

fn parse_strict_utc_seconds(value: &str) -> Option<i64> {
    if !valid_strict_utc_timestamp(value) {
        return None;
    }
    let year = parse_i64_digits(value, 0, 4)?;
    let month = parse_i64_digits(value, 5, 7)?;
    let day = parse_i64_digits(value, 8, 10)?;
    let hour = parse_i64_digits(value, 11, 13)?;
    let minute = parse_i64_digits(value, 14, 16)?;
    let second = parse_i64_digits(value, 17, 19)?;
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
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

fn verify_pack_contents_manifest(
    pack: &PackTrustReport,
    findings: &mut Vec<PackTrustFinding>,
) -> bool {
    let Some(contents) = pack.contents_manifest.as_ref() else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: format!("{} is required for cryptographic verification", PACK_CONTENTS_FILE_NAME),
        });
        return false;
    };

    let pack_root = contents.path.parent().unwrap_or_else(|| Path::new("."));
    let mut valid = true;
    let mut seen_paths = BTreeSet::new();
    let mut previous_path: Option<&str> = None;
    let mut saw_pack_yaml = false;

    for entry in &contents.entries {
        if !safe_relative_pack_path(&entry.path) {
            valid = false;
            findings.push(PackTrustFinding {
                severity: PackTrustSeverity::Error,
                message: format!("content manifest contains unsafe path `{}`", entry.path),
            });
            continue;
        }

        if entry.path == PACK_SIGNATURE_FILE_NAME {
            valid = false;
            findings.push(PackTrustFinding {
                severity: PackTrustSeverity::Error,
                message: "pack.signature.yaml must not be included in signed content manifest entries".to_string(),
            });
        }

        if let Some(previous) = previous_path {
            if previous > entry.path.as_str() {
                valid = false;
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Error,
                    message: "content manifest paths are not sorted in deterministic order".to_string(),
                });
            }
        }
        previous_path = Some(entry.path.as_str());

        if !seen_paths.insert(entry.path.clone()) {
            valid = false;
            findings.push(PackTrustFinding {
                severity: PackTrustSeverity::Error,
                message: format!("content manifest contains duplicate path `{}`", entry.path),
            });
        }

        if entry.path == "pack.yaml" {
            saw_pack_yaml = true;
            if entry.sha256 != pack.manifest_sha256 {
                valid = false;
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Error,
                    message: "content manifest pack.yaml hash does not match loaded pack manifest hash".to_string(),
                });
            }
        }

        let path = pack_root.join(&entry.path);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    valid = false;
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: format!("content manifest path `{}` is a symlink", entry.path),
                    });
                    continue;
                }
                if !metadata.is_file() {
                    valid = false;
                    findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: format!("content manifest path `{}` is not a regular file", entry.path),
                    });
                    continue;
                }
            }
            Err(_) => {
                valid = false;
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Error,
                    message: format!("content manifest path `{}` does not exist", entry.path),
                });
                continue;
            }
        }

        match hash_file(&path) {
            Ok(actual) if actual == entry.sha256 => {}
            Ok(_) => {
                valid = false;
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Error,
                    message: format!("content manifest hash mismatch for `{}`", entry.path),
                });
            }
            Err(_) => {
                valid = false;
                findings.push(PackTrustFinding {
                    severity: PackTrustSeverity::Error,
                    message: format!("failed to hash content manifest path `{}`", entry.path),
                });
            }
        }
    }

    if !saw_pack_yaml {
        valid = false;
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "content manifest must include pack.yaml".to_string(),
        });
    }

    if valid {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Info,
            message: "content manifest file entries and hashes verified".to_string(),
        });
    }

    valid
}

fn build_signature_payload_v1(signature: &PackSignatureSummary) -> Option<String> {
    Some(format!(
        "gadgets-pack-signature-v1\nalgorithm:{}\npublisher:{}\nkey_id:{}\npack_id:{}\npack_version:{}\nmanifest_sha256:{}\ncontents_sha256:{}\ncreated_at:{}\nexpires_at:{}\n",
        required_signature_value(signature.algorithm.as_deref())?,
        required_signature_value(signature.publisher.as_deref())?,
        required_signature_value(signature.key_id.as_deref())?,
        required_signature_value(signature.pack_id.as_deref())?,
        required_signature_value(signature.pack_version.as_deref())?,
        required_signature_value(signature.manifest_sha256.as_deref())?,
        required_signature_value(signature.contents_sha256.as_deref())?,
        required_signature_value(signature.created_at.as_deref())?,
        required_signature_value(signature.expires_at.as_deref())?,
    ))
}

fn required_signature_value(value: Option<&str>) -> Option<&str> {
    let value = value?;
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn verify_ed25519_signature(
    public_key_value: Option<&str>,
    signature_value: Option<&str>,
    payload: &[u8],
    findings: &mut Vec<PackTrustFinding>,
) -> bool {
    let Some(public_key_value) = public_key_value else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "cryptographic verification failed because trust-root public key is missing".to_string(),
        });
        return false;
    };
    let Some(signature_value) = signature_value else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "cryptographic verification failed because signature value is missing".to_string(),
        });
        return false;
    };

    let Ok(public_key_bytes) = BASE64_STANDARD.decode(public_key_value) else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "cryptographic verification failed because trust-root public key is not valid base64".to_string(),
        });
        return false;
    };
    let Ok(public_key_bytes) = <[u8; 32]>::try_from(public_key_bytes.as_slice()) else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "cryptographic verification failed because trust-root public key is not 32 bytes".to_string(),
        });
        return false;
    };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&public_key_bytes) else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "cryptographic verification failed because trust-root public key is invalid".to_string(),
        });
        return false;
    };

    let Ok(signature_bytes) = BASE64_STANDARD.decode(signature_value) else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "cryptographic verification failed because signature is not valid base64".to_string(),
        });
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(signature_bytes.as_slice()) else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "cryptographic verification failed because signature is not 64 bytes".to_string(),
        });
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);

    if verifying_key.verify(payload, &signature).is_ok() {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Info,
            message: "Ed25519 signature verified for signature payload v1".to_string(),
        });
        true
    } else {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Error,
            message: "Ed25519 signature verification failed for signature payload v1".to_string(),
        });
        false
    }
}

fn read_contents_summary(
    path: &Path,
    findings: &mut Vec<PackTrustFinding>,
) -> Result<PackContentsSummary, PackTrustError> {
    let value = read_yaml_value(path)?;
    let mut entries = Vec::new();
    match yaml_get(&value, "files").and_then(Value::as_sequence) {
        Some(files) => {
            for file in files {
                let file_path = yaml_string(file, "path");
                let file_sha256 = yaml_string(file, "sha256");
                match (file_path, file_sha256) {
                    (Some(file_path), Some(file_sha256)) => {
                        if !safe_relative_pack_path(&file_path) {
                            findings.push(PackTrustFinding {
                                severity: PackTrustSeverity::Error,
                                message: format!(
                                    "pack.contents.yaml contains unsafe path `{file_path}`"
                                ),
                            });
                        }
                        entries.push(PackContentEntry {
                            path: file_path,
                            sha256: file_sha256,
                        });
                    }
                    _ => findings.push(PackTrustFinding {
                        severity: PackTrustSeverity::Error,
                        message: "pack.contents.yaml file entry is missing path or sha256".to_string(),
                    }),
                }
            }
        }
        None => findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Warning,
            message: "pack.contents.yaml files list is missing or not a sequence".to_string(),
        }),
    }

    if entries.is_empty() {
        findings.push(PackTrustFinding {
            severity: PackTrustSeverity::Warning,
            message: "pack.contents.yaml contains no file entries".to_string(),
        });
    }

    Ok(PackContentsSummary {
        path: path.to_path_buf(),
        sha256: hash_file(path)?,
        file_count: entries.len(),
        entries,
    })
}

fn read_signature_summary(path: &Path) -> Result<PackSignatureSummary, PackTrustError> {
    let value = read_yaml_value(path)?;
    Ok(PackSignatureSummary {
        path: path.to_path_buf(),
        sha256: hash_file(path)?,
        version: yaml_scalar_string(&value, "version"),
        algorithm: yaml_string(&value, "algorithm"),
        publisher: yaml_string(&value, "publisher"),
        key_id: yaml_string(&value, "key_id"),
        pack_id: yaml_string(&value, "pack_id"),
        pack_version: yaml_string(&value, "pack_version"),
        manifest_sha256: yaml_string(&value, "manifest_sha256"),
        contents_sha256: yaml_string(&value, "contents_sha256"),
        created_at: yaml_string(&value, "created_at"),
        expires_at: yaml_string(&value, "expires_at"),
        signature_present: yaml_get(&value, "signature")
            .and_then(Value::as_str)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
        signature_value: yaml_string(&value, "signature"),
    })
}

fn read_yaml_value(path: &Path) -> Result<Value, PackTrustError> {
    let contents = fs::read_to_string(path).map_err(|source| PackTrustError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_yaml::from_str(&contents).map_err(|source| PackTrustError::Yaml {
        path: path.to_path_buf(),
        source,
    })
}

fn yaml_string(value: &Value, key: &str) -> Option<String> {
    yaml_get(value, key).and_then(Value::as_str).map(str::to_string)
}

fn yaml_scalar_string(value: &Value, key: &str) -> Option<String> {
    yaml_get(value, key).and_then(|value| {
        value
            .as_str()
            .map(str::to_string)
            .or_else(|| value.as_i64().map(|number| number.to_string()))
    })
}

fn yaml_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(&Value::String(key.to_string())))
}

fn hash_file(path: &Path) -> Result<String, PackTrustError> {
    let bytes = fs::read(path).map_err(|source| PackTrustError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
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

fn safe_relative_pack_path(value: &str) -> bool {
    let path = Path::new(value);
    if value.trim().is_empty() || path.is_absolute() {
        return false;
    }
    path.components().all(|component| {
        matches!(
            component,
            Component::Normal(_) | Component::CurDir
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builtin_developer_pack_reports_trusted_builtin() {
        let root = std::env::temp_dir();
        let report = check_pack_trust(&root, "developer").unwrap();
        assert_eq!(report.decision, "trusted_builtin");
        assert_eq!(report.source_kind, PackSourceKind::Builtin);
        assert!(!report.enforcement_enabled);
    }

    #[test]
    fn project_pack_without_signature_reports_unsigned_local() {
        let root = unique_temp_root("gadgets-trust-unsigned");
        let pack_dir = root.join(".gadgets/packs/local-pack/gadgets");
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(
            root.join(".gadgets/packs/local-pack/pack.yaml"),
            r#"schema_version: gadgets.framework/pack/v0.1
kind: GadgetPack
metadata:
  name: local-pack
  version: 0.1.0
  display_name: Local Pack
  description: Local pack.
default_mode: safe
gadgets:
  - filesystem.read
"#,
        )
        .unwrap();

        let report = check_pack_trust(&root, "local-pack").unwrap();
        assert_eq!(report.decision, "allowed_unsigned_local");
        assert_eq!(report.source_kind, PackSourceKind::ProjectLocal);
        assert!(report.signature.is_none());
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.message.contains("pack.signature.yaml is not present")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_parent_traversal_in_content_manifest_paths() {
        let root = unique_temp_root("gadgets-trust-contents");
        let pack_dir = root.join(".gadgets/packs/local-pack");
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(
            pack_dir.join("pack.yaml"),
            r#"schema_version: gadgets.framework/pack/v0.1
kind: GadgetPack
metadata:
  name: local-pack
  version: 0.1.0
  display_name: Local Pack
  description: Local pack.
default_mode: safe
gadgets:
  - filesystem.read
"#,
        )
        .unwrap();
        fs::write(
            pack_dir.join(PACK_CONTENTS_FILE_NAME),
            r#"version: 1
pack_id: local-pack
files:
  - path: ../secret
    sha256: sha256:abc
"#,
        )
        .unwrap();

        let report = check_pack_trust(&root, "local-pack").unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.severity == PackTrustSeverity::Error));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_trust_roots_reports_absent() {
        let root = unique_temp_root("gadgets-trust-roots-missing");
        fs::create_dir_all(&root).unwrap();
        let report = inspect_trust_roots(&root).unwrap();
        assert!(!report.exists);
        assert!(!report.parsed);
        assert_eq!(report.publisher_count, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn trust_roots_reports_publishers() {
        let root = unique_temp_root("gadgets-trust-roots-present");
        let trust_dir = root.join(".gadgets/trust");
        fs::create_dir_all(&trust_dir).unwrap();
        fs::write(
            trust_dir.join("trusted_publishers.yaml"),
            r#"version: 1
trusted_publishers:
  - publisher: gadgets-framework
    key_id: test-key
    algorithm: ed25519
    public_key: {public_key}
    allowed_pack_ids:
      - developer
    expires_at: 2999-01-01T00:00:00Z
"#,
        )
        .unwrap();

        let report = inspect_trust_roots(&root).unwrap();
        assert!(report.exists);
        assert!(report.parsed);
        assert_eq!(report.version.as_deref(), Some("1"));
        assert_eq!(report.publisher_count, 1);
        assert_eq!(report.publishers[0].publisher.as_deref(), Some("gadgets-framework"));
        let _ = fs::remove_dir_all(root);
    }


    #[test]
    fn safe_mode_preview_allows_unsigned_local_diagnostics() {
        let root = unique_temp_root("gadgets-trust-preview-safe");
        let pack_dir = root.join(".gadgets/packs/local-pack/gadgets");
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(
            root.join(".gadgets/packs/local-pack/pack.yaml"),
            r#"schema_version: gadgets.framework/pack/v0.1
kind: GadgetPack
metadata:
  name: local-pack
  version: 0.1.0
  display_name: Local Pack
  description: Local pack.
default_mode: safe
gadgets:
  - filesystem.read
"#,
        )
        .unwrap();

        let report = preview_pack_trust_policy(&root, "local-pack", RuntimeMode::Safe).unwrap();
        assert!(report.would_allow_load);
        assert!(!report.would_require_verified_signature);
        assert_eq!(report.preview_decision, "safe_allow_unsigned_or_unverified_local");
        assert!(!report.enforcement_active);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn production_preview_denies_unsigned_local_pack() {
        let root = unique_temp_root("gadgets-trust-preview-production");
        let pack_dir = root.join(".gadgets/packs/local-pack/gadgets");
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(
            root.join(".gadgets/packs/local-pack/pack.yaml"),
            r#"schema_version: gadgets.framework/pack/v0.1
kind: GadgetPack
metadata:
  name: local-pack
  version: 0.1.0
  display_name: Local Pack
  description: Local pack.
default_mode: safe
gadgets:
  - filesystem.read
"#,
        )
        .unwrap();

        let report = preview_pack_trust_policy(&root, "local-pack", RuntimeMode::Production).unwrap();
        assert!(!report.would_allow_load);
        assert!(report.would_require_verified_signature);
        assert!(report.would_require_trust_root);
        assert_eq!(report.preview_decision, "production_deny_unverified_signature");
        assert!(!report.enforcement_active);
        let _ = fs::remove_dir_all(root);
    }


    #[test]
    fn signature_metadata_report_validates_shape_and_trust_root_reference() {
        let root = unique_temp_root("gadgets-trust-signature-valid");
        let pack_dir = root.join(".gadgets/packs/local-pack");
        fs::create_dir_all(&pack_dir).unwrap();
        let pack_yaml_path = pack_dir.join("pack.yaml");
        fs::write(
            &pack_yaml_path,
            r#"schema_version: gadgets.framework/pack/v0.1
kind: GadgetPack
metadata:
  name: local-pack
  version: 0.1.0
  display_name: Local Pack
  description: Local pack.
default_mode: safe
gadgets:
  - filesystem.read
"#,
        )
        .unwrap();
        let contents_path = pack_dir.join(PACK_CONTENTS_FILE_NAME);
        let manifest_hash = hash_file(&pack_yaml_path).unwrap();
        fs::write(
            &contents_path,
            format!(
                "version: 1
pack_id: local-pack
files:
  - path: pack.yaml
    sha256: {manifest_hash}
"
            ),
        )
        .unwrap();
        let contents_hash = hash_file(&contents_path).unwrap();
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let signature_payload = format!(
            "gadgets-pack-signature-v1\nalgorithm:ed25519\npublisher:local-publisher\nkey_id:local-key\npack_id:local-pack\npack_version:0.1.0\nmanifest_sha256:{manifest_hash}\ncontents_sha256:{contents_hash}\ncreated_at:2026-05-13T00:00:00Z\nexpires_at:2999-01-01T00:00:00Z\n"
        );
        let signature = signing_key.sign(signature_payload.as_bytes());
        let public_key = BASE64_STANDARD.encode(verifying_key.to_bytes());
        let signature_value = BASE64_STANDARD.encode(signature.to_bytes());
        fs::write(
            pack_dir.join(PACK_SIGNATURE_FILE_NAME),
            format!(
                "version: 1
algorithm: ed25519
publisher: local-publisher
key_id: local-key
pack_id: local-pack
pack_version: 0.1.0
manifest_sha256: {manifest_hash}
contents_sha256: {contents_hash}
created_at: 2026-05-13T00:00:00Z
expires_at: 2999-01-01T00:00:00Z
signature: {signature_value}
"
            ),
        )
        .unwrap();
        let trust_dir = root.join(".gadgets/trust");
        fs::create_dir_all(&trust_dir).unwrap();
        fs::write(
            trust_dir.join("trusted_publishers.yaml"),
            format!(
                "version: 1
trusted_publishers:
  - publisher: local-publisher
    key_id: local-key
    algorithm: ed25519
    public_key: {public_key}
    allowed_pack_ids:
      - local-pack
    expires_at: 2999-01-01T00:00:00Z
"
            ),
        )
        .unwrap();

        let report = verify_pack_signature_metadata(&root, "local-pack").unwrap();
        assert!(report.signature_present);
        assert!(report.metadata_valid);
        assert!(report.publisher_reference_found);
        assert!(report.pack_allowed_by_trust_root);
        assert!(report.cryptographic_verification_performed);
        assert!(report.cryptographic_verification_valid);
        assert!(report.content_manifest_valid);
        assert_eq!(report.metadata_decision, "trusted_signed");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn signature_metadata_rejects_invalid_timestamp_shape() {
        let root = unique_temp_root("gadgets-trust-signature-invalid-time");
        let pack_dir = root.join(".gadgets/packs/local-pack");
        fs::create_dir_all(&pack_dir).unwrap();
        let pack_yaml_path = pack_dir.join("pack.yaml");
        fs::write(
            &pack_yaml_path,
            r#"schema_version: gadgets.framework/pack/v0.1
kind: GadgetPack
metadata:
  name: local-pack
  version: 0.1.0
  display_name: Local Pack
  description: Local pack.
default_mode: safe
gadgets:
  - filesystem.read
"#,
        )
        .unwrap();
        let contents_path = pack_dir.join(PACK_CONTENTS_FILE_NAME);
        fs::write(
            &contents_path,
            r#"version: 1
pack_id: local-pack
files:
  - path: pack.yaml
    sha256: sha256:test
"#,
        )
        .unwrap();
        let manifest_hash = hash_file(&pack_yaml_path).unwrap();
        let contents_hash = hash_file(&contents_path).unwrap();
        fs::write(
            pack_dir.join(PACK_SIGNATURE_FILE_NAME),
            format!(
                "version: 1
algorithm: ed25519
publisher: local-publisher
key_id: local-key
pack_id: local-pack
pack_version: 0.1.0
manifest_sha256: {manifest_hash}
contents_sha256: {contents_hash}
created_at: 2026-05-13T00:00:00+00:00
expires_at: 2999-01-01T00:00:00Z
signature: test-signature
"
            ),
        )
        .unwrap();
        let trust_dir = root.join(".gadgets/trust");
        fs::create_dir_all(&trust_dir).unwrap();
        fs::write(
            trust_dir.join("trusted_publishers.yaml"),
            r#"version: 1
trusted_publishers:
  - publisher: local-publisher
    key_id: local-key
    algorithm: ed25519
    public_key: test-public-key
    allowed_pack_ids:
      - local-pack
"#,
        )
        .unwrap();

        let report = verify_pack_signature_metadata(&root, "local-pack").unwrap();
        assert!(!report.metadata_valid);
        assert_eq!(report.metadata_decision, "signature_metadata_invalid");
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.message.contains("created_at")));
        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_root(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }
}
