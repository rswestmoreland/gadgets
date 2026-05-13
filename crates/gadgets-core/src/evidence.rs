use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub evidence_bundle_id: String,
    pub schema_version: String,
    pub created_at: String,
    pub gadget: String,
    pub run_id: String,
    pub summary: String,
    pub status: String,
    #[serde(default)]
    pub artifacts: Vec<EvidenceArtifact>,
    #[serde(default)]
    pub denied_actions: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default)]
    pub bundle_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceArtifact {
    pub artifact_type: String,
    pub path: String,
    #[serde(default)]
    pub sha256: Option<String>,
}
