use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

use crate::error::ValidationError;
use crate::permission::PermissionLevel;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapabilityName(String);

impl CapabilityName {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        validate_capability_name(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn estimated_level(&self) -> PermissionLevel {
        let parts: Vec<&str> = self.0.split('.').collect();
        let action = parts.last().copied().unwrap_or_default();

        if matches!(
            action,
            "apply"
                | "deploy"
                | "delete"
                | "remove"
                | "reboot"
                | "rotate"
                | "restart"
                | "install"
                | "execute"
        ) || self.0.contains("production")
            || self.0.contains("firewall.apply")
            || self.0.contains("migration.apply")
            || self.0.contains("cleanup.execute")
        {
            return PermissionLevel::Release;
        }

        if matches!(
            action,
            "write" | "patch" | "create" | "update" | "commit" | "run" | "reload" | "up" | "down"
        ) {
            return PermissionLevel::Change;
        }

        if matches!(action, "plan" | "propose" | "review" | "validate") {
            return PermissionLevel::Plan;
        }

        PermissionLevel::Observe
    }

    pub fn is_mutating(&self) -> bool {
        self.estimated_level() >= PermissionLevel::Change
    }

    pub fn is_release_level(&self) -> bool {
        self.estimated_level() == PermissionLevel::Release
    }
}

impl fmt::Display for CapabilityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for CapabilityName {
    type Err = ValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl Serialize for CapabilityName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CapabilityName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

fn validate_capability_name(value: &str) -> Result<(), ValidationError> {
    if value.trim() != value || value.is_empty() {
        return Err(ValidationError::InvalidCapabilityName {
            value: value.to_string(),
            reason: "must be non-empty and must not contain leading or trailing whitespace".into(),
        });
    }

    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() < 2 {
        return Err(ValidationError::InvalidCapabilityName {
            value: value.to_string(),
            reason: "must contain at least two dot-separated parts".into(),
        });
    }

    for part in parts {
        if part.is_empty() {
            return Err(ValidationError::InvalidCapabilityName {
                value: value.to_string(),
                reason: "must not contain empty path segments".into(),
            });
        }

        let mut chars = part.chars();
        let first = chars.next().unwrap();
        if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
            return Err(ValidationError::InvalidCapabilityName {
                value: value.to_string(),
                reason: "segments must start with lowercase ASCII letters or digits".into(),
            });
        }

        if !part
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(ValidationError::InvalidCapabilityName {
                value: value.to_string(),
                reason: "segments may contain only lowercase ASCII letters, digits, underscore, or hyphen".into(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_basic_capability_names() {
        CapabilityName::new("repo.read").unwrap();
        CapabilityName::new("database.migration.apply").unwrap();
        CapabilityName::new("linux.firewall.read").unwrap();
    }

    #[test]
    fn rejects_malformed_capability_names() {
        assert!(CapabilityName::new("Repo.Read").is_err());
        assert!(CapabilityName::new("repo..read").is_err());
        assert!(CapabilityName::new("repo").is_err());
        assert!(CapabilityName::new(" repo.read").is_err());
    }

    #[test]
    fn estimates_risk_level() {
        assert_eq!(
            CapabilityName::new("file.read").unwrap().estimated_level(),
            PermissionLevel::Observe
        );
        assert_eq!(
            CapabilityName::new("file.write").unwrap().estimated_level(),
            PermissionLevel::Change
        );
        assert_eq!(
            CapabilityName::new("linux.firewall.apply")
                .unwrap()
                .estimated_level(),
            PermissionLevel::Release
        );
    }
}
