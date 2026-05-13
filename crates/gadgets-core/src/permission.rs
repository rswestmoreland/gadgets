use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::ValidationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    Observe,
    Plan,
    Change,
    Release,
}

impl PermissionLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Plan => "plan",
            Self::Change => "change",
            Self::Release => "release",
        }
    }
}

impl fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PermissionLevel {
    type Err = ValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "observe" => Ok(Self::Observe),
            "plan" => Ok(Self::Plan),
            "change" => Ok(Self::Change),
            "release" => Ok(Self::Release),
            other => Err(ValidationError::InvalidPermissionLevel {
                value: other.to_string(),
            }),
        }
    }
}
