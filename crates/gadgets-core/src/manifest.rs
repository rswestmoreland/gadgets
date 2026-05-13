use serde::{Deserialize, Serialize};

use crate::capability::CapabilityName;
use crate::error::{GadgetCoreError, ValidationError, ValidationReportError};
use crate::permission::PermissionLevel;
use crate::validation::{Validate, ValidationReport};
use crate::zone::BoundarySet;

pub const GADGET_SCHEMA_VERSION: &str = "gadgets.framework/v0.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GadgetManifest {
    pub schema_version: String,
    pub kind: ManifestKind,
    pub metadata: GadgetMetadata,
    pub runtime: GadgetRuntime,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub capabilities: Vec<CapabilityName>,
    #[serde(default)]
    pub boundaries: BoundarySet,
    #[serde(default)]
    pub tools: ToolBlock,
    #[serde(default)]
    pub handoffs: HandoffBlock,
    #[serde(default)]
    pub evidence: EvidenceRequirements,
    #[serde(default)]
    pub approval: ApprovalRequirements,
}

impl GadgetManifest {
    pub fn from_yaml_str(input: &str) -> Result<Self, GadgetCoreError> {
        let manifest: Self = serde_yaml::from_str(input)?;
        let report = manifest.validate();
        if report.is_ok() {
            Ok(manifest)
        } else {
            Err(GadgetCoreError::Validation(ValidationReportError {
                errors: report.errors,
            }))
        }
    }
}

impl Validate for GadgetManifest {
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::default();

        if self.schema_version != GADGET_SCHEMA_VERSION {
            report.push(ValidationError::InvalidSchemaVersion {
                expected: GADGET_SCHEMA_VERSION,
                found: self.schema_version.clone(),
            });
        }

        if self.capabilities.is_empty() {
            report.push(ValidationError::EmptyCapabilities);
        }

        let has_boundary = self.boundaries.has_explicit_boundary();
        for capability in &self.capabilities {
            if capability.is_mutating() && !has_boundary {
                report.push(ValidationError::MutatingCapabilityWithoutBoundary {
                    capability: capability.to_string(),
                });
            }

            if capability.is_release_level() && self.approval.required_for.is_empty() {
                report.push(ValidationError::ReleaseCapabilityWithoutApproval {
                    capability: capability.to_string(),
                });
            }
        }

        if self.permission_level == PermissionLevel::Release && self.approval.required_for.is_empty() {
            report.push(ValidationError::ReleaseCapabilityWithoutApproval {
                capability: "permission_level.release".into(),
            });
        }

        report
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifestKind {
    #[serde(rename = "Gadget")]
    Gadget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GadgetMetadata {
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GadgetRuntime {
    pub model_profile: String,
    pub execution_mode: PermissionLevel,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub max_tool_calls: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ToolBlock {
    #[serde(default)]
    pub allowed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HandoffBlock {
    #[serde(default)]
    pub allowed_targets: Vec<String>,
    #[serde(default)]
    pub denied_targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EvidenceRequirements {
    #[serde(default)]
    pub required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ApprovalRequirements {
    #[serde(default)]
    pub required_for: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_READ_GADGET: &str = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: filesystem.read
  version: 0.1.0
  display_name: Filesystem Read Gadget
  description: Reads scoped project files.
runtime:
  model_profile: mock_default
  execution_mode: observe
permission_level: observe
capabilities:
  - repo.read
  - file.read
boundaries:
  zones:
    - local_repo
  filesystem:
    roots:
      - "."
    writable: false
    denied_paths:
      - ".git/"
tools:
  allowed:
    - file.read
handoffs:
  allowed_targets:
    - documentation.writer
evidence:
  required:
    - summary
    - files_read
approval:
  required_for: []
"#;

    #[test]
    fn parses_valid_read_manifest() {
        let manifest = GadgetManifest::from_yaml_str(VALID_READ_GADGET).unwrap();
        assert_eq!(manifest.metadata.name, "filesystem.read");
        assert_eq!(manifest.permission_level, PermissionLevel::Observe);
        assert_eq!(manifest.capabilities.len(), 2);
    }

    #[test]
    fn rejects_invalid_permission_level() {
        let yaml = VALID_READ_GADGET.replace("permission_level: observe", "permission_level: god_mode");
        assert!(GadgetManifest::from_yaml_str(&yaml).is_err());
    }

    #[test]
    fn rejects_malformed_capability_name() {
        let yaml = VALID_READ_GADGET.replace("file.read", "File.Read");
        assert!(GadgetManifest::from_yaml_str(&yaml).is_err());
    }

    #[test]
    fn rejects_mutating_capability_without_boundary() {
        let yaml = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: patch.writer
  version: 0.1.0
  display_name: Patch Writer
  description: Writes patches.
runtime:
  model_profile: mock_default
  execution_mode: change
permission_level: change
capabilities:
  - file.write
approval:
  required_for:
    - local_write
"#;
        let err = GadgetManifest::from_yaml_str(yaml).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("requires explicit boundaries"));
    }

    #[test]
    fn rejects_release_capability_without_approval() {
        let yaml = r#"
schema_version: gadgets.framework/v0.1
kind: Gadget
metadata:
  name: firewall.executor
  version: 0.1.0
  display_name: Firewall Executor
  description: Applies firewall rules.
runtime:
  model_profile: mock_default
  execution_mode: release
permission_level: release
capabilities:
  - linux.firewall.apply
boundaries:
  zones:
    - local_host_change
"#;
        let err = GadgetManifest::from_yaml_str(yaml).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("requires approval rules"));
    }
}
