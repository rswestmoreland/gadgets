use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum GadgetCoreError {
    ParseYaml(serde_yaml::Error),
    Validation(ValidationReportError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReportError {
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingField {
        field: &'static str,
    },
    InvalidSchemaVersion {
        expected: &'static str,
        found: String,
    },
    InvalidKind {
        expected: &'static str,
        found: String,
    },
    InvalidPermissionLevel {
        value: String,
    },
    InvalidCapabilityName {
        value: String,
        reason: String,
    },
    UnknownCapability {
        value: String,
    },
    MutatingCapabilityWithoutBoundary {
        capability: String,
    },
    ReleaseCapabilityWithoutApproval {
        capability: String,
    },
    EmptyCapabilities,
    EmptyPackGadgets,
    UnknownHandoffTarget {
        target: String,
    },
}

impl fmt::Display for GadgetCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseYaml(err) => write!(f, "failed to parse YAML: {err}"),
            Self::Validation(err) => write!(f, "validation failed: {err}"),
        }
    }
}

impl Error for GadgetCoreError {}

impl From<serde_yaml::Error> for GadgetCoreError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::ParseYaml(value)
    }
}

impl fmt::Display for ValidationReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errors.is_empty() {
            return write!(f, "no validation errors");
        }

        for (idx, err) in self.errors.iter().enumerate() {
            if idx > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{err}")?;
        }
        Ok(())
    }
}

impl Error for ValidationReportError {}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField { field } => write!(f, "missing required field {field}"),
            Self::InvalidSchemaVersion { expected, found } => {
                write!(f, "invalid schema_version {found:?}, expected {expected:?}")
            }
            Self::InvalidKind { expected, found } => {
                write!(f, "invalid kind {found:?}, expected {expected:?}")
            }
            Self::InvalidPermissionLevel { value } => {
                write!(f, "invalid permission level {value:?}")
            }
            Self::InvalidCapabilityName { value, reason } => {
                write!(f, "invalid capability name {value:?}: {reason}")
            }
            Self::UnknownCapability { value } => write!(f, "unknown capability {value:?}"),
            Self::MutatingCapabilityWithoutBoundary { capability } => {
                write!(
                    f,
                    "mutating capability {capability:?} requires explicit boundaries"
                )
            }
            Self::ReleaseCapabilityWithoutApproval { capability } => {
                write!(
                    f,
                    "release capability {capability:?} requires approval rules"
                )
            }
            Self::EmptyCapabilities => write!(f, "manifest must declare at least one capability"),
            Self::EmptyPackGadgets => write!(f, "pack must contain at least one Gadget"),
            Self::UnknownHandoffTarget { target } => write!(f, "unknown handoff target {target:?}"),
        }
    }
}
