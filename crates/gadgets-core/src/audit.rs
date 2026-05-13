use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub schema_version: String,
    pub timestamp: String,
    pub event_type: String,
    pub actor: AuditActor,
    #[serde(default)]
    pub target: Option<AuditTarget>,
    pub run_id: String,
    pub decision: String,
    pub summary: String,
    #[serde(default)]
    pub previous_event_hash: Option<String>,
    pub event_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditActor {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditTarget {
    pub kind: String,
    pub id: String,
}
