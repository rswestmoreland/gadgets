//! Pack and Gadget manifest loading for the Gadgets CLI.
//!
//! Step 11 adds pack validation. The loader can read project-local pack and
//! Gadget manifests from `.gadgets/`, then fall back to built-in manifests.
//! Validation reports missing manifests as warnings by default and as errors in
//! strict mode.

use gadgets_core::{GadgetManifest, PackManifest, PermissionLevel};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub const DEVELOPER_PACK: &str = "developer";
pub const FILESYSTEM_READ_GADGET: &str = "filesystem.read";
pub const PATCH_WRITER_GADGET: &str = "patch.writer";
pub const TEST_RUNNER_GADGET: &str = "test.runner";
pub const GIT_PR_GADGET: &str = "git.pr";

const BUILTIN_DEVELOPER_PACK: &str = include_str!("../../../packs/developer/pack.yaml");
const BUILTIN_LINUX_ADMIN_OBSERVE_PACK: &str =
    include_str!("../../../packs/linux-admin-observe/pack.yaml");
const BUILTIN_LINUX_ADMIN_CHANGE_PACK: &str =
    include_str!("../../../packs/linux-admin-change/pack.yaml");

const BUILTIN_COORDINATOR_GADGET: &str =
    include_str!("../../../packs/developer/gadgets/coordinator.yaml");
const BUILTIN_POLICY_GADGET: &str = include_str!("../../../packs/developer/gadgets/policy.yaml");
const BUILTIN_AUDIT_LEDGER_GADGET: &str =
    include_str!("../../../packs/developer/gadgets/audit.ledger.yaml");
const BUILTIN_APPROVAL_GADGET: &str =
    include_str!("../../../packs/developer/gadgets/approval.yaml");
const BUILTIN_FILESYSTEM_READ_GADGET: &str =
    include_str!("../../../packs/developer/gadgets/filesystem.read.yaml");
const BUILTIN_PATCH_WRITER_GADGET: &str =
    include_str!("../../../packs/developer/gadgets/patch.writer.yaml");
const BUILTIN_TEST_RUNNER_GADGET: &str =
    include_str!("../../../packs/developer/gadgets/test.runner.yaml");
const BUILTIN_GIT_PR_GADGET: &str = include_str!("../../../packs/developer/gadgets/git.pr.yaml");
const BUILTIN_DOCUMENTATION_WRITER_GADGET: &str =
    include_str!("../../../packs/developer/gadgets/documentation.writer.yaml");
const BUILTIN_SECRETS_GUARDIAN_GADGET: &str =
    include_str!("../../../packs/developer/gadgets/secrets.guardian.yaml");

#[derive(Debug, Clone)]
pub struct LoadedPackManifest {
    pub manifest: PackManifest,
    pub source: ManifestSource,
}

#[derive(Debug, Clone)]
pub struct LoadedGadgetManifest {
    pub manifest: GadgetManifest,
    pub source: ManifestSource,
    #[allow(dead_code)]
    pub pack_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestSource {
    Project(PathBuf),
    Builtin(&'static str),
}

impl ManifestSource {
    pub fn label(&self) -> String {
        match self {
            Self::Project(path) => path.display().to_string(),
            Self::Builtin(label) => format!("built-in:{label}"),
        }
    }
}

#[derive(Debug)]
pub enum ManifestLoadError {
    NoInstalledPacks,
    PackNotInstalled(String),
    PackNotFound(String),
    GadgetNotDeclared {
        pack: String,
        gadget: String,
    },
    GadgetNotFound {
        pack: String,
        gadget: String,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    PackParse {
        source: String,
        error: String,
    },
    GadgetParse {
        source: String,
        error: String,
    },
}

impl fmt::Display for ManifestLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoInstalledPacks => write!(
                f,
                "no installed packs are configured; run `gadgets init` or add `installed_packs` to .gadgets/config.yaml"
            ),
            Self::PackNotInstalled(pack) => write!(
                f,
                "pack `{pack}` is not listed in .gadgets/config.yaml installed_packs"
            ),
            Self::PackNotFound(pack) => write!(
                f,
                "pack manifest for `{pack}` was not found in .gadgets/packs or built-in packs"
            ),
            Self::GadgetNotDeclared { pack, gadget } => write!(
                f,
                "Gadget `{gadget}` is not declared by pack `{pack}`"
            ),
            Self::GadgetNotFound { pack, gadget } => write!(
                f,
                "Gadget manifest `{gadget}` for pack `{pack}` was not found in .gadgets overrides or built-in manifests"
            ),
            Self::Io { path, source } => {
                write!(f, "failed to read manifest at {}: {source}", path.display())
            }
            Self::PackParse { source, error } => {
                write!(f, "failed to parse pack manifest from {source}: {error}")
            }
            Self::GadgetParse { source, error } => {
                write!(f, "failed to parse Gadget manifest from {source}: {error}")
            }
        }
    }
}

impl Error for ManifestLoadError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

impl ValidationSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackValidationIssue {
    pub severity: ValidationSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GadgetValidationRow {
    pub name: String,
    pub status: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackValidationReport {
    #[allow(dead_code)]
    pub pack_name: String,
    pub pack_source: String,
    pub strict: bool,
    pub gadgets_checked: usize,
    pub gadgets_valid: usize,
    pub gadgets_missing: usize,
    pub gadget_rows: Vec<GadgetValidationRow>,
    pub issues: Vec<PackValidationIssue>,
}

impl PackValidationReport {
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error)
    }
}

pub fn ensure_pack_installed(
    installed_packs: &[String],
    pack_name: &str,
) -> Result<(), ManifestLoadError> {
    if installed_packs.is_empty() {
        return Err(ManifestLoadError::NoInstalledPacks);
    }

    if installed_packs
        .iter()
        .any(|installed| installed == pack_name)
    {
        return Ok(());
    }

    Err(ManifestLoadError::PackNotInstalled(pack_name.to_string()))
}

pub fn load_installed_pack_manifests(
    project_root: &Path,
    installed_packs: &[String],
) -> Result<Vec<LoadedPackManifest>, ManifestLoadError> {
    if installed_packs.is_empty() {
        return Err(ManifestLoadError::NoInstalledPacks);
    }

    installed_packs
        .iter()
        .map(|pack| load_pack_manifest(project_root, pack))
        .collect()
}

pub fn validate_installed_packs(
    project_root: &Path,
    installed_packs: &[String],
    strict: bool,
) -> Result<Vec<PackValidationReport>, ManifestLoadError> {
    if installed_packs.is_empty() {
        return Err(ManifestLoadError::NoInstalledPacks);
    }

    installed_packs
        .iter()
        .map(|pack| validate_pack_tree(project_root, pack, strict))
        .collect()
}

pub fn validate_pack_tree(
    project_root: &Path,
    pack_name: &str,
    strict: bool,
) -> Result<PackValidationReport, ManifestLoadError> {
    let loaded_pack = load_pack_manifest(project_root, pack_name)?;
    let highest_level = loaded_pack
        .manifest
        .safety
        .highest_permission_level
        .as_deref()
        .and_then(|value| PermissionLevel::from_str(value).ok());

    let mut report = PackValidationReport {
        pack_name: loaded_pack.manifest.metadata.name.clone(),
        pack_source: loaded_pack.source.label(),
        strict,
        gadgets_checked: loaded_pack.manifest.gadgets.len(),
        gadgets_valid: 0,
        gadgets_missing: 0,
        gadget_rows: Vec::new(),
        issues: Vec::new(),
    };

    if loaded_pack
        .manifest
        .safety
        .highest_permission_level
        .is_some()
        && highest_level.is_none()
    {
        report.issues.push(PackValidationIssue {
            severity: ValidationSeverity::Error,
            message: format!(
                "pack `{}` has invalid safety.highest_permission_level `{}`",
                loaded_pack.manifest.metadata.name,
                loaded_pack
                    .manifest
                    .safety
                    .highest_permission_level
                    .as_deref()
                    .unwrap_or_default()
            ),
        });
    }

    for gadget_name in &loaded_pack.manifest.gadgets {
        match load_gadget_manifest(project_root, &loaded_pack, gadget_name) {
            Ok(loaded_gadget) => {
                report.gadgets_valid += 1;
                report.gadget_rows.push(GadgetValidationRow {
                    name: gadget_name.clone(),
                    status: "valid".to_string(),
                    source: Some(loaded_gadget.source.label()),
                });

                if loaded_gadget.manifest.metadata.name != *gadget_name {
                    report.issues.push(PackValidationIssue {
                        severity: ValidationSeverity::Error,
                        message: format!(
                            "pack `{}` declares Gadget `{}`, but loaded manifest metadata.name is `{}`",
                            loaded_pack.manifest.metadata.name,
                            gadget_name,
                            loaded_gadget.manifest.metadata.name
                        ),
                    });
                }

                if let Some(level) = highest_level {
                    if loaded_gadget.manifest.permission_level > level {
                        report.issues.push(PackValidationIssue {
                            severity: ValidationSeverity::Error,
                            message: format!(
                                "Gadget `{}` permission level `{}` exceeds pack safety highest level `{}`",
                                gadget_name,
                                loaded_gadget.manifest.permission_level,
                                level
                            ),
                        });
                    }
                }
            }
            Err(ManifestLoadError::GadgetNotFound { .. }) => {
                report.gadgets_missing += 1;
                report.gadget_rows.push(GadgetValidationRow {
                    name: gadget_name.clone(),
                    status: "missing".to_string(),
                    source: None,
                });
                report.issues.push(PackValidationIssue {
                    severity: if strict {
                        ValidationSeverity::Error
                    } else {
                        ValidationSeverity::Warning
                    },
                    message: format!(
                        "pack `{}` declares Gadget `{}`, but no Gadget manifest was found",
                        loaded_pack.manifest.metadata.name, gadget_name
                    ),
                });
            }
            Err(err) => {
                report.gadget_rows.push(GadgetValidationRow {
                    name: gadget_name.clone(),
                    status: "invalid".to_string(),
                    source: None,
                });
                report.issues.push(PackValidationIssue {
                    severity: ValidationSeverity::Error,
                    message: format!("Gadget `{gadget_name}` failed validation: {err}"),
                });
            }
        }
    }

    Ok(report)
}

pub fn load_pack_manifest(
    project_root: &Path,
    pack_name: &str,
) -> Result<LoadedPackManifest, ManifestLoadError> {
    let project_path = project_pack_manifest_path(project_root, pack_name);
    if project_path.exists() {
        let yaml = fs::read_to_string(&project_path).map_err(|source| ManifestLoadError::Io {
            path: project_path.clone(),
            source,
        })?;
        let source = ManifestSource::Project(project_path);
        let manifest = parse_pack_manifest(&yaml, &source)?;
        return Ok(LoadedPackManifest { manifest, source });
    }

    let Some((label, yaml)) = builtin_pack_yaml(pack_name) else {
        return Err(ManifestLoadError::PackNotFound(pack_name.to_string()));
    };
    let source = ManifestSource::Builtin(label);
    let manifest = parse_pack_manifest(yaml, &source)?;
    Ok(LoadedPackManifest { manifest, source })
}

pub fn load_gadget_manifest(
    project_root: &Path,
    loaded_pack: &LoadedPackManifest,
    gadget_name: &str,
) -> Result<LoadedGadgetManifest, ManifestLoadError> {
    let pack_name = loaded_pack.manifest.metadata.name.clone();
    if !loaded_pack
        .manifest
        .gadgets
        .iter()
        .any(|gadget| gadget == gadget_name)
    {
        return Err(ManifestLoadError::GadgetNotDeclared {
            pack: pack_name,
            gadget: gadget_name.to_string(),
        });
    }

    for path in project_gadget_manifest_paths(
        project_root,
        &loaded_pack.manifest.metadata.name,
        gadget_name,
    ) {
        if path.exists() {
            let yaml = fs::read_to_string(&path).map_err(|source| ManifestLoadError::Io {
                path: path.clone(),
                source,
            })?;
            let source = ManifestSource::Project(path);
            let manifest = parse_gadget_manifest(&yaml, &source)?;
            return Ok(LoadedGadgetManifest {
                manifest,
                source,
                pack_name: loaded_pack.manifest.metadata.name.clone(),
            });
        }
    }

    if let Some((label, yaml)) =
        builtin_gadget_yaml(&loaded_pack.manifest.metadata.name, gadget_name)
    {
        let source = ManifestSource::Builtin(label);
        let manifest = parse_gadget_manifest(yaml, &source)?;
        return Ok(LoadedGadgetManifest {
            manifest,
            source,
            pack_name: loaded_pack.manifest.metadata.name.clone(),
        });
    }

    Err(ManifestLoadError::GadgetNotFound {
        pack: loaded_pack.manifest.metadata.name.clone(),
        gadget: gadget_name.to_string(),
    })
}

pub fn gadget_manifest_available(
    project_root: &Path,
    loaded_pack: &LoadedPackManifest,
    gadget_name: &str,
) -> bool {
    load_gadget_manifest(project_root, loaded_pack, gadget_name).is_ok()
}

fn project_pack_manifest_path(project_root: &Path, pack_name: &str) -> PathBuf {
    project_root
        .join(".gadgets")
        .join("packs")
        .join(pack_name)
        .join("pack.yaml")
}

fn project_gadget_manifest_paths(
    project_root: &Path,
    pack_name: &str,
    gadget_name: &str,
) -> Vec<PathBuf> {
    vec![
        project_root
            .join(".gadgets")
            .join("packs")
            .join(pack_name)
            .join("gadgets")
            .join(format!("{gadget_name}.yaml")),
        project_root
            .join(".gadgets")
            .join("gadgets")
            .join(format!("{gadget_name}.yaml")),
    ]
}

fn builtin_pack_yaml(pack_name: &str) -> Option<(&'static str, &'static str)> {
    match pack_name {
        DEVELOPER_PACK => Some(("developer/pack.yaml", BUILTIN_DEVELOPER_PACK)),
        "linux-admin-observe" => Some((
            "linux-admin-observe/pack.yaml",
            BUILTIN_LINUX_ADMIN_OBSERVE_PACK,
        )),
        "linux-admin-change" => Some((
            "linux-admin-change/pack.yaml",
            BUILTIN_LINUX_ADMIN_CHANGE_PACK,
        )),
        _ => None,
    }
}

fn builtin_gadget_yaml(pack_name: &str, gadget_name: &str) -> Option<(&'static str, &'static str)> {
    if pack_name != DEVELOPER_PACK {
        return None;
    }

    match gadget_name {
        "coordinator" => Some((
            "developer/gadgets/coordinator.yaml",
            BUILTIN_COORDINATOR_GADGET,
        )),
        "policy" => Some(("developer/gadgets/policy.yaml", BUILTIN_POLICY_GADGET)),
        "audit.ledger" => Some((
            "developer/gadgets/audit.ledger.yaml",
            BUILTIN_AUDIT_LEDGER_GADGET,
        )),
        "approval" => Some(("developer/gadgets/approval.yaml", BUILTIN_APPROVAL_GADGET)),
        FILESYSTEM_READ_GADGET => Some((
            "developer/gadgets/filesystem.read.yaml",
            BUILTIN_FILESYSTEM_READ_GADGET,
        )),
        PATCH_WRITER_GADGET => Some((
            "developer/gadgets/patch.writer.yaml",
            BUILTIN_PATCH_WRITER_GADGET,
        )),
        TEST_RUNNER_GADGET => Some((
            "developer/gadgets/test.runner.yaml",
            BUILTIN_TEST_RUNNER_GADGET,
        )),
        GIT_PR_GADGET => Some(("developer/gadgets/git.pr.yaml", BUILTIN_GIT_PR_GADGET)),
        "documentation.writer" => Some((
            "developer/gadgets/documentation.writer.yaml",
            BUILTIN_DOCUMENTATION_WRITER_GADGET,
        )),
        "secrets.guardian" => Some((
            "developer/gadgets/secrets.guardian.yaml",
            BUILTIN_SECRETS_GUARDIAN_GADGET,
        )),
        _ => None,
    }
}

fn parse_pack_manifest(
    yaml: &str,
    source: &ManifestSource,
) -> Result<PackManifest, ManifestLoadError> {
    PackManifest::from_yaml_str(yaml).map_err(|error| ManifestLoadError::PackParse {
        source: source.label(),
        error: error.to_string(),
    })
}

fn parse_gadget_manifest(
    yaml: &str,
    source: &ManifestSource,
) -> Result<GadgetManifest, ManifestLoadError> {
    GadgetManifest::from_yaml_str(yaml).map_err(|error| ManifestLoadError::GadgetParse {
        source: source.label(),
        error: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn loads_builtin_developer_pack() {
        let root = std::env::temp_dir();
        let pack = load_pack_manifest(&root, DEVELOPER_PACK).unwrap();
        assert_eq!(pack.manifest.metadata.name, DEVELOPER_PACK);
        assert!(matches!(pack.source, ManifestSource::Builtin(_)));
    }

    #[test]
    fn loads_builtin_filesystem_read_gadget() {
        let root = std::env::temp_dir();
        let pack = load_pack_manifest(&root, DEVELOPER_PACK).unwrap();
        let gadget = load_gadget_manifest(&root, &pack, FILESYSTEM_READ_GADGET).unwrap();
        assert_eq!(gadget.manifest.metadata.name, FILESYSTEM_READ_GADGET);
        assert_eq!(gadget.pack_name, DEVELOPER_PACK);
    }

    #[test]
    fn loads_builtin_test_runner_gadget() {
        let root = std::env::temp_dir();
        let pack = load_pack_manifest(&root, DEVELOPER_PACK).unwrap();
        let gadget = load_gadget_manifest(&root, &pack, TEST_RUNNER_GADGET).unwrap();
        assert_eq!(gadget.manifest.metadata.name, TEST_RUNNER_GADGET);
        assert_eq!(gadget.pack_name, DEVELOPER_PACK);
    }

    #[test]
    fn validates_builtin_developer_pack_strict() {
        let root = std::env::temp_dir();
        let report = validate_pack_tree(&root, DEVELOPER_PACK, true).unwrap();
        assert!(report.is_valid());
        assert_eq!(report.gadgets_checked, 10);
        assert_eq!(report.gadgets_missing, 0);
    }

    #[test]
    fn project_pack_overrides_builtin_pack() {
        let root = unique_temp_root("gadgets-pack-override");
        let pack_dir = root.join(".gadgets/packs/developer");
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(
            pack_dir.join("pack.yaml"),
            r#"schema_version: gadgets.framework/pack/v0.1
kind: GadgetPack
metadata:
  name: developer
  version: 0.1.1
  display_name: Developer Pack Override
  description: Local override.
default_mode: safe
gadgets:
  - filesystem.read
"#,
        )
        .unwrap();

        let pack = load_pack_manifest(&root, DEVELOPER_PACK).unwrap();
        assert_eq!(pack.manifest.metadata.version, "0.1.1");
        assert!(matches!(pack.source, ManifestSource::Project(_)));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_missing_installed_pack() {
        assert!(matches!(
            ensure_pack_installed(&["other".to_string()], DEVELOPER_PACK),
            Err(ManifestLoadError::PackNotInstalled(_))
        ));
    }

    fn unique_temp_root(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }
}
