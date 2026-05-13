use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BoundarySet {
    #[serde(default)]
    pub zones: Vec<String>,
    #[serde(default)]
    pub filesystem: Option<FilesystemBoundary>,
}

impl BoundarySet {
    pub fn has_explicit_boundary(&self) -> bool {
        !self.zones.is_empty() || self.filesystem.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemBoundary {
    #[serde(default)]
    pub roots: Vec<String>,
    #[serde(default)]
    pub readable_paths: Vec<String>,
    #[serde(default)]
    pub writable_paths: Vec<String>,
    #[serde(default)]
    pub denied_paths: Vec<String>,
    #[serde(default)]
    pub writable: bool,
}
