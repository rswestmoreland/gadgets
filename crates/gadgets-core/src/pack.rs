use serde::{Deserialize, Serialize};

use crate::error::{GadgetCoreError, ValidationError, ValidationReportError};
use crate::validation::{Validate, ValidationReport};

pub const PACK_SCHEMA_VERSION: &str = "gadgets.framework/pack/v0.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackManifest {
    pub schema_version: String,
    pub kind: PackKind,
    pub metadata: PackMetadata,
    #[serde(default)]
    pub default_mode: Option<String>,
    #[serde(default)]
    pub gadgets: Vec<String>,
    #[serde(default)]
    pub requires: PackRequires,
    #[serde(default)]
    pub safety: PackSafety,
}

impl PackManifest {
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

impl Validate for PackManifest {
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::default();

        if self.schema_version != PACK_SCHEMA_VERSION {
            report.push(ValidationError::InvalidSchemaVersion {
                expected: PACK_SCHEMA_VERSION,
                found: self.schema_version.clone(),
            });
        }

        if self.gadgets.is_empty() {
            report.push(ValidationError::EmptyPackGadgets);
        }

        report
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackKind {
    #[serde(rename = "GadgetPack")]
    GadgetPack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackMetadata {
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PackRequires {
    #[serde(default)]
    pub providers: Vec<String>,
    #[serde(default)]
    pub zones: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PackSafety {
    #[serde(default)]
    pub highest_permission_level: Option<String>,
    #[serde(default)]
    pub production_capable: bool,
    #[serde(default)]
    pub destructive_capable: bool,
    #[serde(default)]
    pub requires_approval: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_pack_manifest() {
        let yaml = r#"
schema_version: gadgets.framework/pack/v0.1
kind: GadgetPack
metadata:
  name: developer
  version: 0.1.0
  display_name: Developer Pack
  description: Safe local developer automation.
default_mode: safe
gadgets:
  - coordinator
  - filesystem.read
requires:
  providers:
    - filesystem
  zones:
    - local_repo
"#;
        let pack = PackManifest::from_yaml_str(yaml).unwrap();
        assert_eq!(pack.metadata.name, "developer");
        assert_eq!(pack.gadgets.len(), 2);
    }

    #[test]
    fn rejects_empty_pack() {
        let yaml = r#"
schema_version: gadgets.framework/pack/v0.1
kind: GadgetPack
metadata:
  name: empty
  version: 0.1.0
  display_name: Empty Pack
  description: Empty.
"#;
        let err = PackManifest::from_yaml_str(yaml).unwrap_err();
        assert!(err
            .to_string()
            .contains("pack must contain at least one Gadget"));
    }
}
